# UJANI Public AI — implementation claims ledger

This ledger records what the current remediation branch can support. A source-code contract is not evidence of a live transport, OS sandbox or production service.

| Claim | Evidence to inspect | Last source verification |
|---|---|---|
| Peer envelope v2 is recipient-bound and uses domain-separated, strict Ed25519 verification | `crates/ujani-core/src/peer_protocol.rs`; tests `signs_verifies_and_rejects_replay`, `signature_binds_consent_scope_and_task_content` | `5c53d5557139fdb7cfdc758616b83345aa70275f` |
| Peer replay nonce claims are persisted before accepting a trusted peer envelope | `apps/desktop/src-tauri/src/security_store.rs`; persistent replay tests | `f10b3b43c6ee3830769f7603c93a1fe6d8d4660a` |
| Device identity is stored in the OS credential vault and peer enrollment/revocation requires native confirmation | `apps/desktop/src-tauri/src/device_identity.rs`, `security_store.rs` | `f10b3b43c6ee3830769f7603c93a1fe6d8d4660a` |
| PDF worker has platform-specific process restrictions; this is not a full application sandbox | `apps/desktop/src-tauri/src/task_engine.rs`, `docs/ARCHITECTURE.md` | `f10b3b43c6ee3830769f7603c93a1fe6d8d4660a` |
| Successful local/approved cloud chat receipts are locally persisted and hash-chain verifiable; receipts are unsigned and not externally anchored | `apps/desktop/src-tauri/src/security_store.rs`, `App.tsx`, `docs/IMPLEMENTATION_STATUS.md` | `f10b3b43c6ee3830769f7603c93a1fe6d8d4660a` |
| Document/PDF/translation commands use task IDs, cancellation and an overall time budget | `apps/desktop/src-tauri/src/lib.rs`, `App.tsx` | PR #18 head at last check: `d259a739d1d7cf6cee3db4bbe87c7e6fc5273f98` |
| Full Fortress OS isolation, encrypted peer transport, dispatch and remote execution are not implemented | `docs/PROJECT_STATUS_AND_RELEASE_GATES.md`, `docs/IMPLEMENTATION_STATUS.md` | `5c53d5557139fdb7cfdc758616b83345aa70275f` |

CI runs `scripts/check-doc-claims.sh` to catch selected stale protocol/security claims. It is a narrow consistency check, not a proof that every documentation statement is correct. Update the evidence and SHA after material implementation changes.
