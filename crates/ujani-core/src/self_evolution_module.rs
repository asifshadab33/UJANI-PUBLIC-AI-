//! Optional user-controlled Self-Evolution module policy.
//!
//! Installing the optional module does not enable it. This evaluator permits
//! local analysis and proposal preparation only. Staging and applying changes
//! remain unavailable until a separate typed approval-and-replay host pipeline
//! is integrated. This module performs no filesystem or process operations.

use std::collections::BTreeSet;

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
    ApprovalPipelineUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionUserSettings {
    pub installation: ModuleInstallState,
    pub enabled_by_user: bool,
    pub selected_model_id: String,
    /// Model IDs approved by the device owner for this feature.
    pub approved_model_ids: BTreeSet<String>,
    /// Supplied only by the trusted native host after source/build checks.
    pub trusted_source_and_build_environment_configured: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfEvolutionPermission {
    pub may_analyse: bool,
    pub may_prepare_proposals: bool,
    /// Always false until typed host approval and durable replay consumption
    /// are wired into a separate, reviewed staging workflow.
    pub may_stage_approved_proposal: bool,
    /// Why analysis/proposal preparation is unavailable, if applicable.
    pub blocked_by: Option<SelfEvolutionBlock>,
    /// Staging remains unavailable until the host approval pipeline is integrated.
    pub staging_blocked_by: Option<SelfEvolutionBlock>,
}

pub fn evaluate_self_evolution_permission(
    settings: &SelfEvolutionUserSettings,
) -> SelfEvolutionPermission {
    let blocked_by = if settings.installation == ModuleInstallState::NotInstalled {
        Some(SelfEvolutionBlock::ModuleNotInstalled)
    } else if !settings.enabled_by_user {
        Some(SelfEvolutionBlock::UserOptInRequired)
    } else if !valid_model_id(&settings.selected_model_id)
        || !settings
            .approved_model_ids
            .contains(&settings.selected_model_id)
    {
        Some(SelfEvolutionBlock::ApprovedModelRequired)
    } else if !settings.trusted_source_and_build_environment_configured {
        Some(SelfEvolutionBlock::TrustedBuildEnvironmentRequired)
    } else {
        None
    };

    let may_prepare = blocked_by.is_none();
    SelfEvolutionPermission {
        may_analyse: may_prepare,
        may_prepare_proposals: may_prepare,
        may_stage_approved_proposal: false,
        blocked_by,
        staging_blocked_by: Some(
            blocked_by.unwrap_or(SelfEvolutionBlock::ApprovalPipelineUnavailable),
        ),
    }
}

fn valid_model_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_settings() -> SelfEvolutionUserSettings {
        SelfEvolutionUserSettings {
            installation: ModuleInstallState::Installed,
            enabled_by_user: true,
            selected_model_id: "local-coder-v1".into(),
            approved_model_ids: BTreeSet::from(["local-coder-v1".into()]),
            trusted_source_and_build_environment_configured: true,
        }
    }

    #[test]
    fn absent_module_is_unavailable_by_default() {
        let mut settings = enabled_settings();
        settings.installation = ModuleInstallState::NotInstalled;
        let result = evaluate_self_evolution_permission(&settings);
        assert!(!result.may_analyse);
        assert!(!result.may_prepare_proposals);
        assert!(!result.may_stage_approved_proposal);
        assert_eq!(result.blocked_by, Some(SelfEvolutionBlock::ModuleNotInstalled));
    }

    #[test]
    fn installation_alone_does_not_enable_the_module() {
        let mut settings = enabled_settings();
        settings.enabled_by_user = false;
        let result = evaluate_self_evolution_permission(&settings);
        assert!(!result.may_prepare_proposals);
        assert_eq!(result.blocked_by, Some(SelfEvolutionBlock::UserOptInRequired));
    }

    #[test]
    fn selected_model_must_be_explicitly_approved() {
        let mut settings = enabled_settings();
        settings.selected_model_id = "unapproved-model".into();
        let result = evaluate_self_evolution_permission(&settings);
        assert!(!result.may_prepare_proposals);
        assert_eq!(result.blocked_by, Some(SelfEvolutionBlock::ApprovedModelRequired));
    }

    #[test]
    fn trusted_build_environment_is_required() {
        let mut settings = enabled_settings();
        settings.trusted_source_and_build_environment_configured = false;
        let result = evaluate_self_evolution_permission(&settings);
        assert!(!result.may_prepare_proposals);
        assert_eq!(
            result.blocked_by,
            Some(SelfEvolutionBlock::TrustedBuildEnvironmentRequired)
        );
    }

    #[test]
    fn proposal_preparation_does_not_authorize_staging_or_application() {
        let result = evaluate_self_evolution_permission(&enabled_settings());
        assert!(result.may_analyse);
        assert!(result.may_prepare_proposals);
        assert!(!result.may_stage_approved_proposal);
        assert_eq!(result.blocked_by, None);
        assert_eq!(
            result.staging_blocked_by,
            Some(SelfEvolutionBlock::ApprovalPipelineUnavailable)
        );
    }
}
