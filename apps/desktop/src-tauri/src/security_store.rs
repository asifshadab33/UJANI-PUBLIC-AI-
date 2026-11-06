//! Durable security state for provenance receipts, peer replay protection and cloud budgets.
//! This module stores metadata only; it never stores prompts, outputs, API keys or peer private keys.
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager};
use ujani_core::{sha256_hex, validate_receipt, ExecutionReceipt};

const MAX_RECEIPTS: i64 = 10_000;
const MAX_REPLAY_ENTRIES: i64 = 4_096;
const MAX_NONCE_TTL_SECONDS: u64 = 300;
const DAILY_CLOUD_TOKEN_RESERVATION: i64 = 250_000;
const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Serialize)]
pub struct ReceiptLedgerStatus {
    pub entries: i64,
    pub valid: bool,
    pub head_sha256: String,
}

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
         CREATE TABLE IF NOT EXISTS peer_replay_nonces(
           nonce TEXT PRIMARY KEY,
           expires_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_peer_nonce_expiry ON peer_replay_nonces(expires_at);
         CREATE TABLE IF NOT EXISTS cloud_budget(
           utc_day INTEGER PRIMARY KEY,
           reserved_tokens INTEGER NOT NULL CHECK(reserved_tokens >= 0)
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
    let previous: Option<String> = tx.query_row(
        "SELECT record_hash FROM execution_receipts ORDER BY id DESC LIMIT 1", [], |r| r.get(0)
    ).optional().map_err(|e| e.to_string())?;
    let previous_hash = previous.unwrap_or_else(|| GENESIS_HASH.to_string());
    let record_hash = sha256_hex(format!("{previous_hash}:{receipt_json}").as_bytes());
    tx.execute(
        "INSERT INTO execution_receipts(task_id,receipt_json,previous_hash,record_hash) VALUES(?1,?2,?3,?4)",
        params![receipt.task_id, receipt_json, previous_hash, record_hash]
    ).map_err(|e| format!("Could not append execution receipt: {e}"))?;
    tx.execute(
        "DELETE FROM execution_receipts WHERE id NOT IN (SELECT id FROM execution_receipts ORDER BY id DESC LIMIT ?1)",
        params![MAX_RECEIPTS]
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(record_hash)
}

fn verify_receipts_db(db: &Connection) -> Result<ReceiptLedgerStatus, String> {
    let mut statement = db.prepare(
        "SELECT receipt_json,previous_hash,record_hash FROM execution_receipts ORDER BY id ASC"
    ).map_err(|e| e.to_string())?;
    let rows = statement.query_map([], |r| Ok((
        r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?
    ))).map_err(|e| e.to_string())?;
    let mut previous = GENESIS_HASH.to_string();
    let mut entries = 0_i64;
    let mut valid = true;
    for row in rows {
        let (receipt_json, previous_hash, record_hash) = row.map_err(|e| e.to_string())?;
        let parsed: ExecutionReceipt = match serde_json::from_str(&receipt_json) {
            Ok(value) => value,
            Err(_) => { valid = false; break; }
        };
        if validate_receipt(&parsed).is_err() || previous_hash != previous
            || sha256_hex(format!("{previous_hash}:{receipt_json}").as_bytes()) != record_hash {
            valid = false;
            break;
        }
        previous = record_hash;
        entries += 1;
    }
    Ok(ReceiptLedgerStatus { entries, valid, head_sha256: previous })
}

fn claim_nonce_db(db: &mut Connection, nonce: &str, expires_at: u64, now: u64) -> Result<(), String> {
    if nonce.len() != 32 || !nonce.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err("Peer nonce must be 32 lowercase hexadecimal characters.".into());
    }
    if expires_at <= now || expires_at.saturating_sub(now) > MAX_NONCE_TTL_SECONDS {
        return Err("Peer nonce expiry is invalid or exceeds the five-minute protocol limit.".into());
    }
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM peer_replay_nonces WHERE expires_at <= ?1", params![now as i64])
        .map_err(|e| e.to_string())?;
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM peer_replay_nonces", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count >= MAX_REPLAY_ENTRIES {
        return Err("Persistent peer replay store is full; new peer tasks are fail-closed.".into());
    }
    tx.execute(
        "INSERT INTO peer_replay_nonces(nonce,expires_at) VALUES(?1,?2)",
        params![nonce, expires_at as i64]
    ).map_err(|e| {
        if e.to_string().contains("UNIQUE") { "Peer task replay detected; nonce was already used.".into() }
        else { format!("Could not persist peer replay nonce: {e}") }
    })?;
    tx.commit().map_err(|e| e.to_string())
}

fn reserve_budget_db(db: &mut Connection, prompt_bytes: usize, completion_tokens: u16, now: u64) -> Result<CloudBudgetStatus, String> {
    if prompt_bytes == 0 || prompt_bytes > 32 * 1024 || completion_tokens == 0 || completion_tokens > 8_192 {
        return Err("Cloud token reservation request is outside the supported bounds.".into());
    }
    let day = (now / 86_400) as i64;
    let prompt_tokens = ((prompt_bytes as u64).saturating_add(1) / 2) as i64;
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

pub fn claim_peer_nonce(app: &AppHandle, nonce: &str, expires_at: u64, now: u64) -> Result<(), String> {
    claim_nonce_db(&mut open(app)?, nonce, expires_at, now)
}

pub fn reserve_cloud_tokens(app: &AppHandle, prompt_bytes: usize, completion_tokens: u16) -> Result<CloudBudgetStatus, String> {
    reserve_budget_db(&mut open(app)?, prompt_bytes, completion_tokens, now_seconds()?)
}

#[tauri::command]
pub fn append_execution_receipt(app: AppHandle, receipt: ExecutionReceipt) -> Result<String, String> {
    append_receipt_db(&mut open(&app)?, &receipt)
}

#[tauri::command]
pub fn verify_execution_receipt_ledger(app: AppHandle) -> Result<ReceiptLedgerStatus, String> {
    verify_receipts_db(&open(&app)?)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use ujani_core::{ExecutionLocation, ExecutionReceipt};

    fn db() -> Connection { let c = Connection::open_in_memory().unwrap(); initialize(&c).unwrap(); c }
    fn receipt(task_id: &str) -> ExecutionReceipt {
        ExecutionReceipt {
            schema_version: 1, task_id: task_id.into(), operation_id: "document.summarize".into(),
            location: ExecutionLocation::LocalCpu, adapter_id: "ollama.local".into(),
            adapter_version: "1.0.0".into(), model_id: Some("qwen2.5".into()),
            model_version: Some("local-runtime".into()), input_sha256: sha256_hex(b"input"),
            output_sha256: sha256_hex(b"output"), started_at_unix_ms: 1_000,
            completed_at_unix_ms: 2_000, user_approval_id: None,
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
        assert!(claim_nonce_db(&mut db, nonce, 200, 100).is_ok());
        assert!(claim_nonce_db(&mut db, nonce, 200, 100).is_err());
        assert!(claim_nonce_db(&mut db, nonce, 400, 201).is_ok());
    }

    #[test]
    fn cloud_budget_reserves_tokens_atomically_and_fails_closed() {
        let mut db = db();
        let status = reserve_budget_db(&mut db, 4_000, 8_192, 86_400).unwrap();
        assert_eq!(status.reserved_tokens, 9_192);
        assert!(reserve_budget_db(&mut db, 32 * 1024, 8_192, 86_400).is_err());
        let next_day = reserve_budget_db(&mut db, 100, 100, 172_800).unwrap();
        assert_eq!(next_day.reserved_tokens, 125);
    }
}
