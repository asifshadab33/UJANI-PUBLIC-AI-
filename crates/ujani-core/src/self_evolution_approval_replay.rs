//! Atomic replay-consumption boundary for verified self-evolution approvals.
//!
//! Verification alone does not make an approval single-use. A trusted host must
//! call this boundary and durably claim the approver/nonce pair before it stages
//! or applies any change. Implementations must make `claim_nonce` atomic across
//! processes and persist the claim before returning success. A memory-only map
//! is suitable for tests, not production. This module never applies a patch.

use crate::self_evolution_approval::VerifiedSelfEvolutionApproval;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalNonceStoreError {
    /// This approver/nonce pair was already consumed.
    AlreadyConsumed,
    /// The durable store could not confirm the claim.
    StorageUnavailable,
    /// The store rejected malformed or inconsistent record data.
    InvalidRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalConsumptionError {
    Expired,
    Store(ApprovalNonceStoreError),
}

/// Trusted host persistence boundary for single-use approval receipts.
///
/// Implementations must:
/// - atomically reject an existing (approver_id, nonce) pair;
/// - persist the new claim, including its expiry, before returning Ok;
/// - serialize concurrent calls across threads/processes;
/// - fail closed if durable storage is unavailable or integrity checks fail;
/// - never delete unexpired claims as a cleanup shortcut.
///
/// Expired claims may be compacted only when the store can prove they cannot
/// make a still-valid approval replayable.
pub trait SelfEvolutionApprovalNonceStore {
    fn claim_nonce(
        &mut self,
        approver_id: &str,
        nonce: &[u8; 16],
        expires_at_unix_ms: u64,
        now_unix_ms: u64,
    ) -> Result<(), ApprovalNonceStoreError>;
}

/// Consume one already-verified approval exactly once.
///
/// The caller must invoke this immediately before staging an approved patch,
/// and must not stage or apply anything unless this function returns Ok.
/// Re-verification is still required against the current repository revision
/// and exact proposal immediately before any future application step.
pub fn consume_verified_self_evolution_approval(
    approval: &VerifiedSelfEvolutionApproval,
    store: &mut impl SelfEvolutionApprovalNonceStore,
    now_unix_ms: u64,
) -> Result<(), ApprovalConsumptionError> {
    if approval.expires_at_unix_ms() <= now_unix_ms {
        return Err(ApprovalConsumptionError::Expired);
    }

    store
        .claim_nonce(
            approval.approver_id(),
            approval.nonce(),
            approval.expires_at_unix_ms(),
            now_unix_ms,
        )
        .map_err(ApprovalConsumptionError::Store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_evolution_approval::{
        verify_self_evolution_approval, SelfEvolutionApprovalClaims, SignedSelfEvolutionApproval,
    };
    use crate::self_evolution::{
        FindingCategory, FindingSeverity, ProposedChange, ProposedChangeKind, ProposalTarget,
        SelfEvolutionFinding, SelfEvolutionProposalV1, SELF_EVOLUTION_PROPOSAL_SCHEMA_VERSION,
    };
    use ed25519_dalek::{Signer, SigningKey};
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct MemoryStore {
        claims: BTreeMap<(String, [u8; 16]), u64>,
        fail_closed: bool,
    }

    impl SelfEvolutionApprovalNonceStore for MemoryStore {
        fn claim_nonce(
            &mut self,
            approver_id: &str,
            nonce: &[u8; 16],
            expires_at_unix_ms: u64,
            now_unix_ms: u64,
        ) -> Result<(), ApprovalNonceStoreError> {
            if self.fail_closed {
                return Err(ApprovalNonceStoreError::StorageUnavailable);
            }
            if approver_id.is_empty() || expires_at_unix_ms <= now_unix_ms {
                return Err(ApprovalNonceStoreError::InvalidRecord);
            }
            let key = (approver_id.to_owned(), *nonce);
            if self.claims.contains_key(&key) {
                return Err(ApprovalNonceStoreError::AlreadyConsumed);
            }
            self.claims.insert(key, expires_at_unix_ms);
            Ok(())
        }
    }

    fn verified_approval() -> VerifiedSelfEvolutionApproval {
        let target = ProposalTarget {
            repository: "owner/repository".into(),
            base_ref: "main".into(),
            source_revision: "a".repeat(40),
        };
        let proposal = SelfEvolutionProposalV1 {
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
        };
        let key = SigningKey::from_bytes(&[11; 32]);
        let claims = SelfEvolutionApprovalClaims {
            key_id: "approval-key-v1".into(),
            proposal_sha256: crate::self_evolution_approval::proposal_sha256(&proposal).unwrap(),
            repository: target.repository.clone(),
            base_ref: target.base_ref.clone(),
            source_revision: target.source_revision.clone(),
            approver_id: "human-admin-1".into(),
            approved_scope: vec!["apps/desktop/src".into()],
            approved_at_unix_ms: 1_000,
            expires_at_unix_ms: 60_000,
            nonce: [9; 16],
        };
        let mut signed = SignedSelfEvolutionApproval {
            claims: claims.clone(),
            signature: [0; 64],
        };
        signed.signature = key.sign(
            &approval_message_for_test(&claims)
        ).to_bytes();
        verify_self_evolution_approval(
            &signed,
            &[("approval-key-v1".into(), "human-admin-1".into(), key.verifying_key())],
            &target,
            &proposal,
            &["apps/desktop/src".into()],
            2_000,
        ).unwrap()
    }

    fn approval_message_for_test(claims: &SelfEvolutionApprovalClaims) -> Vec<u8> {
        // Must match the domain-separated, serialized claims verified by the
        // sibling module. Kept local to this test to avoid exposing a signer.
        let encoded = serde_json::to_vec(claims).unwrap();
        let mut message = b"UJANI-PUBLIC-SELF-EVOLUTION-APPROVAL-V1\0".to_vec();
        message.extend_from_slice(&encoded);
        message
    }

    #[test]
    fn consumes_an_approval_once() {
        let approval = verified_approval();
        let mut store = MemoryStore::default();
        assert_eq!(
            consume_verified_self_evolution_approval(&approval, &mut store, 2_000),
            Ok(())
        );
        assert_eq!(
            consume_verified_self_evolution_approval(&approval, &mut store, 2_001),
            Err(ApprovalConsumptionError::Store(ApprovalNonceStoreError::AlreadyConsumed))
        );
    }

    #[test]
    fn fails_closed_when_storage_is_unavailable() {
        let approval = verified_approval();
        let mut store = MemoryStore { fail_closed: true, ..MemoryStore::default() };
        assert_eq!(
            consume_verified_self_evolution_approval(&approval, &mut store, 2_000),
            Err(ApprovalConsumptionError::Store(ApprovalNonceStoreError::StorageUnavailable))
        );
    }

    #[test]
    fn refuses_expired_approval_before_touching_store() {
        let approval = verified_approval();
        let mut store = MemoryStore::default();
        assert_eq!(
            consume_verified_self_evolution_approval(&approval, &mut store, 60_000),
            Err(ApprovalConsumptionError::Expired)
        );
        assert!(store.claims.is_empty());
    }
}
