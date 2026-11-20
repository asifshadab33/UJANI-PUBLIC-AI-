import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { BellOff, CalendarClock, Check, Clock3, Pencil, Plus, Trash2 } from "lucide-react";

type ReminderSettings = { enabled: boolean; notifications_enabled: boolean };
type LocalReminder = {
  id: string;
  title: string;
  notes: string | null;
  due_at_unix_ms: number;
  timezone: string;
  state: "active" | "completed";
  snoozed_until_unix_ms: number | null;
  last_notified_due_at_unix_ms: number | null;
  created_at_unix_ms: number;
  updated_at_unix_ms: number;
};
type ReminderResponse =
  | { kind: "settings"; value: ReminderSettings }
  | { kind: "list"; value: LocalReminder[] }
  | { kind: "reminder"; value: LocalReminder }
  | { kind: "reminder_saved_with_receipt_warning"; value: { reminder: LocalReminder; warning: string } }
  | { kind: "agenda"; value: LocalReminder[] }
  | { kind: "deleted" }
  | { kind: "cleared"; value: { deleted: number } };

async function manage(request: Record<string, unknown>): Promise<ReminderResponse> {
  return invoke<ReminderResponse>("manage_local_reminders", { request });
}

function localDateTimeInput(timestamp: number): string {
  const date = new Date(timestamp);
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

function formatDue(timestamp: number, timezone: string): string {
  try {
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
      timeZone: timezone,
    }).format(new Date(timestamp));
  } catch {
    return new Date(timestamp).toLocaleString();
  }
}

function localTimezone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
  } catch {
    return "UTC";
  }
}

export default function ReminderCentre() {
  const [settings, setSettings] = useState<ReminderSettings | null>(null);
  const [reminders, setReminders] = useState<LocalReminder[]>([]);
  const [title, setTitle] = useState("");
  const [notes, setNotes] = useState("");
  const [dueInput, setDueInput] = useState(() => localDateTimeInput(Date.now() + 60 * 60 * 1000));
  const [editingId, setEditingId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    const nextSettings = await manage({ action: "get_settings" });
    if (nextSettings.kind !== "settings") throw new Error("Reminder settings response was invalid.");
    setSettings(nextSettings.value);
    const nextList = await manage({ action: "list" });
    if (nextList.kind !== "list") throw new Error("Reminder list response was invalid.");
    setReminders(nextList.value);
  }

  useEffect(() => {
    let active = true;
    (async () => {
      try {
        const nextSettings = await manage({ action: "get_settings" });
        if (!active) return;
        if (nextSettings.kind !== "settings") throw new Error("Reminder service returned an unexpected settings response.");
        setSettings(nextSettings.value);
        const nextList = await manage({ action: "list" });
        if (!active) return;
        if (nextList.kind !== "list") throw new Error("Reminder service returned an unexpected list response.");
        setReminders(nextList.value);
      } catch (reason) {
        if (active) setError("Local reminders are unavailable: " + String(reason));
      } finally {
        if (active) setLoading(false);
      }
    })();
    return () => { active = false; };
  }, []);

  async function run(action: () => Promise<void>) {
    if (loading) return;
    setBusy(true);
    setError(null);
    try {
      await action();
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  async function toggleEnabled(enabled: boolean) {
    await run(async () => {
      const response = await manage({ action: "set_enabled", enabled });
      if (response.kind !== "settings") throw new Error("Reminder settings could not be updated.");
      setSettings(response.value);
      if (!enabled) {
        setEditingId(null);
        setTitle("");
        setNotes("");
      }
    });
  }

  async function saveReminder(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const titleValue = title.trim();
    const notesValue = notes.trim() ? notes.trim() : null;
    const encoder = new TextEncoder();
    if (encoder.encode(titleValue).length > 160) {
      setError("Reminder title must be no more than 160 UTF-8 bytes.");
      return;
    }
    if (notesValue !== null && encoder.encode(notesValue).length > 2000) {
      setError("Reminder notes must be no more than 2,000 UTF-8 bytes.");
      return;
    }
    const dueAt = new Date(dueInput).getTime();
    if (!Number.isFinite(dueAt) || dueAt <= 0) {
      setError("Choose a valid reminder date and time.");
      return;
    }
    await run(async () => {
      const common = {
        title: titleValue,
        notes: notesValue,
        due_at_unix_ms: dueAt,
        timezone: localTimezone(),
      };
      const response = editingId
        ? await manage({ action: "update", id: editingId, ...common })
        : await manage({ action: "create", ...common });
      if (response.kind === "reminder_saved_with_receipt_warning") {
        setError("Reminder was saved, but its local audit receipt needs attention: " + response.value.warning);
      } else if (response.kind !== "reminder") {
        throw new Error("Reminder was not saved.");
      }
      setEditingId(null);
      setTitle("");
      setNotes("");
      setDueInput(localDateTimeInput(Date.now() + 60 * 60 * 1000));
    });
  }

  function beginEdit(reminder: LocalReminder) {
    setEditingId(reminder.id);
    setTitle(reminder.title);
    setNotes(reminder.notes ?? "");
    setDueInput(localDateTimeInput(reminder.due_at_unix_ms));
  }

  async function completeReminder(id: string) {
    await run(async () => {
      const response = await manage({ action: "complete", id });
      if (response.kind !== "reminder") throw new Error("Reminder could not be completed.");
    });
  }

  async function snoozeReminder(reminder: LocalReminder) {
    await run(async () => {
      const currentDue = reminder.snoozed_until_unix_ms ?? reminder.due_at_unix_ms;
      const until = Math.max(Date.now(), currentDue) + 60 * 60 * 1000;
      const response = await manage({ action: "snooze", id: reminder.id, until_unix_ms: until });
      if (response.kind !== "reminder") throw new Error("Reminder could not be snoozed.");
    });
  }

  async function deleteReminder(id: string) {
    await run(async () => {
      const response = await manage({ action: "delete", id });
      if (response.kind !== "deleted") throw new Error("Reminder could not be deleted.");
      if (editingId === id) setEditingId(null);
    });
  }

  async function clearAllReminders() {
    // Native IPC owns the destructive-action confirmation; avoid a duplicate webview prompt.
    await run(async () => {
      const response = await manage({ action: "clear_all" });
      if (response.kind !== "cleared") throw new Error("Local reminders could not be cleared.");
      setEditingId(null);
      setTitle("");
      setNotes("");
    });
  }

  return (
    <section className="task-panel reminder-centre" aria-labelledby="reminder-centre-title">
      <div className="section-heading">
        <div>
          <h2 id="reminder-centre-title"><CalendarClock size={18}/> Personal reminders</h2>
          <p>Private, on-device reminders. No calendar or email account is connected.</p>
        </div>
        <span className="muted">OPTIONAL · LOCAL</span>
      </div>

      <div className="reminder-consent">
        <div>
          <b>Local reminder storage</b>
          <small>Off by default. Reminder titles and notes are encrypted with a key held in your operating-system credential vault. Turning this off stops new edits and upcoming-agenda results; saved items remain visible for deletion and encrypted until you delete them.</small>
        </div>
        <label className="reminder-toggle">
          <input
            type="checkbox"
            checked={settings?.enabled ?? false}
            disabled={busy || loading || settings === null}
            onChange={(event) => void toggleEnabled(event.target.checked)}
          />
          <span>{settings?.enabled ? "Enabled" : "Disabled"}</span>
        </label>
      </div>

      {settings && (
        <div className="reminder-maintenance">
          <small>Saved reminders remain encrypted until deleted, even when the feature is disabled.</small>
          <button type="button" disabled={busy || loading} onClick={() => void clearAllReminders()}><Trash2 size={13}/> Clear all</button>
        </div>
      )}

      {settings?.enabled && (
        <form className="reminder-form" onSubmit={saveReminder}>
          <label className="reminder-field">
            Reminder title
            <input value={title} maxLength={160} required disabled={busy || loading} onChange={(event) => setTitle(event.target.value)} placeholder="e.g. Renew vehicle insurance"/>
          </label>
          <label className="reminder-field">
            Date and time
            <input type="datetime-local" required value={dueInput} disabled={busy || loading} onChange={(event) => setDueInput(event.target.value)}/>
          </label>
          <label className="reminder-field reminder-notes">
            Notes (optional)
            <textarea rows={2} maxLength={2000} value={notes} disabled={busy || loading} onChange={(event) => setNotes(event.target.value)} placeholder="Add a short note"/>
          </label>
          <div className="reminder-form-actions">
            <button className="model-run" type="submit" disabled={busy || loading || !title.trim()}>
              {editingId ? <Check size={14}/> : <Plus size={14}/>}
              {editingId ? "Save changes" : "Add reminder"}
            </button>
            {editingId && <button className="reminder-secondary" type="button" disabled={busy || loading} onClick={() => { setEditingId(null); setTitle(""); setNotes(""); }}>Cancel edit</button>}
          </div>
        </form>
      )}

      {settings && !settings.notifications_enabled && (
        <p className="reminder-limitation"><BellOff size={14}/> OS notifications and background delivery are not enabled in this release. The agenda is available when UJANI is open; this feature will not wake a sleeping device.</p>
      )}

      {error && <p className="error" role="alert">{error}</p>}
      {reminders.length === 0 ? (
        <p className="reminder-empty">{settings?.enabled ? "No reminders yet. Add one above." : "Enable local reminders to create a private reminder list."}</p>
      ) : (
        <div className="reminder-list" aria-label="Saved reminders">
          {reminders.map((reminder) => (
            <article className={"reminder-item" + (reminder.state === "completed" ? " is-complete" : "")} key={reminder.id}>
              <div className="reminder-item-main">
                <b>{reminder.title}</b>
                {reminder.notes && <p>{reminder.notes}</p>}
                <small><Clock3 size={12}/> {formatDue(reminder.snoozed_until_unix_ms ?? reminder.due_at_unix_ms, reminder.timezone)} · {reminder.timezone} · {reminder.state === "completed" ? "Completed" : "Active"}</small>
              </div>
              <div className="reminder-item-actions">
                {settings?.enabled && reminder.state === "active" && <>
                  <button type="button" title="Edit reminder" aria-label={"Edit " + reminder.title} disabled={busy || loading} onClick={() => beginEdit(reminder)}><Pencil size={14}/></button>
                  <button type="button" title="Snooze one hour" aria-label={"Snooze " + reminder.title + " for one hour"} disabled={busy || loading} onClick={() => void snoozeReminder(reminder)}><Clock3 size={14}/></button>
                  <button type="button" title="Mark complete" aria-label={"Complete " + reminder.title} disabled={busy || loading} onClick={() => void completeReminder(reminder.id)}><Check size={14}/></button>
                </>}
                <button type="button" title="Delete reminder" aria-label={"Delete " + reminder.title} disabled={busy || loading} onClick={() => void deleteReminder(reminder.id)}><Trash2 size={14}/></button>
              </div>
            </article>
          ))}
        </div>
      )}
      <small className="reminder-footnote">This first release supports one-time reminders and manual snoozing only. Recurrence, strict IANA timezone/DST validation, OS notifications and calendar/email integrations remain future work and are not silently simulated. Key rotation, backup and recovery are not yet implemented; if the OS vault key is lost, saved reminders cannot be recovered, but their encrypted rows can still be cleared.</small>
    </section>
  );
}
