import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Clock3, CopyCheck, FileSearch, FolderSearch, ShieldCheck } from "lucide-react";

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
type LargeFile = { file_name: string; path: string; size_bytes: number; modified_unix_ms: number | null };
type LargeFileScanResult = {
  root_name: string;
  scanned_files: number;
  large_files: LargeFile[];
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
  const [largeResult, setLargeResult] = useState<LargeFileScanResult | null>(null);
  const [largeBusy, setLargeBusy] = useState(false);
  const [largeTaskId, setLargeTaskId] = useState("");
  const [largeError, setLargeError] = useState("");
  const [minimumSizeMb, setMinimumSizeMb] = useState("100");

  useEffect(() => {
    // The device-local clock is available in the webview without a native IPC call.
    // This keeps the browser preview useful and avoids a misleading native-clock error.
    setClock(new Date());
    const timer = window.setInterval(() => setClock(new Date()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  async function scanDuplicates() {
    if (busy || !isTauri()) return;
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

  async function scanLargeFiles() {
    if (largeBusy || !isTauri()) return;
    const id = `large-files-${Date.now()}`;
    setLargeTaskId(id);
    setLargeBusy(true);
    setLargeError("");
    setLargeResult(null);
    try {
      setLargeResult(await invoke<LargeFileScanResult>("find_large_files", {
        taskId: id,
        minSizeBytes: Number(minimumSizeMb) * 1024 * 1024,
      }));
    } catch (reason) {
      setLargeError(String(reason));
    } finally {
      setLargeBusy(false);
      setLargeTaskId("");
    }
  }

  async function cancelLargeScan() {
    if (!largeTaskId) return;
    try {
      await invoke<boolean>("cancel_active_task", { taskId: largeTaskId });
    } catch (reason) {
      setLargeError(`Could not cancel the scan: ${String(reason)}`);
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

    <div className="section-heading">
      <div><h3><FileSearch size={17}/> Find large files</h3><p>Locate storage-heavy files using metadata only. File contents are never opened.</p></div>
    </div>
    <label className="muted" htmlFor="large-file-threshold">Minimum file size</label>
    <select id="large-file-threshold" value={minimumSizeMb} onChange={(event) => setMinimumSizeMb(event.target.value)} disabled={largeBusy}>
      <option value="50">50 MiB</option>
      <option value="100">100 MiB</option>
      <option value="250">250 MiB</option>
      <option value="500">500 MiB</option>
      <option value="1024">1 GiB</option>
    </select>
    <button className="model-run" type="button" onClick={scanLargeFiles} disabled={largeBusy || !isTauri()}>
      {largeBusy ? "Scanning selected folder…" : "Choose folder and find large files"} <FileSearch size={15}/>
    </button>
    {largeBusy && <><p className="progress" role="status">Reading file metadata only. No file contents are opened.</p><button className="model-run" type="button" onClick={cancelLargeScan} disabled={!largeTaskId}>Cancel scan</button></>}
    <small className="muted">Search is limited to your selected folder, 5,000 entries, six directory levels, 100 displayed results and 60 seconds. Symbolic links and reparse points are skipped. Nothing is changed.</small>
    {largeError && <p className="error" role="alert">{largeError}</p>}
    {largeResult && <div className="result">
      <div className="result-title">Large-file preview <span>{largeResult.large_files.length} results · {largeResult.scanned_files} files checked</span></div>
      <p>{largeResult.root_name}</p>
      {largeResult.truncated && <p className="muted">The scan reached a safety limit or skipped inaccessible entries. Choose a smaller folder for a more complete result.</p>}
      {largeResult.large_files.length ? largeResult.large_files.map((file) => <div className="saved-history-item" key={file.path}>
        <b>{formatSize(file.size_bytes)} · {file.file_name}</b>
        <small>{file.path}</small>
        <small>{file.modified_unix_ms ? `Modified: ${new Date(file.modified_unix_ms).toLocaleString()}` : "Modified time unavailable"}</small>
      </div>) : <p>No files met the selected size threshold within the scan limits.</p>}
      <small>{largeResult.note}</small>
    </div>}
    <p className="muted"><ShieldCheck size={14}/> These tools only create previews. They never delete, move, rename, overwrite or upload your files.</p>
  </section>;
}

const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
