//! Conservative energy-aware placement policy for UJANI.
//! Pure policy logic only: it never wakes devices, reads sensors, transfers data,
//! reserves resources or launches workloads. Missing evidence fails closed.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevicePowerState { AlreadyRunning, Sleeping, PoweredOff }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerCandidate {
    pub worker_id: String,
    pub power_state: DevicePowerState,
    pub owner_opted_in: bool,
    pub within_approved_schedule: bool,
    pub interactive_user_present: bool,
    pub on_ac_power: bool,
    pub thermal_state_safe: bool,
    pub telemetry_fresh: bool,
    pub data_policy_allows: bool,
    /// Incremental worker energy above its normal-use baseline.
    pub incremental_device_energy_wh: u64,
    pub network_energy_wh: u64,
    pub retry_and_failure_energy_wh: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementInput {
    /// Energy for the equivalent successful workload on the central baseline.
    pub central_baseline_energy_wh: u64,
    pub equivalent_workload_verified: bool,
    pub candidates: Vec<WorkerCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerEstimate {
    pub worker_id: String,
    pub estimated_total_energy_wh: u64,
    pub estimated_net_savings_wh: i128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementDecision {
    SelectDistributedWorker(WorkerEstimate),
    KeepCentral(String),
    Reject(String),
}

pub fn plan_energy_aware_placement(input: &PlacementInput) -> PlacementDecision {
    if !input.equivalent_workload_verified {
        return PlacementDecision::Reject("equivalent workload and output quality have not been verified".into());
    }
    if input.central_baseline_energy_wh == 0 {
        return PlacementDecision::Reject("a measured or defensible central baseline is required".into());
    }

    let mut best: Option<WorkerEstimate> = None;
    for worker in &input.candidates {
        if worker.worker_id.trim().is_empty() || worker.worker_id.len() > 128 { continue; }
        if worker.power_state != DevicePowerState::AlreadyRunning
            || !worker.owner_opted_in
            || !worker.within_approved_schedule
            || worker.interactive_user_present
            || !worker.on_ac_power
            || !worker.thermal_state_safe
            || !worker.telemetry_fresh
            || !worker.data_policy_allows
        { continue; }

        let Some(total) = worker.incremental_device_energy_wh
            .checked_add(worker.network_energy_wh)
            .and_then(|v| v.checked_add(worker.retry_and_failure_energy_wh))
        else { continue; };
        if total >= input.central_baseline_energy_wh { continue; }

        let estimate = WorkerEstimate {
            worker_id: worker.worker_id.clone(),
            estimated_total_energy_wh: total,
            estimated_net_savings_wh: i128::from(input.central_baseline_energy_wh) - i128::from(total),
        };
        if best.as_ref().map(|b| estimate.estimated_total_energy_wh < b.estimated_total_energy_wh).unwrap_or(true) {
            best = Some(estimate);
        }
    }

    match best {
        Some(estimate) => PlacementDecision::SelectDistributedWorker(estimate),
        None => PlacementDecision::KeepCentral("no already-running, opted-in worker is safe and estimated to use less total energy".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn worker(id: &str, device_wh: u64, network_wh: u64) -> WorkerCandidate {
        WorkerCandidate {
            worker_id: id.into(), power_state: DevicePowerState::AlreadyRunning,
            owner_opted_in: true, within_approved_schedule: true,
            interactive_user_present: false, on_ac_power: true,
            thermal_state_safe: true, telemetry_fresh: true, data_policy_allows: true,
            incremental_device_energy_wh: device_wh, network_energy_wh: network_wh,
            retry_and_failure_energy_wh: 0,
        }
    }
    fn input(candidates: Vec<WorkerCandidate>) -> PlacementInput {
        PlacementInput { central_baseline_energy_wh: 100, equivalent_workload_verified: true, candidates }
    }
    #[test] fn selects_lowest_energy_safe_worker() {
        let result = plan_energy_aware_placement(&input(vec![worker("a",55,10),worker("b",25,5)]));
        assert_eq!(result, PlacementDecision::SelectDistributedWorker(WorkerEstimate {
            worker_id:"b".into(), estimated_total_energy_wh:30, estimated_net_savings_wh:70
        }));
    }
    #[test] fn never_wakes_sleeping_or_powered_off_devices() {
        let mut a=worker("sleep",1,1); a.power_state=DevicePowerState::Sleeping;
        let mut b=worker("off",1,1); b.power_state=DevicePowerState::PoweredOff;
        assert!(matches!(plan_energy_aware_placement(&input(vec![a,b])),PlacementDecision::KeepCentral(_)));
    }
    #[test] fn keeps_work_central_when_distributed_energy_is_not_lower() {
        assert!(matches!(plan_energy_aware_placement(&input(vec![worker("a",95,5)])),PlacementDecision::KeepCentral(_)));
    }
    #[test] fn rejects_unverified_comparison() {
        let mut r=input(vec![worker("a",10,5)]); r.equivalent_workload_verified=false;
        assert!(matches!(plan_energy_aware_placement(&r),PlacementDecision::Reject(_)));
    }
    #[test] fn excludes_user_activity_and_missing_consent() {
        let mut a=worker("active",1,1); a.interactive_user_present=true;
        let mut b=worker("no-consent",1,1); b.owner_opted_in=false;
        assert!(matches!(plan_energy_aware_placement(&input(vec![a,b])),PlacementDecision::KeepCentral(_)));
    }
    #[test] fn includes_network_and_retry_energy() {
        let mut a=worker("costly",20,20); a.retry_and_failure_energy_wh=70;
        assert!(matches!(plan_energy_aware_placement(&input(vec![a])),PlacementDecision::KeepCentral(_)));
    }
    #[test] fn rejects_missing_baseline_and_ignores_overflow() {
        let mut r=input(vec![worker("overflow",u64::MAX,1)]);
        assert!(matches!(plan_energy_aware_placement(&r),PlacementDecision::KeepCentral(_)));
        r.central_baseline_energy_wh=0;
        assert!(matches!(plan_energy_aware_placement(&r),PlacementDecision::Reject(_)));
    }
}
