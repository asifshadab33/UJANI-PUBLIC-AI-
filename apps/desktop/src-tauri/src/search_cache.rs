//! Encrypted, bounded metadata-only cache for repeated local file searches.
//! Cache entries are hints: callers must compare the current full directory inventory
//! and re-open every returned file before displaying a cached result.
use aes_gcm::{aead::{Aead, AeadCore, KeyInit, Payload}, Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use std::{fs, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager};
use zeroize::Zeroizing;

const KEYRING_SERVICE: &str = "org.ujani.publicai";
const KEYRING_USER: &str = "local-search-cache-v1";
const CIPHER_PREFIX: &str = "ujani-search:v1:";
const MAX_CACHE_ENTRIES: i64 = 50;
const MAX_CACHE_BYTES: i64 = 10 * 1024 * 1024;
const MAX_CACHE_ROW_BYTES: usize = 4 * 1024 * 1024;
const NEGATIVE_TTL_SECONDS: i64 = 24 * 60 * 60;
const ALLOWED_RETENTION_DAYS: [i64; 4] = [0, 7, 30, 90];

#[derive(Debug, Clone, Serialize)]
pub struct CacheSettings {
    pub enabled: bool,
    /// 0 means retain until the user clears the cache; storage remains bounded.
    pub retention_days: i64,
    pub max_entries: i64,
    pub max_bytes: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentSearch {
    pub query: String,
    pub created_at: i64,
}

fn now_seconds() -> Result<i64, String> {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .map_err(|error| format!("System clock is before the Unix epoch: {error}"))
}

fn open(app: &AppHandle) -> Result<Connection, String> {
    let mut dir: PathBuf = app.path().app_data_dir()
        .map_err(|error| format!("Could not locate UJANI app data directory: {error}"))?;
    fs::create_dir_all(&dir).map_err(|error| format!("Could not create UJANI app data directory: {error}"))?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).map_err(|error| error.to_string())?;
    }
    dir.push("local-search-cache.sqlite3");
    open_database(&dir)
}

fn open_database(path: &Path) -> Result<Connection, String> {
    let db = Connection::open(path).map_err(|error| format!("Could not open local search cache: {error}"))?;
    db.busy_timeout(std::time::Duration::from_secs(3)).map_err(|error| error.to_string())?;
    if let Err(error) = initialize(&db) {
        // Never destroy a cache written by a newer application schema. Recover only
        // from explicit corruption signals; other failures remain cache misses.
        if error.contains("newer than this UJANI build") {
            return Err(error);
        }
        let corrupt = error.contains("integrity check failed")
            || error.contains("malformed")
            || error.contains("not a database");
        if !corrupt { return Err(error); }
        drop(db);
        fs::remove_file(path).map_err(|remove_error| format!("Could not recover the local search cache after corruption ({error}): {remove_error}"))?;
        let recovered = Connection::open(path).map_err(|open_error| open_error.to_string())?;
        recovered.busy_timeout(std::time::Duration::from_secs(3)).map_err(|busy_error| busy_error.to_string())?;
        initialize(&recovered)?;
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|permission_error| permission_error.to_string())?;
        }
        return Ok(recovered);
    }
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|error| error.to_string())?;
    }
    Ok(db)
}

fn initialize(db: &Connection) -> Result<(), String> {
    let current_schema: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| format!("Could not read search-cache schema version: {error}"))?;
    if current_schema > 1 {
        return Err("Local search cache schema is newer than this UJANI build.".into());
    }
    db.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA secure_delete=ON;
         PRAGMA journal_mode=DELETE;
         PRAGMA auto_vacuum=FULL;
         CREATE TABLE IF NOT EXISTS cache_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
         INSERT OR IGNORE INTO cache_settings(key,value) VALUES('enabled','true');
         INSERT OR IGNORE INTO cache_settings(key,value) VALUES('retention_days','30');
         CREATE TABLE IF NOT EXISTS approved_scopes(
           scope_id TEXT PRIMARY KEY,
           encrypted_path TEXT NOT NULL,
           created_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS search_cache(
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           scope_id TEXT NOT NULL,
           request_key TEXT NOT NULL,
           inventory_ciphertext TEXT NOT NULL,
           payload_ciphertext TEXT NOT NULL,
           is_negative INTEGER NOT NULL CHECK(is_negative IN (0,1)),
           created_at INTEGER NOT NULL,
           last_access INTEGER NOT NULL,
           UNIQUE(scope_id,request_key),
           FOREIGN KEY(scope_id) REFERENCES approved_scopes(scope_id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_search_cache_lru ON search_cache(last_access ASC);
         CREATE INDEX IF NOT EXISTS idx_search_cache_scope ON search_cache(scope_id);
         CREATE TABLE IF NOT EXISTS recent_searches(
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           scope_id TEXT NOT NULL,
           request_key TEXT NOT NULL,
           encrypted_query TEXT NOT NULL,
           is_negative INTEGER NOT NULL CHECK(is_negative IN (0,1)),
           created_at INTEGER NOT NULL,
           last_access INTEGER NOT NULL,
           UNIQUE(scope_id,request_key),
           FOREIGN KEY(scope_id) REFERENCES approved_scopes(scope_id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_recent_searches_lru ON recent_searches(created_at DESC);"
    ).map_err(|error| format!("Could not initialise local search cache: {error}"))?;
    db.execute_batch("PRAGMA user_version=1;")
        .map_err(|error| format!("Could not update local search cache schema version: {error}"))?;
    let check: String = db.query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|error| format!("Could not verify local search cache: {error}"))?;
    if check != "ok" {
        return Err("Local search cache integrity check failed.".into());
    }
    Ok(())
}

fn settings_from_db(db: &Connection) -> Result<CacheSettings, String> {
    let enabled: String = db.query_row("SELECT value FROM cache_settings WHERE key='enabled'", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let retention: String = db.query_row("SELECT value FROM cache_settings WHERE key='retention_days'", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let retention_days = retention.parse::<i64>().unwrap_or(30);
    Ok(CacheSettings {
        enabled: enabled == "true",
        retention_days: if ALLOWED_RETENTION_DAYS.contains(&retention_days) { retention_days } else { 30 },
        max_entries: MAX_CACHE_ENTRIES,
        max_bytes: MAX_CACHE_BYTES,
    })
}

fn cache_key() -> Result<Zeroizing<[u8; 32]>, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|error| format!("OS credential vault unavailable: {error}"))?;
    match entry.get_password() {
        Ok(encoded) => {
            let encoded = Zeroizing::new(encoded);
            let mut decoded = Zeroizing::new(vec![0_u8; encoded.len()]);
            let length = B64.decode_slice(encoded.as_bytes(), &mut decoded[..])
                .map_err(|_| "Stored search-cache key is invalid.".to_string())?;
            decoded.truncate(length);
            let key: [u8; 32] = decoded.as_slice().try_into()
                .map_err(|_| "Stored search-cache key has an invalid length.".to_string())?;
            Ok(Zeroizing::new(key))
        }
        Err(keyring::Error::NoEntry) => {
            let key = Zeroizing::new(<[u8; 32]>::from(Aes256Gcm::generate_key(&mut OsRng)));
            let encoded = Zeroizing::new(B64.encode(*key));
            entry.set_password(encoded.as_str())
                .map_err(|error| format!("Could not protect search-cache key in the OS credential vault: {error}"))?;
            Ok(key)
        }
        Err(error) => Err(format!("Could not read search-cache key from the OS credential vault: {error}")),
    }
}

fn encrypt(key: &[u8; 32], value: &[u8], aad: &[u8]) -> Result<String, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "Invalid search-cache key.".to_string())?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher.encrypt(&nonce, Payload { msg: value, aad })
        .map_err(|_| "Could not encrypt local search-cache data.".to_string())?;
    let mut payload = nonce.to_vec();
    payload.extend_from_slice(&ciphertext);
    Ok(format!("{CIPHER_PREFIX}{}", B64.encode(payload)))
}

fn decrypt(key: &[u8; 32], value: &str, aad: &[u8]) -> Result<Vec<u8>, String> {
    let encoded = value.strip_prefix(CIPHER_PREFIX)
        .ok_or_else(|| "Search-cache entry has an unsupported encryption version.".to_string())?;
    let payload = B64.decode(encoded).map_err(|_| "Search-cache ciphertext is malformed.".to_string())?;
    if payload.len() < 28 { return Err("Search-cache ciphertext is incomplete.".into()); }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "Invalid search-cache key.".to_string())?;
    let (nonce, ciphertext) = payload.split_at(12);
    cipher.decrypt(Nonce::from_slice(nonce), Payload { msg: ciphertext, aad })
        .map_err(|_| "Search-cache authentication failed.".into())
}

fn scope_aad(scope_id: &str) -> Vec<u8> {
    format!("ujani|local-search-scope|v1|{scope_id}").into_bytes()
}

fn entry_aad(scope_id: &str, request_key: &str, kind: &str) -> Vec<u8> {
    format!("ujani|local-search-entry|v1|{scope_id}|{request_key}|{kind}").into_bytes()
}

fn scope_for_path(db: &mut Connection, key: &[u8; 32], root: &Path) -> Result<String, String> {
    let canonical = fs::canonicalize(root).map_err(|error| format!("Could not resolve selected folder: {error}"))?;
    let canonical_text = canonical.to_str().ok_or_else(|| "This folder path cannot be safely represented for encrypted caching; search will run without cache.".to_string())?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
    let mut statement = tx.prepare("SELECT scope_id,encrypted_path FROM approved_scopes")
        .map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|error| error.to_string())?;
    let mut found = None;
    for row in rows {
        let (scope_id, encrypted_path) = row.map_err(|error| error.to_string())?;
        if decrypt(key, &encrypted_path, &scope_aad(&scope_id)).ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .as_deref() == Some(canonical_text) {
            found = Some(scope_id);
            break;
        }
    }
    drop(statement);
    if let Some(scope_id) = found {
        tx.commit().map_err(|error| error.to_string())?;
        return Ok(scope_id);
    }
    let mut random = [0_u8; 16];
    OsRng.fill_bytes(&mut random);
    let scope_id = random.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    let encrypted_path = encrypt(key, canonical_text.as_bytes(), &scope_aad(&scope_id))?;
    tx.execute(
        "INSERT INTO approved_scopes(scope_id,encrypted_path,created_at) VALUES(?1,?2,?3)",
        params![scope_id, encrypted_path, now_seconds()?],
    ).map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(scope_id)
}

fn request_key(scope_id: &str, query: &str, search_pdf_text: bool) -> String {
    super::sha256_hex(format!(
        "UJANI-LOCAL-SEARCH-CACHE-V2|scope={scope_id}|query={query}|pdf_text={search_pdf_text}|provider=bounded-local-search|parser=2"
    ).as_bytes())
}

fn trim_cache(db: &Connection, settings: &CacheSettings, now: i64) -> Result<(), String> {
    if settings.retention_days > 0 {
        let cutoff = now.saturating_sub(settings.retention_days.saturating_mul(86_400));
        db.execute("DELETE FROM search_cache WHERE created_at < ?1", params![cutoff])
            .map_err(|error| error.to_string())?;
    }
    db.execute("DELETE FROM search_cache WHERE is_negative=1 AND created_at < ?1",
        params![now.saturating_sub(NEGATIVE_TTL_SECONDS)]).map_err(|error| error.to_string())?;
    if settings.retention_days > 0 {
        let cutoff = now.saturating_sub(settings.retention_days.saturating_mul(86_400));
        db.execute("DELETE FROM recent_searches WHERE created_at < ?1", params![cutoff])
            .map_err(|error| error.to_string())?;
    }
    db.execute("DELETE FROM recent_searches WHERE is_negative=1 AND created_at < ?1",
        params![now.saturating_sub(NEGATIVE_TTL_SECONDS)]).map_err(|error| error.to_string())?;
    db.execute("DELETE FROM recent_searches WHERE id NOT IN (SELECT id FROM recent_searches ORDER BY created_at DESC,id DESC LIMIT 20)", [])
        .map_err(|error| error.to_string())?;
    loop {
        let count: i64 = db.query_row("SELECT COUNT(*) FROM search_cache", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        let bytes: i64 = db.query_row(
            "SELECT COALESCE(SUM(length(inventory_ciphertext)+length(payload_ciphertext)),0) FROM search_cache",
            [], |row| row.get(0)
        ).map_err(|error| error.to_string())?;
        if count <= MAX_CACHE_ENTRIES && bytes <= MAX_CACHE_BYTES { break; }
        let oldest: Option<i64> = db.query_row(
            "SELECT id FROM search_cache ORDER BY last_access ASC,created_at ASC LIMIT 1",
            [], |row| row.get(0)
        ).optional().map_err(|error| error.to_string())?;
        match oldest {
            Some(id) => { db.execute("DELETE FROM search_cache WHERE id=?1", params![id]).map_err(|error| error.to_string())?; }
            None => break,
        }
    }
    db.execute("DELETE FROM approved_scopes WHERE scope_id NOT IN (SELECT DISTINCT scope_id FROM search_cache UNION SELECT DISTINCT scope_id FROM recent_searches)", [])
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Return a cached payload only when the full current metadata inventory matches exactly.
/// Any keyring, database, decryption or freshness failure is a cache miss at the call site.
pub fn load(
    app: &AppHandle,
    root: &Path,
    query: &str,
    search_pdf_text: bool,
    inventory_json: &str,
) -> Result<Option<String>, String> {
    if inventory_json.len() > MAX_CACHE_ROW_BYTES { return Ok(None); }
    let mut db = open(app)?;
    let settings = settings_from_db(&db)?;
    if !settings.enabled { return Ok(None); }
    trim_cache(&db, &settings, now_seconds()?)?;
    let key = cache_key()?;
    let scope_id = match scope_for_path(&mut db, &key, root) {
        Ok(value) => value,
        Err(error) if error.contains("cannot be safely represented") => return Ok(None),
        Err(error) => return Err(error),
    };
    let request_key = request_key(&scope_id, query, search_pdf_text);
    let row: Option<(i64, String, String, i64, i64)> = db.query_row(
        "SELECT id,inventory_ciphertext,payload_ciphertext,is_negative,created_at FROM search_cache WHERE scope_id=?1 AND request_key=?2",
        params![scope_id, request_key],
        |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))
    ).optional().map_err(|error| error.to_string())?;
    let Some((id, encrypted_inventory, encrypted_payload, is_negative, created_at)) = row else {
        return Ok(None);
    };
    let now = now_seconds()?;
    let expired_by_retention = settings.retention_days > 0
        && created_at < now.saturating_sub(settings.retention_days.saturating_mul(86_400));
    let expired_negative = is_negative == 1 && created_at < now.saturating_sub(NEGATIVE_TTL_SECONDS);
    if expired_by_retention || expired_negative {
        db.execute("DELETE FROM search_cache WHERE id=?1", params![id]).map_err(|error| error.to_string())?;
        return Ok(None);
    }
    let stored_inventory = decrypt(&key, &encrypted_inventory, &entry_aad(&scope_id, &request_key, "inventory"))?;
    if stored_inventory != inventory_json.as_bytes() {
        db.execute("DELETE FROM search_cache WHERE id=?1", params![id]).map_err(|error| error.to_string())?;
        return Ok(None);
    }
    let payload = decrypt(&key, &encrypted_payload, &entry_aad(&scope_id, &request_key, "payload"))?;
    let payload = String::from_utf8(payload).map_err(|_| "Cached search result is not valid UTF-8.".to_string())?;
    db.execute("UPDATE search_cache SET last_access=?1 WHERE id=?2", params![now,id])
        .map_err(|error| error.to_string())?;
    Ok(Some(payload))
}

/// Persist a metadata-only result. Oversized, truncated or unavailable cache states are skipped.
pub fn store(
    app: &AppHandle,
    root: &Path,
    query: &str,
    search_pdf_text: bool,
    inventory_json: &str,
    payload_json: &str,
    is_negative: bool,
) -> Result<(), String> {
    if inventory_json.len() > MAX_CACHE_ROW_BYTES || payload_json.len() > MAX_CACHE_ROW_BYTES {
        return Ok(());
    }
    let mut db = open(app)?;
    let settings = settings_from_db(&db)?;
    if !settings.enabled { return Ok(()); }
    let key = cache_key()?;
    let scope_id = scope_for_path(&mut db, &key, root)?;
    let request_key = request_key(&scope_id, query, search_pdf_text);
    let encrypted_inventory = encrypt(&key, inventory_json.as_bytes(), &entry_aad(&scope_id, &request_key, "inventory"))?;
    let encrypted_payload = encrypt(&key, payload_json.as_bytes(), &entry_aad(&scope_id, &request_key, "payload"))?;
    let now = now_seconds()?;
    let encrypted_query = encrypt(&key, query.as_bytes(), &entry_aad(&scope_id, &request_key, "query"))?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
    tx.execute(
        "INSERT INTO search_cache(scope_id,request_key,inventory_ciphertext,payload_ciphertext,is_negative,created_at,last_access)
         VALUES(?1,?2,?3,?4,?5,?6,?6)
         ON CONFLICT(scope_id,request_key) DO UPDATE SET
           inventory_ciphertext=excluded.inventory_ciphertext,
           payload_ciphertext=excluded.payload_ciphertext,
           is_negative=excluded.is_negative,
           created_at=excluded.created_at,
           last_access=excluded.last_access",
        params![scope_id,request_key,encrypted_inventory,encrypted_payload,if is_negative {1} else {0},now]
    ).map_err(|error| error.to_string())?;
    tx.execute(
        "INSERT INTO recent_searches(scope_id,request_key,encrypted_query,is_negative,created_at,last_access)
         VALUES(?1,?2,?3,?4,?5,?5)
         ON CONFLICT(scope_id,request_key) DO UPDATE SET
           encrypted_query=excluded.encrypted_query,
           is_negative=excluded.is_negative,
           created_at=excluded.created_at,
           last_access=excluded.last_access",
        params![scope_id,request_key,encrypted_query,if is_negative {1} else {0},now]
    ).map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    trim_cache(&db, &settings, now)
}

#[tauri::command]
pub fn get_local_search_cache_settings(app: AppHandle) -> Result<CacheSettings, String> {
    let db = open(&app)?;
    let settings = settings_from_db(&db)?;
    trim_cache(&db, &settings, now_seconds()?)?;
    Ok(settings)
}

#[tauri::command]
pub fn list_recent_local_searches(app: AppHandle) -> Result<Vec<RecentSearch>, String> {
    let db = open(&app)?;
    let settings = settings_from_db(&db)?;
    if !settings.enabled { return Ok(Vec::new()); }
    trim_cache(&db, &settings, now_seconds()?)?;
    let key = cache_key()?;
    let mut statement = db.prepare(
        "SELECT r.scope_id,r.request_key,r.encrypted_query,r.created_at,s.encrypted_path
         FROM recent_searches r JOIN approved_scopes s ON s.scope_id=r.scope_id
         ORDER BY r.created_at DESC,r.id DESC LIMIT 20"
    ).map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| Ok((
        row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?,
        row.get::<_,i64>(3)?, row.get::<_,String>(4)?
    ))).map_err(|error| error.to_string())?;
    let mut recent = Vec::new();
    for row in rows {
        let (scope_id, request_key, encrypted_query, created_at, encrypted_path) = row.map_err(|error| error.to_string())?;
        let Some(path) = decrypt(&key, &encrypted_path, &scope_aad(&scope_id)).ok()
            .and_then(|bytes| String::from_utf8(bytes).ok()) else { continue; };
        let path = PathBuf::from(path);
        if !path.is_dir() || fs::canonicalize(&path).ok().as_deref() != Some(path.as_path()) { continue; }
        let query = decrypt(&key, &encrypted_query, &entry_aad(&scope_id, &request_key, "query"))
            .ok().and_then(|bytes| String::from_utf8(bytes).ok());
        if let Some(query) = query {
            recent.push(RecentSearch { query, created_at });
        }
    }
    Ok(recent)
}

#[tauri::command]
pub fn set_local_search_cache_settings(
    app: AppHandle,
    enabled: bool,
    retention_days: i64,
) -> Result<CacheSettings, String> {
    if !ALLOWED_RETENTION_DAYS.contains(&retention_days) {
        return Err("Choose 7, 30, 90 days, or Until cleared.".into());
    }
    let db = open(&app)?;
    db.execute("UPDATE cache_settings SET value=?1 WHERE key='enabled'", params![if enabled {"true"} else {"false"}])
        .map_err(|error| error.to_string())?;
    db.execute("UPDATE cache_settings SET value=?1 WHERE key='retention_days'", params![retention_days.to_string()])
        .map_err(|error| error.to_string())?;
    if !enabled {
        db.execute("DELETE FROM search_cache", []).map_err(|error| error.to_string())?;
        db.execute("DELETE FROM recent_searches", []).map_err(|error| error.to_string())?;
        db.execute("DELETE FROM approved_scopes", []).map_err(|error| error.to_string())?;
    } else {
        let settings = settings_from_db(&db)?;
        trim_cache(&db, &settings, now_seconds()?)?;
    }
    settings_from_db(&db)
}

#[tauri::command]
pub fn clear_local_search_cache(app: AppHandle) -> Result<(), String> {
    let db = open(&app)?;
    db.execute("DELETE FROM search_cache", []).map_err(|error| error.to_string())?;
    db.execute("DELETE FROM recent_searches", []).map_err(|error| error.to_string())?;
    db.execute("DELETE FROM approved_scopes", []).map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> [u8; 32] {
        Aes256Gcm::generate_key(&mut OsRng).into()
    }

    #[test]
    fn encrypted_values_are_authenticated_and_context_bound() {
        let key = key();
        let aad = entry_aad("scope-a", "request-a", "payload");
        let encrypted = encrypt(&key, b"private file metadata", &aad).unwrap();
        assert_eq!(decrypt(&key, &encrypted, &aad).unwrap(), b"private file metadata");
        assert!(decrypt(&key, &encrypted, &entry_aad("scope-b", "request-a", "payload")).is_err());
        assert!(decrypt(&key, &encrypted, &entry_aad("scope-a", "request-a", "inventory")).is_err());
        let query_aad = entry_aad("scope-a", "request-a", "query");
        let encrypted_query = encrypt(&key, b"private search phrase", &query_aad).unwrap();
        assert_eq!(decrypt(&key, &encrypted_query, &query_aad).unwrap(), b"private search phrase");
        assert!(decrypt(&key, &encrypted_query, &entry_aad("scope-b", "request-a", "query")).is_err());
    }

    #[test]
    fn cache_keys_are_isolated_by_scope_and_search_preferences() {
        let first = request_key("scope-a", "invoice", false);
        assert_ne!(first, request_key("scope-b", "invoice", false));
        assert_ne!(first, request_key("scope-a", "invoice", true));
        assert_ne!(first, request_key("scope-a", "contract", false));
    }

    #[test]
    fn retention_choices_are_explicit_and_bounded() {
        assert_eq!(ALLOWED_RETENTION_DAYS, [0, 7, 30, 90]);
        assert_eq!(NEGATIVE_TTL_SECONDS, 86_400);
        assert_eq!(MAX_CACHE_ENTRIES, 50);
        assert_eq!(MAX_CACHE_BYTES, 10 * 1024 * 1024);
    }
}
