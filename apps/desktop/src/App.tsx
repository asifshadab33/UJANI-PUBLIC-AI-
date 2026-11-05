import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Activity, Cpu, HardDrive, ShieldCheck, Sparkles, Zap, LockKeyhole, Send, Monitor, CheckCircle2, CircleHelp } from "lucide-react";

type DeviceInfo = { os: string; architecture: string; logical_cpus: number; total_memory_mb: number };
type RunResult = { operation: string; output: string; execution: string };
const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export default function App() {
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [deviceError, setDeviceError] = useState("");
  const [request, setRequest] = useState("");
  const [result, setResult] = useState<RunResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [policy, setPolicy] = useState("local");
  const [history, setHistory] = useState<string[]>([]);

  useEffect(() => {
    if (!isTauri()) {
      setDeviceError("Browser preview: launch the Tauri desktop app to read device information.");
      return;
    }
    invoke<DeviceInfo>("get_device_info").then(setDevice).catch((error) => setDeviceError(String(error)));
  }, []);

  async function runTask() {
    const text = request.trim();
    if (!text || busy) return;
    setBusy(true);
    setResult(null);
    try {
      const output: RunResult = isTauri()
        ? await invoke<RunResult>("run_local_task", { request: text })
        : {
            operation: "local.word_count",
            output: `Local preview completed. Your request contains ${text.split(/\s+/).length} words. This is a demo tool, not an AI model.`,
            execution: "Browser demo (not a native device runtime)"
          };
      setResult(output);
      setHistory((items) => [text, ...items].slice(0, 5));
    } catch (error) {
      setResult({ operation: "local.task", output: `Task failed safely: ${String(error)}`, execution: "Local" });
    } finally {
      setBusy(false);
    }
  }

  const memory = device ? `${(device.total_memory_mb / 1024).toFixed(1)} GB` : "—";
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
          <div className="section-heading"><div><h2>What would you like to do?</h2><p>Try the safe local demonstration tool.</p></div><Zap size={20}/></div>
          <textarea value={request} onChange={(event) => setRequest(event.target.value)} placeholder="Describe a task… e.g. Count the words in this paragraph." rows={5}/>
          <div className="controls"><label>Execution policy <select value={policy} onChange={(event) => setPolicy(event.target.value)}><option value="local">Local only</option><option value="ask" disabled>Ask before external help (coming soon)</option></select></label><button onClick={runTask} disabled={!request.trim() || busy}>{busy ? "Running…" : "Run local task"} <Send size={15}/></button></div>
          {result && <div className="result"><div className="result-title"><CheckCircle2 size={17}/> Result <span>{result.execution}</span></div><p>{result.output}</p><small>Operation: {result.operation}</small></div>}
        </div>
        <div className="side-panel"><h3>Execution principles</h3><div className="principle"><CheckCircle2/><div><b>Local by default</b><small>Tasks stay on this device in this prototype.</small></div></div><div className="principle"><ShieldCheck/><div><b>Permission first</b><small>No shell commands or hidden device sharing.</small></div></div><div className="principle"><CircleHelp/><div><b>Honest capability status</b><small>Unavailable features are labelled, not simulated.</small></div></div>
          <div className="history"><h3>Recent tasks</h3>{history.length ? history.map((item, index) => <button key={index} onClick={() => setRequest(item)}>{item}</button>) : <small>Your recent tasks will appear here.</small>}</div>
        </div>
      </div>
      <footer>UJANI Public AI · Open source · Early prototype / pre-alpha <span>Local intelligence first.</span></footer>
    </section>
  </main>;
}