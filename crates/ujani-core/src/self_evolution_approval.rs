//! Host-verifiable approval receipts for bounded self-evolution proposals.
//!
//! This module verifies a separate, explicit human approval over one exact
//! proposal, repository revision and write scope. It does not apply patches,
//! write repository files, run code, install updates or authorize protected core.
//! The caller must persist consumed approval nonces before any future apply step.

use crate::self_evolution::{ProposalTarget, SelfEvolutionProposalV1};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SELF_EVOLUTION_APPROVAL_MAX_TTL_MS: u64 = 30 * 60 * 1000;
const APPROVAL_DOMAIN: &[u8] = b"UJANI-PUBLIC-SELF-EVOLUTION-APPROVAL-V1\0";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SelfEvolutionApprovalClaims {
    pub key_id: String,
    pub proposal_sha256: String,
    pub repository: String,
    pub base_ref: String,
    pub source_revision: String,
    pub approver_id: String,
    pub approved_scope: Vec<String>,
    pub approved_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub nonce: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedSelfEvolutionApproval {
    pub claims: SelfEvolutionApprovalClaims,
    pub signature: [u8; 64],
}

/// An approval that has passed signature, target, proposal, scope and expiry
/// validation. Fields are private so callers cannot construct one from model output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSelfEvolutionApproval {
    claims: SelfEvolutionApprovalClaims,
}

impl VerifiedSelfEvolutionApproval {
    pub fn approver_id(&self) -> &str {
        &self.claims.approver_id
    }

    pub fn proposal_sha256(&self) -> &str {
        &self.claims.proposal_sha256
    }

    pub fn approved_scope(&self) -> &[String] {
        &self.claims.approved_scope
    }

    pub fn expires_at_unix_ms(&self) -> u64 {
        self.claims.expires_at_unix_ms
    }

    /// A stable identifier the host must record in its durable replay ledger.
    pub fn nonce(&self) -> &[u8; 16] {
        &self.claims.nonce
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalVerificationError {
    UnknownKey,
    InvalidClaims,
    InvalidScope,
    ProposalDigestMismatch,
    TargetMismatch,
    Expired,
    FutureApproval,
    InvalidLifetime,
    InvalidSignature,
    SerializationFailure,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

fn canonical_scope(scope: &[String]) -> Result<Vec<String>, ApprovalVerificationError> {
    if scope.is_empty() || scope.len() > 32 {
        return Err(ApprovalVerificationError::InvalidScope);
    }
    let mut normalized = BTreeSet::new();
    for path in scope {
        if path.is_empty()
            || path.len() > 512
            || path.starts_with('/')
            || path.contains('\\')
            || path.split('/').any(|part| {
                part.is_empty()
                    || part == "."
                    || part == ".."
                    || part.eq_ignore_ascii_case(".git")
                    || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            })
        {
            return Err(ApprovalVerificationError::InvalidScope);
        }
        if !normalized.insert(path.to_ascii_lowercase()) {
            return Err(ApprovalVerificationError::InvalidScope);
        }
    }
    Ok(normalized.into_iter().collect())
}

pub fn proposal_sha256(
    proposal: &SelfEvolutionProposalV1,
) -> Result<String, ApprovalVerificationError> {
    let encoded = serde_json::to_vec(proposal)
        .map_err(|_| ApprovalVerificationError::SerializationFailure)?;
    Ok(hex(&Sha256::digest(encoded)))
}

fn approval_signing_bytes(
    claims: &SelfEvolutionApprovalClaims,
) -> Result<Vec<u8>, ApprovalVerificationError> {
    let encoded = serde_json::to_vec(claims)
        .map_err(|_| ApprovalVerificationError::SerializationFailure)?;
    let mut message = Vec::with_capacity(APPROVAL_DOMAIN.len() + encoded.len());
    message.extend_from_slice(APPROVAL_DOMAIN);
    message.extend_from_slice(&encoded);
    Ok(message)
}

fn hex(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(TABLE[(byte >> 4) as usize] as char);
        out.push(TABLE[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Verify a host-issued signature against the trusted key map and the exact
/// proposal/target/scope currently being reviewed. Key IDs and approver IDs
/// must come from trusted host configuration, never from model output.
pub fn verify_self_evolution_approval(
    signed: &SignedSelfEvolutionApproval,
    trusted_keys: &[(String, String, VerifyingKey)],
    trusted_target: &ProposalTarget,
    proposal: &SelfEvolutionProposalV1,
    expected_scope: &[String],
    now_unix_ms: u64,
) -> Result<VerifiedSelfEvolutionApproval, ApprovalVerificationError> {
    let claims = &signed.claims;
    if !valid_id(&claims.key_id)
        || !valid_id(&claims.approver_id)
        || claims.nonce.iter().all(|byte| *byte == 0)
        || claims.approved_at_unix_ms == 0
    {
        return Err(ApprovalVerificationError::InvalidClaims);
    }
    if claims.approved_at_unix_ms > now_unix_ms.saturating_add(30_000) {
        return Err(ApprovalVerificationError::FutureApproval);
    }
    if claims.expires_at_unix_ms <= claims.approved_at_unix_ms
        || claims.expires_at_unix_ms.saturating_sub(claims.approved_at_unix_ms)
            > SELF_EVOLUTION_APPROVAL_MAX_TTL_MS
    {
        return Err(ApprovalVerificationError::InvalidLifetime);
    }
    if claims.expires_at_unix_ms <= now_unix_ms {
        return Err(ApprovalVerificationError::Expired);
    }
    if claims.repository != trusted_target.repository
        || claims.base_ref != trusted_target.base_ref
        || claims.source_revision != trusted_target.source_revision
    {
        return Err(ApprovalVerificationError::TargetMismatch);
    }

    proposal
        .validate_against(trusted_target, expected_scope)
        .map_err(|_| ApprovalVerificationError::InvalidScope)?;
    let expected_scope = canonical_scope(expected_scope)?;
    let claimed_scope = canonical_scope(&claims.approved_scope)?;
    if expected_scope != claimed_scope {
        return Err(ApprovalVerificationError::InvalidScope);
    }

    let digest = proposal_sha256(proposal)?;
    if claims.proposal_sha256 != digest {
        return Err(ApprovalVerificationError::ProposalDigestMismatch);
    }

    let key = trusted_keys
        .iter()
        .find(|(key_id, approver_id, _)| {
            key_id == &claims.key_id && approver_id == &claims.approver_id
        })
        .map(|(_, _, key)| key)
        .ok_or(ApprovalVerificationError::UnknownKey)?;
    let message = approval_signing_bytes(claims)?;
    key.verify(&message, &Signature::from_bytes(&signed.signature))
        .map_err(|_| ApprovalVerificationError::InvalidSignature)?;

    Ok(VerifiedSelfEvolutionApproval {
        claims: claims.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_evolution::{
        FindingCategory, FindingSeverity, ProposedChange, ProposedChangeKind,
        SelfEvolutionFinding, SELF_EVOLUTION_PROPOSAL_SCHEMA_VERSION,
    };
    use ed25519_dalek::{Signer, SigningKey};

    fn target() -> ProposalTarget {
        ProposalTarget {
            repository: "owner/repository".into(),
            base_ref: "main".into(),
            source_revision: "a".repeat(40),
        }
    }

    fn proposal() -> SelfEvolutionProposalV1 {
        SelfEvolutionProposalV1 {
            schema_version: SELF_EVOLUTION_PROPOSAL_SCHEMA_VERSION,
            summary: "Improve a non-core accessible label.".into(),
            findings: vec![SelfEvolutionFinding {
                category: FindingCategory::Accessibility,
                severity: FindingSeverity::Low,
                path: "apps/desktop/src/Panel.tsx".into(),
                line_start: Some(1),
                line_end: Some(2),
                description: "A control needs a label.".into(),
                recommendation: "Add an accessible label.".into(),
            }],
            changes: vec![ProposedChange {
                path: "apps/desktop/src/Panel.tsx".into(),
                operation: ProposedChangeKind::Modify,
                summary: "Add an accessible label.".into(),
                patch_sha256: "b".repeat(64),
            }],
        }
    }

    fn signed() -> (SignedSelfEvolutionApproval, SigningKey) {
        let key = SigningKey::from_bytes(&[7; 32]);
        let claims = SelfEvolutionApprovalClaims {
            key_id: "approval-key-v1".into(),
            proposal_sha256: proposal_sha256(&proposal()).unwrap(),
            repository: "owner/repository".into(),
            base_ref: "main".into(),
            source_revision: "a".repeat(40),
            approver_id: "human-admin-1".into(),
            approved_scope: vec!["apps/desktop/src".into()],
            approved_at_unix_ms: 1_000,
            expires_at_unix_ms: 60_000,
            nonce: [9; 16],
        };
        let signature = key.sign(&approval_signing_bytes(&claims).unwrap()).to_bytes();
        (SignedSelfEvolutionApproval { claims, signature }, key)
    }

    #[test]
    fn verifies_only_an_exact_signed_proposal_and_scope() {
        let (signed, key) = signed();
        let keys = vec![("approval-key-v1".into(), "human-admin-1".into(), key.verifying_key())];
        let verified = verify_self_evolution_approval(
            &signed, &keys, &target(), &proposal(),
            &["apps/desktop/src".into()], 2_000,
        ).unwrap();
        assert_eq!(verified.approver_id(), "human-admin-1");
        assert_eq!(verified.nonce(), &[9; 16]);
    }

    #[test]
    fn rejects_changed_proposal_target_or_scope() {
        let (signed, key) = signed();
        let keys = vec![("approval-key-v1".into(), "human-admin-1".into(), key.verifying_key())];
        let mut changed = proposal();
        changed.summary.push_str(" changed");
        assert_eq!(
            verify_self_evolution_approval(&signed, &keys, &target(), &changed,
                &["apps/desktop/src".into()], 2_000),
            Err(ApprovalVerificationError::ProposalDigestMismatch)
        );
        let mut other_target = target();
        other_target.source_revision = "c".repeat(40);
        assert_eq!(
            verify_self_evolution_approval(&signed, &keys, &other_target, &proposal(),
                &["apps/desktop/src".into()], 2_000),
            Err(ApprovalVerificationError::TargetMismatch)
        );
        assert_eq!(
            verify_self_evolution_approval(&signed, &keys, &target(), &proposal(),
                &["apps/desktop/src-tauri".into()], 2_000),
            Err(ApprovalVerificationError::InvalidScope)
        );
    }

    #[test]
    fn rejects_expired_unknown_key_tampered_and_overlong_approvals() {
        let (mut signed, key) = signed();
        let keys = vec![("approval-key-v1".into(), "human-admin-1".into(), key.verifying_key())];
        assert_eq!(
            verify_self_evolution_approval(&signed, &keys, &target(), &proposal(),
                &["apps/desktop/src".into()], 60_000),
            Err(ApprovalVerificationError::Expired)
        );
        assert_eq!(
            verify_self_evolution_approval(&signed, &[], &target(), &proposal(),
                &["apps/desktop/src".into()], 2_000),
            Err(ApprovalVerificationError::UnknownKey)
        );
        let wrong_identity = vec![("approval-key-v1".into(), "different-admin".into(), key.verifying_key())];
        assert_eq!(
            verify_self_evolution_approval(&signed, &wrong_identity, &target(), &proposal(),
                &["apps/desktop/src".into()], 2_000),
            Err(ApprovalVerificationError::UnknownKey)
        );
        signed.claims.approver_id = "other-admin".into();
        assert_eq!(
            verify_self_evolution_approval(&signed, &keys, &target(), &proposal(),
                &["apps/desktop/src".into()], 2_000),
            Err(ApprovalVerificationError::InvalidSignature)
        );
    }

    #[test]
    fn approval_never_authorizes_protected_or_duplicate_scope() {
        let (mut signed, key) = signed();
        let keys = vec![("approval-key-v1".into(), "human-admin-1".into(), key.verifying_key())];
        signed.claims.approved_scope = vec!["crates/ujani-core".into()];
        let signature = key.sign(&approval_signing_bytes(&signed.claims).unwrap()).to_bytes();
        signed.signature = signature;
        assert_eq!(
            verify_self_evolution_approval(&signed, &keys, &target(), &proposal(),
                &["crates/ujani-core".into()], 2_000),
            Err(ApprovalVerificationError::InvalidScope)
        );
        assert!(canonical_scope(&["apps/src".into(), "APPS/src".into()]).is_err());
    }
}
