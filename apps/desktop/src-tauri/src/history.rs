//! Optional local SQLite task history. Recording is disabled by default.
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager}; 
use aes_gcm::{aead::{Aead, AeadCore, KeyInit, Payload}, Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand::rngs::OsRng;
use zeroize::Zeroizing;

const KEYRING_SERVICE: &str = "org.ujani.publicai";
const KEYRING_USER: &str = "local-history-v1";
const ENCRYPTED_PREFIX: &str = "ujani:v1:";
const ENC_V2: &str = "ujani:v2:";


const RETENTION_SECONDS: i64 = 30 * 24 * 60 * 60;
const MAX_ENTRIES: i64 = 100;
const MAX_PROMPT: usize = 32 * 1024;
const MAX_OUTPUT: usize = 2 * 1024 * 1024;
const MAX_STORED_OUTPUT: usize = 64 * 1024;

#[derive(Debug, Serialize)]
pub struct HistoryEntry { pub id:i64, pub prompt:String, pub output:String, pub mode:String, pub model:String, pub kind:String, pub created_at:i64 }
#[derive(Debug, Serialize)]
pub struct HistorySettings { pub enabled:bool, pub retention_days:i64, pub max_entries:i64 }


fn history_key() -> Result<Zeroizing<[u8; 32]>, String> {
 let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
  .map_err(|e| format!("OS credential vault is unavailable; encrypted history is disabled: {e}"))?;
 match entry.get_password() {
  Ok(encoded) => {
   let encoded = Zeroizing::new(encoded);
   // Decode into caller-owned zeroizing storage so even a partially written
   // output buffer is erased if the decoder reports malformed base64.
   let mut decoded = Zeroizing::new(vec![0_u8; encoded.len()]);
   let decoded_len = B64.decode_slice(encoded.as_bytes(), &mut decoded[..])
       .map_err(|_| "Stored history key is invalid; clear the OS credential entry before continuing.".to_string())?;
   decoded.truncate(decoded_len);
   let key: [u8; 32] = decoded.as_slice().try_into().map_err(|_| "Stored history key has an invalid length.".to_string())?;
   Ok(Zeroizing::new(key))
  }
  Err(keyring::Error::NoEntry) => {
   let key = Zeroizing::new(<[u8; 32]>::from(Aes256Gcm::generate_key(&mut OsRng)));
   let encoded = Zeroizing::new(B64.encode(&*key));
   entry.set_password(encoded.as_str()).map_err(|e| format!("Could not store history key in the OS credential vault: {e}"))?;
   Ok(key)
  }
  Err(e) => Err(format!("Could not read history key from OS credential vault: {e}")),
 }
}

fn row_aad(id: i64, column: &str) -> Vec<u8> {
 format!("ujani|{id}|{column}").into_bytes()
}

fn encrypt_text(key: &[u8; 32], value: &str, id: i64, column: &str) -> Result<String, String> {
 let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "Invalid history encryption key.".to_string())?;
 let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
 let encrypted = cipher.encrypt(&nonce, Payload { msg: value.as_bytes(), aad: &row_aad(id, column) })
  .map_err(|_| "Could not encrypt history value.".to_string())?;
 let mut payload = nonce.to_vec(); payload.extend_from_slice(&encrypted);
 Ok(format!("{ENC_V2}{}", B64.encode(payload)))
}

fn decrypt_text(key: &[u8; 32], value: &str, id: i64, column: &str) -> Result<String, String> {
 let (encoded, version_two) = if let Some(v) = value.strip_prefix(ENC_V2) { (v, true) }
  else if let Some(v) = value.strip_prefix(ENCRYPTED_PREFIX) { (v, false) }
  else { return Err("History row is not encrypted; it must be migrated before use.".into()); };
 let payload = B64.decode(encoded).map_err(|_| "Encrypted history value is corrupted.".to_string())?;
 if payload.len() < 28 { return Err("Encrypted history value is incomplete.".into()); }
 let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "Invalid history encryption key.".to_string())?;
 let (nonce, ciphertext) = payload.split_at(12);
 let plaintext = if version_two {
  cipher.decrypt(Nonce::from_slice(nonce), Payload { msg: ciphertext, aad: &row_aad(id, column) })
 } else {
  cipher.decrypt(Nonce::from_slice(nonce), ciphertext)
 }.map_err(|_| "History authentication failed; the OS key may have changed or data was moved.".to_string())?;
 String::from_utf8(plaintext).map_err(|_| "Decrypted history text is not valid UTF-8.".into())
}

fn migrate_plaintext_rows(db: &Connection, key: &[u8; 32]) -> Result<(), String> {
 let mut statement = db.prepare("SELECT id,prompt,output FROM task_history").map_err(|e| format!("Could not inspect existing history: {e}"))?;
 let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))).map_err(|e| e.to_string())?;
 let mut pending = Vec::new();
 for row in rows {
  let (id, prompt, output) = row.map_err(|e| e.to_string())?;
  if !prompt.starts_with(ENC_V2) || !output.starts_with(ENC_V2) {
   let plain_prompt = if prompt.starts_with(ENCRYPTED_PREFIX) {
    match decrypt_text(key, &prompt, id, "prompt") { Ok(value) => value, Err(_) => continue }
   } else { prompt };
   let plain_output = if output.starts_with(ENCRYPTED_PREFIX) {
    match decrypt_text(key, &output, id, "output") { Ok(value) => value, Err(_) => continue }
   } else { output };
   pending.push((id, encrypt_text(key, &plain_prompt, id, "prompt")?, encrypt_text(key, &plain_output, id, "output")?));
  }
 }
 drop(statement);
 let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
 for (id, prompt, output) in pending {
  tx.execute("UPDATE task_history SET prompt=?1,output=?2 WHERE id=?3", params![prompt, output, id]).map_err(|e| e.to_string())?;
 }
 tx.commit().map_err(|e| e.to_string())
}

fn save_secure(db: &mut Connection, key: &[u8; 32], prompt: &str, output: &str, mode: &str, model: &str, kind: &str, created_at: i64) -> Result<i64, String> {
 if !is_enabled(db)? { return Err("Task history is disabled.".into()); }
 if prompt.trim().is_empty() || prompt.len() > MAX_PROMPT { return Err("History prompt must be non-empty and at most 32 KiB.".into()); }
 if output.trim().is_empty() || output.len() > MAX_OUTPUT { return Err("History output must be non-empty and at most 2 MiB.".into()); }
 for (name,value,max) in [("mode",mode,32usize),("model",model,128usize),("kind",kind,32usize)] {
  if value.len()>max || value.chars().any(char::is_control) { return Err(format!("Invalid history {name}.")); }
 }
 let stored_output = if output.len() > MAX_STORED_OUTPUT {
  let mut boundary=MAX_STORED_OUTPUT; while !output.is_char_boundary(boundary){boundary-=1;}
  format!("{}\n\n[History result truncated to 64 KiB.]",&output[..boundary])
 } else { output.to_string() };
 let tx=db.transaction().map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO task_history(prompt,output,mode,model,kind,created_at) VALUES('','',?1,?2,?3,?4)",params![mode,model,kind,created_at]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 let encrypted_prompt=encrypt_text(key,prompt,id,"prompt")?;
 let encrypted_output=encrypt_text(key,&stored_output,id,"output")?;
 tx.execute("UPDATE task_history SET prompt=?1,output=?2 WHERE id=?3",params![encrypted_prompt,encrypted_output,id]).map_err(|e|e.to_string())?;
 tx.execute("DELETE FROM task_history WHERE created_at < ?1",params![created_at.saturating_sub(RETENTION_SECONDS)]).map_err(|e|e.to_string())?;
 tx.execute("DELETE FROM task_history WHERE id NOT IN (SELECT id FROM task_history ORDER BY created_at DESC,id DESC LIMIT ?1)",params![MAX_ENTRIES]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;
 Ok(id)
}

fn list_secure(db: &Connection, key: &[u8; 32]) -> Result<Vec<HistoryEntry>, String> {
 let mut q=db.prepare("SELECT id,prompt,output,mode,model,kind,created_at FROM task_history ORDER BY created_at DESC,id DESC LIMIT ?1").map_err(|e|e.to_string())?;
 let rows=q.query_map(params![MAX_ENTRIES],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?))).map_err(|e|e.to_string())?;
 let mut entries=Vec::new();
 for row in rows {
  let (id,prompt,output,mode,model,kind,created_at)=row.map_err(|e|e.to_string())?;
  entries.push(HistoryEntry{id,prompt:decrypt_text(key,&prompt,id,"prompt").unwrap_or_else(|_| "[unreadable entry: key changed or data corrupted]".into()),output:decrypt_text(key,&output,id,"output").unwrap_or_else(|_| "[unreadable entry: key changed or data corrupted]".into()),mode,model,kind,created_at});
 }
 Ok(entries)
}

fn now() -> Result<i64,String> { SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_secs() as i64).map_err(|e|e.to_string()) }

fn initialize(db:&Connection)->Result<(),String>{
 db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA secure_delete=ON; PRAGMA journal_mode=DELETE;
 CREATE TABLE IF NOT EXISTS task_history(id INTEGER PRIMARY KEY AUTOINCREMENT,prompt TEXT NOT NULL,output TEXT NOT NULL,mode TEXT NOT NULL,model TEXT NOT NULL,kind TEXT NOT NULL,created_at INTEGER NOT NULL);
 CREATE INDEX IF NOT EXISTS idx_task_history_created_at ON task_history(created_at DESC);
 CREATE TABLE IF NOT EXISTS history_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
 INSERT OR IGNORE INTO history_settings(key,value) VALUES('enabled','false');")
 .map_err(|e|format!("Could not initialise local history: {e}"))
}
fn open(app:&AppHandle)->Result<Connection,String>{
 let mut dir:PathBuf=app.path().app_data_dir().map_err(|e|format!("Could not locate app data directory: {e}"))?;
 fs::create_dir_all(&dir).map_err(|e|format!("Could not create app data directory: {e}"))?;
 #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; fs::set_permissions(&dir,fs::Permissions::from_mode(0o700)).map_err(|e|e.to_string())?; }
 dir.push("task-history.sqlite3");
 let db=Connection::open(&dir).map_err(|e|format!("Could not open local history: {e}"))?;
 #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; fs::set_permissions(&dir,fs::Permissions::from_mode(0o600)).map_err(|e|e.to_string())?; }
 db.busy_timeout(std::time::Duration::from_secs(3)).map_err(|e|e.to_string())?;
 initialize(&db)?;
 Ok(db)
}
fn is_enabled(db:&Connection)->Result<bool,String>{
 let value:Option<String>=db.query_row("SELECT value FROM history_settings WHERE key='enabled'",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
 Ok(value.as_deref()==Some("true"))
}
fn settings(db:&Connection)->Result<HistorySettings,String>{Ok(HistorySettings{enabled:is_enabled(db)?,retention_days:30,max_entries:MAX_ENTRIES})}

#[cfg(test)]
fn save(db:&mut Connection,prompt:&str,output:&str,mode:&str,model:&str,kind:&str,created_at:i64)->Result<i64,String>{
 if !is_enabled(db)? { return Err("Task history is disabled.".into()); }
 if prompt.trim().is_empty()||prompt.len()>MAX_PROMPT{return Err("History prompt must be non-empty and at most 32 KiB.".into());}
 if output.trim().is_empty()||output.len()>MAX_OUTPUT{return Err("History output must be non-empty and at most 2 MiB.".into());}
 for (name,value,max) in [("mode",mode,32usize),("model",model,128usize),("kind",kind,32usize)] {if value.len()>max||value.chars().any(char::is_control){return Err(format!("Invalid history {name}."));}}
 let stored_output = if output.len() > MAX_STORED_OUTPUT {
  let mut boundary = MAX_STORED_OUTPUT;
  while !output.is_char_boundary(boundary) { boundary -= 1; }
  format!("{}\n\n[History result truncated to 64 KiB.]", &output[..boundary])
 } else { output.to_string() };
 let tx=db.transaction().map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO task_history(prompt,output,mode,model,kind,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![prompt,stored_output,mode,model,kind,created_at]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("DELETE FROM task_history WHERE created_at < ?1",params![created_at.saturating_sub(RETENTION_SECONDS)]).map_err(|e|e.to_string())?;
 tx.execute("DELETE FROM task_history WHERE id NOT IN (SELECT id FROM task_history ORDER BY created_at DESC,id DESC LIMIT ?1)",params![MAX_ENTRIES]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?; Ok(id)
}
#[cfg(test)]
fn list(db:&Connection)->Result<Vec<HistoryEntry>,String>{
 let mut q=db.prepare("SELECT id,prompt,output,mode,model,kind,created_at FROM task_history ORDER BY created_at DESC,id DESC LIMIT ?1").map_err(|e|e.to_string())?;
 let rows=q.query_map(params![MAX_ENTRIES],|r|Ok(HistoryEntry{id:r.get(0)?,prompt:r.get(1)?,output:r.get(2)?,mode:r.get(3)?,model:r.get(4)?,kind:r.get(5)?,created_at:r.get(6)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}

#[tauri::command] pub fn get_task_history_settings(app:AppHandle)->Result<HistorySettings,String>{settings(&open(&app)?)}
#[tauri::command] pub fn set_task_history_enabled(app:AppHandle,enabled_value:bool)->Result<HistorySettings,String>{
 let db=open(&app)?;
 if enabled_value { let key=history_key()?; migrate_plaintext_rows(&db,&key)?; }
 db.execute("UPDATE history_settings SET value=?1 WHERE key='enabled'",params![if enabled_value{"true"}else{"false"}]).map_err(|e|e.to_string())?;
 settings(&db)
}
#[tauri::command] pub fn save_task_history(app:AppHandle,prompt:String,output:String,mode:String,model:String,kind:String)->Result<i64,String>{
 let mut db=open(&app)?;
 if !is_enabled(&db)? { return Err("Task history is disabled.".into()); }
 let key=history_key()?; migrate_plaintext_rows(&db,&key)?; save_secure(&mut db,&key,&prompt,&output,&mode,&model,&kind,now()?)
}
#[tauri::command] pub fn list_task_history(app:AppHandle)->Result<Vec<HistoryEntry>,String>{
 let db=open(&app)?;
 // Fresh installs must not prompt for or initialize an OS keyring when history is off.
 if !is_enabled(&db)? { return Ok(Vec::new()); }
 let key=history_key()?; migrate_plaintext_rows(&db,&key)?;
 db.execute("DELETE FROM task_history WHERE created_at < ?1",params![now()?.saturating_sub(RETENTION_SECONDS)]).map_err(|e|e.to_string())?;
 list_secure(&db,&key)
}
#[tauri::command] pub fn delete_task_history(app:AppHandle,entry_id:i64)->Result<(),String>{
 if entry_id<=0{return Err("Invalid history entry identifier.".into());} open(&app)?.execute("DELETE FROM task_history WHERE id=?1",params![entry_id]).map_err(|e|e.to_string())?; Ok(())
}
#[tauri::command] pub fn clear_task_history(app:AppHandle)->Result<(),String>{
 open(&app)?.execute("DELETE FROM task_history",[]).map_err(|e|e.to_string())?; Ok(())
}

#[cfg(test)] mod tests{
 use super::*;
 fn db()->Connection{let c=Connection::open_in_memory().unwrap();initialize(&c).unwrap();c}
 fn test_key()->[u8;32]{Aes256Gcm::generate_key(&mut OsRng).into()}
 #[test] fn encrypted_history_round_trips_and_detects_tampering() {
  let key=test_key();
  let encrypted=encrypt_text(&key,"private prompt",1,"prompt").unwrap();
  assert!(encrypted.starts_with(ENC_V2));
  assert_eq!(decrypt_text(&key,&encrypted,1,"prompt").unwrap(),"private prompt");
  let tampered=format!("{}A",&encrypted);
  assert!(decrypt_text(&key,&tampered,1,"prompt").is_err());
 }
 #[test] fn ciphertext_cannot_be_moved_between_rows_or_columns() {
  let key=test_key();
  let encrypted=encrypt_text(&key,"secret",1,"prompt").unwrap();
  assert_eq!(decrypt_text(&key,&encrypted,1,"prompt").unwrap(),"secret");
  assert!(decrypt_text(&key,&encrypted,2,"prompt").is_err());
  assert!(decrypt_text(&key,&encrypted,1,"output").is_err());
 }
 #[test] fn one_bad_row_does_not_break_the_history_list() {
  let mut c=db(); c.execute("UPDATE history_settings SET value='true'",[]).unwrap();
  let key=test_key();
  c.execute("INSERT INTO task_history(prompt,output,mode,model,kind,created_at) VALUES('garbage','garbage','smart','m','chat',1)",[]).unwrap();
  save_secure(&mut c,&key,"good prompt","good output","smart","m","chat",2).unwrap();
  let entries=list_secure(&c,&key).unwrap();
  assert_eq!(entries.len(),2);
  assert!(entries.iter().any(|entry|entry.prompt=="[unreadable entry: key changed or data corrupted]"));
  assert!(entries.iter().any(|entry|entry.prompt=="good prompt"));
 }
 #[test] fn plaintext_history_rows_are_migrated() {
  let db=db();
  db.execute("INSERT INTO task_history(prompt,output,mode,model,kind,created_at) VALUES('old prompt','old output','smart','m','chat',1)",[]).unwrap();
  let key=test_key(); migrate_plaintext_rows(&db,&key).unwrap();
  let (p,o):(String,String)=db.query_row("SELECT prompt,output FROM task_history WHERE id=1",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
  assert!(p.starts_with(ENC_V2)); assert!(o.starts_with(ENC_V2));
  assert_eq!(decrypt_text(&key,&p,1,"prompt").unwrap(),"old prompt");
 }
 #[test] fn corrupt_v1_row_does_not_block_good_history_migration() {
  let db = db();
  db.execute("INSERT INTO task_history(prompt,output,mode,model,kind,created_at) VALUES('ujani:v1:%%%','ujani:v1:%%%','smart','m','chat',1)",[]).unwrap();
  db.execute("INSERT INTO task_history(prompt,output,mode,model,kind,created_at) VALUES('good prompt','good output','smart','m','chat',2)",[]).unwrap();
  let key = test_key();
  migrate_plaintext_rows(&db, &key).unwrap();
  let (bad_prompt, bad_output): (String, String) = db.query_row("SELECT prompt,output FROM task_history WHERE id=1", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
  assert_eq!(bad_prompt, "ujani:v1:%%%");
  assert_eq!(bad_output, "ujani:v1:%%%");
  let (good_prompt, good_output): (String, String) = db.query_row("SELECT prompt,output FROM task_history WHERE id=2", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
  assert!(good_prompt.starts_with(ENC_V2));
  assert!(good_output.starts_with(ENC_V2));
  let entries = list_secure(&db, &key).unwrap();
  assert_eq!(entries.len(), 2);
  assert!(entries.iter().any(|entry| entry.prompt == "[unreadable entry: key changed or data corrupted]"));
  assert!(entries.iter().any(|entry| entry.prompt == "good prompt"));
 }
 #[test] fn disabled_by_default(){let mut c=db();assert!(!is_enabled(&c).unwrap());assert!(save(&mut c,"p","o","smart","m","chat",1).is_err());}
 #[test] fn disabled_history_does_not_require_a_credential_key(){let c=db();assert!(!is_enabled(&c).unwrap());assert!(list(&c).unwrap().is_empty());}
 #[test] fn saves_lists_and_deletes(){let mut c=db();c.execute("UPDATE history_settings SET value='true'",[]).unwrap();let id=save(&mut c,"p","o","smart","m","chat",100).unwrap();assert_eq!(list(&c).unwrap().len(),1);c.execute("DELETE FROM task_history WHERE id=?1",params![id]).unwrap();assert!(list(&c).unwrap().is_empty());}
 #[test] fn enforces_retention_and_limit(){let mut c=db();c.execute("UPDATE history_settings SET value='true'",[]).unwrap();save(&mut c,"old","o","smart","m","chat",1).unwrap();let t=RETENTION_SECONDS+100;for n in 0..(MAX_ENTRIES+2){save(&mut c,&format!("p{n}"),"o","smart","m","chat",t+n).unwrap();}let rows=list(&c).unwrap();assert_eq!(rows.len() as i64,MAX_ENTRIES);assert!(rows.iter().all(|r|r.prompt!="old"));}
 #[test] fn stored_results_are_bounded(){let mut c=db();c.execute("UPDATE history_settings SET value='true'",[]).unwrap();let large="x".repeat(MAX_STORED_OUTPUT+100);save(&mut c,"p",&large,"smart","m","chat",1).unwrap();let row=list(&c).unwrap().remove(0);assert!(row.output.len()<=MAX_STORED_OUTPUT+64);assert!(row.output.contains("truncated to 64 KiB"));}
 #[test] fn rejects_oversized_values(){let mut c=db();c.execute("UPDATE history_settings SET value='true'",[]).unwrap();assert!(save(&mut c,&"p".repeat(MAX_PROMPT+1),"o","smart","m","chat",1).is_err());assert!(save(&mut c,"p",&"o".repeat(MAX_OUTPUT+1),"smart","m","chat",1).is_err());}
}