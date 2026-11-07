//! Auditable execution provenance contracts.
//!
//! Receipts are metadata contracts only. They do not execute or verify a model
//! result by themselves; trusted adapters must compute digests from the exact
//! bytes sent/received and persist receipts only after successful completion.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Provenance classification for where inference was performed.
pub enum ExecutionLocation {
    LocalCpu,
    LocalGpu,
    LocalUnverified,
    ExternalProvider,
    DeterministicTool,
}

/// Confidence attached to a network observation; unknown values never become zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NetworkObservation<T> {
    Measured { value: T },
    Inferred { value: T },
    Unknown { reason: String },
}

impl<T> Default for NetworkObservation<T> {
    fn default() -> Self {
        Self::Unknown {
            reason: "Not observed at the UJANI application boundary.".into(),
        }
    }
}

/// Network telemetry with explicit evidence quality and a bounded measurement scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTelemetry {
    #[serde(default)]
    pub destinations: NetworkObservation<Vec<String>>,
    #[serde(default)]
    pub resolved_addresses: NetworkObservation<Vec<String>>,
    #[serde(default)]
    pub application_request_body_bytes: NetworkObservation<u64>,
    #[serde(default)]
    pub application_response_body_bytes: NetworkObservation<u64>,
    #[serde(default = "default_network_note")]
    pub note: String,
}

fn default_network_note() -> String {
    "Unknown means UJANI did not observe this value; application-body byte counts exclude HTTP headers, TLS, transport framing, DNS traffic and other process/system traffic.".into()
}

impl Default for NetworkTelemetry {
    fn default() -> Self {
        Self {
            destinations: NetworkObservation::default(),
            resolved_addresses: NetworkObservation::default(),
            application_request_body_bytes: NetworkObservation::default(),
            application_response_body_bytes: NetworkObservation::default(),
            note: default_network_note(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Metadata-only record of an inference operation and exact input/output digests.
pub struct ExecutionReceipt {
    pub schema_version: u16,
    pub task_id: String,
    pub operation_id: String,
    pub location: ExecutionLocation,
    pub adapter_id: String,
    pub adapter_version: String,
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    pub input_sha256: String,
    pub output_sha256: String,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub user_approval_id: Option<String>,
    /// Missing in legacy v1 receipts; deserializes to explicit unknown observations.
    #[serde(default)]
    pub network: NetworkTelemetry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Structural validation failures for execution provenance receipts.
pub enum ReceiptError {
    UnsupportedSchema,
    MissingIdentity,
    InvalidIdentifier,
    InvalidDigest,
    InvalidTimeRange,
    MissingModelProvenance,
    MissingExternalApproval,
    InvalidNetworkTelemetry,
}

fn valid_identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.chars().all(|c| c.is_ascii_alphanumeric() || "._:@+-_".contains(c))
}

fn valid_model_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.chars().any(char::is_control)
        && !value.chars().any(char::is_whitespace)
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains("//")
        && !value.split('/').any(|part| part == "." || part == "..")
        && value.chars().all(|c| c.is_ascii_alphanumeric() || "._:@+-_/".contains(c))
}

/// Validate an Ollama/provider model identifier before dispatching inference.
pub fn validate_model_identifier(value: &str) -> bool {
    valid_model_identifier(value)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn valid_unknown_reason(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn valid_network_name(value: &str) -> bool {
    let normalized = value.strip_suffix('.').unwrap_or(value);
    !normalized.is_empty() && value.len() <= 260
        && normalized.bytes().all(|byte| byte.is_ascii_alphanumeric() || b".:-[]".contains(&byte))
        && !normalized.starts_with('.') && !normalized.ends_with('.')
        && !normalized.contains("..")
}

fn validate_network_telemetry(network: &NetworkTelemetry) -> Result<(), ReceiptError> {
    let validate_names = |observation: &NetworkObservation<Vec<String>>| match observation {
        NetworkObservation::Measured { value } | NetworkObservation::Inferred { value } => {
            !value.is_empty() && value.len() <= 16 && value.iter().all(|name| valid_network_name(name))
        }
        NetworkObservation::Unknown { reason } => valid_unknown_reason(reason),
    };
    let validate_bytes = |observation: &NetworkObservation<u64>| match observation {
        NetworkObservation::Measured { value } => *value <= 1_073_741_824,
        // An inferred zero would falsely imply no traffic; zero must be directly measured.
        NetworkObservation::Inferred { value } => *value > 0 && *value <= 1_073_741_824,
        NetworkObservation::Unknown { reason } => valid_unknown_reason(reason),
    };
    if !validate_names(&network.destinations)
        || !validate_names(&network.resolved_addresses)
        || !validate_bytes(&network.application_request_body_bytes)
        || !validate_bytes(&network.application_response_body_bytes)
        || network.note.len() > 512
        || network.note.chars().any(char::is_control)
    {
        return Err(ReceiptError::InvalidNetworkTelemetry);
    }
    Ok(())
}

/// Hash exact bytes using lowercase hexadecimal SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Validate structural provenance before a trusted adapter persists a receipt.
pub fn validate_receipt(receipt: &ExecutionReceipt) -> Result<(), ReceiptError> {
    if !matches!(receipt.schema_version, 1 | 2) {
        return Err(ReceiptError::UnsupportedSchema);
    }
    validate_network_telemetry(&receipt.network)?;
    if receipt.task_id.trim().is_empty()
        || receipt.operation_id.trim().is_empty()
        || receipt.adapter_id.trim().is_empty()
        || receipt.adapter_version.trim().is_empty()
    {
        return Err(ReceiptError::MissingIdentity);
    }
    if !valid_identifier(&receipt.task_id, 96)
        || !valid_identifier(&receipt.operation_id, 128)
        || !valid_identifier(&receipt.adapter_id, 96)
        || !valid_identifier(&receipt.adapter_version, 64)
        || receipt.model_id.as_ref().is_some_and(|v| !valid_model_identifier(v))
        || receipt.model_version.as_ref().is_some_and(|v| !valid_identifier(v, 64))
        || receipt.user_approval_id.as_ref().is_some_and(|v| !valid_identifier(v, 128))
    {
        return Err(ReceiptError::InvalidIdentifier);
    }
    if !valid_sha256(&receipt.input_sha256) || !valid_sha256(&receipt.output_sha256) {
        return Err(ReceiptError::InvalidDigest);
    }
    if receipt.started_at_unix_ms == 0
        || receipt.completed_at_unix_ms < receipt.started_at_unix_ms
    {
        return Err(ReceiptError::InvalidTimeRange);
    }
    if matches!(receipt.location, ExecutionLocation::LocalCpu | ExecutionLocation::LocalGpu | ExecutionLocation::LocalUnverified)
        && receipt.model_id.is_some() != receipt.model_version.is_some()
    {
        return Err(ReceiptError::MissingModelProvenance);
    }
    if receipt.location == ExecutionLocation::ExternalProvider
        && receipt.user_approval_id.as_deref().is_none_or(str::is_empty)
    {
        return Err(ReceiptError::MissingExternalApproval);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt() -> ExecutionReceipt {
        ExecutionReceipt {
            schema_version: 1,
            task_id: "task-1".into(),
            operation_id: "document.summarize".into(),
            location: ExecutionLocation::LocalCpu,
            adapter_id: "ollama.local".into(),
            adapter_version: "1.0.0".into(),
            model_id: Some("qwen2.5".into()),
            model_version: Some("sha256:abc123".into()),
            input_sha256: sha256_hex(b"input"),
            output_sha256: sha256_hex(b"output"),
            started_at_unix_ms: 100,
            completed_at_unix_ms: 200,
            user_approval_id: None,
            network: NetworkTelemetry::default(),
        }
    }

    #[test]
    fn accepts_v2_receipts_with_explicit_measured_inferred_and_unknown_network_values() {
        let mut r = receipt();
        r.schema_version = 2;
        r.network = NetworkTelemetry {
            destinations: NetworkObservation::Inferred { value: vec!["api.example.com".into()] },
            resolved_addresses: NetworkObservation::Inferred { value: vec!["203.0.113.8:443".into()] },
            application_request_body_bytes: NetworkObservation::Measured { value: 0 },
            application_response_body_bytes: NetworkObservation::Unknown {
                reason: "Response stream ended before the body could be measured.".into(),
            },
            note: "Measured body bytes only; no wire-level traffic observation.".into(),
        };
        assert_eq!(validate_receipt(&r), Ok(()));
    }

    #[test]
    fn legacy_v1_receipts_without_network_fields_remain_readable_as_unknown() {
        let mut value = serde_json::to_value(receipt()).unwrap();
        value.as_object_mut().unwrap().remove("network");
        let legacy: ExecutionReceipt = serde_json::from_value(value).unwrap();
        assert_eq!(legacy.schema_version, 1);
        assert!(matches!(legacy.network.application_request_body_bytes, NetworkObservation::Unknown { .. }));
        assert_eq!(validate_receipt(&legacy), Ok(()));
    }

    #[test]
    fn rejects_unbounded_or_malformed_network_telemetry() {
        let mut r = receipt();
        r.schema_version = 2;
        r.network.destinations = NetworkObservation::Measured { value: vec!["https://user:secret@example.com/path".into()] };
        assert_eq!(validate_receipt(&r), Err(ReceiptError::InvalidNetworkTelemetry));
        r.network = NetworkTelemetry::default();
        r.network.application_request_body_bytes = NetworkObservation::Unknown { reason: "".into() };
        assert_eq!(validate_receipt(&r), Err(ReceiptError::InvalidNetworkTelemetry));
        r.network = NetworkTelemetry::default();
        r.network.application_response_body_bytes = NetworkObservation::Inferred { value: 0 };
        assert_eq!(validate_receipt(&r), Err(ReceiptError::InvalidNetworkTelemetry));
    }

    #[test]
    fn hashes_exact_bytes_deterministically() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_ne!(sha256_hex(b"abc"), sha256_hex(b"abd"));
    }

    #[test]
    fn accepts_well_formed_local_receipt() {
        assert_eq!(validate_receipt(&receipt()), Ok(()));
    }

    #[test]
    fn rejects_external_receipt_without_exact_approval_reference() {
        let mut r = receipt();
        r.location = ExecutionLocation::ExternalProvider;
        assert_eq!(validate_receipt(&r), Err(ReceiptError::MissingExternalApproval));
        r.user_approval_id = Some("approval-7".into());
        assert_eq!(validate_receipt(&r), Ok(()));
    }

    #[test]
    fn rejects_invalid_digest_and_time_order() {
        let mut r = receipt();
        r.output_sha256 = "not-a-digest".into();
        assert_eq!(validate_receipt(&r), Err(ReceiptError::InvalidDigest));
        r = receipt();
        r.completed_at_unix_ms = 99;
        assert_eq!(validate_receipt(&r), Err(ReceiptError::InvalidTimeRange));
    }

    #[test]
    fn accepts_common_local_model_tags_and_registry_names() {
        let mut r = receipt();
        r.model_id = Some("hf.co/user/model_name:Q4_K_M".into());
        r.model_version = Some("unknown".into());
        assert_eq!(validate_receipt(&r), Ok(()));
    }

    #[test]
    fn model_identifier_preflight_rejects_ambiguous_names() {
        assert!(validate_model_identifier("qwen2.5:3b"));
        assert!(validate_model_identifier("hf.co/user/model_name:Q4_K_M"));
        assert!(!validate_model_identifier("../escape"));
        assert!(!validate_model_identifier("model name"));
        assert!(!validate_model_identifier("model\\nname"));
    }

    #[test]
    fn rejects_path_like_identifiers() {
        let mut r = receipt();
        r.task_id = "../escape".into();
        assert_eq!(validate_receipt(&r), Err(ReceiptError::InvalidIdentifier));
    }

    #[test]
    fn requires_model_and_version_to_be_paired() {
        let mut r = receipt();
        r.model_version = None;
        assert_eq!(validate_receipt(&r), Err(ReceiptError::MissingModelProvenance));
    }
}
