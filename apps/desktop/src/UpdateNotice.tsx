import { useEffect, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export default function UpdateNotice() {
  const [available, setAvailable] = useState<Update | null>(null);
  const [message, setMessage] = useState("");
  const [installing, setInstalling] = useState(false);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    if (!isTauri() || import.meta.env.DEV) return;
    let active = true;
    check()
      .then((update) => { if (active && update) setAvailable(update); })
      .catch((error) => { if (active) setMessage("Automatic update check failed: " + String(error)); });
    return () => { active = false; };
  }, []);

  async function installUpdate() {
    if (!available || installing) return;
    setInstalling(true);
    setMessage("Downloading and verifying the signed update…");
    try {
      await available.downloadAndInstall();
      setMessage("Update installed. Restarting UJANI…");
      await relaunch();
    } catch (error) {
      setMessage("Update could not be installed: " + String(error));
      setInstalling(false);
    }
  }

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
          {available ? "UJANI update available · " + available.version : "UJANI updater"}
        </strong>
        <span style={{ fontSize: 13, opacity: .85, overflowWrap: "anywhere" }}>
          {message || available?.body || "A signed update is ready to install."}
        </span>
      </div>
      <div style={{ display: "flex", flexShrink: 0, gap: 8 }}>
        {available && <button type="button" onClick={installUpdate} disabled={installing}>
          {installing ? "Installing…" : "Install and restart"}
        </button>}
        {!installing && <button type="button" onClick={() => { void available?.close(); setDismissed(true); }} aria-label="Dismiss update notice">Later</button>}
      </div>
    </section>
  );
}
