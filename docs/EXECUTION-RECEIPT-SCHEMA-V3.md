# Execution receipt schema v3

UJANI execution receipts are local metadata records in a hash-chained SQLite ledger. The chain is not digitally signed, independently anchored, or proof of full-system activity. A local administrator who can replace the application database can replace the chain.

## Compatibility

- Schema v1 remains readable; missing network evidence is interpreted as unknown.
- Schema v2 remains readable and continues to be used for existing inference receipts.
- Schema v3 is additive. It requires an outcome and supports external_resource for an approved resource-management action that is not inference.
- The outcome field is omitted when absent so re-serializing legacy v1/v2 receipts preserves their original JSON shape and does not rewrite their ledger hashes.

## Outcome values

| Value | Meaning |
|---|---|
| completed | The adapter reported successful completion |
| failed | The adapter returned an error after the approved operation was admitted |
| cancelled | The user cancelled the admitted operation |
| timed_out | The bounded operation deadline expired |

An outcome records what UJANI observed. It does not prove that an external daemon rolled back partial work after failure or cancellation.

## Approved model downloads

The Public AI pull_ollama_model command writes a v3 model.download receipt after a native consent approval and the bounded download lifecycle:

- location is external_resource: this is a model-store mutation initiated through the local Ollama daemon, not an inference-location claim.
- user_approval_id is a stable digest bound to the task ID, selected model, Smart mode and start timestamp.
- input_sha256 binds the operation, model identifier and mode. No model bytes, prompt text, credentials or filesystem paths are stored.
- output_sha256 is a digest of the bounded outcome marker, not a digest of the downloaded model artifact.
- model_version remains unknown because the current adapter does not verify a downloaded artifact digest.
- network_activity remains unknown. UJANI contacts 127.0.0.1:11434, but the Ollama daemon performs downstream registry traffic. UJANI does not currently verify that registry host or measure its bytes; unknown does not mean zero.
- A declined/expired consent prompt does not dispatch a download and therefore does not create a model-download receipt. Once an approved attempt enters the download phase, completed, failed, cancelled and timed-out outcomes are recorded.

The receipt-slot capacity is reserved before the download request. If capacity cannot be reserved, the download is not started. A process crash between dispatch and receipt persistence can still leave an incomplete operation record; this is not a transactional guarantee over Ollama's model store.

## Remaining limits

This change does not prove daemon-side cancellation, cleanup of partially downloaded model files, the actual downstream registry destination, network byte counts, model artifact integrity, or OS-enforced isolation. It does not complete the command-by-command native IPC audit. Those remain release-gate work.
