import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Search, FolderSearch, FileVideo, FileText, Trash2, RefreshCw, Clock3 } from "lucide-react";

type SearchMatch = { path: string; file_name: string; extension: string; size_bytes: number; modified_unix_ms: number | null; matched_in: string[] };
type SearchResult = { cancelled: boolean; root_name: string | null; scanned_files: number; matches: SearchMatch[]; truncated: boolean; note: string; cache_hit: boolean };
type CacheSettings = { enabled: boolean; retention_days: number; max_entries: number; max_bytes: number };
type RecentSearch = { query: string; created_at: number };

const formatSize = (bytes: number) => bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;

export default function LocalFinder() {
  const [query, setQuery] = useState("");
  const [resultQuery, setResultQuery] = useState("");
  const [pdfText, setPdfText] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [result, setResult] = useState<SearchResult | null>(null);
  const [taskId, setTaskId] = useState("");
  const [cacheSettings, setCacheSettings] = useState<CacheSettings>({ enabled: false, retention_days: 30, max_entries: 50, max_bytes: 10 * 1024 * 1024 });
  const [cacheMessage, setCacheMessage] = useState("Search caching is off until you opt in.");
  const [cacheBusy, setCacheBusy] = useState(false);
  const [recentSearches, setRecentSearches] = useState<RecentSearch[]>([]);

  async function refreshRecentSearches() {
    try { setRecentSearches(await invoke<RecentSearch[]>("list_recent_local_searches")); }
    catch { setRecentSearches([]); }
  }

  useEffect(() => {
    // Preferences and recent-query metadata load asynchronously; they never delay the launcher.
    invoke<CacheSettings>("get_local_search_cache_settings")
      .then((settings) => {
        setCacheSettings(settings);
        setCacheMessage(settings.enabled ? "Encrypted local search cache is on." : "Search cache is off; searches will always run fresh.");
      })
      .catch(() => setCacheMessage("Cache settings unavailable. Searches still run without cached results."));
    invoke<RecentSearch[]>("list_recent_local_searches").then(setRecentSearches).catch(() => setRecentSearches([]));
  }, []);

  async function updateCacheSettings(enabled: boolean, retention_days: number) {
    const previous = cacheSettings;
    setCacheSettings({ ...previous, enabled, retention_days });
    setCacheBusy(true);
    setCacheMessage("Saving cache preference…");
    try {
      const saved = await invoke<CacheSettings>("set_local_search_cache_settings", { enabled, retentionDays: retention_days });
      setCacheSettings(saved);
      setCacheMessage(saved.enabled
        ? (saved.retention_days === 0 ? "Cache retained until cleared; storage remains bounded." : `Cache retained for ${saved.retention_days} days.`)
        : "Cache disabled and existing cached entries cleared.");
      if (!saved.enabled) setRecentSearches([]);
      else await refreshRecentSearches();
    } catch (e) {
      setCacheSettings(previous);
      setCacheMessage(`Could not save cache preference: ${String(e)}`);
    } finally { setCacheBusy(false); }
  }

  async function clearCache() {
    if (cacheBusy || busy) return;
    setCacheBusy(true);
    setCacheMessage("Clearing encrypted search cache…");
    try {
      await invoke("clear_local_search_cache");
      setRecentSearches([]);
      setCacheMessage("Recent search cache cleared.");
    } catch (e) {
      setCacheMessage(`Could not clear cache: ${String(e)}`);
    } finally { setCacheBusy(false); }
  }

  async function searchFor(searchQuery: string, forceRefresh = false) {
    if (!searchQuery.trim() || busy || cacheBusy) return;
    setQuery(searchQuery);
    const id = `finder-${Date.now()}`;
    setTaskId(id); setBusy(true); setError(""); setResult(null);
    try {
      const found = await invoke<SearchResult>("find_local_files", {
        query: searchQuery,
        searchPdfText: pdfText,
        forceRefresh,
        taskId: id
      });
      setResult(found);
      setResultQuery(searchQuery);
      await refreshRecentSearches();
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); setTaskId(""); }
  }

  async function cancelSearch() {
    if (!taskId) return;
    try { await invoke<boolean>("cancel_active_task", { taskId }); }
    catch (e) { setError(`Could not cancel search: ${String(e)}`); }
  }

  return <section className="model-box local-finder" id="local-file-finder">
    <div className="section-heading"><div><h2><FolderSearch size={18}/> Find files on this computer</h2><p>Find videos, PDFs, documents and other files in a folder you choose.</p></div><span className="muted">LOCAL · READ-ONLY</span></div>
    <form onSubmit={(event) => { event.preventDefault(); void searchFor(query); }}>
      <label className="model-select">What are you looking for?
        <input value={query} maxLength={160} onChange={(event) => setQuery(event.target.value)} placeholder="e.g. meeting recording, supplier agreement, invoice 2026" />
      </label>
      <label className="consent-row"><input type="checkbox" checked={pdfText} onChange={(event) => setPdfText(event.target.checked)}/> Search inside text-based PDFs as well as filenames</label>
      <button className="model-run" type="submit" disabled={!query.trim() || busy || cacheBusy}>{busy ? "Searching selected folder…" : "Choose folder and search"} <Search size={15}/></button>
    </form>
    <div className="local-cache-controls">
      <div className="section-heading"><div><h3>Recent search cache</h3><p>Encrypted metadata only; document contents are never cached.</p></div></div>
      <label className="consent-row"><input type="checkbox" checked={cacheSettings.enabled} disabled={cacheBusy || busy} onChange={(event) => void updateCacheSettings(event.target.checked, cacheSettings.retention_days)}/> Remember validated search results (opt-in)</label>
      <label className="model-select">Delete cached results after
        <select value={String(cacheSettings.retention_days)} disabled={cacheBusy || busy || !cacheSettings.enabled} onChange={(event) => void updateCacheSettings(cacheSettings.enabled, Number(event.target.value))}>
          <option value="7">7 days</option>
          <option value="30">30 days (recommended)</option>
          <option value="90">90 days</option>
          <option value="0">Until I clear them</option>
        </select>
      </label>
      <div className="result-title"><small className="muted">{cacheMessage}</small><button className="model-run" type="button" onClick={clearCache} disabled={cacheBusy || busy}><Trash2 size={14}/> Clear cache</button></div>
      <small className="muted">Maximum 50 cached searches or 10 MB; least-recently-used entries are removed first. No-result searches expire after 24 hours. If the OS credential vault is unavailable, UJANI skips caching and performs a fresh search.</small>
      {recentSearches.length > 0 && <div className="recent-searches">
        <b><Clock3 size={14}/> Recent searches</b>
        {recentSearches.map((item, index) => <button className="model-run" type="button" key={`${item.created_at}-${index}`} disabled={busy || cacheBusy} onClick={() => void searchFor(item.query)}>
          {item.query} <small>{new Date(item.created_at * 1000).toLocaleDateString()}</small>
        </button>)}
      </div>}
    </div>
    {busy && <button className="model-run" type="button" onClick={cancelSearch} disabled={!taskId}>Cancel search</button>}
    <small className="muted">UJANI opens a native folder picker. It does not scan the whole disk, upload files, move or delete anything. Search is bounded to 5,000 entries, six directory levels, 20 PDFs and 180 seconds. Scanned PDFs need OCR, which is not included.</small>
    {error && <p className="error" role="alert">{error}</p>}
    {result && !result.cancelled && <div className="result">
      <div className="result-title">Search results <span>{result.matches.length} matches · {result.scanned_files} files checked · {result.cache_hit ? "validated cache" : "fresh search"}</span></div>
      <p>{result.note}</p>
      <button className="model-run" type="button" onClick={() => void searchFor(resultQuery, true)} disabled={busy || cacheBusy || !resultQuery}><RefreshCw size={14}/> Refresh results (ignore cache)</button>
      {result.truncated && <p className="muted">Some entries were skipped or search limits were reached; choose a narrower folder to search the remaining files.</p>}
      {result.matches.length ? result.matches.map((item) => <div className="saved-history-item" key={item.path}>
        <b>{item.extension === "mp4" || item.extension === "mov" || item.extension === "mkv" ? <FileVideo size={14}/> : <FileText size={14}/>} {item.file_name}</b>
        <small>{item.path}</small>
        <small>{formatSize(item.size_bytes)} · {item.matched_in.join(", ")} · {item.modified_unix_ms ? new Date(item.modified_unix_ms).toLocaleString() : "Modified time unavailable"}</small>
      </div>) : <p>No matches found in the selected folder. Try a shorter phrase or another folder.</p>}
    </div>}
  </section>;
}
