//! Encrypted, bounded local reminder persistence.
//!
//! This first runtime slice stores one-time absolute due instants only. It does
//! not schedule background work, send OS notifications, implement recurrence,
//! connect to calendar/email apps, or wake the device. Those are separate gates.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
use zeroize::Zeroizing;

use ujani_core::{upcoming_reminders, LocalReminder, ReminderState};
use ujani_core::reminders::MAX_AGENDA_HORIZON_MS;

const KEYRING_SERVICE: &str = "org.ujani.publicai";
const KEYRING_USER: &str = "local-reminders-v1";
const CIPHER_PREFIX: &str = "ujani-reminder:v1:";
const MAX_REMINDERS: i64 = 500;
const MAX_SNOOZE_MS: i64 = 31 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReminderSettings {
    pub enabled: bool,
    /// False until a native OS notification adapter and permission flow are installed.
    pub notifications_enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ReminderRequest {
    GetSettings,
    SetEnabled { enabled: bool },
    List,
    Create { title: String, notes: Option<String>, due_at_unix_ms: i64, timezone: String },
    Update { id: String, title: String, notes: Option<String>, due_at_unix_ms: i64, timezone: String },
    Complete { id: String },
    Snooze { id: String, until_unix_ms: i64 },
    Delete { id: String },
    ClearAll,
    Agenda { now_unix_ms: i64, horizon_ms: i64 },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ReminderResponse {
    Settings(ReminderSettings),
    List(Vec<LocalReminder>),
    Reminder(LocalReminder),
    Agenda(Vec<LocalReminder>),
    Deleted,
    Cleared { deleted: usize },
}

fn now_unix_ms() -> Result<i64, String> {
    let value = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|error| format!("System clock is before the Unix epoch: {error}"))?
        .as_millis();
    i64::try_from(value).map_err(|_| "System clock is outside the supported reminder range.".into())
}

fn open(app: &AppHandle) -> Result<Connection, String> {
    let mut dir: PathBuf = app.path().app_data_dir()
        .map_err(|error| format!("Could not locate UJANI app data directory: {error}"))?;
    fs::create_dir_all(&dir).map_err(|error| format!("Could not create UJANI app data directory: {error}"))?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("Could not secure UJANI app data directory: {error}"))?;
    }
    dir.push("local-reminders.sqlite3");
    let db = Connection::open(&dir).map_err(|error| format!("Could not open local reminders: {error}"))?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("Could not secure local reminder database: {error}"))?;
    }
    db.busy_timeout(std::time::Duration::from_secs(3)).map_err(|error| error.to_string())?;
    initialize(&db)?;
    Ok(db)
}

fn initialize(db: &Connection) -> Result<(), String> {
    let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| format!("Could not read reminder schema version: {error}"))?;
    if version > 1 {
        return Err("Local reminder database is newer than this UJANI build.".into());
    }
    db.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA secure_delete=ON;
         PRAGMA journal_mode=DELETE;
         CREATE TABLE IF NOT EXISTS reminder_settings(
           key TEXT PRIMARY KEY,
           value TEXT NOT NULL
         );
         INSERT OR IGNORE INTO reminder_settings(key,value) VALUES('enabled','false');
         CREATE TABLE IF NOT EXISTS local_reminders(
           id TEXT PRIMARY KEY,
           ciphertext TEXT NOT NULL
         );
         PRAGMA user_version=1;"
    ).map_err(|error| format!("Could not initialise local reminder storage: {error}"))
}

fn settings_from_db(db: &Connection) -> Result<ReminderSettings, String> {
    let enabled: Option<String> = db.query_row(
        "SELECT value FROM reminder_settings WHERE key='enabled'", [], |row| row.get(0)
    ).optional().map_err(|error| error.to_string())?;
    Ok(ReminderSettings {
        enabled: enabled.as_deref() == Some("true"),
        notifications_enabled: false,
    })
}

fn ensure_capacity(current_count: i64) -> Result<(), String> {
    if !(0..MAX_REMINDERS).contains(&current_count) {
        Err("Local reminder limit reached. Delete an old reminder first.".into())
    } else {
        Ok(())
    }
}

fn require_enabled(db: &Connection) -> Result<(), String> {
    if settings_from_db(db)?.enabled { Ok(()) }
    else { Err("Local reminders are disabled. Enable them before creating or changing reminders.".into()) }
}

fn reminder_key(create_if_missing: bool) -> Result<Zeroizing<[u8; 32]>, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| format!("OS credential vault unavailable: {error}"))?;
    match entry.get_password() {
        Ok(encoded) => {
            let encoded = Zeroizing::new(encoded);
            let mut decoded = Zeroizing::new(vec![0_u8; encoded.len()]);
            let length = B64.decode_slice(encoded.as_bytes(), &mut decoded[..])
                .map_err(|_| "Stored reminder encryption key is invalid.".to_string())?;
            decoded.truncate(length);
            let key: [u8; 32] = decoded.as_slice().try_into()
                .map_err(|_| "Stored reminder encryption key has an invalid length.".to_string())?;
            Ok(Zeroizing::new(key))
        }
        Err(keyring::Error::NoEntry) if !create_if_missing => {
            Err("The OS credential vault has no key for existing reminders; refusing to replace it with a new key.".into())
        }
        Err(keyring::Error::NoEntry) => {
            let key = Zeroizing::new(<[u8; 32]>::from(Aes256Gcm::generate_key(&mut OsRng)));
            let encoded = Zeroizing::new(B64.encode(*key));
            entry.set_password(encoded.as_str())
                .map_err(|error| format!("Could not protect reminder key in the OS credential vault: {error}"))?;
            Ok(key)
        }
        Err(error) => Err(format!("Could not read reminder key from the OS credential vault: {error}")),
    }
}

fn aad(id: &str) -> Vec<u8> {
    let mut value = b"ujani|local-reminders|v1".to_vec();
    value.extend_from_slice(&(id.len() as u64).to_be_bytes());
    value.extend_from_slice(id.as_bytes());
    value
}

fn encrypt_record(key: &[u8; 32], reminder: &LocalReminder) -> Result<String, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "Invalid reminder encryption key.".to_string())?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let plaintext = Zeroizing::new(serde_json::to_vec(reminder).map_err(|error| format!("Could not encode local reminder: {error}"))?);
    let encrypted = cipher.encrypt(&nonce, Payload { msg: plaintext.as_slice(), aad: &aad(&reminder.id) })
        .map_err(|_| "Could not encrypt local reminder.".to_string())?;
    let mut payload = nonce.to_vec();
    payload.extend_from_slice(&encrypted);
    Ok(format!("{CIPHER_PREFIX}{}", B64.encode(payload)))
}

fn decrypt_record(key: &[u8; 32], id: &str, value: &str) -> Result<LocalReminder, String> {
    let encoded = value.strip_prefix(CIPHER_PREFIX)
        .ok_or_else(|| "Reminder record is not encrypted with the supported format.".to_string())?;
    let payload = B64.decode(encoded).map_err(|_| "Encrypted reminder is corrupted.".to_string())?;
    if payload.len() < 28 { return Err("Encrypted reminder is incomplete.".into()); }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "Invalid reminder encryption key.".to_string())?;
    let (nonce, ciphertext) = payload.split_at(12);
    let plaintext = Zeroizing::new(cipher.decrypt(Nonce::from_slice(nonce), Payload { msg: ciphertext, aad: &aad(id) })
        .map_err(|_| "Reminder authentication failed; the OS key may have changed or data was altered.".to_string())?);
    let reminder: LocalReminder = serde_json::from_slice(plaintext.as_slice())
        .map_err(|error| format!("Encrypted reminder payload is invalid: {error}"))?;
    if reminder.id != id { return Err("Reminder identifier does not match its encrypted record.".into()); }
    reminder.validate().map_err(|_| "Stored reminder failed validation.".to_string())?;
    Ok(reminder)
}

fn valid_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 96 || !id.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) {
        Err("Reminder identifier is invalid.".into())
    } else { Ok(()) }
}

fn generate_id() -> Result<String, String> {
    let mut random = [0u8; 16];
    OsRng.try_fill_bytes(&mut random).map_err(|_| "OS random source unavailable; reminder was not created.".to_string())?;
    Ok(format!("r-{}", random.iter().map(|byte| format!("{byte:02x}")).collect::<String>()))
}

fn load_all(db: &Connection, key: &[u8; 32]) -> Result<Vec<LocalReminder>, String> {
    let mut statement = db.prepare("SELECT id,ciphertext FROM local_reminders ORDER BY id")
        .map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|error| error.to_string())?;
    let mut reminders = Vec::new();
    for row in rows {
        if reminders.len() as i64 >= MAX_REMINDERS {
            return Err("Local reminder store exceeds its safety limit.".into());
        }
        let (id, ciphertext) = row.map_err(|error| error.to_string())?;
        reminders.push(decrypt_record(key, &id, &ciphertext)?);
    }
    reminders.sort_by(|a,b| a.due_at_unix_ms.cmp(&b.due_at_unix_ms).then_with(|| a.id.cmp(&b.id)));
    Ok(reminders)
}

fn load_one(db: &Connection, key: &[u8; 32], id: &str) -> Result<LocalReminder, String> {
    valid_id(id)?;
    let ciphertext: Option<String> = db.query_row(
        "SELECT ciphertext FROM local_reminders WHERE id=?1", params![id], |row| row.get(0)
    ).optional().map_err(|error| error.to_string())?;
    let ciphertext = ciphertext.ok_or_else(|| "Reminder was not found.".to_string())?;
    decrypt_record(key, id, &ciphertext)
}

fn save_record(tx: &Transaction<'_>, key: &[u8; 32], reminder: &LocalReminder) -> Result<(), String> {
    reminder.validate().map_err(|_| "Reminder failed validation.".to_string())?;
    let ciphertext = encrypt_record(key, reminder)?;
    tx.execute(
        "INSERT INTO local_reminders(id,ciphertext) VALUES(?1,?2)
         ON CONFLICT(id) DO UPDATE SET ciphertext=excluded.ciphertext",
        params![reminder.id, ciphertext],
    ).map_err(|error| error.to_string())?;
    Ok(())
}

fn audit_metadata(action: &str, id: &str, due_at_unix_ms: Option<i64>) -> Vec<u8> {
    serde_json::to_vec(&(action, id, due_at_unix_ms)).unwrap_or_default()
}

fn create(app: AppHandle, title: String, notes: Option<String>, due_at_unix_ms: i64, timezone: String) -> Result<ReminderResponse, String> {
    let now = now_unix_ms()?;
    let reminder = LocalReminder {
        id: generate_id()?,
        title,
        notes,
        due_at_unix_ms,
        timezone,
        state: ReminderState::Active,
        snoozed_until_unix_ms: None,
        last_notified_due_at_unix_ms: None,
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
    };
    reminder.validate().map_err(|_| "Reminder details are invalid or exceed their limits.".to_string())?;
    let metadata = audit_metadata("create", &reminder.id, Some(reminder.due_at_unix_ms));
    crate::with_local_state_receipt(&app, "reminder.create", &metadata, || {
        let mut db = open(&app)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
        require_enabled(&tx)?;
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM local_reminders", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        ensure_capacity(count)?;
        let key = reminder_key(count == 0)?;
        save_record(&tx, &key, &reminder)?;
        tx.commit().map_err(|error| error.to_string())?;
        let output = audit_metadata("created", &reminder.id, Some(reminder.due_at_unix_ms));
        Ok((ReminderResponse::Reminder(reminder.clone()), output))
    })
}

fn update(app: AppHandle, id: String, title: String, notes: Option<String>, due_at_unix_ms: i64, timezone: String) -> Result<ReminderResponse, String> {
    valid_id(&id)?;
    let now = now_unix_ms()?;
    let metadata = audit_metadata("update", &id, Some(due_at_unix_ms));
    crate::with_local_state_receipt(&app, "reminder.update", &metadata, || {
        let mut db = open(&app)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
        require_enabled(&tx)?;
        let key = reminder_key(false)?;
        let mut reminder = load_one(&tx, &key, &id)?;
        reminder.title = title;
        reminder.notes = notes;
        reminder.due_at_unix_ms = due_at_unix_ms;
        reminder.timezone = timezone;
        reminder.snoozed_until_unix_ms = None;
        reminder.last_notified_due_at_unix_ms = None;
        reminder.updated_at_unix_ms = now.max(reminder.updated_at_unix_ms);
        reminder.validate().map_err(|_| "Reminder details are invalid or exceed their limits.".to_string())?;
        save_record(&tx, &key, &reminder)?;
        tx.commit().map_err(|error| error.to_string())?;
        let output = audit_metadata("updated", &id, Some(due_at_unix_ms));
        Ok((ReminderResponse::Reminder(reminder), output))
    })
}

fn complete(app: AppHandle, id: String) -> Result<ReminderResponse, String> {
    valid_id(&id)?;
    let now = now_unix_ms()?;
    let metadata = audit_metadata("complete", &id, None);
    crate::with_local_state_receipt(&app, "reminder.complete", &metadata, || {
        let mut db = open(&app)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
        require_enabled(&tx)?;
        let key = reminder_key(false)?;
        let mut reminder = load_one(&tx, &key, &id)?;
        reminder.state = ReminderState::Completed;
        reminder.snoozed_until_unix_ms = None;
        reminder.updated_at_unix_ms = now.max(reminder.updated_at_unix_ms);
        save_record(&tx, &key, &reminder)?;
        tx.commit().map_err(|error| error.to_string())?;
        let output = audit_metadata("completed", &id, None);
        Ok((ReminderResponse::Reminder(reminder), output))
    })
}

fn snooze(app: AppHandle, id: String, until_unix_ms: i64) -> Result<ReminderResponse, String> {
    valid_id(&id)?;
    let now = now_unix_ms()?;
    if until_unix_ms <= now || until_unix_ms > now.saturating_add(MAX_SNOOZE_MS) {
        return Err("Snooze time must be in the future and within 31 days.".into());
    }
    let metadata = audit_metadata("snooze", &id, Some(until_unix_ms));
    crate::with_local_state_receipt(&app, "reminder.snooze", &metadata, || {
        let mut db = open(&app)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
        require_enabled(&tx)?;
        let key = reminder_key(false)?;
        let mut reminder = load_one(&tx, &key, &id)?;
        if reminder.state != ReminderState::Active { return Err("Completed reminders cannot be snoozed.".into()); }
        if until_unix_ms <= reminder.effective_due_at_unix_ms() {
            return Err("Snooze time must move the reminder later than its current due instant.".into());
        }
        reminder.snoozed_until_unix_ms = Some(until_unix_ms);
        reminder.updated_at_unix_ms = now.max(reminder.updated_at_unix_ms);
        save_record(&tx, &key, &reminder)?;
        tx.commit().map_err(|error| error.to_string())?;
        let output = audit_metadata("snoozed", &id, Some(until_unix_ms));
        Ok((ReminderResponse::Reminder(reminder), output))
    })
}

fn delete(app: AppHandle, id: String) -> Result<ReminderResponse, String> {
    valid_id(&id)?;
    let metadata = audit_metadata("delete", &id, None);
    crate::with_local_state_receipt(&app, "reminder.delete", &metadata, || {
        let mut db = open(&app)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
        tx.execute("DELETE FROM local_reminders WHERE id=?1", params![id])
            .map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
        let output = audit_metadata("deleted", &id, None);
        Ok((ReminderResponse::Deleted, output))
    })
}

fn clear_all_records(tx: &Transaction<'_>) -> Result<usize, String> {
    tx.execute("DELETE FROM local_reminders", []).map_err(|error| error.to_string())
}

fn clear_all(app: AppHandle) -> Result<ReminderResponse, String> {
    let metadata = audit_metadata("clear_all", "all", None);
    crate::with_local_state_receipt(&app, "reminder.clear_all", &metadata, || {
        let mut db = open(&app)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
        let deleted = clear_all_records(&tx)?;
        tx.commit().map_err(|error| error.to_string())?;
        let output = serde_json::to_vec(&("cleared", deleted)).map_err(|error| error.to_string())?;
        Ok((ReminderResponse::Cleared { deleted }, output))
    })
}

fn manage_blocking(app: AppHandle, request: ReminderRequest) -> Result<ReminderResponse, String> {
    match request {
        ReminderRequest::GetSettings => Ok(ReminderResponse::Settings(settings_from_db(&open(&app)?)?)),
        ReminderRequest::SetEnabled { enabled } => {
            let metadata = serde_json::to_vec(&("settings", enabled)).map_err(|error| error.to_string())?;
            crate::with_local_state_receipt(&app, "reminder.settings.update", &metadata, || {
                let mut db = open(&app)?;
                let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
                tx.execute("UPDATE reminder_settings SET value=?1 WHERE key='enabled'", params![if enabled { "true" } else { "false" }])
                    .map_err(|error| error.to_string())?;
                tx.commit().map_err(|error| error.to_string())?;
                let settings = settings_from_db(&db)?;
                let output = serde_json::to_vec(&settings).map_err(|error| error.to_string())?;
                Ok((ReminderResponse::Settings(settings), output))
            })
        }
        ReminderRequest::List => {
            let db = open(&app)?;
            let count: i64 = db.query_row("SELECT COUNT(*) FROM local_reminders", [], |row| row.get(0))
                .map_err(|error| error.to_string())?;
            if count == 0 { return Ok(ReminderResponse::List(Vec::new())); }
            let key = reminder_key(false)?;
            Ok(ReminderResponse::List(load_all(&db, &key)?))
        }
        ReminderRequest::Create { title, notes, due_at_unix_ms, timezone } =>
            create(app, title, notes, due_at_unix_ms, timezone),
        ReminderRequest::Update { id, title, notes, due_at_unix_ms, timezone } =>
            update(app, id, title, notes, due_at_unix_ms, timezone),
        ReminderRequest::Complete { id } => complete(app, id),
        ReminderRequest::Snooze { id, until_unix_ms } => snooze(app, id, until_unix_ms),
        ReminderRequest::Delete { id } => delete(app, id),
        ReminderRequest::ClearAll => clear_all(app),
        ReminderRequest::Agenda { now_unix_ms, horizon_ms } => {
            if now_unix_ms <= 0 || !(0..=MAX_AGENDA_HORIZON_MS).contains(&horizon_ms) {
                return Err("Agenda time or horizon is invalid.".into());
            }
            let db = open(&app)?;
            if !settings_from_db(&db)?.enabled { return Ok(ReminderResponse::Agenda(Vec::new())); }
            let count: i64 = db.query_row("SELECT COUNT(*) FROM local_reminders", [], |row| row.get(0))
                .map_err(|error| error.to_string())?;
            if count == 0 { return Ok(ReminderResponse::Agenda(Vec::new())); }
            let key = reminder_key(false)?;
            let reminders = load_all(&db, &key)?;
            let agenda = upcoming_reminders(&reminders, now_unix_ms, horizon_ms)
                .into_iter().cloned().collect();
            Ok(ReminderResponse::Agenda(agenda))
        }
    }
}

#[tauri::command]
pub async fn manage_local_reminders(app: AppHandle, request: ReminderRequest) -> Result<ReminderResponse, String> {
    tauri::async_runtime::spawn_blocking(move || manage_blocking(app, request))
        .await.map_err(|error| format!("Reminder storage worker failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reminder() -> LocalReminder {
        LocalReminder {
            id: "r-test".into(),
            title: "Renew policy".into(),
            notes: Some("Check the renewal date".into()),
            due_at_unix_ms: 20_000,
            timezone: "Asia/Kolkata".into(),
            state: ReminderState::Active,
            snoozed_until_unix_ms: None,
            last_notified_due_at_unix_ms: None,
            created_at_unix_ms: 10_000,
            updated_at_unix_ms: 10_000,
        }
    }

    fn memory_db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        db
    }

    #[test]
    fn reminders_are_disabled_by_default_and_notification_delivery_is_off() {
        let db = memory_db();
        assert_eq!(settings_from_db(&db).unwrap(), ReminderSettings { enabled: false, notifications_enabled: false });
        assert!(require_enabled(&db).is_err());
    }

    #[test]
    fn encrypted_record_round_trips_and_binds_identifier() {
        let key = [7u8; 32];
        let item = reminder();
        let encrypted = encrypt_record(&key, &item).unwrap();
        assert!(encrypted.starts_with(CIPHER_PREFIX));
        assert!(!encrypted.contains("Renew policy"));
        assert!(!encrypted.contains("Check the renewal date"));
        assert_eq!(decrypt_record(&key, &item.id, &encrypted).unwrap(), item);
        assert!(decrypt_record(&key, "r-other", &encrypted).is_err());
    }

    #[test]
    fn ciphertext_tampering_fails_authentication() {
        let key = [7u8; 32];
        let encrypted = encrypt_record(&key, &reminder()).unwrap();
        let mut bytes = B64.decode(encrypted.strip_prefix(CIPHER_PREFIX).unwrap()).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        let tampered = format!("{CIPHER_PREFIX}{}", B64.encode(bytes));
        assert!(decrypt_record(&key, "r-test", &tampered).is_err());
    }

    #[test]
    fn database_persists_only_encrypted_reminder_payloads() {
        let mut db = memory_db();
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).unwrap();
        save_record(&tx, &[7u8; 32], &reminder()).unwrap();
        tx.commit().unwrap();
        let ciphertext: String = db.query_row("SELECT ciphertext FROM local_reminders WHERE id='r-test'", [], |row| row.get(0)).unwrap();
        assert!(!ciphertext.contains("Renew policy"));
        assert!(!ciphertext.contains("Check the renewal date"));
        assert_eq!(load_all(&db, &[7u8; 32]).unwrap(), vec![reminder()]);
    }

    #[test]
    fn record_limit_and_invalid_scope_are_enforced() {
        assert!(valid_id("../escape").is_err());
        assert!(valid_id(&"x".repeat(97)).is_err());
        let item = reminder();
        assert!(item.validate().is_ok());
        assert!(ensure_capacity(MAX_REMINDERS - 1).is_ok());
        assert!(ensure_capacity(MAX_REMINDERS).is_err());
        assert!(ensure_capacity(-1).is_err());
    }

    #[test]
    fn clear_all_removes_encrypted_rows_without_needing_the_vault_key() {
        let mut db = memory_db();
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).unwrap();
        tx.execute("INSERT INTO local_reminders(id,ciphertext) VALUES('r-corrupt','not-decryptable')", []).unwrap();
        assert_eq!(clear_all_records(&tx).unwrap(), 1);
        tx.commit().unwrap();
        let count: i64 = db.query_row("SELECT COUNT(*) FROM local_reminders", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn agenda_returns_only_active_valid_items_within_horizon() {
        let items = vec![reminder()];
        assert_eq!(upcoming_reminders(&items, 15_000, 10_000).len(), 1);
        assert!(upcoming_reminders(&items, 15_000, 1_000).is_empty());
    }
}
