# Local model choices and first-run onboarding

## Product decision

UJANI Public AI remains local-first and optional-model by default. Initial setup must work without downloading any model. Users may continue with native operating-system tools and deterministic local features, choose one or more local models, or defer the decision and return to Settings later.

The curated candidate catalogue is `catalogs/local-model-catalog-v1.json`. It is descriptive metadata, not a trusted download manifest and not permission to download. The exact runtime artifact, quantisation, digest, licence, download bytes and installed-size estimate must be resolved and verified before showing the final confirmation.

## First-run flow

1. Explain in plain language what local AI models do and that they are optional.
2. Detect OS, architecture, available RAM, free storage and available acceleration through supported native APIs. Unknown values remain Unknown; do not infer hardware capability.
3. Present four choices: **Basic**, **Advanced**, **Choose models**, and **Not now**. Do not preselect a download.
4. Show each model as a plain-language card: purpose, example tasks, limitations, exact download size, estimated installed size, available/recommended RAM, runtime/platform support and licence.
5. Distinguish installed models from downloadable catalogue entries. A model is not installed merely because it appears in the catalogue.
6. Before download, show the exact artifact name, publisher/source, quantisation, version/digest, download and disk-space requirements, licence and any network transfer. Require explicit confirmation.
7. Show bounded progress, allow cancellation, and report success/failure. Never claim installation succeeded until the runtime confirms it.
8. Let users change or remove models later; never switch a selected model silently.

## Hardware guidance

RAM bands in the catalogue are conservative planning hints only, not hard guarantees. The advisor must consider available memory, total memory, OS overhead, runtime overhead, context length, quantisation, concurrent workloads and GPU/accelerator memory where supported. The existing native model inventory and pre-inference fit checks remain authoritative for actual runtime admission. If evidence is missing or stale, show **Cannot verify fit** and let the user decide only where policy permits; do not label it compatible.

Disk checks must reserve headroom for the download, temporary files and installed artifact. Explain that model files are additional to the UJANI application and optional Ollama runtime. Never bundle a large model or Ollama into the default installer.

## Model cards and trust

- Use the same stable catalogue metadata in Public and Enterprise; product-specific policy is applied outside the catalogue.
- Exact artifact sizes are dynamic and quantisation-specific. Never hardcode an approximate size as a guaranteed download size.
- Verify publisher/source, immutable version or digest, file integrity and licence before offering an artifact.
- Do not fetch or execute model-supplied code, templates or tool definitions as trusted instructions.
- Treat model output as untrusted; tool calls pass through the existing typed capability registry and policy gate.
- Explain that a small model may make mistakes and identify tasks for which it is not suitable.
- Keep model prompts and local files local unless the user separately authorises a specific external action.

## Runtime strategy

Ollama remains an optional desktop runtime, not a mandatory dependency. Reuse the existing loopback-only Ollama adapter and model inventory. Evaluate llama.cpp and mobile-native runtimes behind typed adapters; do not claim Android/iOS support until each exact runtime/model combination passes device tests. The model catalogue must distinguish runtime availability from model availability.

## Acceptance criteria

- First run succeeds with no model installed and no model download.
- No download begins before a clear user confirmation.
- Every model card explains purpose, limitations, licence and resource needs.
- Exact download bytes and artifact identity are resolved before confirmation.
- Unknown hardware or runtime evidence is never presented as verified compatibility.
- Installed and downloadable models are visually distinct.
- Model download cancellation, failure, retry and uninstall are observable and tested.
- Existing local-first policy, Fortress restrictions and no-silent-cloud-fallback behavior remain unchanged.
- Catalogue schema/entries are validated in CI and shared with Enterprise byte-for-byte.
- UI accessibility, localization and small-screen layout are tested.
