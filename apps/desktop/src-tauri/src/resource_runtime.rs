//! Process-local bounded admission and queueing for interactive UJANI inference.
//! The semaphore coordinates UJANI tasks only; OS memory observations remain advisory.
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, OnceLock,
};
use sysinfo::System;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

const MIN_FREE_MEMORY_MB: u64 = 1_024;
const MAX_WAITING_TASKS: usize = 4;
const MAX_QUEUE_WAIT_SECONDS: u64 = 600;
static LOCAL_EXECUTION: OnceLock<Arc<Semaphore>> = OnceLock::new();
static WAITING_TASKS: AtomicUsize = AtomicUsize::new(0);

fn semaphore() -> &'static Arc<Semaphore> {
    LOCAL_EXECUTION.get_or_init(|| Arc::new(Semaphore::new(1)))
}

/// Validates a task identifier before it is admitted to the local resource queue.
pub fn validate_task_id(task_id: &str) -> Result<(), String> {
    if task_id.is_empty() || task_id.len() > 96
        || !task_id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
    {
        return Err("Invalid task identifier for resource reservation.".into());
    }
    Ok(())
}

fn validate_available_memory(task_id: &str, available_mb: u64) -> Result<(), String> {
    if available_mb < MIN_FREE_MEMORY_MB {
        return Err(format!("Local inference requires at least {MIN_FREE_MEMORY_MB} MiB of currently available RAM; only {available_mb} MiB is available for task {task_id}."));
    }
    Ok(())
}

fn current_available_memory_mb() -> u64 {
    let mut system = System::new();
    system.refresh_memory();
    system.available_memory() / (1024 * 1024)
}

struct QueueTicket<'a> { counter: &'a AtomicUsize }

impl<'a> QueueTicket<'a> {
    fn reserve(counter: &'a AtomicUsize, maximum: usize) -> Result<Self, String> {
        let waiting = counter.fetch_add(1, Ordering::AcqRel);
        if waiting >= maximum {
            counter.fetch_sub(1, Ordering::AcqRel);
            return Err(format!("UJANI's local task queue is full ({maximum} waiting tasks). Wait for a task to finish and try again."));
        }
        Ok(Self { counter })
    }
}

impl Drop for QueueTicket<'_> {
    fn drop(&mut self) { self.counter.fetch_sub(1, Ordering::AcqRel); }
}

/// Holds the single local inference slot for the lifetime of a task.
pub struct MemoryReservation {
    _task_id: String,
    _permit: OwnedSemaphorePermit,
}

impl MemoryReservation {
    /// Immediate admission for synchronous, one-shot native calls.
    /// Attempts immediate admission for synchronous, one-shot native calls.
    pub fn acquire(task_id: &str) -> Result<Self, String> {
        validate_task_id(task_id)?;
        let permit = semaphore().clone().try_acquire_owned()
            .map_err(|_| "Another local inference task is using the device. Managed tasks are queued; retry this one after it completes.".to_string())?;
        validate_available_memory(task_id, current_available_memory_mb())?;
        Ok(Self { _task_id: task_id.to_string(), _permit: permit })
    }

    /// FIFO semaphore admission with a bounded waiting queue and cancellation.
    /// The caller registers the cancellation token before entering this method.
    pub async fn acquire_queued(task_id: &str, cancellation: &CancellationToken) -> Result<Self, String> {
        validate_task_id(task_id)?;
        if cancellation.is_cancelled() {
            return Err("Task cancelled while waiting for local device capacity.".into());
        }
        let _queue_ticket = QueueTicket::reserve(&WAITING_TASKS, MAX_WAITING_TASKS)?;
        let permit = tokio::select! {
            acquired = semaphore().clone().acquire_owned() => acquired
                .map_err(|_| "Local task queue is unavailable.".to_string())?,
            _ = cancellation.cancelled() => return Err("Task cancelled while waiting for local device capacity.".into()),
            _ = tokio::time::sleep(std::time::Duration::from_secs(MAX_QUEUE_WAIT_SECONDS)) => {
                return Err("Local task waited more than 10 minutes for device capacity and was removed from the queue.".into());
            }
        };
        if cancellation.is_cancelled() {
            return Err("Task cancelled while waiting for local device capacity.".into());
        }
        validate_available_memory(task_id, current_available_memory_mb())?;
        Ok(Self { _task_id: task_id.to_string(), _permit: permit })
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_available_memory, validate_task_id, QueueTicket, MIN_FREE_MEMORY_MB};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn admission_rejects_low_memory_and_invalid_identifiers() {
        assert!(validate_available_memory("task-1", MIN_FREE_MEMORY_MB - 1).is_err());
        assert!(validate_task_id("../escape").is_err());
        assert!(validate_task_id("").is_err());
        assert!(validate_available_memory("task-1", MIN_FREE_MEMORY_MB).is_ok());
    }

    #[tokio::test]
    async fn queued_task_can_be_cancelled_while_another_task_holds_the_device() {
        use super::{semaphore, MemoryReservation, WAITING_TASKS};
        use tokio_util::sync::CancellationToken;
        let active = semaphore().clone().acquire_owned().await.unwrap();
        let cancellation = CancellationToken::new();
        let cancel = cancellation.clone();
        let waiter = tokio::spawn(async move {
            MemoryReservation::acquire_queued("queued-task", &cancellation).await
        });
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while WAITING_TASKS.load(std::sync::atomic::Ordering::Acquire) != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("waiter must register in the queue before cancellation");
        cancel.cancel();
        let result = waiter.await.unwrap();
        assert!(result.unwrap_err().contains("cancelled"));
        assert_eq!(WAITING_TASKS.load(std::sync::atomic::Ordering::Acquire), 0);
        drop(active);
    }

    #[test]
    fn waiting_queue_is_bounded_and_slots_are_released() {
        let waiting = AtomicUsize::new(0);
        let first = QueueTicket::reserve(&waiting, 2).unwrap();
        let second = QueueTicket::reserve(&waiting, 2).unwrap();
        assert!(QueueTicket::reserve(&waiting, 2).is_err());
        drop(first);
        assert_eq!(waiting.load(Ordering::Acquire), 1);
        let third = QueueTicket::reserve(&waiting, 2).unwrap();
        drop(second);
        drop(third);
        assert_eq!(waiting.load(Ordering::Acquire), 0);
    }
}
