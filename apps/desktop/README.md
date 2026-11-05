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
- No shell execution, model-generated code execution, cloud calls, arbitrary filesystem path access, device sharing or background compute.\n- User-selected text files are analysed in memory (1 MiB limit).\n- User-selected PNG/JPEG/GIF/WebP/BMP image bytes are header-inspected in memory (20 MiB limit).\n- Ollama calls are fixed to the loopback service at 127.0.0.1:11434, with prompt/response size limits and request timeouts.

## Using local Ollama\n\n1. Install Ollama separately from its official distribution.\n2. Start the Ollama service on the same device.\n3. Pull a model you have enough RAM/VRAM to run (for example, `ollama pull llama3.2`).\n4. Launch UJANI Desktop and select the detected model under Local Ollama AI.\n5. Choose Fortress or Smart and run a prompt. The app sends the prompt only to `127.0.0.1:11434`; it has no cloud fallback.\n\nOllama must remain bound to the local loopback interface. Do not expose its API to an untrusted network. Fortress here means no cloud AI transfer through UJANI; it does not currently apply an OS-level firewall rule, and the local Ollama HTTP service is still contacted over loopback.\n\n## Not implemented yet

- GPU/NPU and runtime compatibility detection.
- Real local text inference through an explicitly installed Ollama service at 127.0.0.1:11434 (Fortress and Smart only; no cloud fallback).
- Automatic Ollama installation/model downloads and model lifecycle management.
- Task decomposition and model-based planning.\n- Visual image understanding and OCR (the current image tool reads basic metadata only).
- Sandboxing and OS-enforced per-task resource quotas.
- Cloud subtask gateway and production consent workflow (the current control is a preview input only).
- OS-enforced Fortress network isolation (the policy preview cannot disable networking).
- Persistent encrypted history and settings.

The word-count action is a deterministic plumbing test, not an AI feature. Do not describe this scaffold as a finished AI assistant.