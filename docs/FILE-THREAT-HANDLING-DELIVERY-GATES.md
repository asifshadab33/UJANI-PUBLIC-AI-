# File Threat Handling — Public AI Delivery Gates

**Status:** Delivery contract; not a claim that antivirus scanning or quarantine is implemented.
**Default posture:** Local-only, read-only discovery, fail closed for protected processing.

## Verified current boundary

The current Security Centre reads Windows Security Center aggregate antivirus/firewall health. It does not identify the product, scan a file, retrieve malware detections, quarantine, remediate, or inspect file contents. Other platform provider adapters are not implemented in this baseline.

The bounded text analyser is a pure in-memory text operation with a 1 MiB input ceiling; it is not a malware scanner. The PDF worker has a 10 MiB input and 1 MiB output ceiling and Linux syscall restrictions, but parser isolation is not a malware verdict. A successful parse or a healthy OS security-provider status must never be represented as proof that an individual file is clean.

## Required file state machine

Every file entering a UJANI-controlled processing pipeline must have an explicit state:

- `unscanned`: no supported scan evidence exists.
- `scanning`: an explicitly selected local provider is checking a stable file identity.
- `clean_reported`: a named provider reported no detection for this exact file hash at a recorded time.
- `malicious_reported`: a named provider reported a detection for this exact file hash.
- `suspicious`: evidence is incomplete, conflicting, stale, or requires review.
- `scan_unavailable`: provider missing, unhealthy, timed out, or unable to inspect the format.
- `quarantined`: a supported OS/provider workflow confirms isolation.
- `released`: an authorised user explicitly restored/released the file after reviewing the warning.

Never collapse `unscanned`, `scan_unavailable`, or `suspicious` into clean. A verdict is bound to a cryptographic file hash, file size, scan provider/version, timestamp, and scan outcome. If the file changes, the verdict is invalidated.

## Intake and processing requirements

1. Treat filename, extension, MIME type, archive metadata, document metadata and content as untrusted.
2. Identify actual file format using bounded inspection; do not execute, preview with an active-content-capable renderer, or invoke shell handlers during identification.
3. Keep the original file untouched. Scan a stable, read-only handle or a private bounded copy; detect replacement or mutation during scanning by rechecking identity/hash.
4. Enforce per-file and aggregate byte, entry-count, nesting-depth, decompression-ratio, CPU, memory and wall-clock limits for archives and parsers.
5. Reject path traversal, absolute paths, symlinks/hardlinks, device nodes, duplicate/confusable names and archive bombs before extraction. Do not extract archives into user-controlled destinations.
6. Do not send file bytes, hashes or metadata to cloud scanning unless the user explicitly approves the named recipient and data scope. Local-only is the default.
7. A scan timeout, unsupported format, encrypted archive, scanner crash, stale signature database or unavailable provider produces an unknown/unavailable result, never a clean result.
8. Keep file processing disabled for malicious or suspicious files unless a narrow, documented, user-approved safe-inspection mode exists. No model-generated code may execute.
9. Do not delete, repair, rename, move or quarantine files by direct filesystem manipulation. Use documented OS/provider quarantine APIs only after explicit confirmation and show the exact target.
10. Preserve provenance and minimal audit evidence. Do not store file contents or expose sensitive paths in telemetry.

## Scanner integration policy

- Prefer documented operating-system security interfaces and already-installed endpoint protection.
- Use provider-specific adapters with fixed executable/API locations, strict argument construction, no shell interpolation, bounded output, timeouts, cancellation and explicit provider identity.
- Never silently install ClamAV or another scanner, change Defender settings, disable exclusions, or elevate privileges.
- A provider's aggregate "healthy" status is posture information, not a per-file scan result.
- Do not bundle a second antivirus engine into UJANI's core. Optional scanners remain user-selected integrations.

## Delivery batches and acceptance

### FILE-01 — Honest capability contract
- Expose the file state machine to the UI and native layer.
- Display provider, file hash (short form), scan time, signature freshness if available, and uncertainty.
- Add tests proving unscanned/unavailable/unsupported/stale/conflicting results never map to clean.
- Acceptance: Security Centre explicitly states that current provider-health checks do not scan files.

### FILE-02 — Windows read-only scan adapter
- Integrate only documented Microsoft Defender interfaces for on-demand scanning/status where supported.
- No PowerShell string concatenation or shell execution; use a fixed executable/API and structured arguments.
- Add cancellation, bounded output, timeout, provider/version attribution and Windows-version integration tests.
- Do not claim quarantine until a documented quarantine API/workflow is implemented and verified.

### FILE-03 — Linux and macOS provider adapters
- Discover only documented, locally installed providers; no silent installation.
- Add capability/freshness states and platform-specific integration tests.
- Unsupported remains explicit and does not block unrelated app use.

### FILE-04 — Archive and document preflight
- Add bounded format detection, archive metadata inspection and safe extraction into a private temporary area only when required.
- Fuzz parsers and test traversal, symlink, nested archive, bomb, malformed and encrypted inputs.
- Keep PDF parsing isolated; no PDF parser result is a malware verdict.

### FILE-05 — Quarantine / restore
- Use OS/provider-supported quarantine APIs, never a hand-built "move file away" substitute.
- Require exact target preview and native user confirmation; record provider receipt and restore limitations.
- Test permissions, locked files, races, rollback and failed/partial operations.

### FILE-06 — Regression and adversarial validation
- Use the harmless EICAR test file only in an isolated test environment and only to verify provider integration.
- Add benign synthetic fixtures for malformed documents, archive bombs, path traversal, file mutation races, scanner timeout/crash and stale signatures.
- Never use live malware on developer or production machines.
- Run supported OS integration tests and independent security review before enabling by default.

## Release gates

- No "virus protected", "file clean", "quarantined" or "ransomware protected" claims without corresponding provider evidence and tests.
- File processing that requires a scan must fail closed if the required provider is unavailable; users may explicitly choose a clearly labelled unscanned workflow only where policy permits.
- No silent cloud upload, automatic deletion, arbitrary process termination, firewall changes or model-directed remediation.
- All native commands must be added to the command-security map, have receiver-side authorization, bounded inputs, cancellation and tests.
