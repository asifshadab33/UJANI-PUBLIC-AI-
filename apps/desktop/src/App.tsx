import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Activity, Cpu, HardDrive, ShieldCheck, Sparkles, Zap, LockKeyhole, Send, Monitor, CheckCircle2, CircleHelp, BrainCircuit } from "lucide-react";

type DeviceInfo = { os: string; architecture: string; logical_cpus: number; total_memory_mb: number };
type RunResult = { operation: string; output: string; execution: string };
type ModelInfo = { id: string; name: string; provider: string; available: boolean; is_mock: boolean; required_capabilities: string[]; status: string };
type InferenceResult = { model_id: string; model_name: string; output: string; execution: string; is_mock: boolean };
type RouteDecision = { mode: string; action: string; allowed: boolean; external_data_transfer_permitted: boolean; collaboration_rounds: number; reason: string; enforcement_boundary_required: boolean };
type OllamaStatus = { available: boolean; models: { name: string; size: number }[] };
type AiTaskResult = { mode_used: string; execution_location: string; model: string; output: string };
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
  const [ollamaModel, setOllamaModel] = useState("");
  const [aiBusy, setAiBusy] = useState(false);
  const [aiResult, setAiResult] = useState<AiTaskResult | null>(null);
  const [fileAnalysis, setFileAnalysis] = useState<FileAnalysis | null>(null);
  const [imageInfo, setImageInfo] = useState<ImageInfo | null>(null);
  const [fileError, setFileError] = useState("");

  useEffect(() => {
    if (!isTauri()) {
      setDeviceError("Browser preview: launch the Tauri desktop app to read device information.");
      setModels([{ id: "ujani.mock.echo.v1", name: "UJANI Mock Echo (test adapter)", provider: "Built-in deterministic mock", available: true, is_mock: true, required_capabilities: ["cpu"], status: "Browser preview only — mock" }]);
      return;
    }
    invoke<DeviceInfo>("get_device_info").then(setDevice).catch((error) => setDeviceError(String(error)));
    invoke<ModelInfo[]>("list_local_models").then(setModels).catch((error) => setModelError(String(error)));
    invoke<OllamaStatus>("get_ollama_status").then((status) => {
      setOllamaStatus(status);
      if (status.models.length) setOllamaModel(status.models[0].name);
    }).catch((error) => setOllamaError(String(error)));
  }, []);

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
        availableMemoryMb: device?.total_memory_mb ?? 0,
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
    setFileError(""); setFileAnalysis(null);
    if (file.size > 1024 * 1024) { setFileError("Text files are limited to 1 MiB."); return; }
    if (!isTauri()) { setFileError("Launch the Tauri desktop app to process files."); return; }
    try { setFileAnalysis(await invoke<FileAnalysis>("analyze_text_file", { fileName: file.name, content: await file.text() })); }
    catch (error) { setFileError(String(error)); }
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
        <div className="metric"><HardDrive/><small>SYSTEM MEMORY</small><b>{memory}</b><span>{device ? "Total physical memory" : "Native runtime required"}</span></div>
        <div className="metric"><Monitor/><small>PLATFORM</small><b>{device?.os ?? "Not detected"}</b><span>GPU / NPU detection not yet implemented</span></div>
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
            <div className="ollama-status"><span className={ollamaStatus?.available ? "dot online" : "dot"}/><div><b>Ollama: {ollamaStatus?.available ? "Connected locally" : "Not detected"}</b><small>{ollamaStatus?.available ? ollamaStatus.models.length + " local model(s) detected at 127.0.0.1:11434" : "Install Ollama and start its local service. No remote endpoint is used."}</small></div></div>
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
            <label className="file-picker">Analyze text file (max 1 MiB)
              <input type="file" accept=".txt,.md,.csv,.json,.rs,.py,.js,.ts,.html,.xml,.log" onChange={handleTextFile}/>
            </label>
            {fileAnalysis && <div className="result"><div className="result-title">Text analysis <span>{fileAnalysis.file_name}</span></div><p>{fileAnalysis.word_count} words · {fileAnalysis.line_count} lines · {fileAnalysis.size_bytes} bytes</p><small>Keywords: {fileAnalysis.top_keywords.join(", ") || "None detected"}</small><p>{fileAnalysis.content_preview}</p></div>}
            <label className="file-picker">Inspect image metadata (max 20 MiB)
              <input type="file" accept=".png,.jpg,.jpeg,.gif,.webp,.bmp" onChange={handleImageFile}/>
            </label>
            {imageInfo && <div className="result"><div className="result-title">Image information <span>{imageInfo.file_name}</span></div><p>{imageInfo.description}</p></div>}
            {fileError && <p className="error" role="alert">{fileError}</p>}
            <small className="muted">Image inspection reads basic headers only. It does not perform OCR or visual recognition.</small>
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
