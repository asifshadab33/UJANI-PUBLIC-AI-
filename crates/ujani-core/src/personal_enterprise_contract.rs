//! Native decoder and verifier for the language-neutral Public/Enterprise task contract v1.
//!
//! This module is deliberately a protocol boundary only. It does not open sockets,
//! enroll devices, authorize a workload, reserve resources, dispatch tasks, or persist
//! replay state. Callers must authenticate the key-to-principal/device binding and
//! atomically persist status sequences/nonces before any future execution adapter acts.

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const TASK_CONTRACT_SCHEMA_VERSION: u16 = 1;
pub const MAX_TASK_TTL_MS: u64 = 300_000;
pub const MAX_FUTURE_ISSUE_SKEW_MS: u64 = 60_000;
pub const MAX_PAYLOAD_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;
const SAFE_INTEGER_MAX: u64 = 9_007_199_254_740_991;
const CONSENT_BINDING_DOMAIN: &[u8] = b"UJANI-CONSENT-BINDING-V1\0";
const SIGNATURE_DOMAIN: &[u8] = b"UJANI-PERSONAL-ENTERPRISE-TASK-V1\0";
const BASE64URL: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractError {
    InvalidJson,
    UnsupportedVersion,
    InvalidIdentifier,
    InvalidTimestamp,
    Expired,
    ExcessiveLifetime,
    InvalidNonce,
    InvalidDigest,
    PayloadMismatch,
    InvalidConsent,
    ConsentBindingMismatch,
    InvalidResourceLimit,
    InvalidCapability,
    InvalidNetworkPolicy,
    InvalidSignature,
    UnknownSigningKey,
    InvalidState,
    StaleSequence,
    InvalidReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContractSignature {
    pub algorithm: String,
    pub key_id: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SenderIdentity {
    pub principal_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskAudience {
    pub organisation_id: String,
    pub service_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PayloadDescriptor {
    pub sha256: String,
    pub byte_length: u64,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    pub cpu_millis: u64,
    /// Memory is always mebibytes (MiB), never decimal MB.
    pub memory_mb: u64,
    pub max_runtime_ms: u64,
    pub max_output_bytes: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyClassification {
    None,
    Personal,
    SensitivePersonal,
    Regulated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NetworkAccess {
    None,
    EnterpriseInternal,
    Allowlisted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsentDecision {
    Granted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAuthority {
    OrganisationAdmin,
    WorkerDeviceOwner,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UserConsent {
    pub decision: ConsentDecision,
    pub consent_id: String,
    pub subject_id: String,
    pub granted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub binding_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConsentRequirements {
    pub user: UserConsent,
    pub required_approvals: Vec<ApprovalAuthority>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequiredTrustState {
    EnrolledActive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum NullableIdentifier {
    Value(String),
    Null(()),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceTrustRequirement {
    /// Required on the wire, but nullable when no target device has been selected yet.
    pub target_device_id: NullableIdentifier,
    pub required_trust_state: RequiredTrustState,
    pub require_owner_opt_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskEnvelopeV1 {
    pub message_type: String,
    pub schema_version: u16,
    pub task_id: String,
    pub sender: SenderIdentity,
    pub audience: TaskAudience,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub nonce: String,
    pub operation_id: String,
    pub operation_version: u16,
    pub payload: PayloadDescriptor,
    pub data_classification: DataClassification,
    pub privacy_classification: PrivacyClassification,
    pub required_scopes: Vec<String>,
    pub required_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_context_length_tokens: Option<u64>,
    pub resource_limits: ResourceLimits,
    pub network_access: NetworkAccess,
    pub consent: ConsentRequirements,
    pub device_trust: DeviceTrustRequirement,
    pub signature: ContractSignature,
    pub network_destinations: Vec<String>,
}

impl TaskEnvelopeV1 {
    /// Parse one strict v1 envelope. Duplicate and unknown struct fields are rejected by Serde.
    pub fn parse(input: &[u8]) -> Result<Self, ContractError> {
        let envelope: Self = serde_json::from_slice(input).map_err(|_| ContractError::InvalidJson)?;
        Ok(envelope)
    }

    /// Validate structural and cross-field rules that JSON Schema cannot express.
    /// Cryptographic verification and payload-byte verification remain separate explicit calls.
    pub fn validate(&self, now_unix_ms: u64) -> Result<(), ContractError> {
        if self.message_type != "task_envelope" || self.schema_version != TASK_CONTRACT_SCHEMA_VERSION {
            return Err(ContractError::UnsupportedVersion);
        }
        if !valid_identifier(&self.task_id, 128)
            || !valid_identifier(&self.sender.principal_id, 128)
            || !valid_identifier(&self.sender.device_id, 128)
            || !valid_identifier(&self.audience.organisation_id, 128)
            || !valid_identifier(&self.audience.service_id, 128)
            || !valid_identifier(&self.operation_id, 128)
        {
            return Err(ContractError::InvalidIdentifier);
        }
        if self.operation_version == 0
            || self.issued_at_unix_ms == 0
            || self.expires_at_unix_ms > SAFE_INTEGER_MAX
            || self.issued_at_unix_ms > SAFE_INTEGER_MAX
            || self.issued_at_unix_ms > now_unix_ms.saturating_add(MAX_FUTURE_ISSUE_SKEW_MS)
            || self.expires_at_unix_ms <= self.issued_at_unix_ms
        {
            return Err(ContractError::InvalidTimestamp);
        }
        if self.expires_at_unix_ms <= now_unix_ms {
            return Err(ContractError::Expired);
        }
        if self.expires_at_unix_ms.saturating_sub(self.issued_at_unix_ms) > MAX_TASK_TTL_MS {
            return Err(ContractError::ExcessiveLifetime);
        }
        if decode_base64url(&self.nonce, 32).is_err() {
            return Err(ContractError::InvalidNonce);
        }
        if !valid_sha256(&self.payload.sha256)
            || self.payload.byte_length > MAX_PAYLOAD_BYTES
            || !valid_media_type(&self.payload.media_type)
        {
            return Err(ContractError::InvalidDigest);
        }
        if self.required_scopes.is_empty()
            || self.required_scopes.len() > 16
            || !unique(&self.required_scopes)
            || self.required_scopes.iter().any(|scope| !valid_scope(scope))
        {
            return Err(ContractError::InvalidCapability);
        }
        const CAPABILITIES: &[&str] = &[
            "chat", "reasoning", "coding", "vision", "embeddings", "tool_use",
            "streaming", "audio_input", "audio_output",
        ];
        if self.required_capabilities.is_empty()
            || self.required_capabilities.len() > 16
            || !unique(&self.required_capabilities)
            || self.required_capabilities.iter().any(|capability| !CAPABILITIES.contains(&capability.as_str()))
            || self.min_context_length_tokens.is_some_and(|value| value == 0 || value > 1_000_000)
        {
            return Err(ContractError::InvalidCapability);
        }
        if self.resource_limits.cpu_millis == 0
            || self.resource_limits.cpu_millis > 1_000_000
            || self.resource_limits.memory_mb == 0
            || self.resource_limits.memory_mb > 4 * 1024 * 1024
            || self.resource_limits.max_runtime_ms == 0
            || self.resource_limits.max_runtime_ms > 604_800_000
            || self.resource_limits.max_output_bytes == 0
            || self.resource_limits.max_output_bytes > MAX_OUTPUT_BYTES
        {
            return Err(ContractError::InvalidResourceLimit);
        }
        if self.network_destinations.len() > 8
            || !unique(&self.network_destinations)
            || self.network_destinations.iter().any(|host| !valid_dns_hostname(host))
            || (self.network_access == NetworkAccess::Allowlisted && self.network_destinations.is_empty())
            || (self.network_access != NetworkAccess::Allowlisted && !self.network_destinations.is_empty())
        {
            return Err(ContractError::InvalidNetworkPolicy);
        }
        if self.consent.required_approvals.len() != 2
            || !unique(&self.consent.required_approvals)
            || !self.consent.required_approvals.contains(&ApprovalAuthority::OrganisationAdmin)
            || !self.consent.required_approvals.contains(&ApprovalAuthority::WorkerDeviceOwner)
            || !valid_identifier(&self.consent.user.consent_id, 128)
            || !valid_identifier(&self.consent.user.subject_id, 128)
            || self.consent.user.subject_id != self.sender.principal_id
            || self.consent.user.granted_at_unix_ms > self.issued_at_unix_ms
            || self.consent.user.expires_at_unix_ms < self.expires_at_unix_ms
            || self.consent.user.expires_at_unix_ms <= self.consent.user.granted_at_unix_ms
        {
            return Err(ContractError::InvalidConsent);
        }
        if !self.device_trust.require_owner_opt_in
            || matches!(&self.device_trust.target_device_id, NullableIdentifier::Value(id) if !valid_identifier(id, 128))
        {
            return Err(ContractError::InvalidConsent);
        }
        if self.signature.algorithm != "ed25519"
            || !valid_identifier(&self.signature.key_id, 128)
            || decode_base64url(&self.signature.value, 64).is_err()
        {
            return Err(ContractError::InvalidSignature);
        }
        if self.consent.user.binding_sha256 != self.consent_binding_sha256()? {
            return Err(ContractError::ConsentBindingMismatch);
        }
        Ok(())
    }

    /// Hash the exact received payload bytes; never hash a re-serialized payload.
    pub fn verify_payload(&self, payload: &[u8]) -> Result<(), ContractError> {
        if payload.len() as u64 != self.payload.byte_length || sha256_hex(payload) != self.payload.sha256 {
            return Err(ContractError::PayloadMismatch);
        }
        Ok(())
    }

    /// Return the RFC 8785 JCS preimage used by the consent-binding digest.
    pub fn consent_binding_preimage(&self) -> Result<Vec<u8>, ContractError> {
        let mut value = serde_json::to_value(self).map_err(|_| ContractError::InvalidJson)?;
        let object = value.as_object_mut().ok_or(ContractError::InvalidJson)?;
        let _ = object.remove("signature");
        let consent = object.get_mut("consent").and_then(Value::as_object_mut).ok_or(ContractError::InvalidConsent)?;
        let user = consent.get_mut("user").and_then(Value::as_object_mut).ok_or(ContractError::InvalidConsent)?;
        let _ = user.remove("binding_sha256");
        let mut canonical = Vec::new();
        canonical_json(&value, &mut canonical)?;
        Ok(canonical)
    }

    pub fn consent_binding_sha256(&self) -> Result<String, ContractError> {
        let mut bytes = CONSENT_BINDING_DOMAIN.to_vec();
        bytes.extend_from_slice(&self.consent_binding_preimage()?);
        Ok(sha256_hex(&bytes))
    }

    pub fn verify_signature(
        &self,
        expected_key_id: &str,
        trusted_key: &VerifyingKey,
    ) -> Result<(), ContractError> {
        verify_signed_value(
            self,
            "task_envelope",
            &self.signature,
            expected_key_id,
            trusted_key,
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserConsentState {
    Granted,
    Revoked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Approved,
    Denied,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConsentState {
    pub user: UserConsentState,
    pub organisation_admin: ApprovalState,
    pub worker_device_owner: ApprovalState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Received,
    Validating,
    AwaitingApproval,
    Queued,
    Reserved,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    InvalidSignature,
    UnknownSender,
    WrongAudience,
    Expired,
    Replay,
    PayloadMismatch,
    ConsentDenied,
    PolicyDenied,
    DeviceUntrusted,
    ResourceUnavailable,
    UnsupportedCapability,
    Cancelled,
    Timeout,
    InternalError,
    ConsentRevoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskStatusV1 {
    pub message_type: String,
    pub schema_version: u16,
    pub task_id: String,
    pub sequence: u64,
    pub state: TaskState,
    pub updated_at_unix_ms: u64,
    pub policy_version: String,
    pub consent_state: ConsentState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<ReasonCode>,
    pub signature: ContractSignature,
}

impl TaskStatusV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.message_type != "task_status" || self.schema_version != TASK_CONTRACT_SCHEMA_VERSION {
            return Err(ContractError::UnsupportedVersion);
        }
        if !valid_identifier(&self.task_id, 128) || !valid_identifier(&self.policy_version, 128)
            || self.sequence == 0 || self.sequence > SAFE_INTEGER_MAX
            || self.updated_at_unix_ms == 0 || self.updated_at_unix_ms > SAFE_INTEGER_MAX
        {
            return Err(ContractError::InvalidIdentifier);
        }
        if matches!(self.state, TaskState::Queued | TaskState::Reserved | TaskState::Running | TaskState::Completed)
            && (self.consent_state.user != UserConsentState::Granted
                || self.consent_state.organisation_admin != ApprovalState::Approved
                || self.consent_state.worker_device_owner != ApprovalState::Approved)
        {
            return Err(ContractError::InvalidConsent);
        }
        if matches!(self.state, TaskState::Failed | TaskState::Cancelled | TaskState::TimedOut | TaskState::Rejected | TaskState::Expired)
            && self.reason_code.is_none()
        {
            return Err(ContractError::InvalidState);
        }
        validate_signature_shape(&self.signature)?;
        Ok(())
    }

    /// The caller must persist the accepted sequence atomically with its task state.
    pub fn validate_after(&self, last_accepted_sequence: Option<u64>) -> Result<(), ContractError> {
        self.validate()?;
        if last_accepted_sequence.is_some_and(|last| self.sequence <= last) {
            return Err(ContractError::StaleSequence);
        }
        Ok(())
    }

    pub fn verify_signature(&self, expected_key_id: &str, trusted_key: &VerifyingKey) -> Result<(), ContractError> {
        verify_signed_value(self, "task_status", &self.signature, expected_key_id, trusted_key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerIdentity {
    pub organisation_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConsentReferences {
    pub user_consent_id: String,
    pub organisation_approval_id: String,
    pub worker_owner_consent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResourceUsage {
    pub cpu_time_ms: Option<u64>,
    pub peak_memory_mb: Option<u64>,
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionLocation {
    EnrolledDevice,
    EnterpriseService,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOutcome {
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum NullableSha256 {
    Digest(String),
    Null(()),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReceiptV1 {
    pub message_type: String,
    pub schema_version: u16,
    pub task_id: String,
    pub sequence: u64,
    pub input_sha256: String,
    /// Required on the wire: null exactly when output_byte_length is zero.
    pub output_sha256: NullableSha256,
    pub output_byte_length: u64,
    pub worker: WorkerIdentity,
    pub execution_location: ExecutionLocation,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub outcome: ExecutionOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_usage: Option<ResourceUsage>,
    pub consent_refs: ConsentReferences,
    pub policy_version: String,
    pub signature: ContractSignature,
}

impl ExecutionReceiptV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.message_type != "execution_receipt" || self.schema_version != TASK_CONTRACT_SCHEMA_VERSION {
            return Err(ContractError::UnsupportedVersion);
        }
        if !valid_identifier(&self.task_id, 128)
            || !valid_identifier(&self.worker.organisation_id, 128)
            || !valid_identifier(&self.worker.device_id, 128)
            || !valid_identifier(&self.policy_version, 128)
            || !valid_identifier(&self.consent_refs.user_consent_id, 128)
            || !valid_identifier(&self.consent_refs.organisation_approval_id, 128)
            || !valid_identifier(&self.consent_refs.worker_owner_consent_id, 128)
            || self.sequence == 0 || self.sequence > SAFE_INTEGER_MAX
        {
            return Err(ContractError::InvalidIdentifier);
        }
        if !valid_sha256(&self.input_sha256)
            || self.output_byte_length > MAX_OUTPUT_BYTES
            || match (&self.output_sha256, self.output_byte_length) {
                (NullableSha256::Null(()), 0) => false,
                (NullableSha256::Digest(digest), length) if length > 0 => !valid_sha256(digest),
                _ => true,
            }
        {
            return Err(ContractError::InvalidReceipt);
        }
        if self.started_at_unix_ms == 0
            || self.completed_at_unix_ms < self.started_at_unix_ms
            || self.completed_at_unix_ms > SAFE_INTEGER_MAX
        {
            return Err(ContractError::InvalidTimestamp);
        }
        if self.resource_usage.as_ref().is_some_and(|usage| {
            [usage.cpu_time_ms, usage.peak_memory_mb, usage.elapsed_ms]
                .into_iter()
                .flatten()
                .any(|value| value > SAFE_INTEGER_MAX)
        }) {
            return Err(ContractError::InvalidResourceLimit);
        }
        validate_signature_shape(&self.signature)?;
        Ok(())
    }

    pub fn verify_signature(&self, expected_key_id: &str, trusted_key: &VerifyingKey) -> Result<(), ContractError> {
        verify_signed_value(self, "execution_receipt", &self.signature, expected_key_id, trusted_key)
    }
}

fn verify_signed_value<T: Serialize>(
    message: &T,
    message_type: &str,
    signature: &ContractSignature,
    expected_key_id: &str,
    trusted_key: &VerifyingKey,
) -> Result<(), ContractError> {
    validate_signature_shape(signature)?;
    if signature.key_id != expected_key_id {
        return Err(ContractError::UnknownSigningKey);
    }
    let mut value = serde_json::to_value(message).map_err(|_| ContractError::InvalidJson)?;
    value.as_object_mut().ok_or(ContractError::InvalidJson)?.remove("signature");
    let mut canonical = Vec::new();
    canonical_json(&value, &mut canonical)?;
    let mut preimage = SIGNATURE_DOMAIN.to_vec();
    preimage.extend_from_slice(message_type.as_bytes());
    preimage.push(0);
    preimage.extend_from_slice(b"ed25519\0");
    preimage.extend_from_slice(signature.key_id.as_bytes());
    preimage.push(0);
    preimage.extend_from_slice(&canonical);
    let bytes = decode_base64url(&signature.value, 64).map_err(|_| ContractError::InvalidSignature)?;
    let parsed = Signature::from_slice(&bytes).map_err(|_| ContractError::InvalidSignature)?;
    trusted_key.verify_strict(&preimage, &parsed).map_err(|_| ContractError::InvalidSignature)
}

fn validate_signature_shape(signature: &ContractSignature) -> Result<(), ContractError> {
    if signature.algorithm != "ed25519"
        || !valid_identifier(&signature.key_id, 128)
        || decode_base64url(&signature.value, 64).is_err()
    {
        return Err(ContractError::InvalidSignature);
    }
    Ok(())
}

fn canonical_json(value: &Value, output: &mut Vec<u8>) -> Result<(), ContractError> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(value) => output.extend_from_slice(if *value { b"true" } else { b"false" }),
        Value::Number(number) => {
            let integer = number.as_u64().ok_or(ContractError::InvalidJson)?;
            if integer > SAFE_INTEGER_MAX {
                return Err(ContractError::InvalidJson);
            }
            output.extend_from_slice(integer.to_string().as_bytes());
        }
        Value::String(value) => {
            if !value.is_ascii() {
                return Err(ContractError::InvalidJson);
            }
            let encoded = serde_json::to_string(value).map_err(|_| ContractError::InvalidJson)?;
            output.extend_from_slice(encoded.as_bytes());
        }
        Value::Array(items) => {
            output.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 { output.push(b','); }
                canonical_json(item, output)?;
            }
            output.push(b']');
        }
        Value::Object(object) => {
            output.push(b'{');
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
            for (index, key) in keys.iter().enumerate() {
                if !key.is_ascii() { return Err(ContractError::InvalidJson); }
                if index > 0 { output.push(b','); }
                let encoded_key = serde_json::to_string(key).map_err(|_| ContractError::InvalidJson)?;
                output.extend_from_slice(encoded_key.as_bytes());
                output.push(b':');
                canonical_json(&object[*key], output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

fn valid_identifier(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_scope(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_media_type(value: &str) -> bool {
    let Some((kind, subtype)) = value.split_once('/') else { return false; };
    !kind.is_empty()
        && !subtype.is_empty()
        && !subtype.contains('/')
        && value.len() <= 96
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || b"!#$&^_.+-/".contains(&byte)
        })
}

fn valid_dns_hostname(value: &str) -> bool {
    if value.is_empty() || value.len() > 253 || !value.is_ascii() || value.to_ascii_lowercase() != value {
        return false;
    }
    if value.parse::<std::net::IpAddr>().is_ok() { return false; }
    let labels: Vec<&str> = value.split('.').collect();
    if labels.iter().any(|label| {
        label.is_empty()
            || label.len() > 63
            || !label.as_bytes()[0].is_ascii_alphanumeric()
            || !label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
            || !label.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    }) {
        return false;
    }
    let numeric_address = labels.iter().all(|label| {
        label.bytes().all(|byte| byte.is_ascii_digit())
            || label.strip_prefix("0x").is_some_and(|hex| !hex.is_empty() && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
    });
    !numeric_address
}

fn unique<T: Ord>(values: &[T]) -> bool {
    let mut set = BTreeSet::new();
    values.iter().all(|value| set.insert(value))
}

fn decode_base64url(value: &str, expected_len: usize) -> Result<Vec<u8>, ContractError> {
    if value.is_empty() || value.contains('=') || !value.is_ascii() {
        return Err(ContractError::InvalidSignature);
    }
    let mut output = Vec::with_capacity(expected_len);
    let mut accumulator: u32 = 0;
    let mut bits: u32 = 0;
    for byte in value.bytes() {
        let digit = BASE64URL.iter().position(|candidate| *candidate == byte)
            .ok_or(ContractError::InvalidSignature)? as u32;
        accumulator = (accumulator << 6) | digit;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((accumulator >> bits) as u8);
            accumulator &= (1u32 << bits) - 1;
        }
    }
    if output.len() != expected_len || accumulator != 0 {
        return Err(ContractError::InvalidSignature);
    }
    Ok(output)
}

fn encode_base64url(input: &[u8]) -> String {
    let mut output = String::new();
    let mut index = 0;
    while index + 3 <= input.len() {
        let block = ((input[index] as u32) << 16)
            | ((input[index + 1] as u32) << 8)
            | input[index + 2] as u32;
        for shift in [18, 12, 6, 0] {
            output.push(BASE64URL[((block >> shift) & 0x3f) as usize] as char);
        }
        index += 3;
    }
    let remaining = input.len() - index;
    if remaining == 1 {
        let block = (input[index] as u32) << 16;
        output.push(BASE64URL[((block >> 18) & 0x3f) as usize] as char);
        output.push(BASE64URL[((block >> 12) & 0x3f) as usize] as char);
    } else if remaining == 2 {
        let block = ((input[index] as u32) << 16) | ((input[index + 1] as u32) << 8);
        output.push(BASE64URL[((block >> 18) & 0x3f) as usize] as char);
        output.push(BASE64URL[((block >> 12) & 0x3f) as usize] as char);
        output.push(BASE64URL[((block >> 6) & 0x3f) as usize] as char);
    }
    output
}

fn sha256_hex(input: &[u8]) -> String {
    Sha256::digest(input).iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[derive(Deserialize)]
    struct ConsentFixture {
        envelope: TaskEnvelopeV1,
        canonical_preimage_utf8: String,
        expected_binding_sha256: String,
    }

    const CONSENT_FIXTURE: &str =
        include_str!("../../../schemas/test-vectors/consent-binding-v1.json");

    #[test]
    fn shared_fixture_matches_the_native_consent_binding_profile() {
        let fixture: ConsentFixture = serde_json::from_str(CONSENT_FIXTURE).unwrap();
        fixture.envelope.validate(1_000).unwrap();
        assert_eq!(fixture.envelope.consent_binding_sha256().unwrap(), fixture.expected_binding_sha256);
        assert_eq!(
            String::from_utf8(fixture.envelope.consent_binding_preimage().unwrap()).unwrap(),
            fixture.canonical_preimage_utf8
        );
    }

    #[test]
    fn consent_binding_covers_resource_limits_but_excludes_signature_bytes() {
        let fixture: ConsentFixture = serde_json::from_str(CONSENT_FIXTURE).unwrap();
        let expected = fixture.expected_binding_sha256;
        let mut changed = fixture.envelope.clone();
        changed.resource_limits.memory_mb += 1;
        assert_ne!(changed.consent_binding_sha256().unwrap(), expected);
        let mut signature_only = fixture.envelope;
        signature_only.signature.value = encode_base64url(&[7u8; 64]);
        assert_eq!(signature_only.consent_binding_sha256().unwrap(), expected);
    }

    #[test]
    fn native_signature_verification_uses_domain_type_and_key_id() {
        let fixture: ConsentFixture = serde_json::from_str(CONSENT_FIXTURE).unwrap();
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let mut envelope = fixture.envelope;
        let mut value = serde_json::to_value(&envelope).unwrap();
        value.as_object_mut().unwrap().remove("signature");
        let mut canonical = Vec::new();
        canonical_json(&value, &mut canonical).unwrap();
        let mut preimage = SIGNATURE_DOMAIN.to_vec();
        preimage.extend_from_slice(b"task_envelope\0ed25519\0sender-key-1\0");
        preimage.extend_from_slice(&canonical);
        envelope.signature.value = encode_base64url(&key.sign(&preimage).to_bytes());
        envelope.verify_signature("sender-key-1", &key.verifying_key()).unwrap();
        assert_eq!(
            envelope.verify_signature("different-key", &key.verifying_key()),
            Err(ContractError::UnknownSigningKey)
        );
        envelope.task_id.push('x');
        assert_eq!(
            envelope.verify_signature("sender-key-1", &key.verifying_key()),
            Err(ContractError::InvalidSignature)
        );
    }

    #[test]
    fn payload_verification_uses_exact_bytes_and_length() {
        let fixture: ConsentFixture = serde_json::from_str(CONSENT_FIXTURE).unwrap();
        assert_eq!(fixture.envelope.verify_payload(b""), Err(ContractError::PayloadMismatch));
        let mut envelope = fixture.envelope;
        let payload = b"approved bytes";
        envelope.payload.sha256 = sha256_hex(payload);
        envelope.payload.byte_length = payload.len() as u64;
        envelope.verify_payload(payload).unwrap();
        assert_eq!(envelope.verify_payload(b"changed bytes"), Err(ContractError::PayloadMismatch));
    }

    #[test]
    fn strict_decoder_rejects_unknown_and_duplicate_fields() {
        let fixture: ConsentFixture = serde_json::from_str(CONSENT_FIXTURE).unwrap();
        let mut value = serde_json::to_value(fixture.envelope).unwrap();
        value.as_object_mut().unwrap().insert("unrecognised".into(), Value::Bool(true));
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(TaskEnvelopeV1::parse(&bytes), Err(ContractError::InvalidJson));
        assert!(TaskEnvelopeV1::parse(br#"{"message_type":"task_envelope","message_type":"task_envelope"}"#).is_err());
    }

    #[test]
    fn status_requires_all_approvals_before_active_states_and_monotonic_sequence() {
        let status: TaskStatusV1 = serde_json::from_value(serde_json::json!({
            "message_type": "task_status",
            "schema_version": 1,
            "task_id": "task-1",
            "sequence": 2,
            "state": "running",
            "updated_at_unix_ms": 1200,
            "policy_version": "policy-1",
            "consent_state": {
                "user": "granted",
                "organisation_admin": "approved",
                "worker_device_owner": "approved"
            },
            "signature": {
                "algorithm": "ed25519",
                "key_id": "service-key-1",
                "value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
            }
        })).unwrap();
        status.validate_after(Some(1)).unwrap();
        assert_eq!(status.validate_after(Some(2)), Err(ContractError::StaleSequence));
        let mut revoked = status;
        revoked.consent_state.worker_device_owner = ApprovalState::Revoked;
        assert_eq!(revoked.validate(), Err(ContractError::InvalidConsent));
    }

    #[test]
    fn receipt_requires_output_digest_exactly_when_output_exists() {
        let mut receipt: ExecutionReceiptV1 = serde_json::from_value(serde_json::json!({
            "message_type": "execution_receipt",
            "schema_version": 1,
            "task_id": "task-1",
            "sequence": 1,
            "input_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
            "output_sha256": null,
            "output_byte_length": 0,
            "worker": { "organisation_id": "org-1", "device_id": "worker-1" },
            "execution_location": "enrolled_device",
            "started_at_unix_ms": 1200,
            "completed_at_unix_ms": 1300,
            "outcome": "completed",
            "consent_refs": {
                "user_consent_id": "consent-1",
                "organisation_approval_id": "approval-1",
                "worker_owner_consent_id": "owner-consent-1"
            },
            "policy_version": "policy-1",
            "signature": {
                "algorithm": "ed25519",
                "key_id": "worker-key-1",
                "value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
            }
        })).unwrap();
        receipt.validate().unwrap();
        receipt.output_byte_length = 1;
        assert_eq!(receipt.validate(), Err(ContractError::InvalidReceipt));
        receipt.output_sha256 = NullableSha256::Digest("0".repeat(64));
        receipt.validate().unwrap();
    }

    #[test]
    fn base64url_decoder_rejects_noncanonical_padding_bits() {
        assert_eq!(decode_base64url(&"A".repeat(86), 64).unwrap().len(), 64);
        let mut invalid = "A".repeat(85);
        invalid.push('B');
        assert_eq!(decode_base64url(&invalid, 64), Err(ContractError::InvalidSignature));
    }
}
