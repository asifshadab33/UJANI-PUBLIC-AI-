//! Workload-normalised energy accounting for UJANI.
//!
//! This is an accounting helper, not a meter or a carbon-credit calculator.
//! Inputs must come from measurements or clearly labelled estimates for the
//! same useful workload and quality level. A positive result is a scenario
//! estimate, not proof that a data centre physically reduced its electricity use.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyAccountingInput {
    /// Data-centre facility electricity for the equivalent baseline workload.
    /// Include IT and attributable facility overhead consistently.
    pub central_datacenter_total_wh: u64,
    /// Incremental electricity used by participating devices above their
    /// counterfactual idle/normal-use baseline.
    pub distributed_device_incremental_wh: u64,
    /// Electricity attributable to moving the workload data/results.
    pub network_energy_wh: u64,
    /// Additional facility/cooling energy attributable to the distributed run.
    pub additional_cooling_wh: u64,
    /// Energy spent on failed attempts, retries and checkpoint overhead.
    pub retry_and_failure_wh: u64,
    /// Must be true only when workload volume and output quality are comparable.
    pub equivalent_workload_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyImpactEstimate {
    pub estimated_datacenter_energy_avoided_wh: u64,
    pub estimated_distributed_system_energy_wh: u64,
    /// Positive means the distributed scenario used less energy; negative
    /// means it used more. This is deliberately not clamped to zero.
    pub estimated_net_energy_savings_wh: i128,
    pub distributed_scenario_is_lower_energy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnergyAccountingError {
    EquivalentWorkloadNotVerified,
    MissingBaselineMeasurement,
    EnergyTotalOverflow,
}

pub fn compare_energy_scenarios(
    input: &EnergyAccountingInput,
) -> Result<EnergyImpactEstimate, EnergyAccountingError> {
    if !input.equivalent_workload_verified {
        return Err(EnergyAccountingError::EquivalentWorkloadNotVerified);
    }
    if input.central_datacenter_total_wh == 0 {
        return Err(EnergyAccountingError::MissingBaselineMeasurement);
    }

    let distributed_total = input
        .distributed_device_incremental_wh
        .checked_add(input.network_energy_wh)
        .and_then(|total| total.checked_add(input.additional_cooling_wh))
        .and_then(|total| total.checked_add(input.retry_and_failure_wh))
        .ok_or(EnergyAccountingError::EnergyTotalOverflow)?;

    let net_savings = i128::from(input.central_datacenter_total_wh)
        - i128::from(distributed_total);

    Ok(EnergyImpactEstimate {
        estimated_datacenter_energy_avoided_wh: input.central_datacenter_total_wh,
        estimated_distributed_system_energy_wh: distributed_total,
        estimated_net_energy_savings_wh: net_savings,
        distributed_scenario_is_lower_energy: net_savings > 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> EnergyAccountingInput {
        EnergyAccountingInput {
            central_datacenter_total_wh: 1_000,
            distributed_device_incremental_wh: 400,
            network_energy_wh: 50,
            additional_cooling_wh: 25,
            retry_and_failure_wh: 25,
            equivalent_workload_verified: true,
        }
    }

    #[test]
    fn reports_net_savings_only_after_equivalent_work_is_verified() {
        let result = compare_energy_scenarios(&sample()).unwrap();
        assert_eq!(result.estimated_distributed_system_energy_wh, 500);
        assert_eq!(result.estimated_net_energy_savings_wh, 500);
        assert!(result.distributed_scenario_is_lower_energy);
    }

    #[test]
    fn honestly_reports_when_distributed_execution_uses_more_energy() {
        let mut input = sample();
        input.distributed_device_incremental_wh = 1_200;
        let result = compare_energy_scenarios(&input).unwrap();
        assert_eq!(result.estimated_net_energy_savings_wh, -300);
        assert!(!result.distributed_scenario_is_lower_energy);
    }

    #[test]
    fn rejects_unverified_workload_comparisons() {
        let mut input = sample();
        input.equivalent_workload_verified = false;
        assert_eq!(
            compare_energy_scenarios(&input),
            Err(EnergyAccountingError::EquivalentWorkloadNotVerified)
        );
    }

    #[test]
    fn rejects_missing_central_baseline() {
        let mut input = sample();
        input.central_datacenter_total_wh = 0;
        assert_eq!(
            compare_energy_scenarios(&input),
            Err(EnergyAccountingError::MissingBaselineMeasurement)
        );
    }

    #[test]
    fn rejects_overflow_in_distributed_energy_total() {
        let mut input = sample();
        input.distributed_device_incremental_wh = u64::MAX;
        input.network_energy_wh = 1;
        assert_eq!(
            compare_energy_scenarios(&input),
            Err(EnergyAccountingError::EnergyTotalOverflow)
        );
    }
}
