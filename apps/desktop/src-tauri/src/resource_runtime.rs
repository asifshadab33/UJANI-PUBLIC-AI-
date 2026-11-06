//! Process-local atomic reservations for interactive UJANI inference.
//! This is a conservative admission gate, not an OS memory limit: other processes
//! can consume RAM after the snapshot and model runtimes may need substantially more.
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use sysinfo::System;

const MIN_FREE_MEMORY_MB: u64 = 1_024;
static RESERVATIONS: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();

fn reservations() -> &'static Mutex<HashMap<String, u64>> {
    RESERVATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn validate_admission(task_id: &str, available_mb: u64, active: &HashMap<String, u64>) -> Result<(), String> {
    if task_id.is_empty() || task_id.len() > 96 || !task_id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
        return Err("Invalid task identifier for resource reservation.".into());
    }
    if active.contains_key(task_id) { return Err("This task already holds a local resource reservation.".into()); }
    if !active.is_empty() { return Err("Another local inference task holds the device reservation; retry after it completes.".into()); }
    if available_mb < MIN_FREE_MEMORY_MB {
        return Err(format!("Local inference requires at least {MIN_FREE_MEMORY_MB} MiB of currently available RAM; only {available_mb} MiB is available."));
    }
    Ok(())
}

pub struct MemoryReservation {
    task_id: String,
}

impl MemoryReservation {
    pub fn acquire(task_id: &str) -> Result<Self, String> {
        let mut system = System::new();
        system.refresh_memory();
        let available_mb = system.available_memory() / (1024 * 1024);
        let mut active = reservations().lock().map_err(|_| "Local resource reservation registry is unavailable.".to_string())?;
        validate_admission(task_id, available_mb, &active)?;
        active.insert(task_id.to_string(), MIN_FREE_MEMORY_MB);
        Ok(Self { task_id: task_id.to_string() })
    }
}

impl Drop for MemoryReservation {
    fn drop(&mut self) {
        if let Ok(mut active) = reservations().lock() {
            active.remove(&self.task_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admission_rejects_low_memory_and_invalid_identifiers() {
        let active = HashMap::new();
        assert!(validate_admission("task-1", MIN_FREE_MEMORY_MB - 1, &active).is_err());
        assert!(validate_admission("../escape", MIN_FREE_MEMORY_MB + 1, &active).is_err());
        assert!(validate_admission("", MIN_FREE_MEMORY_MB + 1, &active).is_err());
    }
    #[test]
    fn admission_is_exclusive_and_accepts_a_safe_request() {
        let active = HashMap::new();
        assert!(validate_admission("task-1", MIN_FREE_MEMORY_MB, &active).is_ok());
        let mut occupied = HashMap::new();
        occupied.insert("task-1".to_string(), MIN_FREE_MEMORY_MB);
        assert!(validate_admission("task-2", MIN_FREE_MEMORY_MB * 8, &occupied).is_err());
        assert!(validate_admission("task-1", MIN_FREE_MEMORY_MB * 8, &occupied).is_err());
    }
}
