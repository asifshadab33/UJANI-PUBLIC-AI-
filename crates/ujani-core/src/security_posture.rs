//! Normalised, evidence-based security posture from existing OS/provider resources.
//!
//! This module does not install, configure, scan, quarantine, isolate, or contact
//! security products. Platform adapters supply observations from documented,
//! read-only operating-system or vendor interfaces. Unknown is never healthy.
use serde::Serialize;

const MAX_ID_LEN: usize = 96;
const MAX_LABEL_LEN: usize = 160;
const MAX_SOURCE_LEN: usize = 96;
const MAX_CLOCK_SKEW_MS: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityCategory {
    MalwareProtection,
    Firewall,
    EndpointDetectionResponse,
    Quarantine,
    DeviceIsolation,
    SecurityUpdates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Healthy,
    Attention,
    Alert,
    NotMonitored,
    Unavailable,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PostureAssessment {
    ProtectionReportedHealthy,
    ProtectionNeedsAttention,
    ThreatOrProtectionAlert,
    NoProviderReported,
    CannotVerify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderObservation {
    /// Stable adapter identifier, not a user-supplied command or URL.
    pub provider_id: String,
    /// Human-readable product/category label, if the OS/vendor exposes it.
    pub display_name: Option<String>,
    pub category: SecurityCategory,
    pub status: EvidenceStatus,
    /// Adapter/API name; never include secrets or raw command lines.
    pub evidence_source: String,
    pub observed_at_unix_ms: u64,
    /// False when the OS reports only aggregate category health.
    pub product_identity_known: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PostureResult {
    pub assessment: PostureAssessment,
    pub observation_count: usize,
    pub product_identity_known: bool,
    pub recommendation: &'static str,
}

/// Assess one security category from fresh, bounded observations.
/// No observation is treated as proof that a product is absent or present.
pub fn assess_category(
    category: SecurityCategory,
    observations: &[ProviderObservation],
    now_unix_ms: u64,
    max_age_ms: u64,
) -> PostureResult {
    let relevant: Vec<&ProviderObservation> = observations
        .iter()
        .filter(|item| item.category == category)
        .collect();

    let fresh: Vec<&ProviderObservation> = relevant
        .into_iter()
        .filter(|item| {
            valid_text(&item.provider_id, MAX_ID_LEN)
                && valid_text(&item.evidence_source, MAX_SOURCE_LEN)
                && item.display_name.as_ref().map_or(true, |name| valid_text(name, MAX_LABEL_LEN))
                && item.observed_at_unix_ms <= now_unix_ms.saturating_add(MAX_CLOCK_SKEW_MS)
                && now_unix_ms.saturating_sub(item.observed_at_unix_ms) <= max_age_ms
        })
        .collect();

    let product_identity_known = fresh.iter().any(|item| item.product_identity_known);
    let assessment = if fresh.is_empty() {
        PostureAssessment::CannotVerify
    } else if fresh.iter().any(|item| item.status == EvidenceStatus::Alert) {
        PostureAssessment::ThreatOrProtectionAlert
    } else if fresh.iter().any(|item| item.status == EvidenceStatus::Attention) {
        PostureAssessment::ProtectionNeedsAttention
    } else if fresh.iter().any(|item| item.status == EvidenceStatus::Healthy) {
        PostureAssessment::ProtectionReportedHealthy
    } else if fresh.iter().any(|item| item.status == EvidenceStatus::NotMonitored) {
        PostureAssessment::NoProviderReported
    } else {
        PostureAssessment::CannotVerify
    };

    let recommendation = match assessment {
        PostureAssessment::ProtectionReportedHealthy =>
            "Keep the existing protection enabled; verify product identity and update/scan status where supported.",
        PostureAssessment::ProtectionNeedsAttention =>
            "Open the existing security product or operating-system security settings and review its warning.",
        PostureAssessment::ThreatOrProtectionAlert =>
            "Follow the operating-system or security-product alert; do not open or process the affected file.",
        PostureAssessment::NoProviderReported =>
            "No provider is reported for this category; ask the user or administrator to review protection and choose an existing trusted product if needed.",
        PostureAssessment::CannotVerify =>
            "Protection could not be verified; do not assume the device is protected. Check the OS or administrator-managed security console.",
    };

    PostureResult {
        assessment,
        observation_count: fresh.len(),
        product_identity_known,
        recommendation,
    }
}

fn valid_text(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && value.chars().all(|c| !c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(status: EvidenceStatus) -> ProviderObservation {
        ProviderObservation {
            provider_id: "windows.security_center".into(),
            display_name: None,
            category: SecurityCategory::MalwareProtection,
            status,
            evidence_source: "Windows Security Center aggregate health API".into(),
            observed_at_unix_ms: 100_000,
            product_identity_known: false,
        }
    }

    #[test]
    fn aggregate_healthy_is_not_misrepresented_as_named_product() {
        let result = assess_category(
            SecurityCategory::MalwareProtection,
            &[observation(EvidenceStatus::Healthy)],
            100_000,
            60_000,
        );
        assert_eq!(result.assessment, PostureAssessment::ProtectionReportedHealthy);
        assert!(!result.product_identity_known);
    }

    #[test]
    fn not_monitored_produces_a_recommendation_not_an_install_action() {
        let result = assess_category(
            SecurityCategory::MalwareProtection,
            &[observation(EvidenceStatus::NotMonitored)],
            100_000,
            60_000,
        );
        assert_eq!(result.assessment, PostureAssessment::NoProviderReported);
        assert!(result.recommendation.contains("choose an existing trusted product"));
    }

    #[test]
    fn stale_future_and_missing_observations_are_unverified() {
        let mut stale = observation(EvidenceStatus::Healthy);
        stale.observed_at_unix_ms = 1;
        let mut future = observation(EvidenceStatus::Healthy);
        future.observed_at_unix_ms = 200_000;
        for item in [stale, future] {
            let result = assess_category(
                SecurityCategory::MalwareProtection,
                &[item],
                100_000,
                60_000,
            );
            assert_eq!(result.assessment, PostureAssessment::CannotVerify);
        }
        let result = assess_category(
            SecurityCategory::MalwareProtection,
            &[],
            100_000,
            60_000,
        );
        assert_eq!(result.assessment, PostureAssessment::CannotVerify);
    }

    #[test]
    fn alert_takes_precedence_over_healthy_observation() {
        let result = assess_category(
            SecurityCategory::MalwareProtection,
            &[observation(EvidenceStatus::Healthy), observation(EvidenceStatus::Alert)],
            100_000,
            60_000,
        );
        assert_eq!(result.assessment, PostureAssessment::ThreatOrProtectionAlert);
    }

    #[test]
    fn unsupported_and_unknown_never_become_healthy() {
        for status in [EvidenceStatus::Unsupported, EvidenceStatus::Unknown, EvidenceStatus::Unavailable] {
            let result = assess_category(
                SecurityCategory::MalwareProtection,
                &[observation(status)],
                100_000,
                60_000,
            );
            assert_eq!(result.assessment, PostureAssessment::CannotVerify);
        }
    }

    #[test]
    fn invalid_adapter_metadata_is_ignored() {
        let mut item = observation(EvidenceStatus::Healthy);
        item.provider_id = "bad\nprovider".into();
        let result = assess_category(
            SecurityCategory::MalwareProtection,
            &[item],
            100_000,
            60_000,
        );
        assert_eq!(result.assessment, PostureAssessment::CannotVerify);
    }
}
