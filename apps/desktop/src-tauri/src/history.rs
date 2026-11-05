//! Optional local SQLite task history. Recording is disabled by default.
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager};

const RETENTION_SECONDS: i64 = 30 * 24 * 60 * 60;
const MAX_ENTRIES: i64 = 100;
const MAX_PROMPT: usize = 32 * 1024;
const MAX_OUTPUT: usize = 2 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct HistoryEntry { pub id:i64, pub prompt:String, pub output:String, pub mode:String, pub model:String, pub kind:String, pub created_at:i64 }
#[derive(Debug, Serialize)]
pub struct HistorySettings { pub enabled:bool, pub retention_days:i64, pub max_entries:i64 }

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
 initialize(&db)?; Ok(db)
}
fn is_enabled(db:&Connection)->Result<bool,String>{
 let value:Option<String>=db.query_row("SELECT value FROM history_settings WHERE key='enabled'",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
 Ok(value.as_deref()==Some("true"))
}
fn settings(db:&Connection)->Result<HistorySettings,String>{Ok(HistorySettings{enabled:is_enabled(db)?,retention_days:30,max_entries:MAX_ENTRIES})}

fn save(db:&mut Connection,prompt:&str,output:&str,mode:&str,model:&str,kind:&str,created_at:i64)->Result<i64,String>{
 if !is_enabled(db)? { return Err("Task history is disabled.".into()); }
 if prompt.trim().is_empty()||prompt.len()>MAX_PROMPT{return Err("History prompt must be non-empty and at most 32 KiB.".into());}
 if output.trim().is_empty()||output.len()>MAX_OUTPUT{return Err("History output must be non-empty and at most 2 MiB.".into());}
 for (name,value,max) in [("mode",mode,32usize),("model",model,128usize),("kind",kind,32usize)] {if value.len()>max||value.chars().any(char::is_control){return Err(format!("Invalid history {name}."));}}
 let tx=db.transaction().map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO task_history(prompt,output,mode,model,kind,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![prompt,output,mode,model,kind,created_at]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("DELETE FROM task_history WHERE created_at < ?1",params![created_at.saturating_sub(RETENTION_SECONDS)]).map_err(|e|e.to_string())?;
 tx.execute("DELETE FROM task_history WHERE id NOT IN (SELECT id FROM task_history ORDER BY created_at DESC,id DESC LIMIT ?1)",params![MAX_ENTRIES]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?; Ok(id)
}
fn list(db:&Connection)->Result<Vec<HistoryEntry>,String>{
 let mut q=db.prepare("SELECT id,prompt,output,mode,model,kind,created_at FROM task_history ORDER BY created_at DESC,id DESC LIMIT ?1").map_err(|e|e.to_string())?;
 let rows=q.query_map(params![MAX_ENTRIES],|r|Ok(HistoryEntry{id:r.get(0)?,prompt:r.get(1)?,output:r.get(2)?,mode:r.get(3)?,model:r.get(4)?,kind:r.get(5)?,created_at:r.get(6)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}

#[tauri::command] pub fn get_task_history_settings(app:AppHandle)->Result<HistorySettings,String>{settings(&open(&app)?)}
#[tauri::command] pub fn set_task_history_enabled(app:AppHandle,enabled_value:bool)->Result<HistorySettings,String>{
 let db=open(&app)?; db.execute("UPDATE history_settings SET value=?1 WHERE key='enabled'",params![if enabled_value{"true"}else{"false"}]).map_err(|e|e.to_string())?; settings(&db)
}
#[tauri::command] pub fn save_task_history(app:AppHandle,prompt:String,output:String,mode:String,model:String,kind:String)->Result<i64,String>{
 save(&mut open(&app)?,&prompt,&output,&mode,&model,&kind,now()?)
}
#[tauri::command] pub fn list_task_history(app:AppHandle)->Result<Vec<HistoryEntry>,String>{
 let db=open(&app)?; db.execute("DELETE FROM task_history WHERE created_at < ?1",params![now()?.saturating_sub(RETENTION_SECONDS)]).map_err(|e|e.to_string())?; list(&db)
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
 #[test] fn disabled_by_default(){let mut c=db();assert!(!is_enabled(&c).unwrap());assert!(save(&mut c,"p","o","smart","m","chat",1).is_err());}
 #[test] fn saves_lists_and_deletes(){let mut c=db();c.execute("UPDATE history_settings SET value='true'",[]).unwrap();let id=save(&mut c,"p","o","smart","m","chat",100).unwrap();assert_eq!(list(&c).unwrap().len(),1);c.execute("DELETE FROM task_history WHERE id=?1",params![id]).unwrap();assert!(list(&c).unwrap().is_empty());}
 #[test] fn enforces_retention_and_limit(){let mut c=db();c.execute("UPDATE history_settings SET value='true'",[]).unwrap();save(&mut c,"old","o","smart","m","chat",1).unwrap();let t=RETENTION_SECONDS+100;for n in 0..(MAX_ENTRIES+2){save(&mut c,&format!("p{n}"),"o","smart","m","chat",t+n).unwrap();}let rows=list(&c).unwrap();assert_eq!(rows.len() as i64,MAX_ENTRIES);assert!(rows.iter().all(|r|r.prompt!="old"));}
 #[test] fn rejects_oversized_values(){let mut c=db();c.execute("UPDATE history_settings SET value='true'",[]).unwrap();assert!(save(&mut c,&"p".repeat(MAX_PROMPT+1),"o","smart","m","chat",1).is_err());assert!(save(&mut c,"p",&"o".repeat(MAX_OUTPUT+1),"smart","m","chat",1).is_err());}
}