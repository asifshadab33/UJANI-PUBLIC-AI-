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
- No shell execution, model-generated code execution, cloud calls, file access, device sharing or background compute.

## Not implemented yet

- GPU/NPU and runtime compatibility detection.
- Local model download, management or inference.
- Task decomposition and model-based planning.
- Sandboxing and OS-enforced per-task resource quotas.
- Cloud subtask gateway and consent workflow.
- Persistent encrypted history and settings.

The word-count action is a deterministic plumbing test, not an AI feature. Do not describe this scaffold as a finished AI assistant.