//! Deterministic, fail-closed local resource scheduling policy.
//!
//! This module makes a point-in-time placement decision only. It does not
//! inspect hardware, reserve resources, start work, or enable background use.
//! Callers must obtain a fresh snapshot and enforce the returned decision in
//! the trusted native execution adapter.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThermalState {
    Normal,
    Warm,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalPlacement {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub device_id: String,
    pub observed_at_unix_ms: u64,
    pub max_age_ms: u64,
    pub logical_cpu_cores: u16,
    /// Estimated currently available CPU capacity in milli-cores (1000 = 1 core).
    pub available_cpu_milli_cores: u32,
    pub available_memory_mb: u64,
    pub gpu_runtime_compatible: bool,
    pub available_gpu_vram_mb: Option<u64>,
    pub thermal_state: ThermalState,
    pub interactive_user_active: bool,
    pub on_ac_power: bool,
    pub active_task_count: u16,
    pub max_parallel_tasks: u16,
    pub background_compute_opted_in: bool,
    pub background_requires_ac_power: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskResourceRequest {
    pub task_id: String,
    pub required_memory_mb: u64,
    pub minimum_cpu_milli_cores: u32,
    pub requires_gpu: bool,
    pub minimum_gpu_vram_mb: u64,
    pub allow_cpu_fallback: bool,
    pub background: bool,
    /// Reserved for deterministic queue ordering; 0 is lowest and 100 highest.
    pub priority: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum ScheduleDecision {
    RunLocal {
        task_id: String,
        placement: LocalPlacement,
        reason: String,
    },
    Defer {
        task_id: String,
        reason: String,
    },
    Reject {
        task_id: String,
        reason: String,
    },
}

fn reject(task_id: &str, reason: &str) -> ScheduleDecision {
    ScheduleDecision::Reject { task_id: task_id.to_string(), reason: reason.to_string() }
}

fn defer(task_id: &str, reason: &str) -> ScheduleDecision {
    ScheduleDecision::Defer { task_id: task_id.to_string(), reason: reason.to_string() }
}

/// Plan one local task against a caller-supplied resource snapshot.
///
/// Freshness is evaluated using the caller's current timestamp, making the
/// policy deterministic and straightforward to test. A successful decision
/// is not a reservation or execution authorization.
pub fn plan_local_schedule(
    request: &TaskResourceRequest,
    snapshot: &ResourceSnapshot,
    now_unix_ms: u64,
) -> ScheduleDecision {
    let id = request.task_id.as_str();
    if id.trim().is_empty()
        || id.len() > 96
        || !id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
    {
        return reject(id, "Task identifier is invalid.");
    }
    if request.required_memory_mb == 0
        || request.minimum_cpu_milli_cores == 0
        || request.priority > 100
        || (request.requires_gpu && request.minimum_gpu_vram_mb == 0)
    {
        return reject(id, "Task resource requirements are invalid.");
    }
    if snapshot.device_id.trim().is_empty()
        || snapshot.observed_at_unix_ms == 0
        || snapshot.max_age_ms == 0
        || now_unix_ms < snapshot.observed_at_unix_ms
        || now_unix_ms.saturating_sub(snapshot.observed_at_unix_ms) > snapshot.max_age_ms
    {
        return reject(id, "Resource telemetry is missing, from the future, or stale; refresh it before scheduling.");
    }
    if snapshot.logical_cpu_cores == 0
        || snapshot.max_parallel_tasks == 0
        || snapshot.thermal_state == ThermalState::Unknown
    {
        return reject(id, "Required device capability or safety telemetry is unknown.");
    }
    if snapshot.thermal_state == ThermalState::Critical {
        return defer(id, "Device thermal state is critical; retry after it cools.");
    }
    if snapshot.active_task_count >= snapshot.max_parallel_tasks {
        return defer(id, "The configured local task concurrency limit has been reached.");
    }
    if request.background {
        if !snapshot.background_compute_opted_in {
            return reject(id, "Background compute is disabled until the device owner explicitly opts in.");
        }
        if snapshot.interactive_user_active {
            return defer(id, "Background work is paused while the device owner is actively using the device.");
        }
        if snapshot.background_requires_ac_power && !snapshot.on_ac_power {
            return defer(id, "Background work is waiting for AC power under the owner's resource policy.");
        }
    }
    if snapshot.thermal_state == ThermalState::Warm {
        return defer(id, "Device is warm; defer new work until thermal conditions return to normal.");
    }
    if snapshot.available_memory_mb < request.required_memory_mb {
        return defer(id, "Available memory is below this task's declared minimum.");
    }
    if snapshot.available_cpu_milli_cores < request.minimum_cpu_milli_cores {
        return defer(id, "Available CPU capacity is below this task's declared minimum.");
    }

    if request.requires_gpu {
        let gpu_ready = snapshot.gpu_runtime_compatible
            && snapshot.available_gpu_vram_mb
                .is_some_and(|vram| vram >= request.minimum_gpu_vram_mb);
        if gpu_ready {
            return ScheduleDecision::RunLocal {
                task_id: id.to_string(),
                placement: LocalPlacement::Gpu,
                reason: "Fresh telemetry satisfies the declared GPU runtime, VRAM, CPU and memory requirements.".into(),
            };
        }
        if !request.allow_cpu_fallback {
            return reject(id, "A compatible GPU runtime with sufficient free VRAM is not verified, and CPU fallback is disabled.");
        }
    }

    ScheduleDecision::RunLocal {
        task_id: id.to_string(),
        placement: LocalPlacement::Cpu,
        reason: if request.requires_gpu {
            "GPU requirements were not verified; the task explicitly permits CPU fallback and local CPU/memory budgets are sufficient.".into()
        } else {
            "Fresh telemetry satisfies the declared local CPU and memory requirements.".into()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> ResourceSnapshot {
        ResourceSnapshot {
            device_id: "device-test".into(),
            observed_at_unix_ms: 10_000,
            max_age_ms: 5_000,
            logical_cpu_cores: 8,
            available_cpu_milli_cores: 4_000,
            available_memory_mb: 8_192,
            gpu_runtime_compatible: true,
            available_gpu_vram_mb: Some(4_096),
            thermal_state: ThermalState::Normal,
            interactive_user_active: false,
            on_ac_power: true,
            active_task_count: 0,
            max_parallel_tasks: 2,
            background_compute_opted_in: false,
            background_requires_ac_power: true,
        }
    }

    fn request() -> TaskResourceRequest {
        TaskResourceRequest {
            task_id: "task-01".into(),
            required_memory_mb: 1_024,
            minimum_cpu_milli_cores: 1_000,
            requires_gpu: false,
            minimum_gpu_vram_mb: 0,
            allow_cpu_fallback: false,
            background: false,
            priority: 50,
        }
    }

    #[test]
    fn schedules_local_cpu_when_budgets_fit() {
        assert!(matches!(plan_local_schedule(&request(), &snapshot(), 12_000),
            ScheduleDecision::RunLocal { placement: LocalPlacement::Cpu, .. }));
    }

    #[test]
    fn selects_gpu_only_with_compatible_runtime_and_vram() {
        let mut task = request();
        task.requires_gpu = true;
        task.minimum_gpu_vram_mb = 2_000;
        assert!(matches!(plan_local_schedule(&task, &snapshot(), 12_000),
            ScheduleDecision::RunLocal { placement: LocalPlacement::Gpu, .. }));
    }

    #[test]
    fn uses_cpu_fallback_only_when_task_allows_it() {
        let mut task = request();
        task.requires_gpu = true;
        task.minimum_gpu_vram_mb = 8_000;
        task.allow_cpu_fallback = true;
        assert!(matches!(plan_local_schedule(&task, &snapshot(), 12_000),
            ScheduleDecision::RunLocal { placement: LocalPlacement::Cpu, .. }));
        task.allow_cpu_fallback = false;
        assert!(matches!(plan_local_schedule(&task, &snapshot(), 12_000), ScheduleDecision::Reject { .. }));
    }

    #[test]
    fn rejects_stale_or_unknown_telemetry() {
        let mut device = snapshot();
        assert!(matches!(plan_local_schedule(&request(), &device, 15_001), ScheduleDecision::Reject { .. }));
        device.thermal_state = ThermalState::Unknown;
        assert!(matches!(plan_local_schedule(&request(), &device, 12_000), ScheduleDecision::Reject { .. }));
    }

    #[test]
    fn defers_on_thermal_memory_cpu_or_concurrency_limits() {
        let mut device = snapshot();
        device.thermal_state = ThermalState::Warm;
        assert!(matches!(plan_local_schedule(&request(), &device, 12_000), ScheduleDecision::Defer { .. }));
        device = snapshot();
        device.available_memory_mb = 100;
        assert!(matches!(plan_local_schedule(&request(), &device, 12_000), ScheduleDecision::Defer { .. }));
        device = snapshot();
        device.available_cpu_milli_cores = 100;
        assert!(matches!(plan_local_schedule(&request(), &device, 12_000), ScheduleDecision::Defer { .. }));
        device = snapshot();
        device.active_task_count = device.max_parallel_tasks;
        assert!(matches!(plan_local_schedule(&request(), &device, 12_000), ScheduleDecision::Defer { .. }));
    }

    #[test]
    fn background_work_requires_opt_in_and_respects_user_activity() {
        let mut task = request();
        task.background = true;
        assert!(matches!(plan_local_schedule(&task, &snapshot(), 12_000), ScheduleDecision::Reject { .. }));
        let mut device = snapshot();
        device.background_compute_opted_in = true;
        device.interactive_user_active = true;
        assert!(matches!(plan_local_schedule(&task, &device, 12_000), ScheduleDecision::Defer { .. }));
        device.interactive_user_active = false;
        device.on_ac_power = false;
        assert!(matches!(plan_local_schedule(&task, &device, 12_000), ScheduleDecision::Defer { .. }));
    }

    #[test]
    fn rejects_invalid_task_identifiers_and_resource_values() {
        let mut task = request();
        task.task_id = "../unsafe".into();
        assert!(matches!(plan_local_schedule(&task, &snapshot(), 12_000), ScheduleDecision::Reject { .. }));
        task = request();
        task.priority = 101;
        assert!(matches!(plan_local_schedule(&task, &snapshot(), 12_000), ScheduleDecision::Reject { .. }));
    }
}
