import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Activity, Cpu, HardDrive, ShieldCheck, Sparkles, Zap, LockKeyhole, Send, Monitor, CheckCircle2, CircleHelp, BrainCircuit } from "lucide-react";

type DeviceInfo = { os: string; architecture: string; logical_cpus: number; total_memory_mb: number; available_memory_mb: number; system_locale: string };
type CapabilityRecord = { id: string; category: string; state: "available" | "unavailable" | "unknown"; source: string; detail: string };
type OperationDescriptor = { id: string; description: string; required_capabilities: string[]; minimum_available_memory_mb: number };
type CapabilitySnapshot = { capabilities: CapabilityRecord[]; operations: OperationDescriptor[]; available_memory_mb: number };
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
const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export default function App() {
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [deviceError, setDeviceError] = useState("");
  const [request, setRequest] = useState("");
  const [result, setResult] = useState<RunResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [policy, setPolicy] = useState("local");
  const [history, setHistory] = useState<string[]>([]);
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
  const [aiBusy, setAiBusy] = useState(false);
  const [aiResult, setAiResult] = useState<AiTaskResult | null>(null);
  const [fileAnalysis, setFileAnalysis] = useState<FileAnalysis | null>(null);
  const [fileText, setFileText] = useState("");
  const [fileSummary, setFileSummary] = useState("");
  const [filePipelineMeta, setFilePipelineMeta] = useState("");
  const [fileTranslatedSummary, setFileTranslatedSummary] = useState("");
  const [translateSummary, setTranslateSummary] = useState(false);
  const [summaryBusy, setSummaryBusy] = useState(false);
  const [imageInfo, setImageInfo] = useState<ImageInfo | null>(null);
  const [fileError, setFileError] = useState("");
  const [translationText, setTranslationText] = useState("");
  const [targetLanguage, setTargetLanguage] = useState("English");
  const [translationResult, setTranslationResult] = useState("");
  const [translationBusy, setTranslationBusy] = useState(false);
  const [capabilitySnapshot, setCapabilitySnapshot] = useState<CapabilitySnapshot | null>(null);
  const [capabilityBusy, setCapabilityBusy] = useState(false);
  const [capabilityError, setCapabilityError] = useState("");
  const [plannedOperation, setPlannedOperation] = useState("local.ai.generate");
  const [taskPlan, setTaskPlan] = useState<TaskPlan | null>(null);
  const [workflowPlan, setWorkflowPlan] = useState<WorkflowPlan | null>(null);
  const [workflowBusy, setWorkflowBusy] = useState(false);
  const [workflowExecution, setWorkflowExecution] = useState<WorkflowExecution | null>(null);
  const [modelRecommendation, setModelRecommendation] = useState<ModelRecommendation | null>(null);
  const [recommendationError, setRecommendationError] = useState("");
  const [recommendationBusy, setRecommendationBusy] = useState(false);

  useEffect(() => {
    if (!isTauri()) {
      setDeviceError("Browser preview: launch the Tauri desktop app to read device information.");
      setModels([{ id: "ujani.mock.echo.v1", name: "UJANI Mock Echo (test adapter)", provider: "Built-in deterministic mock", available: true, is_mock: true, required_capabilities: ["cpu"], status: "Browser preview only — mock" }]);
      return;
    }
    invoke<DeviceInfo>("get_device_info").then(setDevice).catch((error) => setDeviceError(String(error)));
    invoke<CapabilitySnapshot>("get_capability_snapshot").then(setCapabilitySnapshot).catch((error) => setCapabilityError(String(error)));
    invoke<ModelInfo[]>("list_local_models").then(setModels).catch((error) => setModelError(String(error)));
    invoke<OllamaStatus>("get_ollama_status").then((status) => {
      setOllamaStatus(status);
      if (status.models.length) setOllamaModel(status.models[0].name);
    }).catch((error) => setOllamaError(String(error)));
    invoke<ModelRecommendation>("get_model_recommendation").then(setModelRecommendation).catch((error) => setRecommendationError(String(error)));
  }, []);

  async function refreshCapabilities() {
    if (!isTauri() || capabilityBusy) return;
    setCapabilityBusy(true); setCapabilityError(""); setTaskPlan(null);
    try {
      setCapabilitySnapshot(await invoke<CapabilitySnapshot>("get_capability_snapshot"));
    } catch (error) { setCapabilityError(String(error)); }
    finally { setCapabilityBusy(false); }
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
      setOllamaError("Real local inference is available in Fortress and Smart only. Worker and Partner need a secure gateway.");
      return;
    }
    if (!ollamaModel) { setOllamaError("Install and start Ollama, then pull a model before running local AI."); return; }
    setAiBusy(true); setOllamaError(""); setAiResult(null);
    try {
      const result = await invoke<AiTaskResult>("execute_local_ai_task", { prompt: request, mode: executionMode, model: ollamaModel });
      setAiResult(result);
    } catch (error) { setOllamaError(String(error)); }
    finally { setAiBusy(false); }
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

    async function summarizeFile() {
    if (!fileText || !fileAnalysis || summaryBusy) return;
    if (new TextEncoder().encode(fileText).length > 256 * 1024) {
      setFileError("The integrated AI pipeline is limited to 256 KiB per document. Basic text analysis remains available up to 1 MiB.");
      return;
    }
    if (!ollamaModel || !ollamaStatus?.available) { setFileError("Start local Ollama and select an installed model to run the document pipeline."); return; }
    if (executionMode !== "fortress" && executionMode !== "smart") { setFileError("The local document pipeline is available in Fortress and Smart only."); return; }
    setSummaryBusy(true); setFileError(""); setFileSummary(""); setFilePipelineMeta("");
    try {
      const result = await invoke<DocumentPipelineResult>("process_selected_document", {
        fileName: fileAnalysis.file_name, content: fileText, mode: executionMode, model: ollamaModel,
        translationLanguage: translateSummary ? targetLanguage : null
      });
      setFileAnalysis(result.analysis);
      setFileSummary(result.summary);
      setFilePipelineMeta(`Processed ${result.chunks_processed} of ${result.total_chunks} sections · ${result.source_text_bytes.toLocaleString()} source bytes · ${result.execution_location}`);
      setFileTranslatedSummary(result.translated_summary ?? "");
    } catch (error) { setFileError(String(error)); }
    finally { setSummaryBusy(false); }
  }

  async function handlePdfFile(event: import("react").ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    setFileError(""); setFileAnalysis(null); setFileSummary(""); setFilePipelineMeta(""); setFileTranslatedSummary("");
    if (file.size > 10 * 1024 * 1024) { setFileError("PDF files are limited to 10 MiB."); return; }
    if (!isTauri()) { setFileError("Launch the Tauri desktop app to extract PDF text."); return; }
    if (!ollamaStatus?.available || !ollamaModel) { setFileError("Start local Ollama and select a model before processing a PDF."); return; }
    if (executionMode !== "fortress" && executionMode !== "smart") { setFileError("PDF processing is available in Fortress and Smart only."); return; }
    setSummaryBusy(true);
    try {
      const data = Array.from(new Uint8Array(await file.arrayBuffer()));
      const result = await invoke<DocumentPipelineResult>("process_selected_pdf", {
        fileName: file.name, data, mode: executionMode, model: ollamaModel,
        translationLanguage: translateSummary ? targetLanguage : null
      });
      setFileAnalysis(result.analysis);
      setFileSummary(result.summary);
      setFilePipelineMeta(`Processed ${result.chunks_processed} of ${result.total_chunks} sections · ${result.source_text_bytes.toLocaleString()} extracted bytes · ${result.execution_location}`);
      setFileTranslatedSummary(result.translated_summary ?? "");
      setFileText("");
    } catch (error) { setFileError(String(error)); }
    finally { setSummaryBusy(false); event.target.value = ""; }
  }

  async function translateLocally() {
    if (!translationText.trim() || translationBusy) return;
    if (!isTauri() || !ollamaStatus?.available || !ollamaModel) { setOllamaError("Local translation requires the desktop app and a running Ollama model."); return; }
    if (executionMode !== "fortress" && executionMode !== "smart") { setOllamaError("Local translation is available in Fortress and Smart only."); return; }
    setTranslationBusy(true); setTranslationResult(""); setOllamaError("");
    try {
      setTranslationResult(await invoke<string>("translate_text_local", {
        text: translationText, targetLanguage, mode: executionMode, model: ollamaModel
      }));
    } catch (error) { setOllamaError(String(error)); }
    finally { setTranslationBusy(false); }
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
      <div className="notice"><LockKeyhole size={17}/><span><b>Local-only policy is active.</b> This prototype does not send your request to a cloud AI service.</span></div>
      <div className="section-heading"><div><h2>Your device</h2><p>Read-only system information</p></div><span className="muted">No background compute</span></div>
      <div className="metrics">
        <div className="metric"><Cpu/><small>PROCESSOR</small><b>{device ? `${device.logical_cpus} logical cores` : "Not detected"}</b><span>{(device?.architecture ?? deviceError) || "Waiting for desktop runtime"}</span></div>
        <div className="metric"><HardDrive/><small>SYSTEM MEMORY</small><b>{memory}</b><span>{device ? `${(device.available_memory_mb / 1024).toFixed(1)} GB currently available` : "Native runtime required"}</span></div>
        <div className="metric"><Monitor/><small>PLATFORM</small><b>{device?.os ?? "Not detected"}</b><span>GPU / NPU detection not yet implemented</span></div>
      </div>
      <div className="model-box resource-box">
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
      <div className="workspace">
        <div className="task-panel">
          <div className="section-heading"><div><h2>What would you like to do?</h2><p>Enter a prompt to test the local-only execution path.</p></div><Zap size={20}/></div>
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
          <div className="model-box">
            <div className="section-heading"><div><h2><BrainCircuit size={18}/> Local Ollama AI</h2><p>Real inference through the local Ollama service</p></div><span className="muted">LOOPBACK ONLY</span></div>
            <div className="ollama-status"><span className={ollamaStatus?.available ? "dot online" : "dot"}/><div><b>Ollama: {ollamaStatus === null ? "Checking local service…" : ollamaStatus.available ? "Connected locally" : "Not detected"}</b><small>{ollamaStatus === null ? "Checking 127.0.0.1:11434…" : ollamaStatus.available ? ollamaStatus.models.length + " local model(s) detected at 127.0.0.1:11434" : "Install Ollama and start its local service. No remote endpoint is used."}</small></div></div>
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
            <label className="model-select">Installed Ollama model
              <select value={ollamaModel} onChange={(event) => setOllamaModel(event.target.value)} disabled={!ollamaStatus?.models.length}>
                {!ollamaStatus?.models.length && <option value="">No local models detected</option>}
                {ollamaStatus?.models.map((model) => <option key={model.name} value={model.name}>{model.name}</option>)}
              </select>
            </label>
            <button className="model-run" onClick={runLocalAi} disabled={!request.trim() || aiBusy || !ollamaStatus?.available || !ollamaModel || (executionMode !== "fortress" && executionMode !== "smart")}>{aiBusy ? "Running local model…" : "Run real local AI with Ollama"} <Send size={15}/></button>
            {ollamaError && <p className="error" role="alert">{ollamaError}</p>}
            {aiResult && <div className="result"><div className="result-title"><CheckCircle2 size={17}/> Local AI result <span>{aiResult.execution_location}</span></div><p>{aiResult.output}</p><small>{aiResult.mode_used} · {aiResult.model} · no cloud fallback</small></div>}
            <small className="muted">Fortress and Smart only. This uses HTTP to loopback Ollama; it does not itself enforce OS-wide network isolation.</small>
          </div>
          <div className="model-box">
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
            {fileAnalysis && <div className="result"><div className="result-title">Text analysis <span>{fileAnalysis.file_name}</span></div><p>{fileAnalysis.word_count} words · {fileAnalysis.line_count} lines · {fileAnalysis.size_bytes} bytes</p><small>Keywords: {fileAnalysis.top_keywords.join(", ") || "None detected"}</small><p>{fileAnalysis.content_preview}</p></div>}
            {fileAnalysis && fileText && <button className="model-run" onClick={summarizeFile} disabled={summaryBusy || !ollamaStatus?.available || !ollamaModel || (executionMode !== "fortress" && executionMode !== "smart") || new TextEncoder().encode(fileText).length > 256 * 1024}>{summaryBusy ? "Processing sections locally…" : "Run integrated local document pipeline"}</button>}
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
            <div className="install-guide">
              <b>Browser translation (manual, optional)</b>
              <p>Open a translation website, then paste the text yourself. UJANI does not automatically send your text or open a page with your content.</p>
              <a href="https://translate.google.com/" target="_blank" rel="noreferrer">Open Google Translate ↗</a>
              <p>Browser-based translation requires internet access and the provider may process the text. Review its privacy terms before pasting sensitive information.</p>
            </div>
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
        <div className="side-panel"><h3>Execution principles</h3><div className="principle"><CheckCircle2/><div><b>Local by default</b><small>Requests stay on this device in this prototype.</small></div></div><div className="principle"><ShieldCheck/><div><b>Permission first</b><small>No shell commands or hidden device sharing.</small></div></div><div className="principle"><CircleHelp/><div><b>Honest capability status</b><small>The bundled adapter is a mock, not a language model.</small></div></div>
          <div className="history"><h3>Recent tasks</h3>{history.length ? history.map((item, index) => <button key={index} onClick={() => setRequest(item)}>{item}</button>) : <small>Your recent tasks will appear here.</small>}</div>
        </div>
      </div>
      <footer>UJANI Public AI · Open source · Early prototype / pre-alpha <span>Local intelligence first.</span></footer>
    </section>
  </main>;
}
