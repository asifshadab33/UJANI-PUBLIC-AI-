import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Clock3, CopyCheck, FolderSearch, ShieldCheck } from "lucide-react";

type DuplicateFile = { file_name: string; path: string; size_bytes: number };
type DuplicateGroup = { sha256: string; size_bytes: number; files: DuplicateFile[] };
type DuplicateScanResult = {
  root_name: string;
  scanned_files: number;
  hashed_files: number;
  duplicate_groups: DuplicateGroup[];
  truncated: boolean;
  note: string;
};

const formatSize = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
};

export default function EverydayTools() {
  const [clock, setClock] = useState<Date | null>(null);
  const [result, setResult] = useState<DuplicateScanResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [taskId, setTaskId] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    let mounted = true;
    void invoke<number>("get_local_clock")
      .then((timestamp) => { if (mounted) setClock(new Date(timestamp)); })
      .catch((reason) => { if (mounted) setError(`Local clock unavailable: ${String(reason)}`); });
    const timer = window.setInterval(() => {
      if (mounted) setClock(new Date());
    }, 1000);
    return () => { mounted = false; window.clearInterval(timer); };
  }, []);

  async function scanDuplicates() {
    if (busy) return;
    const id = `duplicates-${Date.now()}`;
    setTaskId(id);
    setBusy(true);
    setError("");
    setResult(null);
    try {
      setResult(await invoke<DuplicateScanResult>("find_duplicate_files", { taskId: id }));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
      setTaskId("");
    }
  }

  async function cancelScan() {
    if (!taskId) return;
    try {
      await invoke<boolean>("cancel_active_task", { taskId });
    } catch (reason) {
      setError(`Could not cancel the scan: ${String(reason)}`);
    }
  }

  const zone = clock ? new Intl.DateTimeFormat(undefined, { timeZoneName: "short" }).formatToParts(clock).find((part) => part.type === "timeZoneName")?.value : "";

  return <section className="model-box" id="everyday-tools">
    <div className="section-heading">
      <div><h2><Clock3 size={18}/> Everyday tools</h2><p>Small, local utilities that work without an AI model.</p></div>
      <span className="muted">LOCAL · READ-ONLY</span>
    </div>
    <div className="result">
      <div className="result-title">Local date and time <span>{zone || "Device local time"}</span></div>
      <p>{clock ? clock.toLocaleDateString(undefined, { weekday: "long", year: "numeric", month: "long", day: "numeric" }) : "Reading device clock…"}</p>
      <b>{clock ? clock.toLocaleTimeString() : "—"}</b>
      <small>Displayed in this device's local timezone. No network request is used.</small>
    </div>
    <div className="section-heading">
      <div><h3><CopyCheck size={17}/> Find duplicate files</h3><p>Compare file sizes first, then verify possible duplicates using SHA-256.</p></div>
    </div>
    <button className="model-run" type="button" onClick={scanDuplicates} disabled={busy || !isTauri()}>
      {busy ? "Scanning selected folder…" : "Choose folder and find duplicates"} <FolderSearch size={15}/>
    </button>
    {busy && <><p className="progress" role="status">Read-only scan in progress. Files will not be changed.</p><button className="model-run" type="button" onClick={cancelScan} disabled={!taskId}>Cancel scan</button></>}
    <small className="muted">Uses a native folder picker; searches only the folder you select. Limits: 5,000 entries, six directory levels, 64 MiB per file, 512 MiB total hashing and 60 seconds. Symlinks are skipped. No file is deleted, moved, renamed or overwritten.</small>
    {error && <p className="error" role="alert">{error}</p>}
    {result && <div className="result">
      <div className="result-title">Duplicate preview <span>{result.duplicate_groups.length} groups · {result.scanned_files} files checked</span></div>
      <p>{result.root_name} · {result.hashed_files} files hashed</p>
      {result.truncated && <p className="muted">Some files were skipped or a safety limit was reached. Choose a smaller folder for a complete scan.</p>}
      {result.duplicate_groups.length ? result.duplicate_groups.map((group) => <div className="saved-history-item" key={group.sha256}>
        <b>{formatSize(group.size_bytes)} · {group.files.length} identical files</b>
        <small>SHA-256: {group.sha256}</small>
        {group.files.map((file) => <div key={file.path}><small>{file.file_name}</small><small>{file.path}</small></div>)}
      </div>) : <p>No duplicate files were found within the scan limits.</p>}
      <small>{result.note}</small>
    </div>}
    <p className="muted"><ShieldCheck size={14}/> This tool only creates a preview. You decide what to do with any duplicate files.</p>
  </section>;
}

const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
