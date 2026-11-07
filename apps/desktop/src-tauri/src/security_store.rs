//! Durable security state for provenance receipts, peer replay protection and cloud budgets.
//! This module stores metadata only; it never stores prompts, outputs, API keys or peer private keys.
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use ed25519_dalek::VerifyingKey;
use std::{fs, io::{BufRead, BufReader, Write}, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use std::collections::HashSet;
use ujani_core::peer_protocol::{ReplayGuard, SignedTaskEnvelope};
use ujani_core::registered_operations;
use ujani_core::{sha256_hex, validate_receipt, ExecutionReceipt};

const MAX_RECEIPTS: i64 = 10_000;
const KEEP_AFTER_ARCHIVE: i64 = 5_000;
const ARCHIVE_TRIGGER: i64 = MAX_RECEIPTS - 100;
const RECEIPT_SLOT_TTL_SECONDS: u64 = 900;
const MAX_REPLAY_ENTRIES: i64 = 4_096;
const MAX_NONCE_TTL_SECONDS: u64 = 300;
const DAILY_CLOUD_TOKEN_RESERVATION: i64 = 250_000;
const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Verification summary for the local hash-chained execution receipt ledger.
#[derive(Debug, Clone, Serialize)]
pub struct ReceiptLedgerStatus {
    pub entries: i64,
    pub valid: bool,
    pub head_sha256: String,
}

/// Metadata-only execution receipt shown in the local privacy/provenance panel.
#[derive(Debug, Clone, Serialize)]
pub struct ReceiptLedgerEntry {
    pub receipt: ExecutionReceipt,
    pub record_hash: String,
}

/// Current per-installation BYOK token reservation for the UTC day.
#[derive(Debug, Clone, Serialize)]
pub struct CloudBudgetStatus {
    pub utc_day: i64,
    pub reserved_tokens: i64,
    pub daily_limit_tokens: i64,
    pub remaining_tokens: i64,
}

fn now_seconds() -> Result<u64, String> {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs()).map_err(|e| format!("System clock is before Unix epoch: {e}"))
}

fn open(app: &AppHandle) -> Result<Connection, String> {
    let mut dir: PathBuf = app.path().app_data_dir()
        .map_err(|e| format!("Could not locate UJANI app data directory: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create UJANI app data directory: {e}"))?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("Could not secure UJANI app data directory: {e}"))?;
    }
    dir.push("security-state.sqlite3");
    let db = Connection::open(&dir).map_err(|e| format!("Could not open UJANI security store: {e}"))?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Could not secure UJANI security database: {e}"))?;
    }
    db.busy_timeout(std::time::Duration::from_secs(3)).map_err(|e| e.to_string())?;
    initialize(&db)?;
    Ok(db)
}

fn initialize(db: &Connection) -> Result<(), String> {
    db.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA secure_delete=ON;
         PRAGMA journal_mode=DELETE;
         CREATE TABLE IF NOT EXISTS execution_receipts(
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           task_id TEXT NOT NULL,
           receipt_json TEXT NOT NULL,
           previous_hash TEXT NOT NULL,
           record_hash TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_receipts_task ON execution_receipts(task_id);
         CREATE TABLE IF NOT EXISTS receipt_checkpoint(
           id INTEGER PRIMARY KEY CHECK(id = 1),
           head_hash TEXT NOT NULL,
           archived_entries INTEGER NOT NULL,
           archived_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS receipt_archive_index(
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           archive_file TEXT NOT NULL,
           previous_hash TEXT NOT NULL,
           head_hash TEXT NOT NULL,
           entries INTEGER NOT NULL,
           archived_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS receipt_slots(
           task_id TEXT PRIMARY KEY,
           reserved_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS peer_replay_nonces(
           peer_id TEXT NOT NULL,
           nonce TEXT NOT NULL,
           expires_at INTEGER NOT NULL,
           PRIMARY KEY(peer_id, nonce)
         );
         CREATE INDEX IF NOT EXISTS idx_peer_nonce_expiry ON peer_replay_nonces(expires_at);
         CREATE TABLE IF NOT EXISTS cloud_budget(
           utc_day INTEGER PRIMARY KEY,
           reserved_tokens INTEGER NOT NULL CHECK(reserved_tokens >= 0)
         );
         CREATE TABLE IF NOT EXISTS trusted_peers(
           peer_id TEXT PRIMARY KEY,
           public_key_hex TEXT NOT NULL,
           label TEXT NOT NULL,
           enrolled_at INTEGER NOT NULL,
           revoked_at INTEGER
         );"
    ).map_err(|e| format!("Could not initialise UJANI security store: {e}"))
}

fn append_receipt_db(db: &mut Connection, receipt: &ExecutionReceipt) -> Result<String, String> {
    validate_receipt(receipt).map_err(|e| format!("Execution receipt rejected: {e:?}"))?;
    let receipt_json = serde_json::to_string(receipt)
        .map_err(|e| format!("Could not encode execution receipt: {e}"))?;
    if receipt_json.len() > 16 * 1024 {
        return Err("Execution receipt exceeds the 16 KiB storage limit.".into());
    }
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM execution_receipts", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let reserved: i64 = tx.query_row("SELECT COUNT(*) FROM receipt_slots", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let owns_slot: i64 = tx.query_row("SELECT COUNT(*) FROM receipt_slots WHERE task_id=?1", params![receipt.task_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count >= MAX_RECEIPTS || (owns_slot == 0 && count.saturating_add(reserved) >= MAX_RECEIPTS) {
        return Err("Execution receipt ledger is full or has no reserved capacity; archive support is required before accepting more receipts.".into());
    }
    let previous: Option<String> = tx.query_row(
        "SELECT record_hash FROM execution_receipts ORDER BY id DESC LIMIT 1", [], |r| r.get(0)
    ).optional().map_err(|e| e.to_string())?;
    let checkpoint: Option<String> = tx.query_row(
        "SELECT head_hash FROM receipt_checkpoint WHERE id=1", [], |r| r.get(0)
    ).optional().map_err(|e| e.to_string())?;
    let previous_hash = previous.or(checkpoint).unwrap_or_else(|| GENESIS_HASH.to_string());
    let record_hash = sha256_hex(format!("{previous_hash}:{receipt_json}").as_bytes());
    tx.execute(
        "INSERT INTO execution_receipts(task_id,receipt_json,previous_hash,record_hash) VALUES(?1,?2,?3,?4)",
        params![receipt.task_id, receipt_json, previous_hash, record_hash]
    ).map_err(|e| format!("Could not append execution receipt: {e}"))?;
    tx.execute("DELETE FROM receipt_slots WHERE task_id=?1", params![receipt.task_id])
        .map_err(|e| format!("Could not consume execution receipt reservation: {e}"))?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(record_hash)
}

#[derive(Deserialize)]
struct ArchivedReceipt {
    previous_hash: String,
    record_hash: String,
    receipt: ExecutionReceipt,
}

fn verify_archive_index(
    db: &Connection, archive_dir: &Path,
) -> Result<(bool, i64, String), String> {
    let mut statement = db.prepare(
        "SELECT archive_file,previous_hash,head_hash,entries FROM receipt_archive_index ORDER BY id ASC"
    ).map_err(|e| e.to_string())?;
    let rows = statement.query_map([], |row| Ok((
        row.get::<_, String>(0)?, row.get::<_, String>(1)?,
        row.get::<_, String>(2)?, row.get::<_, i64>(3)?
    ))).map_err(|e| e.to_string())?;
    let mut previous = GENESIS_HASH.to_string();
    let mut total = 0_i64;
    let mut valid = true;
    for row in rows {
        let (archive_file, expected_previous, expected_head, expected_entries) =
            row.map_err(|e| e.to_string())?;
        if expected_previous != previous
            || !archive_file.starts_with("receipts-")
            || !archive_file.ends_with(".jsonl")
            || archive_file.contains("..")
            || !archive_file.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.')
            || Path::new(&archive_file).file_name().and_then(|name| name.to_str()) != Some(archive_file.as_str()) {
            valid = false;
            break;
        }
        let archive_path = archive_dir.join(&archive_file);
        let metadata = match fs::symlink_metadata(&archive_path) {
            Ok(metadata) => metadata, Err(_) => { valid = false; break; }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 {
            valid = false;
            break;
        }
        let file = match fs::File::open(&archive_path) {
            Ok(file) => file, Err(_) => { valid = false; break; }
        };
        let mut count = 0_i64;
        for line in BufReader::new(file).lines() {
            let line = match line { Ok(line) => line, Err(_) => { valid = false; break; } };
            let archived: ArchivedReceipt = match serde_json::from_str(&line) {
                Ok(row) => row, Err(_) => { valid = false; break; }
            };
            let json = serde_json::to_string(&archived.receipt)
                .map_err(|e| format!("Could not encode archived receipt: {e}"))?;
            if archived.previous_hash != previous
                || validate_receipt(&archived.receipt).is_err()
                || sha256_hex(format!("{}:{json}", archived.previous_hash).as_bytes()) != archived.record_hash {
                valid = false;
                break;
            }
            previous = archived.record_hash;
            count += 1;
        }
        if !valid || count != expected_entries || previous != expected_head {
            valid = false;
            break;
        }
        total = total.saturating_add(count);
    }
    Ok((valid, total, previous))
}

fn verify_receipts_db_with_archive(
    db: &Connection, archive_dir: Option<&Path>,
) -> Result<ReceiptLedgerStatus, String> {
    // Keep the checkpoint, archive index and retained rows in one SQLite snapshot.
    let _snapshot = db.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut statement = db.prepare(
        "SELECT receipt_json,previous_hash,record_hash FROM execution_receipts ORDER BY id ASC"
    ).map_err(|e| e.to_string())?;
    let rows = statement.query_map([], |r| Ok((
        r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?
    ))).map_err(|e| e.to_string())?;
    let checkpoint: Option<(String, i64)> = db.query_row(
        "SELECT head_hash,archived_entries FROM receipt_checkpoint WHERE id=1", [],
        |r| Ok((r.get(0)?, r.get(1)?))
    ).optional().map_err(|e| e.to_string())?;
    let (checkpoint_head, archived_entries) =
        checkpoint.unwrap_or_else(|| (GENESIS_HASH.to_string(), 0));
    let (mut previous, mut valid) = (checkpoint_head.clone(), true);
    if archived_entries > 0 {
        match archive_dir {
            Some(dir) => {
                let (archive_valid, count, head) = verify_archive_index(db, dir)?;
                valid = archive_valid && count == archived_entries && head == checkpoint_head;
            }
            None => valid = false,
        }
    } else {
        let archive_count: i64 = db.query_row("SELECT COUNT(*) FROM receipt_archive_index", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if archive_count != 0 || checkpoint_head != GENESIS_HASH { valid = false; }
    }
    let mut entries = 0_i64;
    for row in rows {
        let (receipt_json, previous_hash, record_hash) = row.map_err(|e| e.to_string())?;
        let parsed: ExecutionReceipt = match serde_json::from_str(&receipt_json) {
            Ok(value) => value, Err(_) => { valid = false; break; }
        };
        if validate_receipt(&parsed).is_err() || previous_hash != previous
            || sha256_hex(format!("{previous_hash}:{receipt_json}").as_bytes()) != record_hash {
            valid = false;
            break;
        }
        previous = record_hash;
        entries += 1;
    }
    Ok(ReceiptLedgerStatus {
        entries: entries.saturating_add(archived_entries),
        valid,
        head_sha256: previous,
    })
}

#[cfg(test)]
fn verify_receipts_db(db: &Connection) -> Result<ReceiptLedgerStatus, String> {
    verify_receipts_db_with_archive(db, None)
}

fn claim_nonce_tx(
    tx: &Transaction<'_>,
    peer_id: &str,
    nonce: &str,
    expires_at: u64,
    now: u64,
) -> Result<(), String> {
    if peer_id.len() != 64 || !peer_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Peer replay namespace requires a valid device fingerprint.".into());
    }
    if nonce.len() != 32 || !nonce.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err("Peer nonce must be 32 lowercase hexadecimal characters.".into());
    }
    if expires_at <= now || expires_at.saturating_sub(now) > MAX_NONCE_TTL_SECONDS + 30 {
        return Err("Peer nonce expiry is invalid or exceeds the five-minute protocol limit plus allowed 30-second clock skew.".into());
    }
    tx.execute("DELETE FROM peer_replay_nonces WHERE expires_at <= ?1", params![now as i64])
        .map_err(|e| e.to_string())?;
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM peer_replay_nonces", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count >= MAX_REPLAY_ENTRIES {
        return Err("Persistent peer replay store is full; new peer tasks are fail-closed.".into());
    }
    tx.execute(
        "INSERT INTO peer_replay_nonces(peer_id,nonce,expires_at) VALUES(?1,?2,?3)",
        params![peer_id.to_ascii_lowercase(), nonce, expires_at as i64]
    ).map_err(|e| {
        if e.to_string().contains("UNIQUE") { "Peer task replay detected; nonce was already used.".into() }
        else { format!("Could not persist peer replay nonce: {e}") }
    })?;
    Ok(())
}

#[cfg(test)]
fn claim_nonce_db(db: &mut Connection, peer_id: &str, nonce: &str, expires_at: u64, now: u64) -> Result<(), String> {
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    claim_nonce_tx(&tx, peer_id, nonce, expires_at, now)?;
    tx.commit().map_err(|e| e.to_string())
}

fn reserve_budget_db(db: &mut Connection, prompt_bytes: usize, completion_tokens: u16, now: u64) -> Result<CloudBudgetStatus, String> {
    if prompt_bytes == 0 || prompt_bytes > 32 * 1024 || completion_tokens == 0 || completion_tokens > 8_192 {
        return Err("Cloud token reservation request is outside the supported bounds.".into());
    }
    let day = (now / 86_400) as i64;
    let prompt_tokens = prompt_bytes as i64;
    let reservation = prompt_tokens.saturating_add(completion_tokens as i64);
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    tx.execute("INSERT OR IGNORE INTO cloud_budget(utc_day,reserved_tokens) VALUES(?1,0)", params![day])
        .map_err(|e| e.to_string())?;
    let used: i64 = tx.query_row("SELECT reserved_tokens FROM cloud_budget WHERE utc_day=?1", params![day], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if used.saturating_add(reservation) > DAILY_CLOUD_TOKEN_RESERVATION {
        return Err(format!(
            "Daily cloud token safety limit reached ({used}/{DAILY_CLOUD_TOKEN_RESERVATION} reserved tokens). Try again after the UTC day resets."
        ));
    }
    let next = used.saturating_add(reservation);
    tx.execute("UPDATE cloud_budget SET reserved_tokens=?1 WHERE utc_day=?2", params![next, day])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(CloudBudgetStatus {
        utc_day: day,
        reserved_tokens: next,
        daily_limit_tokens: DAILY_CLOUD_TOKEN_RESERVATION,
        remaining_tokens: DAILY_CLOUD_TOKEN_RESERVATION - next,
    })
}

/// Atomically reserve a conservative local token allowance before a BYOK request is sent.
pub fn reserve_cloud_tokens(app: &AppHandle, prompt_bytes: usize, completion_tokens: u16) -> Result<CloudBudgetStatus, String> {
    reserve_budget_db(&mut open(app)?, prompt_bytes, completion_tokens, now_seconds()?)
}

/// Holds one durable ledger slot while a local or cloud inference is in flight.
pub struct ReceiptSlotReservation {
    app: AppHandle,
    task_id: String,
    consumed: bool,
}

impl ReceiptSlotReservation {
    /// Persist the completed receipt and atomically consume this reservation.
    pub fn persist(mut self, receipt: ExecutionReceipt) -> Result<String, String> {
        if receipt.task_id != self.task_id {
            return Err("Execution receipt task identifier does not match its reserved ledger slot.".into());
        }
        let result = append_execution_receipt(self.app.clone(), receipt)?;
        self.consumed = true;
        Ok(result)
    }
}

impl Drop for ReceiptSlotReservation {
    fn drop(&mut self) {
        if !self.consumed {
            let _ = release_receipt_slot(&self.app, &self.task_id);
        }
    }
}

#[cfg(windows)]
fn move_archive_write_through(from: &Path, to: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let from_wide: Vec<u16> = from.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let to_wide: Vec<u16> = to.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;
    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new_name: *const u16, flags: u32) -> i32;
    }
    // SAFETY: both paths are NUL-terminated UTF-16 buffers that remain alive for the call.
    let moved = unsafe { MoveFileExW(from_wide.as_ptr(), to_wide.as_ptr(), MOVEFILE_WRITE_THROUGH) };
    if moved == 0 {
        return Err(format!("Could not durably publish receipt archive: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}

fn ensure_receipt_archive_dir(archive_dir: &Path, nonce: u128) -> Result<bool, String> {
    if archive_dir.exists() { return Ok(false); }
    let parent = archive_dir.parent()
        .ok_or_else(|| "Receipt archive directory has no parent.".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("Could not create receipt archive parent: {e}"))?;
    #[cfg(windows)]
    {
        let temporary = parent.join(format!(".receipt-archive-{nonce}.tmp"));
        fs::create_dir(&temporary).map_err(|e| format!("Could not create temporary receipt archive directory: {e}"))?;
        if let Err(error) = move_archive_write_through(&temporary, archive_dir) {
            let _ = fs::remove_dir(&temporary);
            if !archive_dir.exists() { return Err(error); }
        }
    }
    #[cfg(not(windows))]
    fs::create_dir_all(archive_dir).map_err(|e| format!("Could not create receipt archive: {e}"))?;
    Ok(true)
}

#[cfg(unix)]
fn sync_receipt_archive_directories(archive_dir: &Path, parent: Option<&Path>) -> Result<(), String> {
    fs::File::open(archive_dir).and_then(|directory| directory.sync_all())
        .map_err(|e| format!("Could not fsync receipt archive directory: {e}"))?;
    if let Some(parent) = parent {
        fs::File::open(parent).and_then(|directory| directory.sync_all())
            .map_err(|e| format!("Could not fsync receipt archive parent directory: {e}"))?;
    }
    Ok(())
}

/// Archive the oldest rows to a fsynced JSONL file before deleting them. The
/// checkpoint keeps the retained chain verifiable across archive rotations.
fn archive_old_receipts(db: &mut Connection, archive_dir: &Path, now: u64) -> Result<i64, String> {
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM execution_receipts", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count <= KEEP_AFTER_ARCHIVE { return Ok(0); }
    let n = count - KEEP_AFTER_ARCHIVE;
    let previous_checkpoint: Option<String> = tx.query_row(
        "SELECT head_hash FROM receipt_checkpoint WHERE id=1", [], |r| r.get(0)
    ).optional().map_err(|e| e.to_string())?;
    let archive_previous = previous_checkpoint.unwrap_or_else(|| GENESIS_HASH.to_string());
    fs::create_dir_all(archive_dir).map_err(|e| format!("Could not create receipt archive: {e}"))?;
    let archive_nonce = SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos()).unwrap_or(now as u128);
    let _archive_dir_created = ensure_receipt_archive_dir(archive_dir, archive_nonce)?;
    let archive_name = format!("receipts-{now}-{count}-{archive_nonce}.jsonl");
    let path = archive_dir.join(&archive_name);
    let temporary_path = archive_dir.join(format!(".{archive_name}.tmp"));
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temporary_path)
        .map_err(|e| format!("Could not create temporary receipt archive file: {e}"))?;
    let mut head = String::new();
    {
        let mut query = tx.prepare(
            "SELECT receipt_json,previous_hash,record_hash FROM execution_receipts ORDER BY id ASC LIMIT ?1"
        ).map_err(|e| e.to_string())?;
        let rows = query.query_map(params![n], |r| Ok((
            r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?
        ))).map_err(|e| e.to_string())?;
        for row in rows {
            let (json, previous_hash, record_hash) = row.map_err(|e| e.to_string())?;
            writeln!(file, "{{\"previous_hash\":\"{previous_hash}\",\"record_hash\":\"{record_hash}\",\"receipt\":{json}}}")
                .map_err(|e| format!("Could not write receipt archive: {e}"))?;
            head = record_hash;
        }
    }
    file.sync_all().map_err(|e| format!("Could not fsync receipt archive: {e}"))?;
    drop(file);
    #[cfg(windows)]
    move_archive_write_through(&temporary_path, &path)?;
    #[cfg(not(windows))]
    fs::rename(&temporary_path, &path).map_err(|e| format!("Could not publish receipt archive: {e}"))?;
    #[cfg(unix)]
    sync_receipt_archive_directories(archive_dir, if _archive_dir_created { archive_dir.parent() } else { None })?;
    let archive_file = path.file_name().and_then(|name| name.to_str())
        .ok_or_else(|| "Receipt archive filename is not valid UTF-8.".to_string())?
        .to_string();
    tx.execute(
        "DELETE FROM execution_receipts WHERE id IN (SELECT id FROM execution_receipts ORDER BY id ASC LIMIT ?1)",
        params![n]
    ).map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO receipt_archive_index(archive_file,previous_hash,head_hash,entries,archived_at) VALUES(?1,?2,?3,?4,?5)",
        params![archive_file, archive_previous, head, n, now as i64]
    ).map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO receipt_checkpoint(id,head_hash,archived_entries,archived_at) VALUES(1,?1,?2,?3)
         ON CONFLICT(id) DO UPDATE SET head_hash=excluded.head_hash,
         archived_entries=receipt_checkpoint.archived_entries+excluded.archived_entries,
         archived_at=excluded.archived_at",
        params![head, n, now as i64]
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(n)
}

fn reserve_receipt_slot_db(
    db: &mut Connection, task_id: &str, now: u64, archive_dir: &Path,
) -> Result<(), String> {
    {
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM receipt_slots WHERE reserved_at <= ?1",
            params![now.saturating_sub(RECEIPT_SLOT_TTL_SECONDS) as i64])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    let (count, reserved): (i64, i64) = {
        let count = db.query_row("SELECT COUNT(*) FROM execution_receipts", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let reserved = db.query_row("SELECT COUNT(*) FROM receipt_slots", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        (count, reserved)
    };
    if count.saturating_add(reserved) >= ARCHIVE_TRIGGER {
        archive_old_receipts(db, archive_dir, now)?;
    }
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM execution_receipts", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let reserved: i64 = tx.query_row("SELECT COUNT(*) FROM receipt_slots", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count.saturating_add(reserved) >= MAX_RECEIPTS {
        return Err("Receipt archive did not free enough capacity; inference is paused to avoid losing provenance.".into());
    }
    tx.execute("INSERT INTO receipt_slots(task_id,reserved_at) VALUES(?1,?2)",
        params![task_id, now as i64])
        .map_err(|e| format!("Could not reserve execution receipt capacity: {e}"))?;
    tx.commit().map_err(|e| e.to_string())
}

fn release_receipt_slot_db(db: &Connection, task_id: &str) -> Result<(), String> {
    db.execute("DELETE FROM receipt_slots WHERE task_id=?1", params![task_id])
        .map(|_| ()).map_err(|e| format!("Could not release execution receipt reservation: {e}"))
}

fn release_receipt_slot(app: &AppHandle, task_id: &str) -> Result<(), String> {
    release_receipt_slot_db(&open(app)?, task_id)
}

fn receipt_archive_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let mut archive_dir = app.path().app_data_dir()
        .map_err(|e| format!("Could not locate receipt archive directory: {e}"))?;
    archive_dir.push("receipt-archive");
    Ok(archive_dir)
}

/// Reserve a durable receipt slot before inference. The reservation is released on drop
/// if the request fails, is cancelled, or never reaches successful receipt persistence.
pub fn reserve_execution_receipt_slot(
    app: &AppHandle,
    task_id: &str,
) -> Result<ReceiptSlotReservation, String> {
    if task_id.is_empty() || task_id.len() > 96
        || !task_id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
        return Err("Invalid task identifier for execution receipt reservation.".into());
    }
    let now = now_seconds()?;
    let mut db = open(app)?;
    let archive_dir = receipt_archive_dir(app)?;
    reserve_receipt_slot_db(&mut db, task_id, now, &archive_dir)?;
    Ok(ReceiptSlotReservation { app: app.clone(), task_id: task_id.to_string(), consumed: false })
}

/// Append a validated execution receipt to the local SQLite hash chain.
pub fn append_execution_receipt(app: AppHandle, receipt: ExecutionReceipt) -> Result<String, String> {
    append_receipt_db(&mut open(&app)?, &receipt)
}

/// Verify the local receipt hash chain and return its entry count and head hash.
#[tauri::command]
pub fn verify_execution_receipt_ledger(app: AppHandle) -> Result<ReceiptLedgerStatus, String> {
    let db = open(&app)?;
    let archive_dir = receipt_archive_dir(&app)?;
    verify_receipts_db_with_archive(&db, Some(&archive_dir))
}

/// Return the latest 50 metadata-only receipts, refusing to display an invalid ledger.
#[tauri::command]
pub fn list_execution_receipts(app: AppHandle) -> Result<Vec<ReceiptLedgerEntry>, String> {
    let db = open(&app)?;
    let archive_dir = receipt_archive_dir(&app)?;
    let status = verify_receipts_db_with_archive(&db, Some(&archive_dir))?;
    if !status.valid {
        return Err("Execution receipt ledger integrity check failed; receipts are hidden until the store is investigated.".into());
    }
    let mut statement = db.prepare(
        "SELECT receipt_json,record_hash FROM execution_receipts ORDER BY id DESC LIMIT 50"
    ).map_err(|e| e.to_string())?;
    let rows = statement.query_map([], |row| Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
    ))).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for row in rows {
        let (json, record_hash) = row.map_err(|e| e.to_string())?;
        let receipt: ExecutionReceipt = serde_json::from_str(&json)
            .map_err(|_| "Receipt ledger contains an unreadable entry.".to_string())?;
        validate_receipt(&receipt)
            .map_err(|_| "Receipt ledger contains an invalid entry.".to_string())?;
        entries.push(ReceiptLedgerEntry { receipt, record_hash });
    }
    Ok(entries)
}

/// Return the current UTC-day token reservation for this installation.
#[tauri::command]
pub fn get_cloud_budget_status(app: AppHandle) -> Result<CloudBudgetStatus, String> {
    let db = open(&app)?;
    let day = (now_seconds()? / 86_400) as i64;
    let used: i64 = db.query_row(
        "SELECT reserved_tokens FROM cloud_budget WHERE utc_day=?1", params![day], |r| r.get(0)
    ).optional().map_err(|e| e.to_string())?.unwrap_or(0);
    Ok(CloudBudgetStatus {
        utc_day: day,
        reserved_tokens: used,
        daily_limit_tokens: DAILY_CLOUD_TOKEN_RESERVATION,
        remaining_tokens: DAILY_CLOUD_TOKEN_RESERVATION - used,
    })
}


/// Public identity and local trust state for an enrolled UJANI peer.
#[derive(Debug, Clone, Serialize)]
pub struct TrustedPeer {
    pub peer_id: String,
    pub public_key_hex: String,
    pub label: String,
    pub enrolled_at: i64,
    pub revoked_at: Option<i64>,
}

fn decode_public_key(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Peer public key must be 32 bytes encoded as 64 hexadecimal characters.".into());
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let pair = std::str::from_utf8(chunk).map_err(|_| "Invalid public key encoding.")?;
        bytes[index] = u8::from_str_radix(pair, 16).map_err(|_| "Invalid public key encoding.")?;
    }
    Ok(bytes)
}

/// Enrol an Ed25519 peer key only after native device-owner confirmation.
#[tauri::command]
pub async fn enroll_trusted_peer(
    app: AppHandle,
    peer_id: String,
    public_key_hex: String,
    label: String,
) -> Result<TrustedPeer, String> {
    if label.trim().is_empty() || label.len() > 64 || label.chars().any(char::is_control) {
        return Err("Peer label must be 1–64 printable characters.".into());
    }
    if peer_id.len() != 64 || !peer_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid peer fingerprint.".into());
    }
    let public = decode_public_key(&public_key_hex)?;
    VerifyingKey::from_bytes(&public).map_err(|_| "Peer Ed25519 public key is invalid.".to_string())?;
    let expected_id = sha256_hex(&public);
    if peer_id.to_ascii_lowercase() != expected_id {
        return Err("Peer identifier does not match the supplied public key.".into());
    }
    let label = label.trim().to_string();
    let approved = app.dialog()
        .message(format!("Trust this device for UJANI peer tasks?\n\nDevice: {label}\nFingerprint: {expected_id}"))
        .title("Approve trusted device enrollment")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNo)
        .blocking_show();
    if !approved {
        return Err("Peer enrollment was not approved by the device owner.".into());
    }
    let now = now_seconds()? as i64;
    let db = open(&app)?;
    db.execute(
        "INSERT INTO trusted_peers(peer_id,public_key_hex,label,enrolled_at,revoked_at)
         VALUES(?1,?2,?3,?4,NULL)
         ON CONFLICT(peer_id) DO UPDATE SET public_key_hex=excluded.public_key_hex,label=excluded.label,revoked_at=NULL",
        params![expected_id, public_key_hex.to_ascii_lowercase(), label, now]
    ).map_err(|e| format!("Could not enrol trusted peer: {e}"))?;
    Ok(TrustedPeer { peer_id: expected_id, public_key_hex: public_key_hex.to_ascii_lowercase(), label, enrolled_at: now, revoked_at: None })

}

/// Check whether a peer fingerprint is currently enrolled and not revoked.
pub fn is_active_trusted_peer(app: &AppHandle, peer_id: &str) -> Result<bool, String> {
    if peer_id.len() != 64 || !peer_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid peer fingerprint.".into());
    }
    let db = open(app)?;
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM trusted_peers WHERE peer_id=?1 AND revoked_at IS NULL",
        params![peer_id.to_ascii_lowercase()], |row| row.get(0)
    ).map_err(|e| e.to_string())?;
    Ok(count == 1)
}

/// Revoke an enrolled peer after native device-owner confirmation.
#[tauri::command]
pub async fn revoke_trusted_peer(app: AppHandle, peer_id: String) -> Result<bool, String> {
    if peer_id.len() != 64 || !peer_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid peer identifier.".into());
    }
    let approved = app.dialog()
        .message(format!("Revoke this trusted UJANI device?\n\nFingerprint: {peer_id}"))
        .title("Confirm device revocation")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNo)
        .blocking_show();
    if !approved { return Err("Peer revocation was cancelled.".into()); }
    let changed = open(&app)?.execute(
        "UPDATE trusted_peers SET revoked_at=?1 WHERE peer_id=?2 AND revoked_at IS NULL",
        params![now_seconds()? as i64, peer_id.to_ascii_lowercase()]
    ).map_err(|e| format!("Could not revoke trusted peer: {e}"))?;
    Ok(changed == 1)
}

/// List enrolled peer public keys and their local revocation state.
#[tauri::command]
pub fn list_trusted_peers(app: AppHandle) -> Result<Vec<TrustedPeer>, String> {
    let db = open(&app)?;
    let mut statement = db.prepare(
        "SELECT peer_id,public_key_hex,label,enrolled_at,revoked_at FROM trusted_peers ORDER BY enrolled_at DESC"
    ).map_err(|e| e.to_string())?;
    let rows = statement.query_map([], |row| Ok(TrustedPeer {
        peer_id: row.get(0)?, public_key_hex: row.get(1)?, label: row.get(2)?,
        enrolled_at: row.get(3)?, revoked_at: row.get(4)?,
    })).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}


/// Verify a signed task from an enrolled, non-revoked peer and persist its nonce
/// before returning. This authenticates an envelope only; no peer transport or
/// Verify a recipient-bound signed peer envelope and atomically claim its replay nonce.
/// This validates authorization only; encrypted transport and remote task execution are not implemented.
#[tauri::command]
pub fn verify_and_claim_peer_envelope(
    app: AppHandle,
    peer_id: String,
    signed: SignedTaskEnvelope,
    payload: Vec<u8>,
) -> Result<(), String> {
    if peer_id.len() != 64 || !peer_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid peer identifier.".into());
    }
    if payload.len() > 10 * 1024 * 1024 {
        return Err("Peer payload exceeds the 10 MiB verification limit.".into());
    }
    let mut db = open(&app)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let public_key_hex: String = tx.query_row(
        "SELECT public_key_hex FROM trusted_peers WHERE peer_id=?1 AND revoked_at IS NULL",
        params![peer_id.to_ascii_lowercase()], |row| row.get(0)
    ).optional().map_err(|e| e.to_string())?
      .ok_or_else(|| "Peer is not enrolled or has been revoked.".to_string())?;
    let public = decode_public_key(&public_key_hex)?;
    let key = VerifyingKey::from_bytes(&public)
        .map_err(|_| "Stored peer public key is invalid.".to_string())?;
    if sha256_hex(&public) != peer_id.to_ascii_lowercase() {
        return Err("Stored peer identity fingerprint does not match its public key.".into());
    }
    let local_identity = crate::device_identity::get_or_create_device_identity(app.clone())?;
    if signed.envelope.recipient_device_id != local_identity.device_id {
        return Err("Signed peer task is addressed to a different UJANI device.".into());
    }
    signed.verify_payload(&payload).map_err(|e| format!("Peer payload rejected: {e:?}"))?;
    let operations = registered_operations().into_iter().map(|operation| operation.id).collect::<HashSet<_>>();
    let now = now_seconds()?;
    signed.verify(&key, now, &mut ReplayGuard::default(), &operations)
        .map_err(|e| format!("Peer task signature or policy rejected: {e:?}"))?;
    claim_nonce_tx(&tx, &peer_id, &signed.envelope.nonce, signed.envelope.expires_at, now)?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ujani_core::{ReceiptExecutionLocation, ExecutionReceipt};

    fn db() -> Connection { let c = Connection::open_in_memory().unwrap(); initialize(&c).unwrap(); c }
    fn receipt(task_id: &str) -> ExecutionReceipt {
        ExecutionReceipt {
            schema_version: 1, task_id: task_id.into(), operation_id: "document.summarize".into(),
            location: ReceiptExecutionLocation::LocalCpu, adapter_id: "ollama.local".into(),
            adapter_version: "1.0.0".into(), model_id: Some("qwen2.5".into()),
            model_version: Some("local-runtime".into()), input_sha256: sha256_hex(b"input"),
            output_sha256: sha256_hex(b"output"), started_at_unix_ms: 1_000,
            completed_at_unix_ms: 2_000, user_approval_id: None,
            network_activity: ujani_core::NetworkActivity::unknown(),
        }
    }

    #[test]
    fn receipt_ledger_appends_and_verifies_hash_chain() {
        let mut db = db();
        append_receipt_db(&mut db, &receipt("task-1")).unwrap();
        append_receipt_db(&mut db, &receipt("task-2")).unwrap();
        let status = verify_receipts_db(&db).unwrap();
        assert!(status.valid);
        assert_eq!(status.entries, 2);
        assert_ne!(status.head_sha256, GENESIS_HASH);
    }

    #[test]
    fn receipt_ledger_detects_row_tampering() {
        let mut db = db();
        append_receipt_db(&mut db, &receipt("task-1")).unwrap();
        db.execute("UPDATE execution_receipts SET receipt_json='{}' WHERE id=1", []).unwrap();
        assert!(!verify_receipts_db(&db).unwrap().valid);
    }

    #[test]
    fn persistent_replay_nonce_rejects_duplicate_and_prunes_expired() {
        let mut db = db();
        let nonce = "0123456789abcdef0123456789abcdef";
        let peer_a = "a".repeat(64);
        let peer_b = "b".repeat(64);
        assert!(claim_nonce_db(&mut db, &peer_a, nonce, 200, 100).is_ok());
        assert!(claim_nonce_db(&mut db, &peer_a, nonce, 200, 100).is_err());
        assert!(claim_nonce_db(&mut db, &peer_b, nonce, 200, 100).is_ok());
        let skew_nonce = "b".repeat(32);
        let too_far_nonce = "c".repeat(32);
        assert!(claim_nonce_db(&mut db, &peer_a, &skew_nonce, 430, 100).is_ok());
        assert!(claim_nonce_db(&mut db, &peer_a, &too_far_nonce, 431, 100).is_err());
        assert!(claim_nonce_db(&mut db, &peer_a, nonce, 400, 201).is_ok());
    }


    #[test]
    fn trusted_peer_public_key_must_be_exactly_32_bytes() {
        assert!(decode_public_key(&"a".repeat(64)).is_ok());
        assert!(decode_public_key("abc").is_err());
        assert!(decode_public_key(&"z".repeat(64)).is_err());
    }

    #[test]
    fn receipt_slot_reservations_are_atomic_and_expire_after_crashes() {
        let mut db = db();
        let archive_dir = std::env::temp_dir();
        assert!(reserve_receipt_slot_db(&mut db, "task-one", 1_000, &archive_dir).is_ok());
        assert!(reserve_receipt_slot_db(&mut db, "task-one", 1_001, &archive_dir).is_err());
        assert_eq!(db.query_row("SELECT COUNT(*) FROM receipt_slots", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
        assert!(release_receipt_slot_db(&db, "task-one").is_ok());
        assert!(reserve_receipt_slot_db(&mut db, "task-two", 2_000, &archive_dir).is_ok());
        assert!(reserve_receipt_slot_db(&mut db, "task-three", 2_901, &archive_dir).is_ok());
        assert_eq!(db.query_row("SELECT COUNT(*) FROM receipt_slots", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
    }

    #[test]
    fn receipt_archive_verifier_rejects_path_traversal_names() {
        let db = db();
        db.execute(
            "INSERT INTO receipt_archive_index(archive_file,previous_hash,head_hash,entries,archived_at) VALUES(?1,?2,?3,0,0)",
            params!["..", GENESIS_HASH, GENESIS_HASH]
        ).unwrap();
        let archive_dir = std::env::temp_dir().join("ujani-receipt-archive-path-test");
        let (valid, count, head) = verify_archive_index(&db, &archive_dir).unwrap();
        assert!(!valid);
        assert_eq!(count, 0);
        assert_eq!(head, GENESIS_HASH);
    }

    #[test]
    fn receipt_ledger_archives_old_rows_and_verifies_the_checkpoint_chain() {
        #[derive(serde::Deserialize)]
        struct ArchivedReceipt {
            previous_hash: String,
            record_hash: String,
            receipt: ExecutionReceipt,
        }

        let mut db = db();
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let archive_dir = std::env::temp_dir().join(format!("ujani-receipt-archive-test-{unique}"));
        let mut previous = GENESIS_HASH.to_string();
        for index in 0..ARCHIVE_TRIGGER {
            let item = receipt(&format!("old-{index}"));
            let json = serde_json::to_string(&item).unwrap();
            let hash = sha256_hex(format!("{previous}:{json}").as_bytes());
            db.execute(
                "INSERT INTO execution_receipts(task_id,receipt_json,previous_hash,record_hash) VALUES(?1,?2,?3,?4)",
                params![item.task_id, json, previous, hash]
            ).unwrap();
            previous = hash;
        }

        reserve_receipt_slot_db(&mut db, "task-new", 3_000, &archive_dir).unwrap();
        let retained: i64 = db.query_row("SELECT COUNT(*) FROM execution_receipts", [], |r| r.get(0)).unwrap();
        assert_eq!(retained, KEEP_AFTER_ARCHIVE);
        let status = verify_receipts_db_with_archive(&db, Some(&archive_dir)).unwrap();
        assert!(status.valid);
        assert_eq!(status.entries, ARCHIVE_TRIGGER);
        assert_eq!(status.head_sha256, previous);

        let archive_path = fs::read_dir(&archive_dir).unwrap().next().unwrap().unwrap().path();
        let archived_text = fs::read_to_string(&archive_path).unwrap();
        let mut archive_previous = GENESIS_HASH.to_string();
        let mut archived_count = 0_i64;
        for line in archived_text.lines() {
            let row: ArchivedReceipt = serde_json::from_str(line).unwrap();
            let json = serde_json::to_string(&row.receipt).unwrap();
            assert_eq!(row.previous_hash, archive_previous);
            assert_eq!(sha256_hex(format!("{}:{json}", row.previous_hash).as_bytes()), row.record_hash);
            archive_previous = row.record_hash;
            archived_count += 1;
        }
        assert_eq!(archived_count, ARCHIVE_TRIGGER - KEEP_AFTER_ARCHIVE);
        let checkpoint_head: String = db.query_row(
            "SELECT head_hash FROM receipt_checkpoint WHERE id=1", [], |row| row.get(0)
        ).unwrap();
        assert_eq!(archive_previous, checkpoint_head);

        append_receipt_db(&mut db, &receipt("task-new")).unwrap();
        assert!(verify_receipts_db_with_archive(&db, Some(&archive_dir)).unwrap().valid);
        fs::remove_dir_all(archive_dir).unwrap();
    }

    #[test]
    fn cloud_budget_reserves_tokens_atomically_and_fails_closed() {
        let mut db = db();
        let status = reserve_budget_db(&mut db, 4_000, 8_192, 86_400).unwrap();
        assert_eq!(status.reserved_tokens, 12_192);
        for _ in 0..5 { reserve_budget_db(&mut db, 32 * 1024, 8_192, 86_400).unwrap(); }
        assert!(reserve_budget_db(&mut db, 32 * 1024, 8_192, 86_400).is_err());
        let next_day = reserve_budget_db(&mut db, 100, 100, 172_800).unwrap();
        assert_eq!(next_day.reserved_tokens, 200);
    }
}
