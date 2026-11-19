//! Pure, fail-closed file scan evidence classification.
//!
//! This module does not scan, open, move, delete, quarantine, or execute files.
//! It only classifies a provider observation after binding it to an expected
//! file identity. Callers must obtain the hash from a stable, read-only handle.
use serde::Serialize;

const MAX_PROVIDER_ID: usize = 64;
const MAX_PROVIDER_VERSION: usize = 64;
const MAX_CLOCK_SKEW_MS: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileScanState {
    Unscanned,
    Scanning,
    CleanReported,
    MaliciousReported,
    Suspicious,
    ScanUnavailable,
    Quarantined,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderVerdict {
    NoDetection,
    Detection,
    Suspicious,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIdentity {
    /// Lowercase SHA-256 hexadecimal digest of the exact file bytes.
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanObservation {
    pub provider_id: String,
    pub provider_version: String,
    pub file_sha256: String,
    pub file_size_bytes: u64,
    pub scanned_at_unix_ms: u64,
    pub verdict: ProviderVerdict,
}

/// Classify a provider observation without ever converting missing, stale,
/// mismatched, malformed, or unavailable evidence into a clean result.
pub fn classify_scan_observation(
    expected: &FileIdentity,
    observation: Option<&ScanObservation>,
    now_unix_ms: u64,
    max_age_ms: u64,
) -> FileScanState {
    let Some(observation) = observation else {
        return FileScanState::Unscanned;
    };

    if !valid_sha256(&expected.sha256)
        || !valid_sha256(&observation.file_sha256)
        || expected.sha256 != observation.file_sha256
        || expected.size_bytes != observation.file_size_bytes
        || observation.provider_id.is_empty()
        || observation.provider_id.len() > MAX_PROVIDER_ID
        || !observation.provider_id.chars().all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c))
        || observation.provider_version.is_empty()
        || observation.provider_version.len() > MAX_PROVIDER_VERSION
    {
        return FileScanState::Suspicious;
    }

    if observation.scanned_at_unix_ms > now_unix_ms.saturating_add(MAX_CLOCK_SKEW_MS)
        || now_unix_ms.saturating_sub(observation.scanned_at_unix_ms) > max_age_ms
    {
        return FileScanState::Suspicious;
    }

    match observation.verdict {
        ProviderVerdict::NoDetection => FileScanState::CleanReported,
        ProviderVerdict::Detection => FileScanState::MaliciousReported,
        ProviderVerdict::Suspicious => FileScanState::Suspicious,
        ProviderVerdict::Unavailable => FileScanState::ScanUnavailable,
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> FileIdentity {
        FileIdentity { sha256: "a".repeat(64), size_bytes: 4096 }
    }

    fn observation(verdict: ProviderVerdict) -> ScanObservation {
        ScanObservation {
            provider_id: "windows.defender".into(),
            provider_version: "1.2.3".into(),
            file_sha256: "a".repeat(64),
            file_size_bytes: 4096,
            scanned_at_unix_ms: 100_000,
            verdict,
        }
    }

    #[test]
    fn missing_scan_is_unscanned_not_clean() {
        assert_eq!(classify_scan_observation(&identity(), None, 100_000, 60_000), FileScanState::Unscanned);
    }

    #[test]
    fn clean_report_is_bound_to_exact_hash_and_size() {
        assert_eq!(classify_scan_observation(&identity(), Some(&observation(ProviderVerdict::NoDetection)), 100_000, 60_000), FileScanState::CleanReported);
    }

    #[test]
    fn malicious_and_suspicious_provider_results_are_preserved() {
        assert_eq!(classify_scan_observation(&identity(), Some(&observation(ProviderVerdict::Detection)), 100_000, 60_000), FileScanState::MaliciousReported);
        assert_eq!(classify_scan_observation(&identity(), Some(&observation(ProviderVerdict::Suspicious)), 100_000, 60_000), FileScanState::Suspicious);
    }

    #[test]
    fn unavailable_provider_is_never_clean() {
        assert_eq!(classify_scan_observation(&identity(), Some(&observation(ProviderVerdict::Unavailable)), 100_000, 60_000), FileScanState::ScanUnavailable);
    }

    #[test]
    fn changed_file_hash_or_size_invalidates_verdict() {
        let mut item = observation(ProviderVerdict::NoDetection);
        item.file_sha256 = "b".repeat(64);
        assert_eq!(classify_scan_observation(&identity(), Some(&item), 100_000, 60_000), FileScanState::Suspicious);
        let mut item = observation(ProviderVerdict::NoDetection);
        item.file_size_bytes += 1;
        assert_eq!(classify_scan_observation(&identity(), Some(&item), 100_000, 60_000), FileScanState::Suspicious);
    }

    #[test]
    fn stale_future_and_malformed_evidence_fail_closed() {
        let mut item = observation(ProviderVerdict::NoDetection);
        item.scanned_at_unix_ms = 1;
        assert_eq!(classify_scan_observation(&identity(), Some(&item), 100_000, 60_000), FileScanState::Suspicious);
        item.scanned_at_unix_ms = 200_000;
        assert_eq!(classify_scan_observation(&identity(), Some(&item), 100_000, 60_000), FileScanState::Suspicious);
        item.file_sha256 = "not-a-hash".into();
        assert_eq!(classify_scan_observation(&identity(), Some(&item), 100_000, 60_000), FileScanState::Suspicious);
    }
}
