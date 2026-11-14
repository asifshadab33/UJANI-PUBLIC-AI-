//! Optional user-controlled Self-Evolution module policy.
//!
//! This module is not part of the mandatory application runtime. Installing it
//! does not enable it. This crate only evaluates permissions; it never installs
//! packages, reads source files, applies patches or executes generated code.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleInstallState {
    NotInstalled,
    Installed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfEvolutionBlock {
    ModuleNotInstalled,
    UserOptInRequired,
    ApprovedModelRequired,
    TrustedBuildEnvironmentRequired,
    VerifiedApprovalRequired,
    ReplayProtectionRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfEvolutionUserSettings {
    pub installation: ModuleInstallState,
    pub enabled_by_user: bool,
    pub selected_model_is_approved: bool,
    pub trusted_source_and_build_environment_configured: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfEvolutionRuntimeEvidence {
    pub verified_host_approval: bool,
    pub approval_nonce_durably_consumed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfEvolutionPermission {
    pub may_analyse: bool,
    pub may_prepare_proposals: bool,
    /// This permits only staging a reviewed proposal in an isolated workspace.
    /// It does not authorize applying, executing, signing or installing changes.
    pub may_stage_approved_proposal: bool,
    pub blocked_by: Option<SelfEvolutionBlock>,
}

pub fn evaluate_self_evolution_permission(
    settings: SelfEvolutionUserSettings,
    evidence: SelfEvolutionRuntimeEvidence,
) -> SelfEvolutionPermission {
    let blocked_by = if settings.installation == ModuleInstallState::NotInstalled {
        Some(SelfEvolutionBlock::ModuleNotInstalled)
    } else if !settings.enabled_by_user {
        Some(SelfEvolutionBlock::UserOptInRequired)
    } else if !settings.selected_model_is_approved {
        Some(SelfEvolutionBlock::ApprovedModelRequired)
    } else if !settings.trusted_source_and_build_environment_configured {
        Some(SelfEvolutionBlock::TrustedBuildEnvironmentRequired)
    } else {
        None
    };

    let may_prepare = blocked_by.is_none();
    let staging_block = if !may_prepare {
        blocked_by
    } else if !evidence.verified_host_approval {
        Some(SelfEvolutionBlock::VerifiedApprovalRequired)
    } else if !evidence.approval_nonce_durably_consumed {
        Some(SelfEvolutionBlock::ReplayProtectionRequired)
    } else {
        None
    };

    SelfEvolutionPermission {
        may_analyse: may_prepare,
        may_prepare_proposals: may_prepare,
        may_stage_approved_proposal: staging_block.is_none(),
        blocked_by: staging_block.or(blocked_by),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_settings() -> SelfEvolutionUserSettings {
        SelfEvolutionUserSettings {
            installation: ModuleInstallState::Installed,
            enabled_by_user: true,
            selected_model_is_approved: true,
            trusted_source_and_build_environment_configured: true,
        }
    }

    fn valid_evidence() -> SelfEvolutionRuntimeEvidence {
        SelfEvolutionRuntimeEvidence {
            verified_host_approval: true,
            approval_nonce_durably_consumed: true,
        }
    }

    #[test]
    fn absent_module_is_unavailable_by_default() {
        let mut settings = enabled_settings();
        settings.installation = ModuleInstallState::NotInstalled;
        let result = evaluate_self_evolution_permission(settings, valid_evidence());
        assert!(!result.may_analyse);
        assert!(!result.may_prepare_proposals);
        assert!(!result.may_stage_approved_proposal);
        assert_eq!(result.blocked_by, Some(SelfEvolutionBlock::ModuleNotInstalled));
    }

    #[test]
    fn installation_alone_does_not_enable_the_module() {
        let mut settings = enabled_settings();
        settings.enabled_by_user = false;
        let result = evaluate_self_evolution_permission(settings, valid_evidence());
        assert!(!result.may_prepare_proposals);
        assert_eq!(result.blocked_by, Some(SelfEvolutionBlock::UserOptInRequired));
    }

    #[test]
    fn model_and_trusted_build_environment_are_required() {
        let mut settings = enabled_settings();
        settings.selected_model_is_approved = false;
        assert_eq!(
            evaluate_self_evolution_permission(settings, valid_evidence()).blocked_by,
            Some(SelfEvolutionBlock::ApprovedModelRequired)
        );
        settings.selected_model_is_approved = true;
        settings.trusted_source_and_build_environment_configured = false;
        assert_eq!(
            evaluate_self_evolution_permission(settings, valid_evidence()).blocked_by,
            Some(SelfEvolutionBlock::TrustedBuildEnvironmentRequired)
        );
    }

    #[test]
    fn staging_requires_verified_single_use_host_approval() {
        let mut evidence = valid_evidence();
        evidence.verified_host_approval = false;
        let result = evaluate_self_evolution_permission(enabled_settings(), evidence);
        assert!(result.may_prepare_proposals);
        assert!(!result.may_stage_approved_proposal);
        assert_eq!(result.blocked_by, Some(SelfEvolutionBlock::VerifiedApprovalRequired));

        evidence.verified_host_approval = true;
        evidence.approval_nonce_durably_consumed = false;
        let result = evaluate_self_evolution_permission(enabled_settings(), evidence);
        assert!(!result.may_stage_approved_proposal);
        assert_eq!(result.blocked_by, Some(SelfEvolutionBlock::ReplayProtectionRequired));
    }

    #[test]
    fn permission_never_authorizes_execution_or_installation() {
        let result = evaluate_self_evolution_permission(enabled_settings(), valid_evidence());
        assert!(result.may_stage_approved_proposal);
        // The permission type intentionally has no execute, sign, install or deploy field.
    }
}
