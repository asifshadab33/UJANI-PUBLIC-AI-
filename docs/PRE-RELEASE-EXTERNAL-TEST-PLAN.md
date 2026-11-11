# Public AI pre-release external test plan

Controlled pre-alpha testing only. Use synthetic/non-sensitive data. Record commit, build checksum, OS version/architecture, hardware, steps, expected/actual result, pass/fail/blocked status and redacted evidence for every case.

## Evidence rules
- CI is evidence only for the exact commit and job; it does not prove physical-device installation, OS permission behaviour or independent security.
- Mark unrun cases Not run. Never infer success from a green unrelated job.
- This checklist does not authorize merge or release. Keep unsigned previews labelled unsigned.

## Test matrix
| ID | Area | Procedure | Expected evidence |
|---|---|---|---|
| P01 | Install | Clean install, launch, upgrade and uninstall each supplied OS package | Version, checksum, OS build, logs, screenshots |
| P02 | Ollama prerequisite | Stop/uninstall Ollama, then open local inference | Clear setup guidance; no hang or silent cloud fallback |
| P03 | Local-only | Run a supported task in Fortress with network disconnected | Task works locally or fails clearly; network trace shows no task egress |
| P04 | Model errors | Test missing/incompatible model, stopped service and slow response | Bounded timeout and actionable recoverable error |
| P05 | Cancellation | Cancel during chat, folder scan and PDF processing | Work stops; no stale success or orphaned busy UI |
| P06 | Filesystem | Test symlink/reparse point, outside-root path, unreadable file, deep/large tree | Unsafe entries rejected/skipped per policy; limits disclosed |
| P07 | PDF | Test text, scanned, malformed, encrypted and oversized PDFs | Supported cases work; unsupported cases fail safely |
| P08 | Cache | Test disabled, expiry, clear, restart and corrupted cache | Disabled/expired/cleared entries not reused; no unrelated data loss |
| P09 | History | Test recording off, encryption, deletion and unavailable credential vault | Defaults and limitations visible; synthetic secrets absent from plaintext |
| P10 | Cloud approval | Exercise deny/ask/allow and inspect outbound request | No transfer without required approval; only approved minimum data sent |
| P11 | UI resilience | Visit every navigation item repeatedly; induce slow/error paths | No reproducible freeze, infinite spinner or silent failure |
| P12 | Privacy | Inspect logs, cache, crash output and storage with synthetic secrets | No unintended credentials, prompts or file contents |

## Platform evidence
For each platform actually supplied, record OS build, architecture, hardware, package checksum/signing state, install/launch, permissions, update/uninstall and known limitations. Do not claim support for untested OS versions.

## Security gates
Independently review local-only network boundaries (including proxies/redirects), filesystem race/reparse handling, approval bypass resistance, native cancellation/resource limits, dependency provenance and installer integrity. Report vulnerabilities privately.

## Tester report
Test ID; commit; build; OS/hardware; preconditions; steps; expected; actual; pass/fail/blocked; evidence link; severity; reproducibility; tester.

## Release blockers
Any unexplained outbound transfer, security-boundary failure, data loss, claimed-platform installer failure or reproducible hang blocks release. Physical-device testing, signing, independent security review and OS support sign-off remain separate gates.