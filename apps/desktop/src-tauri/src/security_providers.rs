//! Read-only operating-system security provider status adapters.
//!
//! The first adapter reads Windows Security Center's aggregate antivirus and
//! firewall health only. It does not identify individual products, inspect
//! files, collect threat events, or change security settings.

use serde::Serialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct SecurityProviderStatus {
    pub provider_id: String,
    pub label: String,
    pub status: String,
    pub source: String,
    pub detail: String,
    pub observed_at_unix_ms: u64,
    pub assessment: String,
    pub recommendation: String,
    pub product_identity_known: bool,
}

fn assess_provider_status(
    provider_id: &str,
    status: &str,
    source: &str,
    observed_at: u64,
) -> (String, String, bool) {
    use ujani_core::{
        assess_category, EvidenceStatus, ProviderObservation, SecurityCategory,
    };
    let category = match provider_id {
        "windows.antivirus" => SecurityCategory::MalwareProtection,
        "windows.firewall" => SecurityCategory::Firewall,
        _ => return (
            "cannot_verify".into(),
            "The provider category is unsupported; check the operating-system security console.".into(),
            false,
        ),
    };
    let evidence_status = match status {
        "healthy" => EvidenceStatus::Healthy,
        "attention" => EvidenceStatus::Attention,
        "alert" => EvidenceStatus::Alert,
        "not_monitored" => EvidenceStatus::NotMonitored,
        "unavailable" => EvidenceStatus::Unavailable,
        "unsupported" => EvidenceStatus::Unsupported,
        _ => EvidenceStatus::Unknown,
    };
    let observation = ProviderObservation {
        provider_id: provider_id.into(),
        display_name: None,
        category,
        status: evidence_status,
        evidence_source: source.into(),
        observed_at_unix_ms: observed_at,
        product_identity_known: false,
    };
    let result = assess_category(category, &[observation], observed_at, 0);
    let assessment = match result.assessment {
        ujani_core::PostureAssessment::ProtectionReportedHealthy => "protection_reported_healthy",
        ujani_core::PostureAssessment::ProtectionNeedsAttention => "protection_needs_attention",
        ujani_core::PostureAssessment::ThreatOrProtectionAlert => "threat_or_protection_alert",
        ujani_core::PostureAssessment::NoProviderReported => "no_provider_reported",
        ujani_core::PostureAssessment::CannotVerify => "cannot_verify",
    };
    (assessment.into(), result.recommendation.into(), result.product_identity_known)
}

fn observed_at_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(any(target_os = "windows", test))]
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
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{
        GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32,
    };

    // Load WSC dynamically from System32 only: Windows Server and some reduced
    // Windows images do not ship wscapi.dll. Restricting the search path also
    // avoids loading an untrusted DLL from the working or application directory.
    let library_name: Vec<u16> = "wscapi.dll".encode_utf16().chain(std::iter::once(0)).collect();
    let module = unsafe {
        LoadLibraryExW(
            library_name.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_SYSTEM32,
        )
    };

    let (status, detail) = if module.is_null() {
        (
            "unavailable",
            "Windows Security Center API library is not available on this Windows installation.",
        )
    } else {
        let function = unsafe {
            GetProcAddress(module, b"WscGetSecurityProviderHealth\0".as_ptr())
        };
        let result = match function {
            Some(function) => {
                type QueryHealth = unsafe extern "system" fn(u32, *mut i32) -> i32;
                let query: QueryHealth = unsafe {
                    std::mem::transmute::<windows_sys::Win32::Foundation::FARPROC, QueryHealth>(
                        Some(function),
                    )
                };
                let mut health = -1i32;
                let hresult = unsafe { query(provider_mask, &mut health) };
                status_from_windows_result(hresult, health)
            }
            None => (
                "unsupported",
                "Windows Security Center health API is not exported by this installation.",
            ),
        };
        unsafe {
            FreeLibrary(module);
        }
        result
    };

    let observed_at = observed_at_unix_ms();
    let source = "Windows Security Center aggregate health API";
    let (assessment, recommendation, product_identity_known) =
        assess_provider_status(provider_id, status, source, observed_at);
    SecurityProviderStatus {
        provider_id: provider_id.into(),
        label: label.into(),
        status: status.into(),
        source: source.into(),
        detail: detail.into(),
        observed_at_unix_ms: observed_at,
        assessment,
        recommendation,
        product_identity_known,
    }
}

/// Read the supported provider categories on this platform. No polling occurs;
/// callers invoke this only after the user explicitly requests a status check.
fn read_provider_status(
    requested_provider_ids: &[String],
) -> Result<Vec<SecurityProviderStatus>, String> {
    if requested_provider_ids.is_empty()
        || requested_provider_ids.len() > 2
        || requested_provider_ids.iter().any(|id| {
            !matches!(id.as_str(), "windows.antivirus" | "windows.firewall")
        })
        || requested_provider_ids
            .iter()
            .enumerate()
            .any(|(index, id)| requested_provider_ids[..index].contains(id))
    {
        return Err("Security provider selection is invalid or exceeds the supported limit.".into());
    }

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::SecurityCenter::{
            WSC_SECURITY_PROVIDER_ANTIVIRUS, WSC_SECURITY_PROVIDER_FIREWALL,
        };

        Ok(requested_provider_ids
            .iter()
            .map(|provider_id| match provider_id.as_str() {
                "windows.antivirus" => windows_category(
                    provider_id,
                    "Windows Security Center — antivirus category",
                    WSC_SECURITY_PROVIDER_ANTIVIRUS as u32,
                ),
                "windows.firewall" => windows_category(
                    provider_id,
                    "Windows Security Center — firewall category",
                    WSC_SECURITY_PROVIDER_FIREWALL as u32,
                ),
                _ => unreachable!("provider identifiers were validated above"),
            })
            .collect())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(requested_provider_ids
            .iter()
            .map(|provider_id| {
                let observed_at = observed_at_unix_ms();
                let source = "Windows Security Center API (Windows only)";
                let (assessment, recommendation, product_identity_known) =
                    assess_provider_status(provider_id, "unsupported", source, observed_at);
                SecurityProviderStatus {
                    provider_id: provider_id.clone(),
                    label: if provider_id == "windows.antivirus" {
                        "Windows Security Center — antivirus category".into()
                    } else {
                        "Windows Security Center — firewall category".into()
                    },
                    status: "unsupported".into(),
                    source: source.into(),
                    detail: "This provider adapter is available only on Windows.".into(),
                    observed_at_unix_ms: observed_at,
                    assessment,
                    recommendation,
                    product_identity_known,
                }
            })
            .collect())
    }
}


#[derive(Debug, Clone, Serialize)]
pub struct SecurityProductStatus {
    pub provider_id: String,
    pub provider_type: String,
    pub display_name: Option<String>,
    pub product_state: String,
    pub signature_status: String,
    pub antivirus_update_substatus: Option<String>,
    pub firewall_domain_profile_substatus: Option<String>,
    pub firewall_private_profile_substatus: Option<String>,
    pub firewall_public_profile_substatus: Option<String>,
    pub product_state_timestamp: Option<String>,
    pub assessment: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityProviderSnapshot {
    pub providers: Vec<SecurityProviderStatus>,
    pub products: Vec<SecurityProductStatus>,
    pub products_status: String,
    pub products_detail: String,
    pub products_checked_at_unix_ms: u64,
}

struct ProductInventoryResult {
    products: Vec<SecurityProductStatus>,
    status: String,
    detail: String,
    checked_at_unix_ms: u64,
}

impl ProductInventoryResult {
    fn unavailable(detail: &str) -> Self {
        Self {
            products: Vec::new(),
            status: "unavailable".into(),
            detail: detail.into(),
            checked_at_unix_ms: unix_now_ms(),
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn unsupported() -> Self {
        Self {
            products: Vec::new(),
            status: "unsupported".into(),
            detail: "Windows Security Center product enumeration is available only on supported Windows client editions.".into(),
            checked_at_unix_ms: unix_now_ms(),
        }
    }
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

pub async fn read_only_status(
    requested_provider_ids: &[String],
) -> Result<SecurityProviderSnapshot, String> {
    let providers = read_provider_status(requested_provider_ids)?;

    #[cfg(target_os = "windows")]
    let inventory = {
        let ids = requested_provider_ids.to_vec();
        match tokio::time::timeout(
            Duration::from_secs(8),
            tokio::task::spawn_blocking(move || {
                std::thread::spawn(move || read_windows_security_products(&ids))
                    .join()
                    .unwrap_or_else(|_| ProductInventoryResult::unavailable(
                        "Windows Security Center product enumeration worker failed."
                    ))
            }),
        )
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => ProductInventoryResult::unavailable(
                "Windows Security Center product enumeration worker failed."
            ),
            Err(_) => ProductInventoryResult::unavailable(
                "Windows Security Center product enumeration timed out; individual product status is unknown."
            ),
        }
    };

    #[cfg(not(target_os = "windows"))]
    let inventory = ProductInventoryResult::unsupported();

    Ok(SecurityProviderSnapshot {
        providers,
        products: inventory.products,
        products_status: inventory.status,
        products_detail: inventory.detail,
        products_checked_at_unix_ms: inventory.checked_at_unix_ms,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityProductStatus {
    pub provider_id: String,
    pub provider_type: String,
    pub display_name: Option<String>,
    pub product_state: String,
    pub signature_status: String,
    pub antivirus_update_substatus: Option<String>,
    pub firewall_domain_profile_substatus: Option<String>,
    pub firewall_private_profile_substatus: Option<String>,
    pub firewall_public_profile_substatus: Option<String>,
    pub product_state_timestamp: Option<String>,
    pub assessment: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityProviderSnapshot {
    pub providers: Vec<SecurityProviderStatus>,
    pub products: Vec<SecurityProductStatus>,
    pub products_status: String,
    pub products_detail: String,
    pub products_checked_at_unix_ms: u64,
}

struct ProductInventoryResult {
    products: Vec<SecurityProductStatus>,
    status: String,
    detail: String,
    checked_at_unix_ms: u64,
}

impl ProductInventoryResult {
    fn unavailable(detail: &str) -> Self {
        Self {
            products: Vec::new(),
            status: "unavailable".into(),
            detail: detail.into(),
            checked_at_unix_ms: observed_at_unix_ms(),
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn unsupported() -> Self {
        Self {
            products: Vec::new(),
            status: "unsupported".into(),
            detail: "Windows Security Center product enumeration is available only on supported Windows client editions.".into(),
            checked_at_unix_ms: observed_at_unix_ms(),
        }
    }
}

pub async fn read_only_status(
    requested_provider_ids: &[String],
) -> Result<SecurityProviderSnapshot, String> {
    let providers = read_provider_status(requested_provider_ids)?;

    #[cfg(target_os = "windows")]
    let inventory = {
        let ids = requested_provider_ids.to_vec();
        match tokio::time::timeout(
            std::time::Duration::from_secs(8),
            tokio::task::spawn_blocking(move || {
                std::thread::spawn(move || read_windows_security_products(&ids))
                    .join()
                    .unwrap_or_else(|_| ProductInventoryResult::unavailable(
                        "Windows Security Center product enumeration worker failed."
                    ))
            }),
        )
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => ProductInventoryResult::unavailable(
                "Windows Security Center product enumeration worker failed."
            ),
            Err(_) => ProductInventoryResult::unavailable(
                "Windows Security Center product enumeration timed out; individual product status is unknown."
            ),
        }
    };

    #[cfg(not(target_os = "windows"))]
    let inventory = ProductInventoryResult::unsupported();

    Ok(SecurityProviderSnapshot {
        providers,
        products: inventory.products,
        products_status: inventory.status,
        products_detail: inventory.detail,
        products_checked_at_unix_ms: inventory.checked_at_unix_ms,
    })
}

/// Detailed, read-only Microsoft Defender status from the documented
/// Get-MpComputerStatus cmdlet. This is separate from Windows Security
/// Center's aggregate category health and does not claim to verify the latest
/// vendor release.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct DefenderStatus {
    pub status: String,
    pub source: String,
    pub detail: String,
    pub antivirus_enabled: Option<bool>,
    pub real_time_protection_enabled: Option<bool>,
    pub signature_age_days: Option<u32>,
    pub signature_update_status: String,
    pub signature_last_updated: Option<String>,
    pub signature_version: Option<String>,
    pub engine_version: Option<String>,
    pub product_version: Option<String>,
    pub service_version: Option<String>,
    pub checked_at_unix_ms: u64,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, serde::Deserialize)]
struct DefenderStatusResult {
    antivirus_enabled: Option<bool>,
    real_time_protection_enabled: Option<bool>,
    antivirus_signature_age_days: Option<u32>,
    defender_signatures_out_of_date: Option<bool>,
    antivirus_signature_last_updated: Option<String>,
    antivirus_signature_version: Option<String>,
    engine_version: Option<String>,
    product_version: Option<String>,
    service_version: Option<String>,
}

#[cfg(target_os = "windows")]
const DEFENDER_STATUS_SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$s = Get-MpComputerStatus -ErrorAction Stop
if ($null -eq $s) { throw 'Defender status is unavailable' }
[pscustomobject]@{
  antivirus_enabled = if ($null -ne $s.PSObject.Properties['AntivirusEnabled'] -and $null -ne $s.AntivirusEnabled) { [bool]$s.AntivirusEnabled } else { $null }
  real_time_protection_enabled = if ($null -ne $s.PSObject.Properties['RealTimeProtectionEnabled'] -and $null -ne $s.RealTimeProtectionEnabled) { [bool]$s.RealTimeProtectionEnabled } else { $null }
  antivirus_signature_age_days = if ($null -ne $s.PSObject.Properties['AntivirusSignatureAge'] -and $null -ne $s.AntivirusSignatureAge) { [uint32]$s.AntivirusSignatureAge } else { $null }
  defender_signatures_out_of_date = if ($null -ne $s.PSObject.Properties['DefenderSignaturesOutOfDate'] -and $null -ne $s.PSObject.Properties['DefenderSignaturesOutOfDate'].Value) { [bool]$s.PSObject.Properties['DefenderSignaturesOutOfDate'].Value } else { $null }
  antivirus_signature_last_updated = if ($null -ne $s.AntivirusSignatureLastUpdated) { $s.AntivirusSignatureLastUpdated.ToUniversalTime().ToString('o') } else { $null }
  antivirus_signature_version = if ($null -ne $s.PSObject.Properties['AntivirusSignatureVersion'] -and $null -ne $s.AntivirusSignatureVersion) { [string]$s.AntivirusSignatureVersion } else { $null }
  engine_version = if ($null -ne $s.PSObject.Properties['AMEngineVersion'] -and $null -ne $s.AMEngineVersion) { [string]$s.AMEngineVersion } else { $null }
  product_version = if ($null -ne $s.PSObject.Properties['AMProductVersion'] -and $null -ne $s.AMProductVersion) { [string]$s.AMProductVersion } else { $null }
  service_version = if ($null -ne $s.PSObject.Properties['AMServiceVersion'] -and $null -ne $s.AMServiceVersion) { [string]$s.AMServiceVersion } else { $null }
} | ConvertTo-Json -Compress
"#;

#[cfg(any(target_os = "windows", test))]
fn signature_update_status(signatures_out_of_date: Option<bool>) -> &'static str {
    match signatures_out_of_date {
        Some(true) => "out_of_date",
        Some(false) => "within_policy_age_limit",
        None => "unknown",
    }
}

#[cfg(any(target_os = "windows", test))]
fn parse_defender_status_result(output: &[u8]) -> Result<DefenderStatusResult, String> {
    let mut parsed: DefenderStatusResult = serde_json::from_slice(output)
        .map_err(|_| "Windows Defender returned an invalid status response.".to_string())?;
    for value in [
        parsed.antivirus_signature_version.as_ref(),
        parsed.engine_version.as_ref(),
        parsed.product_version.as_ref(),
        parsed.service_version.as_ref(),
    ].into_iter().flatten() {
        if value.len() > 64 || value.chars().any(char::is_control) {
            return Err("Windows Defender returned invalid version metadata.".into());
        }
    }
    if parsed.antivirus_signature_last_updated.as_ref().is_some_and(|value| {
        value.is_empty() || value.len() > 40 || value.chars().any(char::is_control)
    }) {
        return Err("Windows Defender returned an invalid signature timestamp.".into());
    }
    parsed.antivirus_signature_age_days = parsed.antivirus_signature_age_days.filter(|age| *age <= 3650);
    parsed.antivirus_signature_version = parsed.antivirus_signature_version.filter(|value| !value.is_empty());
    parsed.engine_version = parsed.engine_version.filter(|value| !value.is_empty());
    parsed.product_version = parsed.product_version.filter(|value| !value.is_empty());
    parsed.service_version = parsed.service_version.filter(|value| !value.is_empty());
    Ok(parsed)
}

#[cfg(target_os = "windows")]
pub async fn read_defender_status() -> Result<DefenderStatus, String> {
    use std::process::Stdio;
    use tokio::io::AsyncReadExt;
    use tokio::process::Command;
    use tokio::time::{timeout, Duration};

    const MAX_OUTPUT_BYTES: usize = 16 * 1024;
    const QUERY_TIMEOUT: Duration = Duration::from_secs(8);
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // Resolve the inbox Windows PowerShell executable from the OS-reported
    // system directory, never from PATH or a caller-controlled environment variable.
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
    let mut system_directory = [0u16; 32_768];
    let directory_length = unsafe {
        GetSystemDirectoryW(system_directory.as_mut_ptr(), system_directory.len() as u32)
    };
    if directory_length == 0 || directory_length as usize >= system_directory.len() {
        return Err("Windows system directory could not be resolved safely.".into());
    }
    let system_directory = std::ffi::OsString::from_wide(
        &system_directory[..directory_length as usize],
    );
    let executable = std::path::PathBuf::from(system_directory)
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    if !executable.is_file() {
        return Err("Windows PowerShell is unavailable; Defender status cannot be queried.".into());
    }

    let mut child = Command::new(executable)
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", DEFENDER_STATUS_SCRIPT])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| "Could not start the read-only Defender status query.".to_string())?;

    let mut stdout = child.stdout.take()
        .ok_or_else(|| "Defender status output is unavailable.".to_string())?;
    let query = async {
        let mut output = Vec::new();
        stdout.take((MAX_OUTPUT_BYTES + 1) as u64).read_to_end(&mut output).await
            .map_err(|_| "Could not read Defender status output.".to_string())?;
        if output.len() > MAX_OUTPUT_BYTES {
            let _ = child.kill().await;
            return Err("Defender status output exceeded the safety limit.".to_string());
        }
        let exit = child.wait().await
            .map_err(|_| "Defender status query did not complete.".to_string())?;
        if !exit.success() {
            return Err(
                "Windows Defender status is unavailable or the query was denied.".to_string(),
            );
        }
        let parsed = parse_defender_status_result(&output)?;
        let signature_update_status = signature_update_status(
            parsed.defender_signatures_out_of_date,
        );
        Ok(DefenderStatus {
            status: "reported".into(),
            source: "Microsoft Defender Get-MpComputerStatus (read-only)".into(),
            detail: "Local Defender status was read. Signature age and versions are provider-reported; comparison with the latest Microsoft release has not been performed.".into(),
            antivirus_enabled: parsed.antivirus_enabled,
            real_time_protection_enabled: parsed.real_time_protection_enabled,
            signature_age_days: parsed.antivirus_signature_age_days,
            signature_update_status: signature_update_status.into(),
            signature_last_updated: parsed.antivirus_signature_last_updated,
            signature_version: parsed.antivirus_signature_version,
            engine_version: parsed.engine_version,
            product_version: parsed.product_version,
            service_version: parsed.service_version,
            checked_at_unix_ms: observed_at_unix_ms(),
        })
    };

    match timeout(QUERY_TIMEOUT, query).await {
        Ok(result) => result,
        Err(_) => {
            let _ = child.kill().await;
            Err("Defender status query timed out; no status was assumed.".into())
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub async fn read_defender_status() -> Result<DefenderStatus, String> {
    Err("Detailed Microsoft Defender status is supported only on Windows.".into())
}

#[cfg(test)]
mod defender_tests {
    use super::parse_defender_status_result;

    const VALID: &[u8] = br#"{
      "antivirus_enabled": true,
      "real_time_protection_enabled": true,
      "antivirus_signature_age_days": 1,
      "defender_signatures_out_of_date": false,
      "antivirus_signature_last_updated": "2026-11-18T01:02:03.0000000Z",
      "antivirus_signature_version": "1.2.3.4",
      "engine_version": "2.3.4.5",
      "product_version": "4.5.6.7",
      "service_version": "8.9.0.1"
    }"#;

    #[test]
    fn accepts_bounded_read_only_defender_metadata() {
        let result = parse_defender_status_result(VALID).unwrap();
        assert_eq!(result.antivirus_enabled, Some(true));
        assert_eq!(result.real_time_protection_enabled, Some(true));
        assert_eq!(result.antivirus_signature_age_days, Some(1));
        assert_eq!(result.defender_signatures_out_of_date, Some(false));
        assert_eq!(result.antivirus_signature_version.as_deref(), Some("1.2.3.4"));
    }

    #[test]
    fn treats_unavailable_signature_age_sentinel_as_unknown() {
        let output = VALID.replace("\"antivirus_signature_age_days\": 1", "\"antivirus_signature_age_days\": 4294967295");
        let parsed = super::parse_defender_status_result(output.as_bytes()).unwrap();
        assert_eq!(parsed.antivirus_signature_age_days, None);
    }

    #[test]
    fn maps_defender_reported_signature_freshness() {
        assert_eq!(super::signature_update_status(Some(false)), "within_policy_age_limit");
        assert_eq!(super::signature_update_status(Some(true)), "out_of_date");
        assert_eq!(super::signature_update_status(None), "unknown");
    }

    #[test]
    fn missing_protection_fields_remain_unknown_not_disabled() {
        let mut value: serde_json::Value = serde_json::from_slice(VALID).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("antivirus_enabled");
        object.remove("real_time_protection_enabled");
        object.remove("antivirus_signature_age_days");
        object.remove("antivirus_signature_version");
        object.remove("engine_version");
        object.remove("product_version");
        object.remove("service_version");
        let bytes = serde_json::to_vec(&value).unwrap();
        let parsed = super::parse_defender_status_result(&bytes).unwrap();
        assert_eq!(parsed.antivirus_enabled, None);
        assert_eq!(parsed.real_time_protection_enabled, None);
        assert_eq!(parsed.antivirus_signature_age_days, None);
        assert_eq!(parsed.antivirus_signature_version, None);
        assert_eq!(parsed.engine_version, None);
        assert_eq!(parsed.product_version, None);
        assert_eq!(parsed.service_version, None);
    }

    #[test]
    fn empty_version_strings_are_normalized_to_unknown() {
        let mut value: serde_json::Value = serde_json::from_slice(VALID).unwrap();
        let object = value.as_object_mut().unwrap();
        object.insert("antivirus_signature_version".into(), serde_json::Value::String(String::new()));
        object.insert("engine_version".into(), serde_json::Value::String(String::new()));
        object.insert("product_version".into(), serde_json::Value::String(String::new()));
        object.insert("service_version".into(), serde_json::Value::String(String::new()));
        let bytes = serde_json::to_vec(&value).unwrap();
        let parsed = super::parse_defender_status_result(&bytes).unwrap();
        assert_eq!(parsed.antivirus_signature_version, None);
        assert_eq!(parsed.engine_version, None);
        assert_eq!(parsed.product_version, None);
        assert_eq!(parsed.service_version, None);
    }

    #[test]
    fn missing_defender_freshness_field_stays_unknown() {
        let missing = VALID
            .windows(b",\n      \"defender_signatures_out_of_date\": false".len())
            .position(|window| window == b",\n      \"defender_signatures_out_of_date\": false")
            .map(|index| {
                let mut bytes = VALID.to_vec();
                bytes.drain(
                    index..index + b",\n      \"defender_signatures_out_of_date\": false".len(),
                );
                bytes
            })
            .unwrap();
        let parsed = super::parse_defender_status_result(&missing).unwrap();
        assert_eq!(parsed.defender_signatures_out_of_date, None);
        assert_eq!(
            super::signature_update_status(parsed.defender_signatures_out_of_date),
            "unknown"
        );
    }

    #[test]
    fn rejects_malformed_and_unbounded_version_metadata() {
        assert!(parse_defender_status_result(b"not-json").is_err());
        let oversized = VALID
            .windows(b"1.2.3.4".len())
            .position(|window| window == b"1.2.3.4")
            .map(|index| {
                let mut bytes = VALID.to_vec();
                bytes.splice(index..index + b"1.2.3.4".len(), vec![b'v'; 65]);
                bytes
            })
            .unwrap();
        assert!(parse_defender_status_result(&oversized).is_err());
    }

    #[test]
    fn rejects_control_characters_in_vendor_metadata() {
        let mut bytes = VALID.to_vec();
        let index = bytes.windows(b"1.2.3.4".len())
            .position(|window| window == b"1.2.3.4").unwrap();
        bytes[index] = 0x01;
        assert!(parse_defender_status_result(&bytes).is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::{assess_provider_status, status_from_windows_result};

    #[test]
    fn shared_posture_model_preserves_aggregate_identity_limits() {
        let (assessment, recommendation, identity) = assess_provider_status(
            "windows.antivirus",
            "healthy",
            "Windows Security Center aggregate health API",
            100_000,
        );
        assert_eq!(assessment, "protection_reported_healthy");
        assert!(!identity);
        assert!(recommendation.contains("Keep the existing protection enabled"));

        let (unknown, _, _) = assess_provider_status(
            "windows.firewall",
            "unsupported",
            "Windows Security Center API (Windows only)",
            100_000,
        );
        assert_eq!(unknown, "cannot_verify");
    }

    #[test]
    fn maps_documented_windows_health_values_without_overstating_protection() {
        assert_eq!(status_from_windows_result(0, 0).0, "healthy");
        assert_eq!(status_from_windows_result(0, 1).0, "not_monitored");
        assert_eq!(status_from_windows_result(0, 2).0, "alert");
        assert_eq!(status_from_windows_result(0, 3).0, "attention");
        assert_eq!(status_from_windows_result(0, 99).0, "unknown");
    }

    #[test]
    fn rejects_empty_unknown_and_duplicate_provider_selections() {
        assert!(super::read_only_status(&[]).is_err());
        assert!(super::read_only_status(&["windows.defender".into()]).is_err());
        assert!(super::read_only_status(&[
            "windows.antivirus".into(),
            "windows.antivirus".into()
        ]).is_err());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn reports_windows_categories_as_unsupported_on_other_platforms() {
        let result = super::read_only_status(&["windows.firewall".into()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, "unsupported");
    }

    #[test]
    fn distinguishes_stopped_service_and_api_failure() {
        assert_eq!(status_from_windows_result(1, 2).0, "unavailable");
        assert_eq!(status_from_windows_result(-2147467259, 0).0, "unknown");
    }
}
