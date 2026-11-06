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

pub struct MemoryReservation {
    task_id: String,
}

impl MemoryReservation {
    pub fn acquire(task_id: &str) -> Result<Self, String> {
        if task_id.is_empty() || task_id.len() > 96
            || !task_id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
            return Err("Invalid task identifier for resource reservation.".into());
        }
        let mut system = System::new();
        system.refresh_memory();
        let available_mb = system.available_memory() / (1024 * 1024);
        let mut active = reservations().lock().map_err(|_| "Local resource reservation registry is unavailable.".to_string())?;
        if active.contains_key(task_id) {
            return Err("This task already holds a local resource reservation.".into());
        }
        if !active.is_empty() {
            return Err("Another local inference task holds the device reservation; retry after it completes.".into());
        }
        if available_mb < MIN_FREE_MEMORY_MB {
            return Err(format!("Local inference requires at least {MIN_FREE_MEMORY_MB} MiB of currently available RAM; only {available_mb} MiB is available."));
        }
        active.insert(task_id.to_string(), MIN_FREE_MEMORY_MB);
        Ok(Self { task_id: task_id.to_string() })
    }

    pub fn reserved_memory_mb(&self) -> u64 { MIN_FREE_MEMORY_MB }
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
    fn reservation_is_exclusive_and_released_on_drop() {
        let id = format!("reservation-test-{}", std::process::id());
        let first = MemoryReservation::acquire(&id);
        if let Ok(guard) = first {
            assert_eq!(guard.reserved_memory_mb(), MIN_FREE_MEMORY_MB);
            assert!(MemoryReservation::acquire("second-task").is_err());
            drop(guard);
            assert!(MemoryReservation::acquire("second-task").is_ok());
        }
    }

    #[test]
    fn rejects_unsafe_task_identifiers() {
        assert!(MemoryReservation::acquire("../escape").is_err());
    }
}
