//! Shared pre-execution policy contract. Native dispatch integration is not yet complete;
//! do not treat this module as enforced until every route calls it with trusted context.
use crate::safety_policy::{evaluate_workload_intent, SafetyDecision, WorkloadIntent};
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DataClass { Public, Internal, Confidential, Restricted }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ExecutionTarget { LocalTool, LocalModel, EnterpriseWorker, CloudProvider }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum GateDecision { Allow, RequireReview, Deny }
#[derive(Debug, Clone, PartialEq, Eq)] pub struct ExecutionRequest {
 pub task_id:String, pub operation_id:String, pub target:ExecutionTarget, pub intent:WorkloadIntent,
 pub authorization_verified:bool, pub data_class:DataClass, pub external_transfer:bool,
 pub user_approved_transfer:bool, pub admin_approved_worker:bool, pub owner_opted_in:bool,
 pub operation_registered:bool, pub policy_version:u32, pub max_runtime_seconds:u32,
 pub requested_runtime_seconds:u32, pub max_memory_mb:u64, pub requested_memory_mb:u64,
}
#[derive(Debug, Clone, PartialEq, Eq)] pub struct GateResult { pub decision:GateDecision, pub reason:&'static str, pub requires_isolation:bool }
fn valid_id(s:&str)->bool { !s.trim().is_empty() && s.len()<=128 && s.chars().all(|c|c.is_ascii_alphanumeric() || "._:-".contains(c)) }
/// Parse the user's explicit data-classification declaration. Unknown values fail closed.
pub fn parse_data_class(value: &str) -> Result<DataClass, &'static str> {
    match value {
        "public" => Ok(DataClass::Public),
        "internal" => Ok(DataClass::Internal),
        "confidential" => Ok(DataClass::Confidential),
        "restricted" => Ok(DataClass::Restricted),
        _ => Err("a valid data classification must be selected"),
    }
}

/// Shared external-transfer policy used by native cloud dispatch.
/// This is a user-declared classification, not automatic content inspection.
pub fn evaluate_external_transfer(data_class: DataClass, user_approved: bool) -> GateDecision {
    if matches!(data_class, DataClass::Confidential | DataClass::Restricted) {
        GateDecision::Deny
    } else if !user_approved {
        GateDecision::RequireReview
    } else {
        GateDecision::Allow
    }
}

/// Pure decision only: invoke at the trusted dispatch boundary on every route.
/// This function does not itself enforce isolation, quotas, egress or cancellation.
pub fn evaluate_execution(r:&ExecutionRequest)->GateResult {
 let deny=|reason|GateResult{decision:GateDecision::Deny,reason,requires_isolation:true};
 let review=|reason|GateResult{decision:GateDecision::RequireReview,reason,requires_isolation:true};
 if !valid_id(&r.task_id)||!valid_id(&r.operation_id){return deny("invalid task or operation identifier")}
 if !r.operation_registered{return deny("operation is not in the trusted registry")}
 if r.policy_version==0{return deny("missing policy version")}
 if r.max_runtime_seconds==0||r.requested_runtime_seconds==0||r.requested_runtime_seconds>r.max_runtime_seconds{return deny("runtime budget invalid or exceeded")}
 if r.max_memory_mb==0||r.requested_memory_mb==0||r.requested_memory_mb>r.max_memory_mb{return deny("memory budget invalid or exceeded")}
 match evaluate_workload_intent(r.intent,r.authorization_verified) {
  SafetyDecision::Deny=>return deny("prohibited workload intent across local and external routes"),
  SafetyDecision::RequireHumanReview=>return review("workload intent requires human review"),
  SafetyDecision::AllowWithLimits=>{}
 }
 if r.external_transfer {
  match evaluate_external_transfer(r.data_class, r.user_approved_transfer) {
   GateDecision::RequireReview => return review("explicit per-task approval required for external transfer"),
   GateDecision::Deny => return deny("confidential or restricted data blocked from generic external transfer"),
   GateDecision::Allow => {}
  }
 }
 match r.target {
  ExecutionTarget::EnterpriseWorker if !r.admin_approved_worker||!r.owner_opted_in=>return deny("worker requires administrator approval and device-owner opt-in"),
  ExecutionTarget::CloudProvider if !r.external_transfer=>return deny("cloud route must explicitly declare data transfer"),
  ExecutionTarget::LocalTool|ExecutionTarget::LocalModel if r.external_transfer=>return deny("local target cannot silently declare external transfer"),
  _=>{}
 }
 GateResult{decision:GateDecision::Allow,reason:"policy checks passed; dispatch must still enforce isolation, quotas, egress and cancellation",requires_isolation:true}
}
#[cfg(test)] mod tests {
 use super::*;
 fn req(target:ExecutionTarget)->ExecutionRequest{ExecutionRequest{task_id:"t-1".into(),operation_id:"document.summarize".into(),target,intent:WorkloadIntent::OrdinaryProductivity,authorization_verified:false,data_class:DataClass::Public,external_transfer:false,user_approved_transfer:false,admin_approved_worker:true,owner_opted_in:true,operation_registered:true,policy_version:1,max_runtime_seconds:60,requested_runtime_seconds:30,max_memory_mb:1024,requested_memory_mb:512}}
 #[test] fn allows_bounded_local_routes(){assert_eq!(evaluate_execution(&req(ExecutionTarget::LocalModel)).decision,GateDecision::Allow);assert_eq!(evaluate_execution(&req(ExecutionTarget::LocalTool)).decision,GateDecision::Allow);}
 #[test] fn prohibited_intent_denied_even_locally(){let mut r=req(ExecutionTarget::LocalModel);r.intent=WorkloadIntent::MalwareDevelopment;assert_eq!(evaluate_execution(&r).decision,GateDecision::Deny);}
 #[test] fn external_transfer_requires_consent(){let mut r=req(ExecutionTarget::CloudProvider);r.external_transfer=true;assert_eq!(evaluate_execution(&r).decision,GateDecision::RequireReview);r.user_approved_transfer=true;assert_eq!(evaluate_execution(&r).decision,GateDecision::Allow);}
 #[test] fn restricted_data_blocked_from_generic_cloud(){let mut r=req(ExecutionTarget::CloudProvider);r.external_transfer=true;r.user_approved_transfer=true;r.data_class=DataClass::Restricted;assert_eq!(evaluate_execution(&r).decision,GateDecision::Deny);}
 #[test] fn worker_requires_admin_and_owner_consent(){let mut r=req(ExecutionTarget::EnterpriseWorker);r.owner_opted_in=false;assert_eq!(evaluate_execution(&r).decision,GateDecision::Deny);}
 #[test] fn unknown_intent_reviewed_on_every_route(){let mut r=req(ExecutionTarget::LocalTool);r.intent=WorkloadIntent::Unknown;assert_eq!(evaluate_execution(&r).decision,GateDecision::RequireReview);}
 #[test] fn budgets_fail_closed(){let mut r=req(ExecutionTarget::LocalTool);r.requested_memory_mb=2048;assert_eq!(evaluate_execution(&r).decision,GateDecision::Deny);}
}

#[cfg(test)]
mod fail_closed_boundary_regressions {
    use super::*;

    fn request(target: ExecutionTarget) -> ExecutionRequest {
        ExecutionRequest {
            task_id: "task-001".into(),
            operation_id: "document.summarize".into(),
            target,
            intent: WorkloadIntent::OrdinaryProductivity,
            authorization_verified: false,
            data_class: DataClass::Public,
            external_transfer: false,
            user_approved_transfer: false,
            admin_approved_worker: false,
            owner_opted_in: false,
            operation_registered: true,
            policy_version: 1,
            max_runtime_seconds: 60,
            requested_runtime_seconds: 30,
            max_memory_mb: 2048,
            requested_memory_mb: 1024,
        }
    }

    #[test]
    fn data_classification_parser_rejects_unknown_and_unclassified_values() {
        assert_eq!(parse_data_class("public"), Ok(DataClass::Public));
        assert_eq!(parse_data_class("internal"), Ok(DataClass::Internal));
        assert_eq!(parse_data_class("confidential"), Ok(DataClass::Confidential));
        assert_eq!(parse_data_class("restricted"), Ok(DataClass::Restricted));
        assert!(parse_data_class("unclassified").is_err());
        assert!(parse_data_class("PUBLIC").is_err());
    }

    #[test]
    fn external_transfer_requires_approval_and_blocks_sensitive_classes() {
        assert_eq!(
            evaluate_external_transfer(DataClass::Public, false),
            GateDecision::RequireReview
        );
        assert_eq!(
            evaluate_external_transfer(DataClass::Internal, true),
            GateDecision::Allow
        );
        for class in [DataClass::Confidential, DataClass::Restricted] {
            assert_eq!(
                evaluate_external_transfer(class, false),
                GateDecision::Deny
            );
            assert_eq!(
                evaluate_external_transfer(class, true),
                GateDecision::Deny
            );
        }
    }

    #[test]
    fn confidential_data_is_blocked_from_generic_cloud_even_after_consent() {
        for data_class in [DataClass::Confidential, DataClass::Restricted] {
            let mut value = request(ExecutionTarget::CloudProvider);
            value.external_transfer = true;
            value.user_approved_transfer = true;
            value.data_class = data_class;
            assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);
        }
    }

    #[test]
    fn cloud_route_cannot_be_selected_without_an_explicit_transfer_declaration() {
        let value = request(ExecutionTarget::CloudProvider);
        assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);
    }

    #[test]
    fn local_route_cannot_smuggle_an_external_transfer_flag() {
        let mut value = request(ExecutionTarget::LocalModel);
        value.external_transfer = true;
        value.user_approved_transfer = true;
        assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);
    }

    #[test]
    fn malformed_task_and_unregistered_operation_identifiers_fail_closed() {
        let mut value = request(ExecutionTarget::LocalTool);
        value.task_id = "../escape".into();
        assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);

        let mut value = request(ExecutionTarget::LocalTool);
        value.operation_id = "shell.run".into();
        value.operation_registered = false;
        assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);
    }

    #[test]
    fn missing_registry_and_policy_version_fail_closed() {
        let mut value = request(ExecutionTarget::LocalTool);
        value.operation_registered = false;
        assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);

        let mut value = request(ExecutionTarget::LocalTool);
        value.policy_version = 0;
        assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);
    }

    #[test]
    fn zero_and_excessive_runtime_budgets_fail_closed() {
        let mut value = request(ExecutionTarget::LocalTool);
        value.max_runtime_seconds = 0;
        assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);

        let mut value = request(ExecutionTarget::LocalTool);
        value.requested_runtime_seconds = value.max_runtime_seconds + 1;
        assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);
    }

    #[test]
    fn worker_requires_both_admin_approval_and_owner_opt_in() {
        let mut value = request(ExecutionTarget::EnterpriseWorker);
        value.admin_approved_worker = true;
        value.owner_opted_in = false;
        assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);

        value.owner_opted_in = true;
        value.admin_approved_worker = false;
        assert_eq!(evaluate_execution(&value).decision, GateDecision::Deny);
    }
}
