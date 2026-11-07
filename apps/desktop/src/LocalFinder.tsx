import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Search, FolderSearch, FileVideo, FileText } from "lucide-react";

type SearchMatch = { path: string; file_name: string; extension: string; size_bytes: number; modified_unix_ms: number | null; matched_in: string[] };
type SearchResult = { cancelled: boolean; root_name: string | null; scanned_files: number; matches: SearchMatch[]; truncated: boolean; note: string };

const formatSize = (bytes: number) => bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;

export default function LocalFinder() {
  const [query, setQuery] = useState("");
  const [pdfText, setPdfText] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [result, setResult] = useState<SearchResult | null>(null);

  async function search() {
    if (!query.trim() || busy) return;
    setBusy(true); setError(""); setResult(null);
    try {
      setResult(await invoke<SearchResult>("find_local_files", { query, searchPdfText: pdfText }));
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }

  return <section className="model-box local-finder" id="everyday-tools">
    <div className="section-heading"><div><h2><FolderSearch size={18}/> Find files on this computer</h2><p>Find videos, PDFs, documents and other files in a folder you choose.</p></div><span className="muted">LOCAL · READ-ONLY</span></div>
    <form onSubmit={(event) => { event.preventDefault(); void search(); }}>
      <label className="model-select">What are you looking for?
        <input value={query} maxLength={160} onChange={(event) => setQuery(event.target.value)} placeholder="e.g. meeting recording, supplier agreement, invoice 2026" />
      </label>
      <label className="consent-row"><input type="checkbox" checked={pdfText} onChange={(event) => setPdfText(event.target.checked)}/> Search inside text-based PDFs as well as filenames</label>
      <button className="model-run" type="submit" disabled={!query.trim() || busy}>{busy ? "Searching selected folder…" : "Choose folder and search"} <Search size={15}/></button>
    </form>
    <small className="muted">UJANI opens a native folder picker. It does not scan the whole disk, upload files, move or delete anything. Search is bounded to 5,000 entries, six directory levels, 20 PDFs and 180 seconds. Scanned PDFs need OCR, which is not included.</small>
    {error && <p className="error" role="alert">{error}</p>}
    {result && !result.cancelled && <div className="result">
      <div className="result-title">Search results <span>{result.matches.length} matches · {result.scanned_files} files checked</span></div>
      <p>{result.note}</p>
      {result.truncated && <p className="muted">Some entries were skipped or search limits were reached; choose a narrower folder to search the remaining files.</p>}
      {result.matches.length ? result.matches.map((item) => <div className="saved-history-item" key={item.path}>
        <b>{item.extension === "mp4" || item.extension === "mov" || item.extension === "mkv" ? <FileVideo size={14}/> : <FileText size={14}/>} {item.file_name}</b>
        <small>{item.path}</small>
        <small>{formatSize(item.size_bytes)} · {item.matched_in.join(", ")} · {item.modified_unix_ms ? new Date(item.modified_unix_ms).toLocaleString() : "Modified time unavailable"}</small>
      </div>) : <p>No matches found in the selected folder. Try a shorter phrase or another folder.</p>}
    </div>}
  </section>;
}
