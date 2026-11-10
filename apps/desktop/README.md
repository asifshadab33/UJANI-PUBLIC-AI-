# UJANI Public AI — Desktop prototype

Status: **pre-alpha desktop application / controlled preview**. The Tauri 2 application now includes real local Ollama inference and bounded native workflows; it is not a production-ready AI runtime. The repository's [implementation status](../../docs/IMPLEMENTATION_STATUS.md) is the authoritative feature and limitation record.

## Prerequisites

- Node.js 20 or newer
- Rust stable toolchain
- Tauri 2 system prerequisites for your operating system
- On Windows: Microsoft C++ Build Tools and WebView2 Runtime

See the official Tauri prerequisites: https://v2.tauri.app/start/prerequisites/

## Browser preview

From this directory:

```sh
npm install
npm run dev
```

The browser preview displays the interface and a clearly labelled demo. It cannot inspect native device hardware.

## Native desktop app

### Signed releases and automatic updates

The app checks GitHub Releases for updates on startup in production builds. An available update is shown in-app; the user chooses when to download and install it, then the app restarts. Updates are signature-verified by Tauri before installation. The updater public key is a public trust anchor and must match the private signing key used by release automation.

Release builds require repository Actions secrets `TAURI_UPDATER_PUBLIC_KEY`, `TAURI_SIGNING_PRIVATE_KEY`, and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Generate the key pair once with `npm run tauri signer generate -- -w ~/.tauri/ujani.key`; store the private key and its password only in GitHub Actions secrets named `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, and store the full `.pub` file contents as `TAURI_UPDATER_PUBLIC_KEY`. The release workflow injects the public key into the build configuration before bundling. Never commit either private key or password. Never rotate or lose the private key without a planned updater-key migration. Tag a release `vX.Y.Z` to build Windows, macOS and Linux installers and publish `latest.json`.

If these secrets are absent, signed release builds intentionally fail rather than publishing unsigned or unverifiable updates. Local development and browser preview do not perform update checks.

To prepare the first release:
1. From `apps/desktop`, run `npm run tauri signer generate -- -w ~/.tauri/ujani-updater.key` and keep the private key file offline in a secure backup.
2. In GitHub repository Settings → Secrets and variables → Actions, add `TAURI_SIGNING_PRIVATE_KEY` (full private-key file contents), `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (the password used when generating it; an empty password may be left unset), and `TAURI_UPDATER_PUBLIC_KEY` (full public-key file contents).
3. Update `apps/desktop/src-tauri/tauri.conf.json` version, commit and push, then create and push the matching tag, for example `v0.1.0`. The tag without its leading `v` must exactly match the app version.
4. The release workflow first runs frontend and Rust checks, then builds Windows MSI/NSIS, Linux DEB/AppImage, and macOS Intel/Apple Silicon DMG artifacts. It merges signed updater entries into `latest.json`, publishes only after all platform builds finish, and attaches SHA-256 checksums.

The Tauri updater signature is separate from operating-system publisher signing. **Before pushing a release tag**, configure these repository Actions values: Windows secrets `WINDOWS_CERTIFICATE` (base64 PFX) and `WINDOWS_CERTIFICATE_PASSWORD`, plus the `WINDOWS_TIMESTAMP_URL` repository variable; macOS secrets `APPLE_CERTIFICATE` (base64 Developer ID .p12), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_KEYCHAIN_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` (app-specific password) and `APPLE_TEAM_ID`. The release workflow rejects a tagged build when the platform-specific values are missing. Keep certificate/private-key material in GitHub secrets only. A valid Authenticode signature verifies the publisher signature but does not guarantee SmartScreen reputation or suppress first-download warnings. macOS signing/notarization must also be verified from the generated app and DMG evidence.

From this directory:

```sh
npm install
npm run tauri dev
```

Build a distributable package:

```sh
npm run tauri build
```

## Current functionality

- React/Vite desktop interface.
- Tauri 2 native command bridge.
- Read-only OS name, CPU architecture, logical CPU count and physical memory reporting.
- A narrowly scoped local word-count demonstration command.
- A typed local model adapter interface and registered-model catalog.
- A deterministic mock adapter with explicit model selection and a local-only automatic-selection path.
- A four-mode policy preview: Fortress, Worker, Smart (default), and Partner.
- Separate per-task external-transfer consent in the routing preview; selecting Worker or Partner does not grant consent.
- Bounded Partner planning (maximum three rounds) and local memory-budget checks in the policy layer.
- Tauri command exposure of the policy preview through the existing application library.
- Capability checks, prompt-size validation, and visible UI status/errors before adapter execution.
- No shell execution, model-generated code execution, arbitrary filesystem path access, device sharing or background compute. Cloud calls are available only through the separate user-configured gateway after per-task consent.
- User-selected text files are analysed in memory (1 MiB limit).
- User-selected PNG/JPEG/GIF/WebP/BMP images are size-checked up to 20 MiB; only the first 64 KiB is sent to the native processor for header inspection.
- Multi-turn local and online chat workspaces retain a bounded recent-turn context in the active app session. Local conversation context is sent only to loopback Ollama; online conversation context is sent only after a fresh per-request approval bound to the current prompt, endpoint, model and mode. Clear-chat controls remove the active UI context. Chat transcripts are not silently written to persistent history.
- Ollama calls are fixed to the loopback service at 127.0.0.1:11434, with prompt/response size limits, 45-second per-attempt timeouts, bounded retries and a 140-second overall deadline for interactive local chat. Local chat and approved cloud requests emit coarse task-stage events, have bounded overall deadlines, and support cancellation by dropping the active request future; provider/model work may not stop instantly after disconnect.
- Optional text-file summarisation runs through local Ollama only. The integrated pipeline accepts up to 256 KiB of text and processes larger inputs as bounded, sequential sections (maximum 16 sections) before a final synthesis step.
- User-selected PDFs (up to 10 MiB) are parsed in memory on a blocking worker, then analysed and summarised locally when extracted text is at most 256 KiB. Scanned PDFs still require OCR, which is not implemented.
- The integrated text/PDF workflow can optionally translate the generated summary into the selected language using a second local Ollama step. Translation is off unless the user selects it.
- Local text translation uses the selected Ollama model in Fortress/Smart modes (16 KiB input limit).
- A read-only, versioned local model inventory reports OS/architecture, CPU/RAM, installed Ollama models and runtime-reported capabilities/context when available. Automatic chat selection requires reported chat capability plus the advisory RAM-fit heuristic; unknown capabilities are not assumed.
- Local date/time display uses the device's local timezone without a network request. A read-only duplicate-file preview is available for a native-picker-selected folder: bounded traversal (depth 6, up to 5,000 entries), up to 64 MiB per file and 512 MiB total content hashing, SHA-256 verification, symlink/reparse-point skipping, a 60-second task budget and no delete/move/rename operations. Folder-based duplicate scanning is explicitly unavailable in the Android preview until Android Storage Access Framework support is implemented.
- Browser translation is offered as a manual link only; users must copy/paste text themselves.

## Using local Ollama

1. Install Ollama separately from its official distribution and start the service on the same device.
2. Open UJANI's Local AI workspace, refresh Ollama status, and use **Download model through Ollama** to enter a library identifier (for example, `llama3.2` or `qwen2.5:3b`). You can also use the Ollama CLI.
3. Review the model's size, license and hardware requirements before downloading. Downloads use your network and disk space and are performed by Ollama from its configured model registry.
4. Select the installed model and choose Fortress or Smart. Inference is sent only to `127.0.0.1:11434`; there is no cloud fallback.

Ollama must remain bound to the local loopback interface. Do not expose its API to an untrusted network. Fortress here means no cloud AI transfer through UJANI; it does not currently apply an OS-level firewall rule, and the local Ollama HTTP service is still contacted over loopback.

## Not implemented yet

- GPU-use evidence: when Ollama is available, UJANI reads its local `/api/ps` runtime telemetry. It reports active GPU model use only when a running model has positive `size_vram`; this is allocated VRAM, not total device VRAM or a full GPU/driver compatibility inventory. NPU and free storage remain Unknown until supported OS/runtime APIs provide reliable evidence. The inventory inspects metadata for at most 32 models per refresh (four concurrent requests, eight-second aggregate deadline); remaining/unsupported model capabilities remain Unknown.
- UJANI now offers an explicit, cancellable Ollama model pull from the local loopback service, with bounded progress parsing and a 30-minute task limit. Ollama model removal, updates and automatic runtime installation are not included.
- Task decomposition and model-based planning.
- OCR for scanned PDFs and images (image-only PDFs return a clear unsupported message).
- Visual image understanding (the current image tool reads basic metadata only).
- Sandboxing and OS-enforced per-task resource quotas; CPU/RAM are currently reported, not reserved or capped.
- Multi-step cloud subtask delegation, OS-backed saved credential vault, provider-specific cost metering, and production-grade cloud consent receipts.
- OS-enforced Fortress isolation remains unimplemented. The capability panel now explicitly reports `fortress.os_isolation` as unavailable. The native cloud command rejects Fortress-mode requests, but this is application-level policy and does not isolate the process or block all network access at the OS level.
- Automatic browser control or browser translation API integration. Browser translation is a manual, external copy/paste path; text is not pre-filled or submitted.
- Native OS translation APIs or downloadable OS language packs; locale detection is informational only.
- Encryption of non-sensitive history metadata and forensic secure erase. Prompt/output fields use AES-256-GCM with a key stored in the OS credential vault; availability depends on the OS vault being configured and accessible.

The Worker/Partner protocol core now has Ed25519-signed task envelopes, expiry, explicit consent, allow-listed scopes, SHA-256 input digest binding and in-memory replay checks. It is not yet a peer network: do not accept remote workers until enrollment, trusted-key rotation/revocation, encrypted transport, task dispatch, durable audit and abuse controls are implemented.\n\nThe word-count action is a deterministic plumbing test, not an AI feature. Do not describe this scaffold as a finished AI assistant.

## Optional cloud provider setup\n\n1. Choose an OpenAI-compatible provider with a Chat Completions endpoint.\n2. Check its pricing, privacy, retention and free-tier limits directly with that provider.\n3. Enter its HTTPS endpoint, model identifier and your own API key in the Optional cloud AI gateway.\n4. Enter a prompt in the main task box and explicitly approve external processing for that task.\n5. Run the cloud request. The key field clears after the attempt; UJANI does not save it.\n\nThe gateway currently supports JSON Chat Completions responses with `choices[0].message.content` and a 90-second provider request timeout (95-second task deadline). It does not support provider-specific formats, image uploads, embeddings, tool calling, or automatic cloud fallback. HTTP, localhost and private/reserved IP targets are rejected; DNS results are checked and pinned for the request, redirects and environment proxies are disabled. Fortress mode is rejected by the native command.\n\n## Security and implementation boundaries

- Optional bring-your-own-provider cloud gateway supports OpenAI-compatible Chat Completions endpoints. Users supply their own HTTPS endpoint, model and API key, and approve each prompt before transfer. Keys are passed only for that invocation and are not persisted. UJANI does not provide free keys or credits.
- Do not set an API key in frontend code, browser storage, or a command argument. A future hosted-provider integration must use OS-backed secret storage, explicit per-task consent, bounded data minimisation, and visible cost limits before it is enabled.
- Ollama retries are limited to three attempts for transient connection, timeout, and server failures. Client errors and invalid/oversized responses are not retried.
- The native device report includes total and currently available system memory. GPU/NPU detection is still explicitly unavailable; the app does not fabricate accelerator capability.
- The current file and image tools process bytes selected by the user in memory. They do not accept arbitrary filesystem paths, so there is no path-based API for an AI prompt to traverse.
- Optional SQLite task history is implemented with recording disabled by default, 30-day retention, a 100-entry cap, per-entry deletion and clear-all controls. Prompt and output fields are encrypted with AES-256-GCM; the key is held in the OS credential vault. Mode, model, type and timestamps remain visible as metadata. The BYOK cloud gateway is a single-provider API integration, not a secure peer-to-peer Worker/Partner network. A core protocol foundation now provides Ed25519-signed task envelopes, expiry, consent, allow-listed scopes, payload digest binding and in-memory replay protection. Authenticated enrollment, trusted-key lifecycle, peer transport, task execution, durable audit receipts, revocation and quotas remain required. Smart, Worker and Partner can send the current prompt to this provider after explicit approval; this does not enroll or execute on other users’ devices.
- Smart mode currently executes locally when the local Ollama route is available. It does not ask a model to estimate confidence and silently escalate. A future external fallback must wait for explicit approval for the exact data/subtask.

## Corrected implementation audit (2026-11-06)

The supplied implementation guide was reviewed against the current architecture. The following distinctions are intentional:

- **Credentials:** the optional cloud gateway is a user-configured OpenAI-compatible provider path. The user supplies an endpoint, model and API key per request, with explicit consent. The key is not persisted; provider-specific cost metering is not implemented.
- **File access:** file selection uses the native UI file picker and passes bounded user-selected bytes to the native processor. The core does not accept arbitrary paths. This is narrower than a persistent approved-folder list and avoids creating a path traversal/symlink boundary that the current product does not need.
- **Async I/O:** current file processing is bounded in-memory parsing of already selected content, not filesystem I/O. Adding Tokio filesystem APIs or `spawn_blocking` would not improve this path.
- **Image processing:** current work is lightweight header inspection only. It is deliberately not full decoding, OCR, or vision inference; the parser is synchronous because it inspects at most 64 KiB. Tests cover supported formats and malformed headers.
- **Smart mode:** local Ollama execution remains local. Model-reported confidence is not a trustworthy security or correctness signal and must never authorize external transfer. If local execution cannot proceed, the app fails safely; no cloud fallback is implemented.
- **Persistence:** successful local AI chat, text/PDF summaries and translations can be saved to local SQLite after explicit opt-in. Prompt/output fields are AES-256-GCM encrypted with an OS credential-vault key; metadata remains plaintext.
- **Hardware:** available RAM is used for runtime routing checks and is reported separately from total RAM. GPU/NPU detection remains unknown until a verified platform implementation exists.
- **Tests:** core routing, text processing, image-header validation, and Ollama identifier/retry-policy tests are present. CI must still be observed on the pushed commit; a code push alone is not evidence that tests pass.

These choices keep the application honest and preserve its pre-alpha status. They are not claims of production-grade sandboxing or OS-level network isolation.


## Audit hardening

See [Security review](../../docs/SECURITY_REVIEW_2026-11.md) and [Fuzzing guide](../../docs/FUZZING.md). The fuzz harnesses are added but not yet run as a completed campaign.


## Platform support policy

See [the proposed platform support baseline and release gates](../../docs/PLATFORM_SUPPORT.md). The matrix is a proposed policy, not certification: minimum-version installation and launch tests must pass before a platform/version is advertised as supported.
