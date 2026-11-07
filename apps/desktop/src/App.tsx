import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import UpdateNotice from "./UpdateNotice";
import { Activity, Cpu, HardDrive, ShieldCheck, Sparkles, Zap, LockKeyhole, Send, Monitor, CheckCircle2, CircleHelp, BrainCircuit } from "lucide-react";

type TaskProgressEvent = { task_id: string; stage: string; message: string; progress_percent: number };
type DeviceInfo = { os: string; architecture: string; logical_cpus: number; total_memory_mb: number; available_memory_mb: number; system_locale: string };
type CapabilityRecord = { id: string; category: string; state: "available" | "unavailable" | "unknown"; source: string; detail: string };
type OperationDescriptor = { id: string; description: string; required_capabilities: string[]; minimum_available_memory_mb: number };
type CapabilitySnapshot = { capabilities: CapabilityRecord[]; operations: OperationDescriptor[]; available_memory_mb: number };
type ToolManifest = { id: string; version: string; name: string; description: string; category: string; execution: string; network: string; permissions: string[]; budget: { max_input_bytes: number; max_output_bytes: number; max_memory_mb: number; max_runtime_ms: number }; adapter_available: boolean };
type WorkflowNode = { id: string; operation_id: string; depends_on: string[] };
type WorkflowPlan = { accepted: boolean; schema_version: number; ordered_node_ids: string[]; required_capabilities: string[]; estimated_peak_memory_mb: number; available_memory_mb: number; reason: string; execution_started: boolean };
type ModelRecommendation = { recommended_model: string | null; available_memory_mb: number; usable_memory_budget_mb: number; candidates: { name: string; size_bytes: number; estimated_working_memory_mb: number | null; estimated_to_fit: boolean; reason: string }[]; explanation: string };
type WorkflowExecution = { plan: WorkflowPlan; result: DocumentPipelineResult };
type TaskPlan = { operation_id: string; accepted: boolean; execution_location: string; required_capabilities: string[]; missing_capabilities: string[]; minimum_available_memory_mb: number; available_memory_mb: number; reason: string; execution_started: boolean };
type RunResult = { operation: string; output: string; execution: string };
type ModelInfo = { id: string; name: string; provider: string; available: boolean; is_mock: boolean; required_capabilities: string[]; status: string };
type InferenceResult = { model_id: string; model_name: string; output: string; execution: string; is_mock: boolean };
type RouteDecision = { mode: string; action: string; allowed: boolean; external_data_transfer_permitted: boolean; collaboration_rounds: number; reason: string; enforcement_boundary_required: boolean };
type OllamaStatus = { available: boolean; models: { name: string; size: number }[] };
type AiTaskResult = { mode_used: string; execution_location: string; model: string; output: string };
type DocumentPipelineResult = { analysis: FileAnalysis; summary: string; translated_summary: string | null; translation_language: string | null; source_text_bytes: number; chunks_processed: number; total_chunks: number; execution_location: string; model: string };
type FileAnalysis = { file_name: string; file_type: string; size_bytes: number; content_preview: string; word_count: number; line_count: number; top_keywords: string[] };
type ImageInfo = { file_name: string; format: string; size_bytes: number; dimensions: [number, number] | null; description: string };
type HistoryEntry = { id: number; prompt: string; output: string; mode: string; model: string; kind: string; created_at: number };
type HistorySettings = { enabled: boolean; retention_days: number; max_entries: number };
type ExecutionReceipt = { schema_version: number; task_id: string; operation_id: string; location: string; adapter_id: string; adapter_version: string; model_id: string | null; model_version: string | null; input_sha256: string; output_sha256: string; started_at_unix_ms: number; completed_at_unix_ms: number; user_approval_id: string | null };
type ReceiptLedgerStatus = { entries: number; valid: boolean; head_sha256: string };
type ReceiptLedgerEntry = { receipt: ExecutionReceipt; record_hash: string };
type ChatMessage = { role: "user" | "assistant"; content: string };
function composeConversationPrompt(messages: ChatMessage[], current: string): string {
  const currentText = current.trim();
  const context = messages.slice(-8).map((message) => `${message.role === "user" ? "User" : "Assistant"}: ${message.content.slice(0, 2400)}`).join("\n\n");
  const contextBudget = Math.max(0, 14000 - currentText.length - 40);
  const clippedContext = contextBudget > 0 ? context.slice(-contextBudget) : "";
  return clippedContext ? `Conversation context (recent messages):\n${clippedContext}\n\nUser: ${currentText}\nAssistant:` : currentText;
}
type FolderFileResult = { file_name: string; size_bytes: number; word_count: number; line_count: number; top_keywords: string[]; summary: string | null; note: string };
type FolderImportResult = { cancelled: boolean; files: FolderFileResult[]; total_bytes: number };
const CLOUD_PROVIDER_PRESETS = [
  { id: "openai", label: "OpenAI", endpoint: "https://api.openai.com/v1/chat/completions", model: "gpt-4o-mini" },
  { id: "openrouter", label: "OpenRouter (check for free models)", endpoint: "https://openrouter.ai/api/v1/chat/completions", model: "" },
  { id: "groq", label: "Groq (check current free limits)", endpoint: "https://api.groq.com/openai/v1/chat/completions", model: "" },
  { id: "together", label: "Together AI", endpoint: "https://api.together.xyz/v1/chat/completions", model: "" },
  { id: "custom", label: "Custom OpenAI-compatible API", endpoint: "", model: "" }
] as const;
const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export default function App() {
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [deviceError, setDeviceError] = useState("");
  const [deviceBusy, setDeviceBusy] = useState(false);
  const [request, setRequest] = useState("");
  const [localChatMessages, setLocalChatMessages] = useState<ChatMessage[]>([]);
  const [cloudChatMessages, setCloudChatMessages] = useState<ChatMessage[]>([]);
  const [result, setResult] = useState<RunResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [policy, setPolicy] = useState("local");
  const [history, setHistory] = useState<string[]>([]);
  const [persistentHistory, setPersistentHistory] = useState<HistoryEntry[]>([]);
  const [historySettings, setHistorySettings] = useState<HistorySettings | null>(null);
  const [historyError, setHistoryError] = useState("");
  const [historyBusy, setHistoryBusy] = useState(false);
  const [receiptLedgerStatus, setReceiptLedgerStatus] = useState<ReceiptLedgerStatus | null>(null);
  const [executionReceipts, setExecutionReceipts] = useState<ReceiptLedgerEntry[]>([]);
  const [receiptBusy, setReceiptBusy] = useState(false);
  const [receiptError, setReceiptError] = useState("");
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [selectedModel, setSelectedModel] = useState("auto");
  const [modelBusy, setModelBusy] = useState(false);
  const [modelResult, setModelResult] = useState<InferenceResult | null>(null);
  const [modelError, setModelError] = useState("");
  const [executionMode, setExecutionMode] = useState("smart");
  const [externalApproved, setExternalApproved] = useState(false);
  const [routeDecision, setRouteDecision] = useState<RouteDecision | null>(null);
  const [routeBusy, setRouteBusy] = useState(false);
  const [routeError, setRouteError] = useState("");
  const [ollamaStatus, setOllamaStatus] = useState<OllamaStatus | null>(null);
  const [ollamaError, setOllamaError] = useState("");
  const [ollamaBusy, setOllamaBusy] = useState(false);
  const [ollamaModel, setOllamaModel] = useState("");
  const [modelPullName, setModelPullName] = useState("llama3.2");
  const [modelPullBusy, setModelPullBusy] = useState(false);
  const [activeModelPullTaskId, setActiveModelPullTaskId] = useState<string | null>(null);
  const [modelPullError, setModelPullError] = useState("");
  const [cloudProviderPreset, setCloudProviderPreset] = useState("openai");
  const [aiBusy, setAiBusy] = useState(false);
  const [activeTaskId, setActiveTaskId] = useState<string | null>(null);
  const [taskProgress, setTaskProgress] = useState<TaskProgressEvent | null>(null);
  const [aiResult, setAiResult] = useState<AiTaskResult | null>(null);
  const [fileAnalysis, setFileAnalysis] = useState<FileAnalysis | null>(null);
  const [fileText, setFileText] = useState("");
  const [fileSummary, setFileSummary] = useState("");
  const [filePipelineMeta, setFilePipelineMeta] = useState("");
  const [fileTranslatedSummary, setFileTranslatedSummary] = useState("");
  const [translateSummary, setTranslateSummary] = useState(false);
  const [summaryBusy, setSummaryBusy] = useState(false);
  const [activeDocumentTaskId, setActiveDocumentTaskId] = useState<string | null>(null);
  const [imageInfo, setImageInfo] = useState<ImageInfo | null>(null);
  const [fileError, setFileError] = useState("");
  const [folderResults, setFolderResults] = useState<FolderFileResult[]>([]);
  const [folderBusy, setFolderBusy] = useState(false);
  const [folderError, setFolderError] = useState("");
  const [translationText, setTranslationText] = useState("");
  const [targetLanguage, setTargetLanguage] = useState("English");
  const [translationResult, setTranslationResult] = useState("");
  const [translationBusy, setTranslationBusy] = useState(false);
  const [activeTranslationTaskId, setActiveTranslationTaskId] = useState<string | null>(null);
  const [capabilitySnapshot, setCapabilitySnapshot] = useState<CapabilitySnapshot | null>(null);
  const [capabilityBusy, setCapabilityBusy] = useState(false);
  const [capabilityError, setCapabilityError] = useState("");
  const [toolCatalogue, setToolCatalogue] = useState<ToolManifest[] | null>(null);
  const [toolCatalogueBusy, setToolCatalogueBusy] = useState(false);
  const [toolCatalogueError, setToolCatalogueError] = useState("");
  const [plannedOperation, setPlannedOperation] = useState("local.ai.generate");
  const [taskPlan, setTaskPlan] = useState<TaskPlan | null>(null);
  const [workflowPlan, setWorkflowPlan] = useState<WorkflowPlan | null>(null);
  const [workflowBusy, setWorkflowBusy] = useState(false);
  const [workflowExecution, setWorkflowExecution] = useState<WorkflowExecution | null>(null);
  const [modelRecommendation, setModelRecommendation] = useState<ModelRecommendation | null>(null);
  const [recommendationError, setRecommendationError] = useState("");
  const [recommendationBusy, setRecommendationBusy] = useState(false);
  const [cloudEndpoint, setCloudEndpoint] = useState("https://api.openai.com/v1/chat/completions");
  const [cloudModel, setCloudModel] = useState("gpt-4o-mini");
  const [cloudTokenParameter, setCloudTokenParameter] = useState("auto");
  const [cloudApiKey, setCloudApiKey] = useState("");
  const [cloudConsent, setCloudConsent] = useState(false);
  const [cloudConsentSnapshot, setCloudConsentSnapshot] = useState("");
  const cloudConversationPrompt = composeConversationPrompt(cloudChatMessages, request);
  const cloudApprovalSnapshot = JSON.stringify([cloudEndpoint.trim(), cloudModel.trim(), cloudTokenParameter, cloudConversationPrompt, executionMode]);
  const [cloudBusy, setCloudBusy] = useState(false);
  const [activeCloudTaskId, setActiveCloudTaskId] = useState<string | null>(null);
  const [cloudResult, setCloudResult] = useState("");
  const [cloudError, setCloudError] = useState("");

  useEffect(() => {
    // Deliberately do not invoke native services during first render. The launcher
    // must be interactive even if Ollama, the OS inventory, history or update service
    // is slow, unavailable, or still starting.
    if (!isTauri()) {
      setDeviceError("Browser preview: launch the Tauri desktop app to use native features.");
      setModels([{ id: "ujani.mock.echo.v1", name: "UJANI Mock Echo (test adapter)", provider: "Built-in deterministic mock", available: true, is_mock: true, required_capabilities: ["cpu"], status: "Browser preview only — mock" }]);
    }
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | undefined;
    listen<TaskProgressEvent>("ujani-task-progress", (event) => setTaskProgress(event.payload))
      .then((stop) => { unlisten = stop; })
      .catch((error) => setOllamaError(String(error)));
    return () => { unlisten?.(); };
  }, []);

  async function loadDeviceInfo() {
    if (!isTauri() || device || deviceBusy) return;
    setDeviceBusy(true);
    setDeviceError("");
    try {
      setDevice(await invoke<DeviceInfo>("get_device_info"));
    } catch (error) {
      setDeviceError("Device information unavailable: " + String(error));
    } finally {
      setDeviceBusy(false);
    }
  }

  function openLocalChat() {
    document.getElementById("local-ai-workspace")?.scrollIntoView({ behavior: "smooth", block: "start" });
    if (!ollamaStatus && !ollamaBusy) void refreshOllamaStatus();
  }

  function openFileTools() {
    document.getElementById("document-workspace")?.scrollIntoView({ behavior: "smooth", block: "start" });
    // File inspection works without Ollama; check local AI only when the user opens file tools.
    if (!ollamaStatus && !ollamaBusy) void refreshOllamaStatus();
  }

  function openLocalSetup() {
    document.getElementById("local-resources")?.scrollIntoView({ behavior: "smooth", block: "start" });
    void loadDeviceInfo();
    if (!ollamaStatus && !ollamaBusy) void refreshOllamaStatus();
  }

  async function refreshPersistentHistory() {
    if (!isTauri()) return;
    setHistoryBusy(true); setHistoryError("");
    try {
      const [settings, entries] = await Promise.all([
        invoke<HistorySettings>("get_task_history_settings"),
        invoke<HistoryEntry[]>("list_task_history")
      ]);
      setHistorySettings(settings); setPersistentHistory(entries);
    } catch (error) { setHistoryError("Local history unavailable: " + String(error)); }
    finally { setHistoryBusy(false); }
  }

  async function refreshExecutionReceipts() {
    if (!isTauri() || receiptBusy) return;
    setReceiptBusy(true); setReceiptError("");
    try {
      const [status, entries] = await Promise.all([
        invoke<ReceiptLedgerStatus>("verify_execution_receipt_ledger"),
        invoke<ReceiptLedgerEntry[]>("list_execution_receipts")
      ]);
      setReceiptLedgerStatus(status);
      setExecutionReceipts(entries);
      if (!status.valid) setReceiptError("Receipt integrity verification failed. Entries are hidden; investigate the local security store.");
    } catch (error) {
      setReceiptError("Privacy receipts unavailable: " + String(error));
      setExecutionReceipts([]);
    } finally { setReceiptBusy(false); }
  }

  async function togglePersistentHistory(enabled: boolean) {
    if (!isTauri() || historyBusy) return;
    setHistoryBusy(true); setHistoryError("");
    try {
      setHistorySettings(await invoke<HistorySettings>("set_task_history_enabled", { enabledValue: enabled }));
      setPersistentHistory(await invoke<HistoryEntry[]>("list_task_history"));
    } catch (error) { setHistoryError(String(error)); }
    finally { setHistoryBusy(false); }
  }

  async function saveToPersistentHistory(prompt: string, output: string, kind: string, model: string) {
    if (!isTauri() || !historySettings?.enabled) return;
    try {
      await invoke<number>("save_task_history", { prompt, output, mode: executionMode, model, kind });
      setPersistentHistory(await invoke<HistoryEntry[]>("list_task_history"));
    } catch (error) { setHistoryError("Could not save this task: " + String(error)); }
  }

  async function deletePersistentHistory(id: number) {
    try { await invoke("delete_task_history", { entryId: id }); setPersistentHistory(await invoke<HistoryEntry[]>("list_task_history")); }
    catch (error) { setHistoryError(String(error)); }
  }

  async function clearPersistentHistory() {
    if (!window.confirm("Delete all saved UJANI task history? This does not securely erase storage media.")) return;
    try { await invoke("clear_task_history"); setPersistentHistory([]); }
    catch (error) { setHistoryError(String(error)); }
  }

  async function refreshCapabilities() {
    if (!isTauri() || capabilityBusy) return;
    setCapabilityBusy(true); setCapabilityError(""); setTaskPlan(null);
    try {
      setCapabilitySnapshot(await invoke<CapabilitySnapshot>("get_capability_snapshot"));
    } catch (error) { setCapabilityError(String(error)); }
    finally { setCapabilityBusy(false); }
  }

  async function refreshToolCatalogue() {
    if (!isTauri() || toolCatalogueBusy) return;
    setToolCatalogueBusy(true); setToolCatalogueError("");
    try { setToolCatalogue(await invoke<ToolManifest[]>("get_tool_catalogue")); }
    catch (error) { setToolCatalogueError(String(error)); }
    finally { setToolCatalogueBusy(false); }
  }

  async function previewLocalPlan() {
    if (!isTauri() || capabilityBusy) return;
    setCapabilityBusy(true); setCapabilityError(""); setTaskPlan(null);
    try {
      setTaskPlan(await invoke<TaskPlan>("plan_local_operation", {
        operationId: plannedOperation, mode: executionMode
      }));
    } catch (error) { setCapabilityError(String(error)); }
    finally { setCapabilityBusy(false); }
  }


  async function previewDocumentWorkflow() {
    if (!isTauri() || workflowBusy) return;
    setWorkflowBusy(true); setWorkflowPlan(null); setCapabilityError("");
    const nodes: WorkflowNode[] = [
      { id: "analyse", operation_id: "file.analyze_text", depends_on: [] },
      { id: "summarise", operation_id: "document.summarize", depends_on: ["analyse"] },
      { id: "translate", operation_id: "translate.text", depends_on: ["summarise"] }
    ];
    try {
      setWorkflowPlan(await invoke<WorkflowPlan>("plan_task_workflow", { nodes, mode: executionMode }));
    } catch (error) { setCapabilityError(String(error)); }
    finally { setWorkflowBusy(false); }
  }

  async function pullLocalModel() {
    if (!isTauri() || !ollamaStatus?.available || modelPullBusy) {
      if (!ollamaStatus?.available) setModelPullError("Start Ollama before downloading a model. Downloads run only through the local Ollama service.");
      return;
    }
    const model = modelPullName.trim();
    if (!model || model.length > 128 || !/^[A-Za-z0-9][A-Za-z0-9._:/-]*$/.test(model) || model.includes("..")) {
      setModelPullError("Enter a valid Ollama model name, for example llama3.2 or qwen2.5:3b.");
      return;
    }
    const taskId = "model-pull-" + Date.now().toString(36);
    setActiveModelPullTaskId(taskId);
    setTaskProgress({ task_id: taskId, stage: "pull_starting", message: "Preparing local model download…", progress_percent: 0 });
    setModelPullBusy(true);
    setModelPullError("");
    try {
      await invoke("pull_ollama_model", { taskId, model, mode: executionMode });
      await refreshOllamaStatus();
      setOllamaModel(model);
    } catch (error) {
      setModelPullError(String(error));
    } finally {
      setModelPullBusy(false);
      setActiveModelPullTaskId(null);
    }
  }

  async function cancelModelPull() {
    if (!activeModelPullTaskId) return;
    try { await invoke<boolean>("cancel_active_task", { taskId: activeModelPullTaskId }); }
    catch (error) { setModelPullError("Could not cancel model download: " + String(error)); }
  }

  async function refreshOllamaStatus() {
    if (!isTauri() || ollamaBusy) return;
    setOllamaBusy(true);
    setOllamaError("");
    try {
      const status = await invoke<OllamaStatus>("get_ollama_status");
      setOllamaStatus(status);
      setOllamaModel((current) => status.models.some((model) => model.name === current)
        ? current
        : (status.models[0]?.name ?? ""));
      if (!status.available) {
        setOllamaError("Ollama is not responding at 127.0.0.1:11434. Start the local Ollama application and retry.");
      } else if (status.models.length === 0) {
        setOllamaError("Ollama is running, but no models are installed. Open a terminal and run: ollama pull llama3.2");
      }
    } catch (error) {
      setOllamaError(`Could not refresh local Ollama status: ${String(error)}`);
    } finally {
      setOllamaBusy(false);
    }
  }

  async function refreshModelRecommendation() {
    if (!isTauri() || recommendationBusy) return;
    setRecommendationBusy(true); setRecommendationError("");
    try { setModelRecommendation(await invoke<ModelRecommendation>("get_model_recommendation")); }
    catch (error) { setRecommendationError(String(error)); }
    finally { setRecommendationBusy(false); }
  }

  async function executeValidatedDocumentWorkflow() {
    if (!isTauri() || workflowBusy || !fileText.trim() || !ollamaModel) return;
    if (executionMode !== "fortress" && executionMode !== "smart") { setCapabilityError("This executable workflow is local-only. Select Fortress or Smart."); return; }
    setWorkflowBusy(true); setCapabilityError(""); setWorkflowExecution(null);
    try {
      const execution = await invoke<WorkflowExecution>("execute_document_workflow", {
        fileName: fileAnalysis?.file_name || "selected-document.txt",
        content: fileText, mode: executionMode, model: ollamaModel,
        translationLanguage: translateSummary ? targetLanguage : null
      });
      setWorkflowExecution(execution);
      await saveToPersistentHistory("Document workflow: " + (fileAnalysis?.file_name || "selected document"), execution.result.summary, "document", execution.result.model);
      setFileSummary(execution.result.summary);
      setFileTranslatedSummary(execution.result.translated_summary || "");
      setFilePipelineMeta(`${execution.result.chunks_processed}/${execution.result.total_chunks} sections · validated order: ${execution.plan.ordered_node_ids.join(" → ")}`);
    } catch (error) { setCapabilityError(String(error)); }
    finally { setWorkflowBusy(false); }
  }

  async function previewRoute() {
    if (!request.trim() || routeBusy) return;
    setRouteBusy(true); setRouteError(""); setRouteDecision(null);
    try {
      const args = {
        taskId: "desktop-task-preview",
        operation: "local.ai.generate",
        mode: executionMode,
        requiredCapabilities: ["cpu"],
        availableCapabilities: device?.logical_cpus ? ["cpu"] : [],
        localMemoryRequiredMb: 128,
        availableMemoryMb: device?.available_memory_mb ?? 0,
        externalDataApproved: externalApproved,
        requestedCollaborationRounds: 2
      };
      if (isTauri()) {
        setRouteDecision(await invoke<RouteDecision>("evaluate_task_route", args));
      } else {
        const local = !!device;
        setRouteDecision({
          mode: executionMode, action: "blocked",
          allowed: false, external_data_transfer_permitted: false, collaboration_rounds: 0,
          reason: "Browser preview cannot evaluate native capabilities. Open the desktop runtime for policy evaluation.",
          enforcement_boundary_required: true
        });
      }
    } catch (error) { setRouteError(String(error)); }
    finally { setRouteBusy(false); }
  }

  async function runLocalAi() {
    if (!request.trim() || aiBusy) return;
    if (executionMode !== "fortress" && executionMode !== "smart") {
      setOllamaError("Real local inference is available in Fortress and Smart only. Worker and Partner peer execution is not implemented in this build; use Smart for optional BYOK cloud processing.");
      return;
    }
    if (!ollamaModel) { setOllamaError("Install and start Ollama, then pull a model before running local AI."); return; }
    const chatPrompt = composeConversationPrompt(localChatMessages, request);
    const userMessage = request.trim();
    const taskId = "local-ai-" + Date.now().toString(36);
    setActiveTaskId(taskId);
    setTaskProgress({ task_id: taskId, stage: "starting", message: "Starting local task…", progress_percent: 5 });
    setAiBusy(true); setOllamaError(""); setAiResult(null);
    try {
      const result = await invoke<AiTaskResult>("execute_local_ai_task", { taskId, prompt: chatPrompt, mode: executionMode, model: ollamaModel });
      setAiResult(result);
      setLocalChatMessages((messages) => [...messages, { role: "user", content: userMessage }, { role: "assistant", content: result.output }]);
      await saveToPersistentHistory(userMessage, result.output, "chat", result.model);
      setRequest("");
    } catch (error) { setOllamaError(String(error)); }
    finally { setAiBusy(false); setActiveTaskId(null); }
  }

  async function cancelLocalAi() {
    if (!activeTaskId) return;
    try { await invoke<boolean>("cancel_active_task", { taskId: activeTaskId }); }
    catch (error) { setOllamaError("Cancellation failed: " + String(error)); }
  }

  async function handleTextFile(event: import("react").ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    setFileError(""); setFileAnalysis(null); setFileText(""); setFileSummary(""); setFilePipelineMeta(""); setFileTranslatedSummary("");
    if (file.size > 1024 * 1024) { setFileError("Text files are limited to 1 MiB."); return; }
    if (!isTauri()) { setFileError("Launch the Tauri desktop app to process files."); return; }
    try { const content = await file.text(); setFileText(content); setFileAnalysis(await invoke<FileAnalysis>("analyze_text_file", { fileName: file.name, content })); }
    catch (error) { setFileError(String(error)); }
  }

  // The native Rust command opens the OS folder dialog; the webview never supplies a path.
  async function handleFolderSelection() {
    setFolderResults([]);
    setFolderError("");
    if (!isTauri()) { setFolderError("Launch the Tauri desktop app to select a folder."); return; }
    if (executionMode !== "fortress" && executionMode !== "smart") {
      setFolderError("Folder processing is local-only. Select Fortress or Smart.");
      return;
    }
    setFolderBusy(true);
    try {
      const result = await invoke<FolderImportResult>("process_selected_folder", {
        mode: executionMode,
        model: ollamaStatus?.available ? ollamaModel : ""
      });
      if (result.cancelled) return;
      setFolderResults(result.files);
      for (const item of result.files) {
        if (item.summary) {
          await saveToPersistentHistory("Folder document: " + item.file_name, item.summary, "document", ollamaModel || "analysis-only");
        }
      }
      if (result.files.length === 0) {
        setFolderError("No supported text files were found in the selected folder.");
      }
    } catch (error) {
      setFolderError(String(error));
    } finally {
      setFolderBusy(false);
    }
  }

  async function summarizeFile() {
    if (!fileText || !fileAnalysis || summaryBusy) return;
    if (new TextEncoder().encode(fileText).length > 256 * 1024) {
      setFileError("The integrated AI pipeline is limited to 256 KiB per document. Basic text analysis remains available up to 1 MiB.");
      return;
    }
    if (!ollamaModel || !ollamaStatus?.available) { setFileError("Start local Ollama and select an installed model to run the document pipeline."); return; }
    if (executionMode !== "fortress" && executionMode !== "smart") { setFileError("The local document pipeline is available in Fortress and Smart only."); return; }
    const taskId = "document-" + Date.now().toString(36);
    setActiveDocumentTaskId(taskId);
    setSummaryBusy(true); setFileError(""); setFileSummary(""); setFilePipelineMeta("");
    try {
      const result = await invoke<DocumentPipelineResult>("process_selected_document", {
        taskId, fileName: fileAnalysis.file_name, content: fileText, mode: executionMode, model: ollamaModel,
        translationLanguage: translateSummary ? targetLanguage : null
      });
      setFileAnalysis(result.analysis);
      setFileSummary(result.summary);
      setFilePipelineMeta(`Processed ${result.chunks_processed} of ${result.total_chunks} sections · ${result.source_text_bytes.toLocaleString()} source bytes · ${result.execution_location}`);
      setFileTranslatedSummary(result.translated_summary ?? "");
      await saveToPersistentHistory("Document: " + fileAnalysis.file_name, result.summary, "document", result.model);
    } catch (error) { setFileError(String(error)); }
    finally { setSummaryBusy(false); setActiveDocumentTaskId(null); }
  }

  async function handlePdfFile(event: import("react").ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    setFileError(""); setFileAnalysis(null); setFileSummary(""); setFilePipelineMeta(""); setFileTranslatedSummary("");
    if (file.size > 10 * 1024 * 1024) { setFileError("PDF files are limited to 10 MiB."); return; }
    if (!isTauri()) { setFileError("Launch the Tauri desktop app to extract PDF text."); return; }
    if (!ollamaStatus?.available || !ollamaModel) { setFileError("Start local Ollama and select a model before processing a PDF."); return; }
    if (executionMode !== "fortress" && executionMode !== "smart") { setFileError("PDF processing is available in Fortress and Smart only."); return; }
    const taskId = "pdf-" + Date.now().toString(36);
    setActiveDocumentTaskId(taskId);
    setSummaryBusy(true);
    try {
      const data = Array.from(new Uint8Array(await file.arrayBuffer()));
      const result = await invoke<DocumentPipelineResult>("process_selected_pdf", {
        taskId, fileName: file.name, data, mode: executionMode, model: ollamaModel,
        translationLanguage: translateSummary ? targetLanguage : null
      });
      setFileAnalysis(result.analysis);
      setFileSummary(result.summary);
      setFilePipelineMeta(`Processed ${result.chunks_processed} of ${result.total_chunks} sections · ${result.source_text_bytes.toLocaleString()} extracted bytes · ${result.execution_location}`);
      setFileTranslatedSummary(result.translated_summary ?? "");
      await saveToPersistentHistory("PDF: " + file.name, result.summary, "pdf", result.model);
      setFileText("");
    } catch (error) { setFileError(String(error)); }
    finally { setSummaryBusy(false); setActiveDocumentTaskId(null); event.target.value = ""; }
  }

  async function cancelDocumentTask() {
    if (!activeDocumentTaskId) return;
    try { await invoke<boolean>("cancel_active_task", { taskId: activeDocumentTaskId }); }
    catch (error) { setFileError("Could not cancel document task: " + String(error)); }
  }

  async function cancelTranslationTask() {
    if (!activeTranslationTaskId) return;
    try { await invoke<boolean>("cancel_active_task", { taskId: activeTranslationTaskId }); }
    catch (error) { setOllamaError("Could not cancel translation: " + String(error)); }
  }

  async function translateLocally() {
    if (!translationText.trim() || translationBusy) return;
    if (!isTauri() || !ollamaStatus?.available || !ollamaModel) { setOllamaError("Local translation requires the desktop app and a running Ollama model."); return; }
    if (executionMode !== "fortress" && executionMode !== "smart") { setOllamaError("Local translation is available in Fortress and Smart only."); return; }
    const taskId = "translation-" + Date.now().toString(36);
    setActiveTranslationTaskId(taskId);
    setTranslationBusy(true); setTranslationResult(""); setOllamaError("");
    try {
      const translated = await invoke<string>("translate_text_local", {
        taskId, text: translationText, targetLanguage, mode: executionMode, model: ollamaModel
      });
      setTranslationResult(translated);
      await saveToPersistentHistory("Translate to " + targetLanguage + ": " + translationText.slice(0, 200), translated, "translation", ollamaModel);
    } catch (error) { setOllamaError(String(error)); }
    finally { setTranslationBusy(false); setActiveTranslationTaskId(null); }
  }

  async function handleImageFile(event: import("react").ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    setFileError(""); setImageInfo(null);
    if (file.size > 20 * 1024 * 1024) { setFileError("Images are limited to 20 MiB."); return; }
    if (!isTauri()) { setFileError("Launch the Tauri desktop app to inspect images."); return; }
    try {
      const data = Array.from(new Uint8Array(await file.slice(0, 64 * 1024).arrayBuffer()));
      setImageInfo(await invoke<ImageInfo>("inspect_image_file", { fileName: file.name, data, fileSizeBytes: file.size }));
    } catch (error) { setFileError(String(error)); }
  }

  async function runTask() {
    const text = request.trim();
    if (!text || busy) return;
    setBusy(true);
    setResult(null);
    try {
      const output: RunResult = isTauri()
        ? await invoke<RunResult>("run_local_task", { request: text })
        : { operation: "local.word_count", output: `Local preview completed. Your request contains ${text.split(/\s+/).length} words. This is a demo tool, not an AI model.`, execution: "Browser demo (not a native device runtime)" };
      setResult(output);
      setHistory((items) => [text, ...items].slice(0, 5));
    } catch (error) {
      setResult({ operation: "local.task", output: `Task failed safely: ${String(error)}`, execution: "Local" });
    } finally {
      setBusy(false);
    }
  }

  async function runModel() {
    const text = request.trim();
    if (!text || modelBusy) return;
    setModelBusy(true);
    setModelResult(null);
    setModelError("");
    try {
      if (!isTauri()) {
        setModelResult({ model_id: "ujani.mock.echo.v1", model_name: "UJANI Mock Echo (test adapter)", output: `MOCK RESPONSE — no model inference occurred. Browser preview received ${text.length} characters locally: ${text}`, execution: "Browser deterministic mock", is_mock: true });
      } else {
        const modelId = selectedModel === "auto" ? "auto" : selectedModel;
        setModelResult(await invoke<InferenceResult>("run_local_model_task", { modelId, prompt: text }));
      }
    } catch (error) {
      setModelError(`Local model task failed: ${String(error)}`);
    } finally {
      setModelBusy(false);
    }
  }

  async function runCloudAi() {
    if (!isTauri() || !request.trim() || cloudBusy) return;
    if (executionMode !== "smart") {
      setCloudError("The BYOK cloud gateway is available in Smart mode only. Worker and Partner peer execution are not implemented in this build; Fortress remains local-only.");
      return;
    }
    if (!cloudConsent || cloudConsentSnapshot !== cloudApprovalSnapshot) {
      setCloudConsent(false); setCloudConsentSnapshot("");
      setCloudError("The prompt, provider endpoint, model, or mode changed after approval. Review the current task and approve it again.");
      return;
    }
    if (!cloudApiKey.trim()) { setCloudError("Enter your own provider API key. UJANI does not supply keys or credits."); return; }
    const userMessage = request.trim();
    const taskId = "cloud-ai-" + Date.now().toString(36);
    setActiveCloudTaskId(taskId);
    setTaskProgress({ task_id: taskId, stage: "cloud_starting", message: "Preparing approved cloud request…", progress_percent: 5 });
    setCloudBusy(true); setCloudError(""); setCloudResult("");
    try {
      const response = await invoke<string>("execute_cloud_ai_task", { taskId, endpoint: cloudEndpoint, model: cloudModel, apiKey: cloudApiKey, prompt: cloudConversationPrompt, mode: executionMode, externalDataApproved: cloudConsent, tokenParameter: cloudTokenParameter });
      setCloudResult(response);
      setCloudChatMessages((messages) => [...messages, { role: "user", content: userMessage }, { role: "assistant", content: response }]);
      setRequest("");
    } catch (error) { setCloudError(String(error)); }
    finally { setCloudBusy(false); setActiveCloudTaskId(null); setCloudApiKey(""); setCloudConsent(false); setCloudConsentSnapshot(""); }
  }

  async function cancelCloudAi() {
    if (!activeCloudTaskId) return;
    try { await invoke<boolean>("cancel_active_task", { taskId: activeCloudTaskId }); }
    catch (error) { setCloudError("Cloud cancellation failed: " + String(error)); }
  }

  const memory = device ? `${(device.total_memory_mb / 1024).toFixed(1)} GB` : "—";
  const selected = models.find((model) => model.id === selectedModel);
  const platformInstallHint = device?.os?.toLowerCase().includes("windows")
    ? "Windows: download and run the official OllamaSetup.exe installer."
    : device?.os?.toLowerCase().includes("darwin") || device?.os?.toLowerCase().includes("mac")
      ? "macOS: download the official Ollama app and move it to Applications."
      : device?.os?.toLowerCase().includes("linux")
        ? "Linux: follow the official Linux installation instructions on the Ollama download page."
        : "Choose your operating system on the official Ollama download page.";
  return <main className="shell">
    <UpdateNotice />
    <aside className="sidebar">
      <div className="brand"><div className="brand-mark">U</div><div><b>UJANI</b><small>PUBLIC AI</small></div></div>
      <div className="nav-label">WORKSPACE</div>
      <div className="nav active"><Sparkles size={17}/> New task</div>
      <div className="nav"><Activity size={17}/> Activity</div>
      <div className="nav"><HardDrive size={17}/> Local resources</div>
      <div className="sidebar-bottom"><ShieldCheck size={17}/><span>Local-first mode<br/><small>Your device, your control</small></span></div>
    </aside>
    <section className="main">
      <header><div><span className="eyebrow">PERSONAL AI RUNTIME</span><h1>Make your device work smarter.</h1><p>Start locally. Keep control. Ask for outside help only when you choose.</p></div><div className="status"><span/> PRE-ALPHA</div></header>
      <div className="notice"><LockKeyhole size={17}/><span><b>Local-first by default.</b> Chat with a local model, or choose an online provider and approve each transfer. UJANI never silently switches your data to the cloud.</span></div>
      <section className="launcher-section" aria-labelledby="launcher-title">
        <div className="section-heading"><div><h2 id="launcher-title">How would you like to work?</h2><p>Choose a workspace. You can change modes at any time.</p></div><span className="muted">QUICK START</span></div>
        <div className="launcher-grid">
          <button className="launcher-card launcher-primary" onClick={openLocalChat}>
            <span className="launcher-icon"><BrainCircuit size={21}/></span><b>Chat with my local AI</b><small>Use models running on this computer. No provider key or cloud transfer.</small><span className="launcher-action">Open local chat →</span>
          </button>
          <button className="launcher-card" onClick={() => document.getElementById("online-ai-workspace")?.scrollIntoView({ behavior: "smooth", block: "start" })}>
            <span className="launcher-icon"><Zap size={21}/></span><b>Chat with online AI</b><small>Use a hosted AI model over the internet. Bring a provider key or compatible API.</small><span className="launcher-action">Set up online chat →</span>
          </button>
          <button className="launcher-card" onClick={openFileTools}>
            <span className="launcher-icon"><HardDrive size={21}/></span><b>Work with my files</b><small>Analyze documents, PDFs, folders and image metadata using local tools.</small><span className="launcher-action">Open file tools →</span>
          </button>
          <button className="launcher-card" onClick={openLocalSetup}>
            <span className="launcher-icon"><Cpu size={21}/></span><b>Set up local AI</b><small>Check Ollama, installed models, memory and available device capabilities.</small><span className="launcher-action">View device setup →</span>
          </button>
          <button className="launcher-card launcher-unavailable" disabled aria-describedby="peer-mode-status">
            <span className="launcher-icon"><Monitor size={21}/></span><b>Worker mode</b><small>Delegate approved tasks to trusted worker devices.</small><span className="launcher-action">Not available yet</span>
          </button>
          <button className="launcher-card launcher-unavailable" disabled aria-describedby="peer-mode-status">
            <span className="launcher-icon"><Activity size={21}/></span><b>Partner mode</b><small>Collaborate with enrolled peers on bounded tasks.</small><span className="launcher-action">Not available yet</span>
          </button>
          <button className="launcher-card" onClick={() => window.dispatchEvent(new Event("ujani-check-updates"))}>
            <span className="launcher-icon"><CheckCircle2 size={21}/></span><b>Updates & releases</b><small>Check for a signed UJANI update and install it when available.</small><span className="launcher-action">Check for updates →</span>
          </button>
        </div>
        <p className="launcher-footnote" id="peer-mode-status">Worker and Partner are intentionally disabled until device enrollment, encrypted peer transport, task isolation and revocation are ready. This protects your files and device from untrusted peers.</p>
      </section>
      <div className="section-heading"><div><h2>Your device</h2><p>Read-only system information</p></div><span className="muted">No background compute</span></div>
      <div className="metrics">
        <div className="metric"><Cpu/><small>PROCESSOR</small><b>{deviceBusy ? "Checking…" : device ? `${device.logical_cpus} logical cores` : "Not checked"}</b><span>{(device?.architecture ?? deviceError) || "Waiting for desktop runtime"}</span></div>
        <div className="metric"><HardDrive/><small>SYSTEM MEMORY</small><b>{memory}</b><span>{device ? `${(device.available_memory_mb / 1024).toFixed(1)} GB currently available` : "Native runtime required"}</span></div>
        <div className="metric"><Monitor/><small>PLATFORM</small><b>{device?.os ?? "Not detected"}</b><span>GPU / NPU detection not yet implemented</span></div>
      </div>
      <div className="model-box resource-box" id="local-resources">
        <div className="section-heading"><div><h2><Cpu size={18}/> Computer resource & capability centre</h2><p>Read-only information UJANI can use to select local work.</p></div><span className="muted">NO BACKGROUND COMPUTE</span></div>
        <div className="metrics">
          <div className="metric"><small>CPU</small><b>{device?.logical_cpus ?? "—"} logical cores</b><span>Available to local applications; usage is not reserved by UJANI.</span></div>
          <div className="metric"><small>AVAILABLE RAM</small><b>{device ? (device.available_memory_mb / 1024).toFixed(1) + " GB" : "—"}</b><span>Changes as other apps use memory.</span></div>
          <div className="metric"><small>OS LANGUAGE</small><b>{device?.system_locale ?? "Not detected"}</b><span>Detected preference only; not a translation engine.</span></div>
        </div>
        <div className="principle"><CheckCircle2/><div><b>Local AI compute</b><small>{ollamaStatus?.available ? "Ollama is available. Its configured backend may use CPU and supported GPU acceleration." : "Install Ollama to use local model inference."} UJANI does not currently detect GPU/NPU readiness itself.</small></div></div>
        <div className="principle"><ShieldCheck/><div><b>Operating-system services</b><small>UJANI can report OS and locale now. Direct speech, OCR, accessibility, translation packs and GPU APIs need separate platform adapters and permissions.</small></div></div>
        <div className="principle"><Monitor/><div><b>Browser capabilities</b><small>External web tools can be opened by choice. This version does not read browser tabs, use browser translation APIs, or transfer content automatically.</small></div></div>
      </div>
      <div className="model-box resource-box">
        <div className="section-heading">
          <div><h2><Cpu size={18}/> Capability registry & task planner</h2><p>Live, read-only capability checks and a safe plan preview. Planning never starts work.</p></div>
          <button className="model-run" onClick={refreshCapabilities} disabled={!isTauri() || capabilityBusy}>{capabilityBusy ? "Checking…" : "Refresh capabilities"}</button>
        </div>
        {capabilityError && <p className="error" role="alert">{capabilityError}</p>}
        {!isTauri() && <p className="muted">Open the desktop app to inspect real device capabilities and plan operations.</p>}
        <div className="capability-list">
          {(capabilitySnapshot?.capabilities ?? []).map((capability) => (
            <div className="principle" key={capability.id}>
              <span className={`dot ${capability.state === "available" ? "online" : ""}`}/>
              <div><b>{capability.id.replace(/[._]/g, " ")}</b><small>{capability.state.toUpperCase()} · {capability.detail} · Source: {capability.source}</small></div>
            </div>
          ))}
        </div>
        <label className="model-select">Preview a registered local operation
          <select value={plannedOperation} onChange={(event) => { setPlannedOperation(event.target.value); setTaskPlan(null); }}>
            {(capabilitySnapshot?.operations ?? []).map((operation) => <option key={operation.id} value={operation.id}>{operation.id} — {operation.description}</option>)}
          </select>
        </label>
        <button className="model-run" onClick={previewLocalPlan} disabled={!isTauri() || capabilityBusy}>{capabilityBusy ? "Planning…" : "Preview local task plan"}</button>
        {taskPlan && <div className="result">
          <div className="result-title"><ShieldCheck size={17}/> {taskPlan.accepted ? "Local plan ready" : "Plan not available"} <span>{taskPlan.execution_location}</span></div>
          <p>{taskPlan.reason}</p>
          <small>Required: {taskPlan.required_capabilities.join(", ") || "none"} · Missing: {taskPlan.missing_capabilities.join(", ") || "none"} · Available RAM: {taskPlan.available_memory_mb} MiB · Minimum preflight: {taskPlan.minimum_available_memory_mb} MiB · Execution started: no</small>
        </div>}

        <div className="result">
          <div className="result-title"><BrainCircuit size={17}/> Integrated workflow graph <span>SCHEMA V1</span></div>
          <p>Preview a three-step local workflow: analyse selected text → summarise with Ollama → translate with Ollama.</p>
          <button className="model-run" onClick={previewDocumentWorkflow} disabled={!isTauri() || workflowBusy}>{workflowBusy ? "Validating workflow…" : "Validate document workflow"}</button>
          {workflowPlan && <>
            <p>{workflowPlan.accepted ? "Workflow plan accepted." : "Workflow cannot be planned."} {workflowPlan.reason}</p>
            <small>Order: {workflowPlan.ordered_node_ids.join(" → ") || "Not available"} · Required: {workflowPlan.required_capabilities.join(", ") || "none"} · Conservative peak-memory preflight: {workflowPlan.estimated_peak_memory_mb} MiB · Available: {workflowPlan.available_memory_mb} MiB · Execution started: {workflowPlan.execution_started ? "yes" : "no"}</small>
          </>}
          <button className="model-run" onClick={executeValidatedDocumentWorkflow} disabled={!isTauri() || workflowBusy || !fileText.trim() || !ollamaStatus?.available || !ollamaModel || (executionMode !== "fortress" && executionMode !== "smart")}>{workflowBusy ? "Executing validated local workflow…" : "Execute validated workflow on selected text"}</button>
          {workflowExecution && <div className="result"><div className="result-title"><CheckCircle2 size={17}/> Workflow executed <span>LOCAL</span></div><p>{workflowExecution.result.summary}</p><small>Order: {workflowExecution.plan.ordered_node_ids.join(" → ")} · {workflowExecution.result.chunks_processed}/{workflowExecution.result.total_chunks} sections · {workflowExecution.result.model} · Execution started: yes</small>{workflowExecution.result.translated_summary && <p><b>Translation ({workflowExecution.result.translation_language}):</b> {workflowExecution.result.translated_summary}</p>}</div>}
          <small className="muted">Only this fixed, validated local sequence executes. It uses text already selected in the app; no arbitrary graph nodes, filesystem paths, shell commands or cloud requests are executed.</small>
        </div>
        <small className="muted">GPU/NPU are reported as unknown until verified. Memory thresholds are coarse preflight checks, not model-specific estimates or reservations. The planner supports local operations only; Worker/Partner gateway is not implemented.</small>
      </div>
      <div className="model-box resource-box" id="tool-catalogue">
        <div className="section-heading">
          <div><h2><HardDrive size={18}/> UJANI tool catalogue</h2><p>Small built-in catalogue loaded only when requested. Manifests describe capabilities; they do not grant permissions.</p></div>
          <button className="model-run" onClick={refreshToolCatalogue} disabled={!isTauri() || toolCatalogueBusy}>{toolCatalogueBusy ? "Loading…" : "Load tool catalogue"}</button>
        </div>
        {toolCatalogueError && <p className="error" role="alert">{toolCatalogueError}</p>}
        {!isTauri() && <p className="muted">Open the desktop application to load the native catalogue.</p>}
        {toolCatalogue && <div className="capability-list">
          {toolCatalogue.map((tool) => <div className="principle" key={tool.id}>
            <span className={`dot ${tool.adapter_available ? "online" : ""}`}/>
            <div><b>{tool.name} <small>v{tool.version}</small></b>
              <small>{tool.description}</small>
              <small>{tool.category.replace(/_/g, " ")} · {tool.execution.replace(/_/g, " ")} · Network: {tool.network} · {tool.adapter_available ? "Native adapter declared" : "Metadata only"}</small>
              <small>Permissions: {tool.permissions.join(", ") || "None declared"} · Limits: {Math.round(tool.budget.max_input_bytes / 1024)} KiB input / {tool.budget.max_memory_mb} MiB memory / {tool.budget.max_runtime_ms} ms</small>
            </div>
          </div>)}
        </div>}
        <small className="muted">This is a curated catalogue, not a claim that 100+ tools are already implemented. Runtime adapters still enforce their own permissions, validation, cancellation and limits.</small>
      </div>
      <div className="workspace" id="chat-workspace">
        <div className="task-panel">
          <div className="section-heading"><div><h2>What would you like to do?</h2><p>Enter a task, or continue in the local or online chat workspace below.</p></div><Zap size={20}/></div>
          <textarea value={request} onChange={(event) => setRequest(event.target.value)} placeholder="Describe a task… e.g. Explain how solar panels work." rows={5}/>
          <div className="controls"><label>Execution policy <select value={policy} onChange={(event) => setPolicy(event.target.value)}><option value="local">Local only</option><option value="ask" disabled>Ask before external help (coming soon)</option></select></label><button onClick={runTask} disabled={!request.trim() || busy}>{busy ? "Running demo…" : "Run word-count demo"} <Send size={15}/></button></div>
          <div className="mode-panel">
            <div className="section-heading"><div><h2>Four execution modes</h2><p>Policy preview only — no cloud request is made.</p></div><ShieldCheck size={19}/></div>
            <label className="model-select">Execution mode
              <select value={executionMode} onChange={(event) => { setExecutionMode(event.target.value); setRouteDecision(null); }}>
                <option value="fortress">Fortress — local only</option>
                <option value="smart">Smart — local first (default)</option>
                <option value="worker">Worker — cloud planning</option>
                <option value="partner">Partner — bounded collaboration</option>
              </select>
            </label>
            <label className="consent-row"><input type="checkbox" checked={externalApproved} onChange={(event) => { setExternalApproved(event.target.checked); setRouteDecision(null); }}/> I explicitly approve external data transfer for this task</label>
            <button className="model-run" onClick={previewRoute} disabled={!request.trim() || routeBusy}>{routeBusy ? "Evaluating policy…" : "Preview execution route"} <Send size={15}/></button>
            {routeError && <p className="error" role="alert">{routeError}</p>}
            {routeDecision && <div className="result"><div className="result-title"><ShieldCheck size={17}/> {routeDecision.action.replace(/_/g, " ")} <span>{routeDecision.allowed ? "Policy permits plan" : "Blocked / approval needed"}</span></div><p>{routeDecision.reason}</p><small>External transfer: {routeDecision.external_data_transfer_permitted ? "consented for a future gateway" : "not permitted"} · Rounds: {routeDecision.collaboration_rounds} · Enforcement boundary required: yes</small></div>}
            <small className="muted">Selecting a mode never grants consent by itself. Fortress policy preview does not disable OS networking. Worker and Partner cannot execute until a secure gateway is implemented.</small>
          </div>
          {result && <div className="result"><div className="result-title"><CheckCircle2 size={17}/> Demo result <span>{result.execution}</span></div><p>{result.output}</p><small>Operation: {result.operation}</small></div>}
          <div className="model-box" id="local-ai-workspace">
            <div className="section-heading"><div><h2><BrainCircuit size={18}/> Local Ollama AI chat</h2><p>Real inference through the local Ollama service</p></div><span className="muted">LOOPBACK ONLY</span></div>
            <div className="ollama-status"><span className={ollamaStatus?.available ? "dot online" : "dot"}/><div><b>Ollama: {ollamaBusy ? "Checking local service…" : ollamaStatus === null ? "Not checked yet" : ollamaStatus.available ? "Connected locally" : "Not detected"}</b><small>{ollamaBusy ? "Checking 127.0.0.1:11434…" : ollamaStatus === null ? "UJANI checks the local service only when you open Local AI or choose Refresh." : ollamaStatus.available ? ollamaStatus.models.length + " local model(s) detected at 127.0.0.1:11434" : "Install Ollama and start its local service. No remote endpoint is used."}</small></div></div>
            <button className="model-run" onClick={refreshOllamaStatus} disabled={!isTauri() || ollamaBusy}>{ollamaBusy ? "Checking Ollama…" : "Refresh Ollama status"}</button>
            {(!ollamaStatus?.available || ollamaStatus.models.length === 0) && <div className="install-guide">
              <b>Set up local AI on this PC</b>
              <p>{platformInstallHint}</p>
              <a href="https://ollama.com/download" target="_blank" rel="noreferrer">Download Ollama from the official website ↗</a>
              <p>After installation, open a terminal and download a model:</p>
              <code>ollama pull llama3.2</code>
              <p>Then keep Ollama running and use “Refresh Ollama status” above. Model files are downloaded separately from the Ollama application.</p>
              <a href="https://ollama.com/library" target="_blank" rel="noreferrer">Browse available models ↗</a>
            </div>}
            <div className="install-guide">
              <b>Local model resource advice (estimate only)</b>
              {modelRecommendation ? <>
                <p>{modelRecommendation.explanation}</p>
                <p>Available RAM: {modelRecommendation.available_memory_mb.toLocaleString()} MiB · Advisory budget: {modelRecommendation.usable_memory_budget_mb.toLocaleString()} MiB</p>
                {modelRecommendation.recommended_model ? <p>Suggested installed model: <b>{modelRecommendation.recommended_model}</b></p> : <p>No installed model can currently be recommended from the available information.</p>}
                {modelRecommendation.candidates.slice(0,5).map(candidate => <p key={candidate.name}>{candidate.name}: {candidate.estimated_working_memory_mb===null?"unknown estimate":candidate.estimated_working_memory_mb.toLocaleString()+" MiB estimated"} · {candidate.estimated_to_fit?"within advisory budget":"not recommended"}</p>)}
                {modelRecommendation.recommended_model && <button className="model-run" onClick={() => setOllamaModel(modelRecommendation.recommended_model!)}>Choose suggested model</button>}
              </> : <p>{recommendationBusy ? "Checking installed models and available memory…" : "No model-fit estimate is available yet. You can retry the check."}</p>}
              {recommendationError && <p className="error" role="alert">Model advice unavailable: {recommendationError}</p>}
              <button className="model-run" onClick={refreshModelRecommendation} disabled={recommendationBusy}>{recommendationBusy?"Checking model advice…":"Refresh model advice"}</button>
            </div>
            <div className="install-guide">
              <b>Install a local model from UJANI</b>
              <p>Enter an Ollama library model name. UJANI sends the name to your local Ollama service at 127.0.0.1 only; the model files are downloaded by Ollama from its configured registry. Large models can take substantial time, bandwidth and disk space.</p>
              <label className="model-select">Model name<input value={modelPullName} onChange={(event) => setModelPullName(event.target.value)} maxLength={128} placeholder="e.g. llama3.2 or qwen2.5:3b" disabled={modelPullBusy}/></label>
              <button className="model-run" onClick={pullLocalModel} disabled={!isTauri() || !ollamaStatus?.available || modelPullBusy || !modelPullName.trim()}>{modelPullBusy ? "Downloading model…" : "Download model through Ollama"}</button>
              {modelPullBusy && <><p className="progress" role="status">{taskProgress?.task_id === activeModelPullTaskId ? taskProgress.message + " " + taskProgress.progress_percent + "%" : "Ollama is downloading the model…"}</p><button className="model-run" onClick={cancelModelPull}>Cancel model download</button></>}
              {modelPullError && <p className="error" role="alert">{modelPullError}</p>}
              <small className="muted">Only download models from sources you trust. Review each model's license, size and hardware requirements before installing.</small>
            </div>
            <label className="model-select">Installed Ollama model
              <select value={ollamaModel} onChange={(event) => setOllamaModel(event.target.value)} disabled={!ollamaStatus?.models.length}>
                {!ollamaStatus?.models.length && <option value="">No local models detected</option>}
                {ollamaStatus?.models.map((model) => <option key={model.name} value={model.name}>{model.name}</option>)}
              </select>
            </label>
            <button className="model-run" onClick={runLocalAi} disabled={!request.trim() || aiBusy || !ollamaStatus?.available || !ollamaModel || (executionMode !== "fortress" && executionMode !== "smart")}>{aiBusy ? "Running local model…" : "Run real local AI with Ollama"} <Send size={15}/></button>
            {aiBusy && <><p className="progress" role="status">{taskProgress?.task_id === activeTaskId ? taskProgress.message + " " + taskProgress.progress_percent + "%" : "Local task running…"}</p><button className="model-run" onClick={cancelLocalAi}>Cancel current task</button></>}
            {ollamaError && <p className="error" role="alert">{ollamaError}</p>}
            <div className="chat-thread" aria-live="polite">
              {localChatMessages.length === 0 ? <p className="chat-empty">Your private local conversation will appear here. Recent turns are included as context for follow-up questions.</p> : localChatMessages.map((message, index) => <div className={`chat-message ${message.role}`} key={index}><b>{message.role === "user" ? "You" : "Local AI"}</b><p>{message.content}</p></div>)}
            </div>
            <div className="chat-footer"><small>{localChatMessages.length ? "Recent conversation context is sent only to your local Ollama service." : "No conversation has started yet."}</small><button type="button" onClick={() => { setLocalChatMessages([]); setAiResult(null); }} disabled={!localChatMessages.length || aiBusy}>Clear local chat</button></div>
            {aiResult && <small className="muted">{aiResult.mode_used} · {aiResult.model} · local Ollama · no cloud fallback</small>}
            <small className="muted">Fortress and Smart only. This uses HTTP to loopback Ollama. Fortress does not yet provide OS-wide process or network isolation on this build.</small>
          </div>
          <div className="model-box" id="document-workspace">
            <div className="section-heading"><div><h2><HardDrive size={18}/> Local file & image tools</h2><p>Only files explicitly selected by you are processed.</p></div><span className="muted">IN-MEMORY</span></div>
            <label className="consent-row"><input type="checkbox" checked={translateSummary} onChange={(event) => { setTranslateSummary(event.target.checked); setFileTranslatedSummary(""); }}/> Also translate document summaries into {targetLanguage} using the same local model</label>
            <label className="model-select">Summary translation language
              <select value={targetLanguage} onChange={(event) => { setTargetLanguage(event.target.value); setFileTranslatedSummary(""); }}>
                {["English","Arabic","Hindi","Urdu","French","Spanish","German","Chinese","Japanese","Portuguese","Russian","Bengali","Tamil"].map((language) => <option key={language} value={language}>{language}</option>)}
              </select>
            </label>
            <label className="file-picker">Analyze text file (max 1 MiB; AI summary up to 256 KiB)
              <input type="file" accept=".txt,.md,.csv,.json,.rs,.py,.js,.ts,.html,.xml,.log" onChange={handleTextFile}/>
            </label>
            <label className="file-picker">Extract and summarise PDF locally (max 10 MiB; extracted text up to 256 KiB)
              <input type="file" accept=".pdf,application/pdf" onChange={handlePdfFile} disabled={summaryBusy}/>
            </label>
            <div className="file-picker">
              <b>Analyze a folder using the native folder picker</b>
              <button type="button" className="model-run" onClick={handleFolderSelection} disabled={!isTauri() || folderBusy || (executionMode !== "fortress" && executionMode !== "smart")}>
                {folderBusy ? "Processing selected folder…" : "Choose folder and analyze"}
              </button>
            </div>
            <small className="muted">The native OS dialog grants this one-time selection. UJANI reads only direct child TXT, MD, CSV, JSON, RS, PY, JS, TS, HTML, XML and LOG files: maximum 10 files, 256 KiB each, 1 MiB total. No recursive traversal or persistent folder permission. Summaries use local Ollama only; without it, analysis still works.</small>
            {folderBusy && <p className="progress" role="status">Processing selected folder locally…</p>}
            {folderError && <p className="error" role="alert">{folderError}</p>}
            {folderResults.map((item) => <div className="result" key={item.file_name}>
              <div className="result-title">{item.file_name}<span>{item.word_count} words · {item.line_count} lines · {item.size_bytes} bytes</span></div>
              <small>Keywords: {item.top_keywords.join(", ") || "None detected"}</small>
              {item.summary && <p>{item.summary}</p>}
              <small>{item.note}</small>
            </div>)}
            {fileAnalysis && <div className="result"><div className="result-title">Text analysis <span>{fileAnalysis.file_name}</span></div><p>{fileAnalysis.word_count} words · {fileAnalysis.line_count} lines · {fileAnalysis.size_bytes} bytes</p><small>Keywords: {fileAnalysis.top_keywords.join(", ") || "None detected"}</small><p>{fileAnalysis.content_preview}</p></div>}
            {fileAnalysis && fileText && <button className="model-run" onClick={summarizeFile} disabled={summaryBusy || !ollamaStatus?.available || !ollamaModel || (executionMode !== "fortress" && executionMode !== "smart") || new TextEncoder().encode(fileText).length > 256 * 1024}>{summaryBusy ? "Processing sections locally…" : "Run integrated local document pipeline"}</button>}
            {summaryBusy && <button type="button" className="model-run" onClick={cancelDocumentTask} disabled={!activeDocumentTaskId}>Cancel document processing</button>}
            {fileSummary && <div className="result"><div className="result-title">Document pipeline result <span>{ollamaModel}</span></div><p>{fileSummary}</p><small>Text analysis + AI summary completed by local Ollama. {filePipelineMeta || (fileAnalysis ? "Source size: " + fileAnalysis.size_bytes.toLocaleString() + " bytes." : "")} No cloud transfer.</small></div>}
            {fileTranslatedSummary && <div className="result"><div className="result-title">Translated summary <span>{targetLanguage}</span></div><p>{fileTranslatedSummary}</p><small>Second local Ollama step; no browser or cloud provider contacted.</small></div>}
            <label className="file-picker">Inspect image metadata (max 20 MiB)
              <input type="file" accept=".png,.jpg,.jpeg,.gif,.webp,.bmp" onChange={handleImageFile}/>
            </label>
            {imageInfo && <div className="result"><div className="result-title">Image information <span>{imageInfo.file_name}</span></div><p>{imageInfo.description}</p></div>}
            {fileError && <p className="error" role="alert">{fileError}</p>}
            <small className="muted">Image inspection reads basic headers only. It does not perform OCR or visual recognition.</small>
          </div>
          <div className="model-box">
            <div className="section-heading"><div><h2><BrainCircuit size={18}/> Translation workspace</h2><p>Choose local AI or a browser translation service.</p></div><span className="muted">USER CONTROL</span></div>
            <label className="file-picker">Text to translate
              <textarea rows={4} value={translationText} onChange={(event) => setTranslationText(event.target.value)} placeholder="Paste text here. For local translation, text stays with the local Ollama service."/>
            </label>
            <label className="model-select">Translate into
              <select value={targetLanguage} onChange={(event) => setTargetLanguage(event.target.value)}>
                {["English","Arabic","Hindi","Urdu","French","Spanish","German","Chinese","Japanese","Portuguese","Russian","Bengali","Tamil"].map((language) => <option key={language} value={language}>{language}</option>)}
              </select>
            </label>
            <button className="model-run" onClick={translateLocally} disabled={!translationText.trim() || translationBusy || !ollamaStatus?.available || !ollamaModel || (executionMode !== "fortress" && executionMode !== "smart")}>{translationBusy ? "Translating on this computer…" : "Translate with local AI"}</button>
            {translationResult && <div className="result"><div className="result-title">Local translation <span>{ollamaModel}</span></div><p>{translationResult}</p><small>Processed through local Ollama on this computer. No browser or cloud translation provider was contacted.</small></div>}
            {translationBusy && <button type="button" className="model-run" onClick={cancelTranslationTask} disabled={!activeTranslationTaskId}>Cancel translation</button>}
            <div className="install-guide">
              <b>Browser translation (manual, optional)</b>
              <p>Open a translation website, then paste the text yourself. UJANI does not automatically send your text or open a page with your content.</p>
              <a href="https://translate.google.com/" target="_blank" rel="noreferrer">Open Google Translate ↗</a>
              <p>Browser-based translation requires internet access and the provider may process the text. Review its privacy terms before pasting sensitive information.</p>
            </div>
          </div>
          <div className="model-box" id="online-ai-workspace">
            <div className="section-heading"><div><h2><BrainCircuit size={18}/> Online AI chat — no local model required</h2><p>Bring your own provider, endpoint, model, and API key.</p></div><span className="muted">OPT-IN ONLY</span></div>
            <p className="muted">This sends your prompt to an AI model hosted by your chosen provider over the internet. You do not need to install or run a local model. Choose a preset or enter your own OpenAI-compatible HTTPS endpoint, model ID and API key. Some providers may offer free tiers or promotional credits; availability, limits, billing and data retention are controlled by that provider. UJANI never supplies keys or promises free usage. Each request requires fresh approval. Smart mode only; Fortress is local-only.</p>
            <label className="model-select">Provider preset<select value={cloudProviderPreset} onChange={(event) => {
              const preset = CLOUD_PROVIDER_PRESETS.find((item) => item.id === event.target.value);
              setCloudProviderPreset(event.target.value);
              if (preset) { setCloudEndpoint(preset.endpoint); setCloudModel(preset.model); }
              setCloudConsent(false); setCloudConsentSnapshot("");
            }}>{CLOUD_PROVIDER_PRESETS.map((preset) => <option key={preset.id} value={preset.id}>{preset.label}</option>)}</select></label>
            <label className="model-select">API endpoint<input type="url" value={cloudEndpoint} onChange={(event) => { setCloudEndpoint(event.target.value); setCloudProviderPreset("custom"); }} placeholder="https://api.example.com/v1/chat/completions"/></label>
            <label className="model-select">Model name<input value={cloudModel} onChange={(event) => { setCloudModel(event.target.value); setCloudConsent(false); setCloudConsentSnapshot(""); }} maxLength={128} placeholder="Provider model identifier"/></label>
            <label className="model-select">Token parameter<select value={cloudTokenParameter} onChange={(event) => { setCloudTokenParameter(event.target.value); setCloudConsent(false); setCloudConsentSnapshot(""); }}><option value="auto">Automatic (recognised model families)</option><option value="max_tokens">max_tokens (standard models)</option><option value="max_completion_tokens">max_completion_tokens (reasoning / compatible deployments)</option></select></label>
            <small className="muted">Automatic mode uses a larger token budget for recognised reasoning models, including on custom endpoints. For provider-specific deployment aliases, choose the compatible field explicitly. Provider support varies.</small>
            <label className="model-select">Provider API key (not saved)<input type="password" autoComplete="off" value={cloudApiKey} onChange={(event) => setCloudApiKey(event.target.value)} placeholder="Paste your own API key"/></label>
            <label className="consent-row"><input type="checkbox" checked={cloudConsent} onChange={(event) => { const checked = event.target.checked; setCloudConsent(checked); setCloudConsentSnapshot(checked ? cloudApprovalSnapshot : ""); }}/> I approve sending this exact conversation and current message to this exact endpoint and model for this task only.</label>
            <small className="muted">Provider terms, data retention and billing apply. Do not send passwords, private keys or sensitive personal data. The key is not written to UJANI history or configuration; it is held temporarily for the request and cleared from the input afterward. JavaScript memory cannot be guaranteed to be securely erased.</small>
            <button className="model-run" onClick={runCloudAi} disabled={!isTauri() || !request.trim() || !cloudConsent || cloudConsentSnapshot !== cloudApprovalSnapshot || !cloudApiKey.trim() || cloudBusy || executionMode !== "smart"}>{cloudBusy ? "Sending approved cloud request…" : "Run this prompt with my cloud provider"} <Send size={15}/></button>
            {cloudBusy && <><p className="progress" role="status">{taskProgress?.task_id === activeCloudTaskId ? taskProgress.message + " " + taskProgress.progress_percent + "%" : "Cloud request running…"}</p><button className="model-run" onClick={cancelCloudAi}>Cancel cloud request</button></>}
            {cloudError && <p className="error" role="alert">{cloudError}</p>}
            <div className="chat-thread cloud-thread" aria-live="polite">
              {cloudChatMessages.length === 0 ? <p className="chat-empty">Your online conversation will appear here. Each new message includes recent turns and requires fresh approval before it is sent.</p> : cloudChatMessages.map((message, index) => <div className={`chat-message ${message.role}`} key={index}><b>{message.role === "user" ? "You" : cloudModel || "Online AI"}</b><p>{message.content}</p></div>)}
            </div>
            <div className="chat-footer"><small>{cloudChatMessages.length ? "Recent conversation context is sent to your selected provider after each approval." : "No online conversation has started."}</small><button type="button" onClick={() => { setCloudChatMessages([]); setCloudResult(""); }} disabled={!cloudChatMessages.length || cloudBusy}>Clear online chat</button></div>
            {cloudResult && <small className="muted">Last response received from {cloudModel}. Online chat is not automatically saved to UJANI history.</small>}
          </div>
          <div className="model-box">
            <div className="section-heading"><div><h2><BrainCircuit size={18}/> Local model adapter</h2><p>Runtime discovery and typed adapter proof-of-concept</p></div><span className="muted">NO CLOUD</span></div>
            <label className="model-select">Model selection<select value={selectedModel} onChange={(event) => { setSelectedModel(event.target.value); setModelResult(null); setModelError(""); }}><option value="auto">Automatic (currently resolves to the only registered local mock)</option>{models.map((model) => <option key={model.id} value={model.id}>{model.name}</option>)}</select></label>
            {models.map((model) => <div className="model-status" key={model.id}><span className={model.available ? "dot online" : "dot"}/><div><b>{model.name}</b><small>{model.status} · {model.required_capabilities.join(", ")} required · {model.is_mock ? "MOCK — not a real model" : "Local model"}</small></div></div>)}
            {modelError && <p className="error" role="alert">{modelError}</p>}
            <button className="model-run" onClick={runModel} disabled={!request.trim() || modelBusy || (!!selected && !selected.available)}>{modelBusy ? "Running locally…" : "Run selected local adapter"} <Send size={15}/></button>
            {modelBusy && <p className="progress" role="status">Local adapter running. No network request is made.</p>}
            {modelResult && <div className="result"><div className="result-title"><CheckCircle2 size={17}/> {modelResult.is_mock ? "Mock adapter result" : "Model result"} <span>{modelResult.execution}</span></div><p>{modelResult.output}</p><small>{modelResult.model_name} · {modelResult.model_id} · {modelResult.is_mock ? "No AI inference occurred" : "Local inference"}</small></div>}
          </div>
        </div>
        <div className="side-panel"><h3>Execution principles</h3><div className="principle"><CheckCircle2/><div><b>Local by default</b><small>Local requests stay on this device; online transfers require task approval.</small></div></div><div className="principle"><ShieldCheck/><div><b>Permission first</b><small>No shell commands or hidden device sharing.</small></div></div><div className="principle"><CircleHelp/><div><b>Honest capability status</b><small>The bundled adapter is a mock, not a language model.</small></div></div>
          <div className="history"><h3>Recent tasks</h3>{history.length ? history.map((item, index) => <button key={index} onClick={() => setRequest(item)}>{item}</button>) : <small>Your recent tasks will appear here.</small>}</div>
          <div className="history receipt-history">
            <h3>Privacy & execution receipts</h3>
            <p>Metadata-only records for supported local chat and approved cloud chat tasks. Document, PDF, translation and model-download operations are not yet connected to this receipt ledger. Prompts and outputs are not stored in this ledger.</p>
            <div className="history-actions">
              <button onClick={refreshExecutionReceipts} disabled={!isTauri() || receiptBusy}>{receiptBusy ? "Verifying…" : "Verify & refresh receipts"}</button>
            </div>
            {receiptLedgerStatus && <small>Ledger: {receiptLedgerStatus.valid ? "Integrity verified" : "Integrity check failed"} · {receiptLedgerStatus.entries} entries · Head SHA-256: {receiptLedgerStatus.head_sha256.slice(0, 16)}…</small>}
            {receiptError && <p className="error" role="alert">{receiptError}</p>}
            {executionReceipts.slice(0, 10).map(({ receipt, record_hash }) => <div className="saved-history-item" key={receipt.task_id + record_hash}>
              <b>{receipt.operation_id} · {receipt.task_id}</b>
              <small>{receipt.location} · {receipt.adapter_id} v{receipt.adapter_version} · {receipt.model_id || "No model recorded"}</small>
              <small>{new Date(receipt.started_at_unix_ms).toLocaleString()} · {Math.max(0, receipt.completed_at_unix_ms - receipt.started_at_unix_ms)} ms</small>
              <details className="saved-history-output"><summary>View provenance hashes</summary>
                <small>Input SHA-256: {receipt.input_sha256}</small>
                <small>Output SHA-256: {receipt.output_sha256}</small>
                <small>Ledger record SHA-256: {record_hash}</small>
                <small>Approval reference: {receipt.user_approval_id || "Not required for this local task"}</small>
              </details>
            </div>)}
            {!executionReceipts.length && !receiptError && <small>No execution receipts recorded yet. Run a supported AI task, then refresh this panel.</small>}
            <small>The ledger is hash-chained and tamper-evident, not digitally signed or independently anchored. A local administrator with database access may still replace the entire store.</small>
          </div>
          <div className="history persistent-history">
            <h3>Saved task history</h3>
            <p>Optional local history. Saved prompts and outputs are encrypted on this device using AES-256-GCM with a key held in the operating-system credential vault.</p>
            {historySettings && <label className="consent-row"><input type="checkbox" checked={historySettings.enabled} disabled={historyBusy} onChange={(event) => togglePersistentHistory(event.target.checked)}/> Save future task prompts and results on this device</label>}
            <small>Retention: {historySettings?.retention_days ?? 30} days · Maximum: {historySettings?.max_entries ?? 100} entries. Recording is off by default.</small>
            <div className="history-actions"><button onClick={refreshPersistentHistory} disabled={historyBusy}>{historyBusy ? "Refreshing…" : "Refresh"}</button><button onClick={clearPersistentHistory} disabled={!persistentHistory.length || historyBusy}>Delete all</button></div>
            {historyError && <p className="error" role="alert">{historyError}</p>}
            {persistentHistory.length ? persistentHistory.map((entry) => <div className="saved-history-item" key={entry.id}>
              <button className="saved-history-prompt" onClick={() => setRequest(entry.prompt)} title="Reuse saved prompt">{entry.prompt}</button>
              <small>{entry.kind} · {entry.mode} · {new Date(entry.created_at * 1000).toLocaleString()}</small>
              <details className="saved-history-output"><summary>View saved result</summary><p>{entry.output}</p><small>Model: {entry.model || "Not recorded"}</small></details>
              <button className="history-delete" onClick={() => deletePersistentHistory(entry.id)} aria-label={"Delete history entry " + entry.id}>Delete</button>
            </div>) : <small>No saved tasks yet.</small>}
            <small>Deletion removes entries from the app database but is not a forensic secure-erase guarantee. Prompt/output fields are encrypted; metadata remains visible.</small>
          </div>
        </div>
      </div>
      <footer>UJANI Public AI · Open source · Early prototype / pre-alpha <span>Local intelligence first.</span></footer>
    </section>
  </main>;
}
