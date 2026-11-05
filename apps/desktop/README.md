# UJANI Public AI — Desktop prototype

Status: **pre-alpha scaffold**. This is the first desktop UI and native command bridge, not a complete AI runtime.

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
- No shell execution, model-generated code execution, cloud calls, arbitrary filesystem path access, device sharing or background compute.
- User-selected text files are analysed in memory (1 MiB limit).
- User-selected PNG/JPEG/GIF/WebP/BMP images are size-checked up to 20 MiB; only the first 64 KiB is sent to the native processor for header inspection.
- Ollama calls are fixed to the loopback service at 127.0.0.1:11434, with prompt/response size limits and request timeouts.
- Optional text-file summarisation runs through local Ollama only (up to 24 KiB of text).
- User-selected PDFs (up to 10 MiB) are parsed in memory on a blocking worker, then analysed and summarised with local Ollama when extracted text is at most 24 KiB.
- The integrated text/PDF workflow can optionally translate the generated summary into the selected language using a second local Ollama step. Translation is off unless the user selects it.
- Local text translation uses the selected Ollama model in Fortress/Smart modes (16 KiB input limit).
- A read-only capability panel reports CPU threads, RAM availability, OS and system locale.
- Browser translation is offered as a manual link only; users must copy/paste text themselves.

## Using local Ollama

1. Install Ollama separately from its official distribution.
2. Start the Ollama service on the same device.
3. Pull a model you have enough RAM/VRAM to run (for example, `ollama pull llama3.2`).
4. Launch UJANI Desktop and select the detected model under Local Ollama AI.
5. Choose Fortress or Smart and run a prompt. The app sends the prompt only to `127.0.0.1:11434`; it has no cloud fallback.

Ollama must remain bound to the local loopback interface. Do not expose its API to an untrusted network. Fortress here means no cloud AI transfer through UJANI; it does not currently apply an OS-level firewall rule, and the local Ollama HTTP service is still contacted over loopback.

## Not implemented yet

- Verified GPU/NPU and runtime compatibility detection; Ollama may use compatible acceleration, but UJANI does not currently verify or schedule it.
- Automatic Ollama installation/model downloads and model lifecycle management.
- Task decomposition and model-based planning.
- OCR for scanned PDFs and images (image-only PDFs return a clear unsupported message).
- Visual image understanding (the current image tool reads basic metadata only).
- Sandboxing and OS-enforced per-task resource quotas; CPU/RAM are currently reported, not reserved or capped.
- Cloud subtask gateway and production consent workflow (the current control is a preview input only).
- OS-enforced Fortress network isolation (the policy preview cannot disable networking).
- Automatic browser control or browser translation API integration. Browser translation is a manual, external copy/paste path; text is not pre-filled or submitted.
- Native OS translation APIs or downloadable OS language packs; locale detection is informational only.
- Persistent encrypted history and settings.

The word-count action is a deterministic plumbing test, not an AI feature. Do not describe this scaffold as a finished AI assistant.

## Security and implementation boundaries

- No OpenAI API key field, cloud credential command, or cloud API integration is present. UJANI does not accept a key from the frontend and does not send prompts to OpenAI.
- Do not set an API key in frontend code, browser storage, or a command argument. A future hosted-provider integration must use OS-backed secret storage, explicit per-task consent, bounded data minimisation, and visible cost limits before it is enabled.
- Ollama retries are limited to three attempts for transient connection, timeout, and server failures. Client errors and invalid/oversized responses are not retried.
- The native device report includes total and currently available system memory. GPU/NPU detection is still explicitly unavailable; the app does not fabricate accelerator capability.
- The current file and image tools process bytes selected by the user in memory. They do not accept arbitrary filesystem paths, so there is no path-based API for an AI prompt to traverse.
- SQLite task history and cloud cost accounting are not enabled. Persisting prompts/results requires a deliberate retention, deletion, and at-rest protection design; cost accounting requires a real cloud provider with authoritative token-usage data. Neither is simulated in this local-only prototype.
- Smart mode currently executes locally when the local Ollama route is available. It does not ask a model to estimate confidence and silently escalate. A future external fallback must wait for explicit approval for the exact data/subtask.

## Corrected implementation audit (2026-11-06)

The supplied implementation guide was reviewed against the current architecture. The following distinctions are intentional:

- **Credentials:** no cloud provider is wired into this prototype, so there is no API-key field, frontend credential path, or unused environment-variable secret. Do not add `UJANI_CLOUD_API_KEY` until a real provider adapter, explicit consent flow, OS-backed secret storage, and cost controls exist.
- **File access:** file selection uses the native UI file picker and passes bounded user-selected bytes to the native processor. The core does not accept arbitrary paths. This is narrower than a persistent approved-folder list and avoids creating a path traversal/symlink boundary that the current product does not need.
- **Async I/O:** current file processing is bounded in-memory parsing of already selected content, not filesystem I/O. Adding Tokio filesystem APIs or `spawn_blocking` would not improve this path.
- **Image processing:** current work is lightweight header inspection only. It is deliberately not full decoding, OCR, or vision inference; the parser is synchronous because it inspects at most 64 KiB. Tests cover supported formats and malformed headers.
- **Smart mode:** local Ollama execution remains local. Model-reported confidence is not a trustworthy security or correctness signal and must never authorize external transfer. If local execution cannot proceed, the app fails safely; no cloud fallback is implemented.
- **Persistence:** task history is currently session/UI state, not a durable database. SQLite is deferred until retention, deletion, migration, app-data location, and at-rest protection are designed together.
- **Hardware:** available RAM is used for runtime routing checks and is reported separately from total RAM. GPU/NPU detection remains unknown until a verified platform implementation exists.
- **Tests:** core routing, text processing, image-header validation, and Ollama identifier/retry-policy tests are present. CI must still be observed on the pushed commit; a code push alone is not evidence that tests pass.

These choices keep the application honest and preserve its pre-alpha status. They are not claims of production-grade sandboxing or OS-level network isolation.
