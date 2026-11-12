# Hardware-aware default model recommendations

## Default user experience

The default installation of UJANI Public AI downloads no model. It remains useful through native OS-powered and deterministic local features. Local model use is optional. UJANI-managed downloads are currently disabled until exact artifact provenance and the Ollama daemon's actual model-store path/free-space can be verified; users may install manually through their trusted Ollama interface after checking the source, licence and storage.

Ollama is the recommended optional runtime for supported desktop operating systems because UJANI already has a loopback-only Ollama adapter. Ollama is a runtime, not a model. It is not bundled in the UJANI installer, installed silently, or required for first launch. Do not assume desktop Ollama support on Android or iOS.

## Conservative starting profiles

The model recommendation uses measured total RAM and current available RAM. The values below are conservative starting thresholds, not vendor guarantees or benchmark results:

| Device memory | Starting choice | Starting context |
|---|---|---:|
| 4 GiB total or less | No local model; use native UJANI features | — |
| 6 GiB total and at least 2.5 GiB available | Qwen3 0.6B | 4K |
| 8 GiB total and at least 3.5 GiB available | Qwen3 1.7B | 4K |
| 16 GiB total and at least 6 GiB available | Qwen3 4B | 4K |
| 24 GiB+ total and at least 10 GiB available | Qwen3 8B | 4K |

The recommendation must choose the largest tier whose total-RAM and currently-available-RAM thresholds both pass. If memory evidence is absent, invalid, stale or inconsistent, recommend no model. On low-memory systems allow one inference at a time and reserve at least 512 MiB beyond the runtime's estimated working set. The native capability/model-fit admission checks remain authoritative and may reject a model even when the onboarding heuristic recommends it.

The official Ollama Qwen3 registry currently lists approximate artifact downloads of 523 MB for 0.6B, 1.4 GB for 1.7B, 2.5 GB for 4B and 5.2 GB for 8B. These are registry snapshots, not installed sizes or RAM requirements; exact manifest bytes, digest, quantisation and licence must be re-resolved immediately before the user confirms download. Context length and runtime buffers increase working memory.

## User safeguards

- Display the recommended model and why, but do not preselect it for inference or download.
- The user may choose another candidate or **Not now**.
- Selecting a recommendation only fills the candidate name for manual Ollama setup; it never starts a download or selects an inference model.
- UJANI-managed downloads remain disabled until a trusted registry manifest, immutable artifact digest, licence, exact download size and the Ollama daemon's actual model-store path/free-space can be verified.
- Recheck free RAM and disk space immediately before inference; any future managed download must recheck both before transfer.
- Never silently switch models, start downloads, or fall back to cloud.
- Do not bundle Ollama or model files in the UJANI installer.
- Do not expose a mobile model recommendation until the exact runtime/model/device combination is validated.

## Enterprise policy

Enterprise does not use one universal model default. The administrator chooses an approved fleet profile, and the same conservative per-device hardware bands provide recommendations only within the tenant's exact model allowlist. A 4 GiB device receives no model recommendation by default. Administrator approval and device-owner consent remain separate. A model's presence or approval does not authorize a workload or remote execution.

## Acceptance tests

- 4 GiB device → no model recommended and no download.
- 6 GiB device with less than 2.5 GiB available → no model recommended.
- 6 GiB device with at least 2.5 GiB available → Qwen3 0.6B candidate.
- 8 GiB device with at least 3.5 GiB available → Qwen3 1.7B candidate.
- 16 GiB device with at least 6 GiB available → Qwen3 4B candidate.
- 24 GiB device with at least 10 GiB available → Qwen3 8B candidate.
- Unknown hardware or mobile platform → no desktop model recommendation.
- Clicking the recommended choice only fills a candidate name; no UJANI-managed download can start until the complete artifact and storage preflight is implemented and validated.
- Runtime capability admission can reject the suggested candidate.
