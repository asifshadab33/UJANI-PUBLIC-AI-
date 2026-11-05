//! Four-mode policy router. Decisions only: this module never executes tasks or opens a network connection.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode { Fortress, Worker, Smart, Partner }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRequest {
    pub task_id: String,
    /// Registered operation identifier, never a shell command or arbitrary code.
    pub operation: String,
    pub mode: ExecutionMode,
    pub required_capabilities: BTreeSet<String>,
    pub available_capabilities: BTreeSet<String>,
    pub local_memory_required_mb: u64,
    pub available_memory_mb: u64,
    /// User's separate, explicit permission for this task to send data externally.
    pub external_data_approved: bool,
    /// Whether the app's external gateway/network capability is available.
    pub network_capability_available: bool,
    /// Maximum collaboration turns requested by the caller. Partner is capped at 3.
    pub requested_collaboration_rounds: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteAction { ExecuteLocally, AwaitExternalApproval, PrepareWorkerPlan, PreparePartnerPlan, Blocked }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeRoutingDecision {
    pub mode: ExecutionMode,
    pub action: RouteAction,
    pub allowed: bool,
    pub external_data_transfer_permitted: bool,
    pub collaboration_rounds: u8,
    pub reason: String,
    /// True because this policy preview is not an OS sandbox or network firewall.
    pub enforcement_boundary_required: bool,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 128 &&
        value.chars().all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c))
}

fn registered_operation(value: &str) -> bool {
    matches!(value, "local.ai.generate" | "local.word_count" | "file.analyze_text" | "image.inspect" | "document.summarize")
}

fn blocked(mode: ExecutionMode, reason: &str) -> ModeRoutingDecision {
    ModeRoutingDecision { mode, action: RouteAction::Blocked, allowed: false,
        external_data_transfer_permitted: false, collaboration_rounds: 0,
        reason: reason.into(), enforcement_boundary_required: true }
}

/// Evaluate a mode against declared capabilities and explicit per-task consent.
/// This is a policy preview only; callers must enforce the decision at execution time.
pub fn evaluate_mode(request: &RouteRequest) -> ModeRoutingDecision {
    let mode = request.mode;
    if !valid_id(&request.task_id) || !valid_id(&request.operation) {
        return blocked(mode, "Task ID and registered operation must be valid identifiers.");
    }
    if !registered_operation(&request.operation) {
        return blocked(mode, "Operation is not registered in the trusted local operation catalog.");
    }
    if request.local_memory_required_mb > request.available_memory_mb {
        return blocked(mode, "The task exceeds the available local memory budget.");
    }
    let local_ready = request.required_capabilities.is_subset(&request.available_capabilities);

    match mode {
        ExecutionMode::Fortress => {
            if local_ready {
                ModeRoutingDecision { mode, action: RouteAction::ExecuteLocally, allowed: true,
                    external_data_transfer_permitted: false, collaboration_rounds: 0,
                    reason: "Fortress permits local execution only; this preview does not itself disable OS networking.".into(),
                    enforcement_boundary_required: true }
            } else { blocked(mode, "Fortress cannot use cloud assistance and required local capabilities are unavailable.") }
        }
        ExecutionMode::Smart => {
            if local_ready {
                ModeRoutingDecision { mode, action: RouteAction::ExecuteLocally, allowed: true,
                    external_data_transfer_permitted: false, collaboration_rounds: 0,
                    reason: "Smart selects local execution because the declared local capabilities are sufficient.".into(),
                    enforcement_boundary_required: true }
            } else if !request.external_data_approved {
                ModeRoutingDecision { mode, action: RouteAction::AwaitExternalApproval, allowed: false,
                    external_data_transfer_permitted: false, collaboration_rounds: 0,
                    reason: "Local capabilities are insufficient; explicit per-task approval is required before any external transfer.".into(),
                    enforcement_boundary_required: true }
            } else if !request.network_capability_available {
                blocked(mode, "External assistance was approved, but no network gateway capability is available.")
            } else {
                ModeRoutingDecision { mode, action: RouteAction::PrepareWorkerPlan, allowed: true,
                    external_data_transfer_permitted: true, collaboration_rounds: 0,
                    reason: "Prepare a bounded external subtask plan; no data is sent by this router.".into(),
                    enforcement_boundary_required: true }
            }
        }
        ExecutionMode::Worker => {
            if !request.external_data_approved {
                ModeRoutingDecision { mode, action: RouteAction::AwaitExternalApproval, allowed: false,
                    external_data_transfer_permitted: false, collaboration_rounds: 0,
                    reason: "Worker requires explicit per-task approval before contacting an external planner.".into(),
                    enforcement_boundary_required: true }
            } else if !request.network_capability_available {
                blocked(mode, "Worker requires an available, separately controlled network gateway.")
            } else {
                ModeRoutingDecision { mode, action: RouteAction::PrepareWorkerPlan, allowed: true,
                    external_data_transfer_permitted: true, collaboration_rounds: 0,
                    reason: "Prepare a cloud planning request; the device may execute only registered, approved local operations.".into(),
                    enforcement_boundary_required: true }
            }
        }
        ExecutionMode::Partner => {
            if !request.external_data_approved {
                ModeRoutingDecision { mode, action: RouteAction::AwaitExternalApproval, allowed: false,
                    external_data_transfer_permitted: false, collaboration_rounds: 0,
                    reason: "Partner requires explicit per-task approval before the first external transfer.".into(),
                    enforcement_boundary_required: true }
            } else if !request.network_capability_available {
                blocked(mode, "Partner requires an available, separately controlled network gateway.")
            } else if request.requested_collaboration_rounds == 0 {
                blocked(mode, "Partner requires at least one collaboration round.")
            } else {
                let rounds = request.requested_collaboration_rounds.min(3);
                ModeRoutingDecision { mode, action: RouteAction::PreparePartnerPlan, allowed: true,
                    external_data_transfer_permitted: true, collaboration_rounds: rounds,
                    reason: "Prepare a bounded collaboration plan; each transfer and returned operation must be separately validated.".into(),
                    enforcement_boundary_required: true }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn req(mode: ExecutionMode) -> RouteRequest {
        RouteRequest { task_id: "task-1".into(), operation: "local.ai.generate".into(), mode,
            required_capabilities: BTreeSet::from(["cpu".into()]),
            available_capabilities: BTreeSet::from(["cpu".into()]),
            local_memory_required_mb: 128, available_memory_mb: 1024,
            external_data_approved: false, network_capability_available: true,
            requested_collaboration_rounds: 8 }
    }
    #[test] fn fortress_stays_local_and_never_approves_transfer() {
        let d = evaluate_mode(&req(ExecutionMode::Fortress));
        assert_eq!(d.action, RouteAction::ExecuteLocally);
        assert!(!d.external_data_transfer_permitted);
    }
    #[test] fn fortress_blocks_missing_local_capability() {
        let mut r = req(ExecutionMode::Fortress); r.available_capabilities.clear();
        assert_eq!(evaluate_mode(&r).action, RouteAction::Blocked);
    }
    #[test] fn smart_prefers_local_when_possible() {
        assert_eq!(evaluate_mode(&req(ExecutionMode::Smart)).action, RouteAction::ExecuteLocally);
    }
    #[test] fn smart_requires_consent_when_local_is_insufficient() {
        let mut r = req(ExecutionMode::Smart); r.available_capabilities.clear();
        assert_eq!(evaluate_mode(&r).action, RouteAction::AwaitExternalApproval);
    }
    #[test] fn worker_never_bypasses_missing_consent() {
        let mut r = req(ExecutionMode::Worker); r.network_capability_available = true;
        assert_eq!(evaluate_mode(&r).action, RouteAction::AwaitExternalApproval);
    }
    #[test] fn worker_needs_gateway_even_after_consent() {
        let mut r = req(ExecutionMode::Worker); r.external_data_approved = true; r.network_capability_available = false;
        assert_eq!(evaluate_mode(&r).action, RouteAction::Blocked);
    }
    #[test] fn partner_is_consent_gated_and_round_limited() {
        let mut r = req(ExecutionMode::Partner); r.external_data_approved = true;
        let d = evaluate_mode(&r);
        assert_eq!(d.action, RouteAction::PreparePartnerPlan);
        assert_eq!(d.collaboration_rounds, 3);
    }
    #[test] fn memory_budget_is_enforced() {
        let mut r = req(ExecutionMode::Smart); r.local_memory_required_mb = 2048;
        assert_eq!(evaluate_mode(&r).action, RouteAction::Blocked);
    }
    #[test] fn unregistered_operation_is_rejected() {
        let mut r = req(ExecutionMode::Smart); r.operation = "shell.run".into();
        assert_eq!(evaluate_mode(&r).action, RouteAction::Blocked);
    }
    #[test] fn malformed_operation_is_rejected() {
        let mut r = req(ExecutionMode::Smart); r.operation = "shell run".into();
        assert_eq!(evaluate_mode(&r).action, RouteAction::Blocked);
    }
}
