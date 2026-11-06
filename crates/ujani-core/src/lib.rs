//! Minimal, side-effect-free foundations for UJANI Public AI.
//! This crate intentionally does not execute tools, inspect hardware, or make network calls.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionPolicy {
    /// Never send task data outside this device.
    LocalOnly,
    /// A remote subtask requires a separate, explicit approval.
    AskBeforeExternal,
    /// Permit only explicitly allow-listed remote subtasks under user-defined limits.
    AllowApprovedExternal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceKind {
    Cpu,
    Memory,
    Gpu,
    Npu,
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub id: String,
    pub kind: ResourceKind,
    pub available: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionLocation {
    Local,
    External,
    NeedsApproval,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRequest {
    pub task_id: String,
    /// Registered operation identifier, never an arbitrary shell command.
    pub operation: String,
    pub required_capabilities: BTreeSet<String>,
    pub external_assistance_required: bool,
    pub user_approved_external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecision {
    pub location: ExecutionLocation,
    pub reason: &'static str,
}

/// Conservative routing decision. This is policy logic only, not an executor.
pub fn route_task(
    task: &TaskRequest,
    policy: &ExecutionPolicy,
    available_capabilities: &BTreeSet<String>,
    registered_operations: &BTreeSet<String>,
) -> RoutingDecision {
    if task.task_id.trim().is_empty() || task.operation.trim().is_empty() {
        return RoutingDecision {
            location: ExecutionLocation::Unsupported,
            reason: "task ID and operation must be non-empty",
        };
    }

    if !registered_operations.contains(&task.operation) {
        return RoutingDecision {
            location: ExecutionLocation::Unsupported,
            reason: "operation is not in the trusted local registry",
        };
    }

    if !task.external_assistance_required {
        if task.required_capabilities.is_subset(available_capabilities) {
            return RoutingDecision {
                location: ExecutionLocation::Local,
                reason: "required capabilities are available locally",
            };
        }
        return RoutingDecision {
            location: ExecutionLocation::Unsupported,
            reason: "required local capabilities are unavailable",
        };
    }

    match policy {
        ExecutionPolicy::LocalOnly => RoutingDecision {
            location: ExecutionLocation::Unsupported,
            reason: "local-only policy blocks external assistance",
        },
        ExecutionPolicy::AskBeforeExternal if !task.user_approved_external => {
            RoutingDecision {
                location: ExecutionLocation::NeedsApproval,
                reason: "external assistance requires explicit approval",
            }
        }
        ExecutionPolicy::AskBeforeExternal | ExecutionPolicy::AllowApprovedExternal => {
            if task.user_approved_external {
                RoutingDecision {
                    location: ExecutionLocation::External,
                    reason: "external subtask is explicitly approved",
                }
            } else {
                RoutingDecision {
                    location: ExecutionLocation::NeedsApproval,
                    reason: "external assistance has not been approved",
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(external: bool, approved: bool) -> TaskRequest {
        TaskRequest {
            task_id: "task-1".into(),
            operation: "pdf.extract_text".into(),
            required_capabilities: BTreeSet::from(["filesystem.read_scoped".into()]),
            external_assistance_required: external,
            user_approved_external: approved,
        }
    }

    fn registries() -> (BTreeSet<String>, BTreeSet<String>) {
        (
            BTreeSet::from(["filesystem.read_scoped".into()]),
            BTreeSet::from(["pdf.extract_text".into()]),
        )
    }

    #[test]
    fn local_task_stays_local() {
        let (caps, ops) = registries();
        let d = route_task(&task(false, false), &ExecutionPolicy::LocalOnly, &caps, &ops);
        assert_eq!(d.location, ExecutionLocation::Local);
    }

    #[test]
    fn local_only_blocks_remote_subtask() {
        let (caps, ops) = registries();
        let d = route_task(&task(true, true), &ExecutionPolicy::LocalOnly, &caps, &ops);
        assert_eq!(d.location, ExecutionLocation::Unsupported);
    }

    #[test]
    fn ask_policy_requires_approval() {
        let (caps, ops) = registries();
        let d = route_task(&task(true, false), &ExecutionPolicy::AskBeforeExternal, &caps, &ops);
        assert_eq!(d.location, ExecutionLocation::NeedsApproval);
    }

    #[test]
    fn approved_remote_subtask_can_be_routed() {
        let (caps, ops) = registries();
        let d = route_task(&task(true, true), &ExecutionPolicy::AskBeforeExternal, &caps, &ops);
        assert_eq!(d.location, ExecutionLocation::External);
    }

    #[test]
    fn unregistered_operation_is_rejected() {
        let (caps, _) = registries();
        let mut t = task(false, false);
        t.operation = "shell.run".into();
        let d = route_task(&t, &ExecutionPolicy::LocalOnly, &caps, &BTreeSet::new());
        assert_eq!(d.location, ExecutionLocation::Unsupported);
    }
}

/// Four-mode execution policy preview; no execution or networking occurs here.
pub mod router;
pub mod file_processor;
pub mod image_processor;
pub mod capability;
pub mod tool_registry;
pub mod impact;

pub use capability::{plan_local_task, registered_operations, CapabilityRecord, CapabilityState, OperationDescriptor, TaskPlan};
pub use file_processor::{analyze_text, FileAnalysis};
pub use image_processor::{inspect_image, ImageInfo};
pub use tool_registry::{builtin_tool_manifests, validate_tool_registry, ExecutionKind, NetworkAccess, ResourceBudget, ToolCategory, ToolManifest, ToolPermission};
pub use impact::{compare_energy_scenarios, EnergyAccountingError, EnergyAccountingInput, EnergyImpactEstimate};

pub mod task_graph;
pub use task_graph::{plan_task_graph, TaskGraph, TaskGraphPlan, TaskNode};

pub mod model_advisor;
pub use model_advisor::{recommend_local_model, InstalledModelSize, ModelFitEstimate, ModelRecommendation};

/// Signed and consent-scoped Worker/Partner task envelope primitives (no transport/enrollment).
pub mod peer_protocol;

pub mod placement;
pub use placement::{plan_energy_aware_placement, DevicePowerState, PlacementDecision, PlacementInput, WorkerCandidate, WorkerEstimate};
pub mod safety_policy;
pub use safety_policy::{evaluate_workload_intent, SafetyDecision, WorkloadIntent};

pub mod policy_gate;
pub use policy_gate::{evaluate_execution, DataClass, ExecutionRequest, ExecutionTarget, GateDecision, GateResult};
pub mod energy_telemetry;
pub use energy_telemetry::{aggregate_observations, validate_observation, DashboardTotals, EnergyObservation, MeasurementKind, ObservationError};


/// Fail-closed resource-aware scheduling policy; decisions do not execute or reserve work.
pub mod resource_scheduler;
pub use resource_scheduler::{plan_local_schedule, LocalPlacement, ResourceSnapshot, ScheduleDecision, TaskResourceRequest, ThermalState};

/// Typed, hash-bound execution provenance receipt contract.
pub mod execution_receipt;
pub use execution_receipt::{sha256_hex, validate_receipt, ExecutionLocation as ReceiptExecutionLocation, ExecutionReceipt, ReceiptError};
pub use execution_receipt::validate_model_identifier;
