//! Explicit workload-intent guardrails for UJANI task orchestration.
//! This is a deterministic policy layer, not an AI safety classifier. Production
//! execution must additionally apply abuse detection, authorization checks,
//! sandboxing, rate limits and output validation at the execution boundary.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadIntent {
    OrdinaryProductivity,
    EducationAndResearch,
    AuthorizedSecurityTesting,
    BulkMessaging,
    CredentialCollection,
    MalwareDevelopment,
    UnauthorizedAccess,
    EvasionOrPersistence,
    BotnetOrAbuseAutomation,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyDecision {
    AllowWithLimits,
    RequireHumanReview,
    Deny,
}

pub fn evaluate_workload_intent(intent: WorkloadIntent, authorization_verified: bool) -> SafetyDecision {
    match intent {
        WorkloadIntent::OrdinaryProductivity | WorkloadIntent::EducationAndResearch => SafetyDecision::AllowWithLimits,
        WorkloadIntent::AuthorizedSecurityTesting if authorization_verified => SafetyDecision::AllowWithLimits,
        WorkloadIntent::AuthorizedSecurityTesting => SafetyDecision::RequireHumanReview,
        WorkloadIntent::BulkMessaging | WorkloadIntent::Unknown => SafetyDecision::RequireHumanReview,
        WorkloadIntent::CredentialCollection
        | WorkloadIntent::MalwareDevelopment
        | WorkloadIntent::UnauthorizedAccess
        | WorkloadIntent::EvasionOrPersistence
        | WorkloadIntent::BotnetOrAbuseAutomation => SafetyDecision::Deny,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn productivity_and_learning_are_allowed_with_limits() {
        assert_eq!(evaluate_workload_intent(WorkloadIntent::OrdinaryProductivity,false),SafetyDecision::AllowWithLimits);
        assert_eq!(evaluate_workload_intent(WorkloadIntent::EducationAndResearch,false),SafetyDecision::AllowWithLimits);
    }
    #[test] fn security_testing_requires_verified_authorization() {
        assert_eq!(evaluate_workload_intent(WorkloadIntent::AuthorizedSecurityTesting,false),SafetyDecision::RequireHumanReview);
        assert_eq!(evaluate_workload_intent(WorkloadIntent::AuthorizedSecurityTesting,true),SafetyDecision::AllowWithLimits);
    }
    #[test] fn bulk_messaging_and_unknown_intents_are_reviewed() {
        assert_eq!(evaluate_workload_intent(WorkloadIntent::BulkMessaging,false),SafetyDecision::RequireHumanReview);
        assert_eq!(evaluate_workload_intent(WorkloadIntent::Unknown,false),SafetyDecision::RequireHumanReview);
    }
    #[test] fn abusive_and_malicious_intents_are_denied() {
        for intent in [WorkloadIntent::CredentialCollection,WorkloadIntent::MalwareDevelopment,
            WorkloadIntent::UnauthorizedAccess,WorkloadIntent::EvasionOrPersistence,
            WorkloadIntent::BotnetOrAbuseAutomation] {
            assert_eq!(evaluate_workload_intent(intent,true),SafetyDecision::Deny);
        }
    }
}
