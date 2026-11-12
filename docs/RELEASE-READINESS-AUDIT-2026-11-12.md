# UJANI Public AI — code and release-readiness audit (12 November 2026)

## Confirmed safeguards in the inspected code

- `pull_ollama_model` rejects downloads in Fortress and permits them only in Smart mode.
- Model identifiers are validated before a pull.
- A native confirmation is required before the local Ollama daemon starts a model download.
- The pull is cancellable and has a bounded 20-minute task deadline; progress and metadata-only execution receipts are recorded.
- Interactive local chat checks the current local capability snapshot and requires the selected model to be installed. It does not silently route to cloud when the local model is unavailable.
- Ollama requests use the fixed loopback endpoint, disable redirects and proxies, and apply bounded request/response handling.
- OS total and available memory are collected through `sysinfo`; unknown values are not inferred.

## Release blockers / work still required

1. **Model-download provenance and preflight:** the current download path delegates registry traffic to the Ollama daemon. The code explicitly records that UJANI cannot verify the downstream registry host, downloaded byte count or final artifact digest. The current native approval identifies the model name but does not show a verified exact download size, installed-size estimate, licence text, free-disk preflight or immutable digest. Do not describe the full model-download safeguards as complete.
2. **Disk telemetry:** model inventory currently sets `storage_free_mb` to unknown. Add a supported, bounded OS free-space observation and reserve headroom for download, temporary files and installed model data before enabling a model download.
3. **Artifact verification:** add a trusted manifest/registry metadata step, immutable digest verification and licence display. The user must approve the exact artifact and network source; an Ollama daemon pull alone does not prove those facts to UJANI.
4. **Context and RAM:** the hardware-aware profile PR sets Ollama's request context to 4,096 tokens and adds a regression test. This still needs exact-head CI and low-memory device benchmarks. The UI RAM tiers are conservative recommendations, not guarantees; the existing runtime capability gate remains authoritative.
5. **Mock adapter distinction:** `model_runtime.rs` contains a deterministic mock adapter. Its command is disabled in release builds. Production local chat currently uses the separate Ollama path. Keep the mock visibly labelled as test-only; do not present it as actual AI inference.
6. **Platform scope:** desktop Ollama recommendations must not be shown as mobile support. Android/iOS model runtimes need exact device/runtime testing before a mobile default is offered.
7. **PDF worker isolation:** the inspected native capability functions still report resource limits as Unknown, filesystem isolation as Unavailable, and network isolation as Unavailable on Windows / Unknown elsewhere. This remains a release blocker for any claim of a fully isolated PDF worker.
8. **Media features:** OCR, image understanding, audio transcription and video understanding remain unsupported locally. The media handoff PR is disclosure/manual links only; it does not upload user files.

## Required acceptance evidence before manual testing/release

- Exact-head CI, Rust tests/lints, CodeQL, RustSec and installer audit.
- Windows, Linux and macOS native build/install smoke tests.
- Real-device memory/disk tests at 4, 6, 8 and 16 GiB tiers.
- Verified registry manifest, digest, licence and disk-preflight tests including cancel, failure, retry and partial-download cleanup.
- Confirm Fortress cannot download or transmit task data and Smart-mode model download requires separate native approval.
- PDF isolation evidence on each claimed platform; do not promote Unknown/Unavailable capability to Available.
- A release checklist distinguishing implemented, mocked, unavailable and untested features.

## Audit conclusion

Public AI has meaningful local-first and consent safeguards, but it is not yet release-ready. In particular, exact model artifact verification and disk-space preflight are not implemented in the inspected download path. The current model recommendations must remain advisory until those gaps are closed and platform tests pass.
