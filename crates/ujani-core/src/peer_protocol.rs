//! Signed, consent-scoped Worker/Partner task envelope contract.
//! This module defines verification primitives only; it does not enroll peers,
//! open network listeners, or dispatch remote tasks.
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_TASK_TTL_SECONDS: u64 = 300;
pub const MAX_REPLAY_ENTRIES: usize = 4096;
const ALLOWED_SCOPES: &[&str] = &["prompt.read", "document.read", "result.write"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskEnvelope {
    pub protocol_version: u16,
    pub task_id: String,
    pub issued_at: u64,
    pub expires_at: u64,
    /// 16 random bytes encoded as 32 lowercase hexadecimal characters.
    pub nonce: String,
    /// Registered operation identifier, never a shell command.
    pub operation: String,
    /// SHA-256 digest of the exact input payload; the payload travels separately.
    pub input_sha256: String,
    /// Narrow data-access permissions for this task only.
    pub requested_scopes: Vec<String>,
    /// Explicit consent for this exact task and scope set.
    pub user_consent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedTaskEnvelope {
    pub envelope: TaskEnvelope,
    /// Ed25519 signature over the canonical serde JSON serialization of envelope.
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    InvalidVersion,
    InvalidTaskId,
    InvalidTimeWindow,
    Expired,
    ExcessiveLifetime,
    InvalidNonce,
    InvalidOperation,
    InvalidDigest,
    MissingConsent,
    InvalidScope,
    InvalidSignature,
    ReplayCacheFull,
    ReplayDetected,
}

#[derive(Default)]
pub struct ReplayGuard {
    seen: HashMap<String, u64>,
}

impl ReplayGuard {
    pub fn len(&self) -> usize { self.seen.len() }
    pub fn is_empty(&self) -> bool { self.seen.is_empty() }
}

impl SignedTaskEnvelope {
    pub fn sign(envelope: TaskEnvelope, key: &SigningKey) -> Result<Self, EnvelopeError> {
        validate_fields(&envelope, envelope.issued_at)?;
        let bytes = serde_json::to_vec(&envelope).map_err(|_| EnvelopeError::InvalidSignature)?;
        let signature = key.sign(&bytes).to_bytes().to_vec();
        Ok(Self { envelope, signature })
    }

    /// Verify that the separately transported task payload matches the signed SHA-256 digest.
    pub fn verify_payload(&self, payload: &[u8]) -> Result<(), EnvelopeError> {
        let digest = Sha256::digest(payload);
        let actual = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        if actual == self.envelope.input_sha256.to_ascii_lowercase() {
            Ok(())
        } else {
            Err(EnvelopeError::InvalidDigest)
        }
    }

    /// Verify against an already trusted peer key, current wall clock and replay guard.
    /// Peer-key enrollment/trust distribution must happen out of band.
    pub fn verify(
        &self,
        trusted_key: &VerifyingKey,
        now: u64,
        replay: &mut ReplayGuard,
    ) -> Result<(), EnvelopeError> {
        validate_fields(&self.envelope, now)?;
        let bytes = serde_json::to_vec(&self.envelope).map_err(|_| EnvelopeError::InvalidSignature)?;
        let signature = Signature::from_slice(&self.signature).map_err(|_| EnvelopeError::InvalidSignature)?;
        trusted_key.verify(&bytes, &signature).map_err(|_| EnvelopeError::InvalidSignature)?;
        // Expired nonce entries can be removed because envelope expiry was checked above.
        replay.seen.retain(|_, expires_at| *expires_at > now);
        if replay.seen.contains_key(&self.envelope.nonce) {
            return Err(EnvelopeError::ReplayDetected);
        }
        if replay.seen.len() >= MAX_REPLAY_ENTRIES {
            return Err(EnvelopeError::ReplayCacheFull);
        }
        replay.seen.insert(self.envelope.nonce.clone(), self.envelope.expires_at);
        Ok(())
    }
}

fn validate_fields(e: &TaskEnvelope, now: u64) -> Result<(), EnvelopeError> {
    if e.protocol_version != PROTOCOL_VERSION { return Err(EnvelopeError::InvalidVersion); }
    if e.task_id.is_empty() || e.task_id.len() > 96
        || !e.task_id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
        return Err(EnvelopeError::InvalidTaskId);
    }
    if e.issued_at > now.saturating_add(30) || e.expires_at <= e.issued_at {
        return Err(EnvelopeError::InvalidTimeWindow);
    }
    if e.expires_at <= now { return Err(EnvelopeError::Expired); }
    if e.expires_at.saturating_sub(e.issued_at) > MAX_TASK_TTL_SECONDS {
        return Err(EnvelopeError::ExcessiveLifetime);
    }
    if e.nonce.len() != 32 || !e.nonce.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(EnvelopeError::InvalidNonce);
    }
    if e.operation.is_empty() || e.operation.len() > 128
        || !e.operation.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c)) {
        return Err(EnvelopeError::InvalidOperation);
    }
    if e.input_sha256.len() != 64 || !e.input_sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(EnvelopeError::InvalidDigest);
    }
    if !e.user_consent { return Err(EnvelopeError::MissingConsent); }
    if e.requested_scopes.is_empty() || e.requested_scopes.len() > ALLOWED_SCOPES.len()
        || e.requested_scopes.iter().any(|scope| !ALLOWED_SCOPES.contains(&scope.as_str())) {
        return Err(EnvelopeError::InvalidScope);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> TaskEnvelope {
        TaskEnvelope {
            protocol_version: PROTOCOL_VERSION,
            task_id: "task-42".into(),
            issued_at: 1_000,
            expires_at: 1_120,
            nonce: "0123456789abcdef0123456789abcdef".into(),
            operation: "document.summarize".into(),
            input_sha256: "a".repeat(64),
            requested_scopes: vec!["document.read".into(), "result.write".into()],
            user_consent: true,
        }
    }

    #[test]
    fn signs_verifies_and_rejects_replay() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let signed = SignedTaskEnvelope::sign(envelope(), &key).unwrap();
        let mut guard = ReplayGuard::default();
        assert_eq!(signed.verify(&key.verifying_key(), 1_001, &mut guard), Ok(()));
        assert_eq!(signed.verify(&key.verifying_key(), 1_001, &mut guard), Err(EnvelopeError::ReplayDetected));
    }

    #[test]
    fn verifies_payload_against_signed_digest() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let payload = b"approved document bytes";
        let mut e = envelope();
        e.input_sha256 = Sha256::digest(payload).iter().map(|byte| format!("{byte:02x}")).collect();
        let signed = SignedTaskEnvelope::sign(e, &key).unwrap();
        assert_eq!(signed.verify_payload(payload), Ok(()));
        assert_eq!(signed.verify_payload(b"different bytes"), Err(EnvelopeError::InvalidDigest));
    }

    #[test]
    fn signature_binds_consent_scope_and_task_content() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let mut signed = SignedTaskEnvelope::sign(envelope(), &key).unwrap();
        signed.envelope.requested_scopes.push("filesystem.root".into());
        let mut guard = ReplayGuard::default();
        assert_eq!(signed.verify(&key.verifying_key(), 1_001, &mut guard), Err(EnvelopeError::InvalidScope));
        signed.envelope.requested_scopes = vec!["document.read".into()];
        assert_eq!(signed.verify(&key.verifying_key(), 1_001, &mut guard), Err(EnvelopeError::InvalidSignature));
    }

    #[test]
    fn requires_explicit_consent_and_short_expiry() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let mut e = envelope();
        e.user_consent = false;
        assert_eq!(SignedTaskEnvelope::sign(e, &key), Err(EnvelopeError::MissingConsent));
        let mut e = envelope();
        e.expires_at = e.issued_at + MAX_TASK_TTL_SECONDS + 1;
        assert_eq!(SignedTaskEnvelope::sign(e, &key), Err(EnvelopeError::ExcessiveLifetime));
    }

    #[test]
    fn expired_replay_nonce_is_pruned_for_new_task() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let first = SignedTaskEnvelope::sign(envelope(), &key).unwrap();
        let mut guard = ReplayGuard::default();
        assert_eq!(first.verify(&key.verifying_key(), 1_001, &mut guard), Ok(()));
        let mut next = envelope();
        next.issued_at = 1_121;
        next.expires_at = 1_241;
        let second = SignedTaskEnvelope::sign(next, &key).unwrap();
        assert_eq!(second.verify(&key.verifying_key(), 1_122, &mut guard), Ok(()));
        assert_eq!(guard.len(), 1);
    }

    #[test]
    fn rejects_expired_and_wrong_identity() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let signed = SignedTaskEnvelope::sign(envelope(), &key).unwrap();
        let mut guard = ReplayGuard::default();
        assert_eq!(signed.verify(&key.verifying_key(), 1_120, &mut guard), Err(EnvelopeError::Expired));
        let other = SigningKey::from_bytes(&[8u8; 32]);
        assert_eq!(signed.verify(&other.verifying_key(), 1_001, &mut guard), Err(EnvelopeError::InvalidSignature));
    }
}
