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
pub fn read_only_status(
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
            .map(|provider_id| SecurityProviderStatus {
                provider_id: provider_id.clone(),
                label: if provider_id == "windows.antivirus" {
                    "Windows Security Center — antivirus category".into()
                } else {
                    "Windows Security Center — firewall category".into()
                },
                status: "unsupported".into(),
                source: "Windows Security Center API (Windows only)".into(),
                detail: "This provider adapter is available only on Windows.".into(),
                observed_at_unix_ms: observed_at_unix_ms(),
            })
            .collect())
    }
}


/// Detailed, read-only Microsoft Defender status from the documented
/// MSFT_MpComputerStatus WMI class. This is separate from Windows Security
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
    pub signature_last_updated: Option<String>,
    pub signature_version: Option<String>,
    pub engine_version: Option<String>,
    pub product_version: Option<String>,
    pub service_version: Option<String>,
    pub checked_at_unix_ms: u64,
}

#[cfg(target_os = "windows")]
#[derive(Debug, serde::Deserialize)]
struct DefenderWmiResult {
    antivirus_enabled: bool,
    real_time_protection_enabled: bool,
    antivirus_signature_age_days: u32,
    antivirus_signature_last_updated: Option<String>,
    antivirus_signature_version: String,
    engine_version: String,
    product_version: String,
    service_version: String,
}

#[cfg(target_os = "windows")]
const DEFENDER_WMI_SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$s = Get-CimInstance -Namespace 'root/Microsoft/Windows/Defender' -ClassName 'MSFT_MpComputerStatus' -ErrorAction Stop
if ($null -eq $s) { throw 'Defender status is unavailable' }
[pscustomobject]@{
  antivirus_enabled = [bool]$s.AntivirusEnabled
  real_time_protection_enabled = [bool]$s.RealTimeProtectionEnabled
  antivirus_signature_age_days = [uint32]$s.AntivirusSignatureAge
  antivirus_signature_last_updated = if ($null -ne $s.AntivirusSignatureLastUpdated) { $s.AntivirusSignatureLastUpdated.ToUniversalTime().ToString('o') } else { $null }
  antivirus_signature_version = [string]$s.AntivirusSignatureVersion
  engine_version = [string]$s.AMEngineVersion
  product_version = [string]$s.AMProductVersion
  service_version = [string]$s.AMServiceVersion
} | ConvertTo-Json -Compress
"#;

#[cfg(target_os = "windows")]
fn parse_defender_wmi_result(output: &[u8]) -> Result<DefenderWmiResult, String> {
    let parsed: DefenderWmiResult = serde_json::from_slice(output)
        .map_err(|_| "Windows Defender returned an invalid status response.".to_string())?;
    for value in [
        &parsed.antivirus_signature_version,
        &parsed.engine_version,
        &parsed.product_version,
        &parsed.service_version,
    ] {
        if value.len() > 64 || value.chars().any(char::is_control) {
            return Err("Windows Defender returned invalid version metadata.".into());
        }
    }
    if parsed.antivirus_signature_last_updated.as_ref().is_some_and(|value| {
        value.len() > 40 || value.chars().any(char::is_control)
    }) {
        return Err("Windows Defender returned an invalid signature timestamp.".into());
    }
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

    // Resolve the inbox Windows PowerShell executable from SystemRoot rather
    // than searching PATH. The script and all arguments are constants; no
    // caller-controlled text is interpolated or executed.
    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| "Windows SystemRoot is unavailable; Defender status cannot be queried.".to_string())?;
    let executable = std::path::PathBuf::from(system_root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    if !executable.is_file() {
        return Err("Windows PowerShell is unavailable; Defender status cannot be queried.".into());
    }

    let mut child = Command::new(executable)
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", DEFENDER_WMI_SCRIPT])
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
            return Err("Windows Defender status is unavailable or the query was denied.".to_string());
        }
        let parsed = parse_defender_wmi_result(&output)?;
        Ok(DefenderStatus {
            status: "reported".into(),
            source: "Microsoft Defender MSFT_MpComputerStatus (read-only WMI)".into(),
            detail: "Local Defender status was read. Signature age and versions are provider-reported; comparison with the latest Microsoft release has not been performed.".into(),
            antivirus_enabled: Some(parsed.antivirus_enabled),
            real_time_protection_enabled: Some(parsed.real_time_protection_enabled),
            signature_age_days: Some(parsed.antivirus_signature_age_days),
            signature_last_updated: parsed.antivirus_signature_last_updated,
            signature_version: Some(parsed.antivirus_signature_version),
            engine_version: Some(parsed.engine_version),
            product_version: Some(parsed.product_version),
            service_version: Some(parsed.service_version),
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

#[cfg(all(test, target_os = "windows"))]
mod defender_tests {
    use super::parse_defender_wmi_result;

    const VALID: &[u8] = br#"{
      "antivirus_enabled": true,
      "real_time_protection_enabled": true,
      "antivirus_signature_age_days": 1,
      "antivirus_signature_last_updated": "2026-11-18T01:02:03.0000000Z",
      "antivirus_signature_version": "1.2.3.4",
      "engine_version": "2.3.4.5",
      "product_version": "4.5.6.7",
      "service_version": "8.9.0.1"
    }"#;

    #[test]
    fn accepts_bounded_read_only_defender_metadata() {
        let result = parse_defender_wmi_result(VALID).unwrap();
        assert!(result.antivirus_enabled);
        assert!(result.real_time_protection_enabled);
        assert_eq!(result.antivirus_signature_age_days, 1);
        assert_eq!(result.antivirus_signature_version, "1.2.3.4");
    }

    #[test]
    fn rejects_malformed_and_unbounded_version_metadata() {
        assert!(parse_defender_wmi_result(b"not-json").is_err());
        let oversized = VALID
            .windows(b"1.2.3.4".len())
            .position(|window| window == b"1.2.3.4")
            .map(|index| {
                let mut bytes = VALID.to_vec();
                bytes.splice(index..index + b"1.2.3.4".len(), vec![b'v'; 65]);
                bytes
            })
            .unwrap();
        assert!(parse_defender_wmi_result(&oversized).is_err());
    }

    #[test]
    fn rejects_control_characters_in_vendor_metadata() {
        let mut bytes = VALID.to_vec();
        let index = bytes.windows(b"1.2.3.4".len())
            .position(|window| window == b"1.2.3.4").unwrap();
        bytes[index] = 0x01;
        assert!(parse_defender_wmi_result(&bytes).is_err());
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
