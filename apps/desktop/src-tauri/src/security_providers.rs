//! Read-only operating-system security provider status adapters.
//!
//! The first adapter reads Windows Security Center's aggregate antivirus and
//! firewall health only. It does not identify individual products, inspect
//! files, collect threat events, or change security settings.

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct SecurityProviderStatus {
    pub provider_id: String,
    pub label: String,
    pub status: String,
    pub source: String,
    pub detail: String,
    pub observed_at_unix_ms: u64,
}

fn observed_at_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn status_from_windows_result(hresult: i32, health: i32) -> (&'static str, &'static str) {
    if hresult == 1 {
        return (
            "unavailable",
            "Windows Security Center service is not running.",
        );
    }
    if hresult != 0 {
        return (
            "unknown",
            "Windows Security Center could not provide a reliable status.",
        );
    }
    match health {
        0 => ("healthy", "Windows Security Center reports this category as healthy."),
        1 => (
            "not_monitored",
            "Windows Security Center is not monitoring this provider category.",
        ),
        2 => (
            "alert",
            "Windows Security Center reports a protection problem in this category.",
        ),
        3 => (
            "attention",
            "Windows Security Center reports that this category is snoozed.",
        ),
        _ => (
            "unknown",
            "Windows Security Center returned an unrecognised health value.",
        ),
    }
}

#[cfg(target_os = "windows")]
fn windows_category(
    provider_id: &str,
    label: &str,
    provider_mask: u32,
) -> SecurityProviderStatus {
    use windows_sys::Win32::System::SecurityCenter::WscGetSecurityProviderHealth;

    let mut health = -1i32;
    // This documented Windows API reads aggregate category health only.
    let hresult = unsafe { WscGetSecurityProviderHealth(provider_mask, &mut health) };
    let (status, detail) = status_from_windows_result(hresult, health);
    SecurityProviderStatus {
        provider_id: provider_id.into(),
        label: label.into(),
        status: status.into(),
        source: "Windows Security Center aggregate health API".into(),
        detail: detail.into(),
        observed_at_unix_ms: observed_at_unix_ms(),
    }
}

/// Read the supported provider categories on this platform. No polling occurs;
/// callers invoke this only after the user explicitly requests a status check.
pub fn read_only_status() -> Vec<SecurityProviderStatus> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::SecurityCenter::{
            WSC_SECURITY_PROVIDER_ANTIVIRUS, WSC_SECURITY_PROVIDER_FIREWALL,
        };

        vec![
            windows_category(
                "windows.antivirus",
                "Windows Security Center — antivirus category",
                WSC_SECURITY_PROVIDER_ANTIVIRUS as u32,
            ),
            windows_category(
                "windows.firewall",
                "Windows Security Center — firewall category",
                WSC_SECURITY_PROVIDER_FIREWALL as u32,
            ),
        ]
    }
    #[cfg(not(target_os = "windows"))]
    {
        ["windows.antivirus", "windows.firewall"]
            .into_iter()
            .map(|provider_id| SecurityProviderStatus {
                provider_id: provider_id.into(),
                label: if provider_id.ends_with("antivirus") {
                    "Windows Security Center — antivirus category".into()
                } else {
                    "Windows Security Center — firewall category".into()
                },
                status: "unsupported".into(),
                source: "Windows Security Center API (Windows only)".into(),
                detail: "This provider adapter is available only on Windows.".into(),
                observed_at_unix_ms: observed_at_unix_ms(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::status_from_windows_result;

    #[test]
    fn maps_documented_windows_health_values_without_overstating_protection() {
        assert_eq!(status_from_windows_result(0, 0).0, "healthy");
        assert_eq!(status_from_windows_result(0, 1).0, "not_monitored");
        assert_eq!(status_from_windows_result(0, 2).0, "alert");
        assert_eq!(status_from_windows_result(0, 3).0, "attention");
        assert_eq!(status_from_windows_result(0, 99).0, "unknown");
    }

    #[test]
    fn distinguishes_stopped_service_and_api_failure() {
        assert_eq!(status_from_windows_result(1, 2).0, "unavailable");
        assert_eq!(status_from_windows_result(-2147467259, 0).0, "unknown");
    }
}
