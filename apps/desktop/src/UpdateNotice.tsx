import { useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const isDevBuild = (import.meta as ImportMeta & { env?: { DEV?: boolean } }).env?.DEV === true;

async function checkWithTimeout(): Promise<Update | null> {
  return check({ timeout: 8_000 });
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function UpdateNotice() {
  const [available, setAvailable] = useState<Update | null>(null);
  const [message, setMessage] = useState("");
  const [installing, setInstalling] = useState(false);
  const [checking, setChecking] = useState(false);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [contentLength, setContentLength] = useState<number | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const checkingRef = useRef(false);
  const installingRef = useRef(false);

  async function checkForUpdates() {
    if (!isTauri() || isDevBuild || installingRef.current || checkingRef.current) return;
    checkingRef.current = true;
    setChecking(true);
    setMessage("Checking for a signed update…");
    try {
      const update = await checkWithTimeout();
      setAvailable(update);
      setMessage(update ? "" : "UJANI is up to date. No signed update is currently available.");
    } catch (error) {
      setMessage("Update check: " + String(error));
    } finally {
      checkingRef.current = false;
      setChecking(false);
    }
  }

  useEffect(() => {
    if (!isTauri() || isDevBuild) return;
    let active = true;
    const runInitialCheck = async () => {
      if (checkingRef.current) return;
      checkingRef.current = true;
      setChecking(true);
      try {
        const update = await checkWithTimeout();
        if (active) {
          setAvailable(update);
          setMessage(update ? "" : "No update available. UJANI is up to date.");
        } else {
          await update?.close();
        }
      } catch (error) {
        if (active) setMessage("Automatic update check failed: " + String(error));
      } finally {
        checkingRef.current = false;
        if (active) setChecking(false);
      }
    };
    const onManualCheck = () => { void checkForUpdates(); };
    window.addEventListener("ujani-check-updates", onManualCheck);
    void runInitialCheck();
    return () => {
      active = false;
      window.removeEventListener("ujani-check-updates", onManualCheck);
    };
  }, []);

  async function installUpdate() {
    if (!available || installingRef.current || checkingRef.current) return;
    installingRef.current = true;
    setInstalling(true);
    setDownloadedBytes(0);
    setContentLength(null);
    setMessage("Downloading the signed update…");
    let downloaded = 0;
    try {
      await available.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            setContentLength(event.data.contentLength ?? null);
            setDownloadedBytes(0);
            setMessage("Downloading the signed update…");
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            setDownloadedBytes(downloaded);
            break;
          case "Finished":
            setMessage("Download complete. Verifying and installing the signed update…");
            break;
        }
      });
      setMessage("Update installed. Restarting UJANI…");
      await relaunch();
    } catch (error) {
      setMessage("Update could not be installed: " + String(error));
      setInstalling(false);
      installingRef.current = false;
    }
  }

  const downloadPercent = contentLength && contentLength > 0
    ? Math.min(100, Math.floor((downloadedBytes / contentLength) * 100))
    : null;

  if (dismissed || (!available && !message)) return null;
  return (
    <section role="status" aria-live="polite" style={{
      position: "fixed", zIndex: 1000, top: 12, left: "50%", transform: "translateX(-50%)",
      width: "min(680px, calc(100vw - 24px))", boxSizing: "border-box",
      display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12,
      padding: "12px 16px", border: "1px solid var(--border, #526174)",
      borderRadius: 12, background: "var(--panel, #17212d)", color: "var(--text, #fff)",
      boxShadow: "0 8px 30px rgba(0,0,0,.25)"
    }}>
      <div style={{ minWidth: 0 }}>
        <strong style={{ display: "block", marginBottom: 3 }}>
          {available ? "UJANI update available · " + available.version : "UJANI update center"}
        </strong>
        <span style={{ fontSize: 13, opacity: .85, overflowWrap: "anywhere" }}>
          {message || available?.body || "A signed update is ready to install."}
        </span>
        {installing && <small style={{ display: "block", marginTop: 5 }}>
          {downloadPercent === null
            ? `Downloaded ${formatBytes(downloadedBytes)}`
            : `${downloadPercent}% · ${formatBytes(downloadedBytes)} of ${formatBytes(contentLength ?? 0)}`}
        </small>}
      </div>
      <div style={{ display: "flex", flexShrink: 0, gap: 8 }}>
        {!available && !installing && <button type="button" onClick={() => { void checkForUpdates(); }} disabled={checking}>
          {checking ? "Checking…" : "Check again"}
        </button>}
        {available && <button type="button" onClick={installUpdate} disabled={installing || checking}>
          {installing ? "Installing…" : "Install and restart"}
        </button>}
        {!installing && <button type="button" onClick={() => { void available?.close(); setDismissed(true); }} disabled={checking} aria-label="Dismiss update notice">Later</button>}
      </div>
    </section>
  );
}
