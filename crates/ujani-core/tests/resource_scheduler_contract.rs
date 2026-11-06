use ujani_core::{
    plan_local_schedule, LocalPlacement, ResourceSnapshot, ScheduleDecision,
    TaskResourceRequest, ThermalState,
};

fn device() -> ResourceSnapshot {
    ResourceSnapshot {
        device_id: "integration-test-device".into(),
        observed_at_unix_ms: 1_000,
        max_age_ms: 10_000,
        logical_cpu_cores: 4,
        available_cpu_milli_cores: 2_000,
        available_memory_mb: 4_096,
        gpu_runtime_compatible: false,
        available_gpu_vram_mb: None,
        thermal_state: ThermalState::Normal,
        interactive_user_active: false,
        on_ac_power: true,
        active_task_count: 0,
        max_parallel_tasks: 1,
        background_compute_opted_in: false,
        background_requires_ac_power: true,
    }
}

fn task() -> TaskResourceRequest {
    TaskResourceRequest {
        task_id: "integration-task".into(),
        required_memory_mb: 512,
        minimum_cpu_milli_cores: 500,
        requires_gpu: false,
        minimum_gpu_vram_mb: 0,
        allow_cpu_fallback: false,
        background: false,
        priority: 10,
    }
}

#[test]
fn public_scheduler_api_returns_local_cpu_plan() {
    assert!(matches!(
        plan_local_schedule(&task(), &device(), 2_000),
        ScheduleDecision::RunLocal { placement: LocalPlacement::Cpu, .. }
    ));
}

#[test]
fn public_scheduler_api_fails_closed_on_expired_snapshot() {
    assert!(matches!(
        plan_local_schedule(&task(), &device(), 11_001),
        ScheduleDecision::Reject { .. }
    ));
}

#[test]
fn public_scheduler_api_never_enables_background_compute_without_opt_in() {
    let mut request = task();
    request.background = true;
    assert!(matches!(
        plan_local_schedule(&request, &device(), 2_000),
        ScheduleDecision::Reject { .. }
    ));
}

#[test]
fn public_scheduler_api_defers_when_thermal_state_is_critical() {
    let mut snapshot = device();
    snapshot.thermal_state = ThermalState::Critical;
    assert!(matches!(
        plan_local_schedule(&task(), &snapshot, 2_000),
        ScheduleDecision::Defer { .. }
    ));
}
