# UJANI Public AI — Manual Smoke-Test Matrix

Status: **test procedure only; no manual device results are claimed**  
Related tracking: [#70 — PDF upload, screenshot capture and UI freeze recovery](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/70), [#72 — PDF upload cancellation](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/pull/72), [#67 — desktop support baseline](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/issues/67).

## Purpose and rules

Use this matrix before promoting a preview build beyond supervised testing. Hosted CI is necessary but does not replace real-device install, launch, interaction, cancellation, accessibility, and recovery checks.

- Record the exact UJANI version, commit SHA, OS version/build, CPU architecture, installer type, and test date.
- Mark each case **PASS**, **FAIL**, **BLOCKED**, or **NOT RUN**. Attach a short reproduction and sanitized evidence for failures.
- Do not include document contents, credentials, private keys, access tokens, or personal file paths in logs or screenshots.
- Use synthetic documents and test accounts. Keep network access and model availability explicit in the test record.
- A failed or blocked case must not be reported as passed. Do not claim support for an OS baseline that was not actually tested.

## A. Install, launch, and recovery

| ID | Scenario | Procedure | Expected result |
|---|---|---|---|
| A-01 | Clean install | Install the preview package on a clean supported test machine. | Installer completes; app launches; no missing-runtime crash. |
| A-02 | First launch without Ollama | Keep Ollama stopped or unavailable and launch UJANI. | App remains usable; local-model unavailability is explained without hanging. |
| A-03 | Ollama available, no model | Start Ollama with no model installed. | Model state is clear; no repeated blocking request or crash. |
| A-04 | Ollama available with model | Install a small approved test model and run one short local task. | Task completes locally; progress and failure states are visible. |
| A-05 | Restart after task failure | Cause a safe, reproducible task failure, then restart the app. | App starts normally; stale task state does not block new work. |
| A-06 | Upgrade and uninstall | Upgrade a prior test build, then uninstall using the supported path. | Upgrade preserves only documented user data; uninstall behavior is clear and predictable. |

## B. PDF upload and cancellation

Use synthetic PDFs, including a valid small PDF, a valid near-limit PDF, an empty file, a malformed file, and an encrypted file. Record actual file sizes; do not use real personal documents.

| ID | Scenario | Procedure | Expected result |
|---|---|---|---|
| B-01 | Normal upload | Upload and process a small valid PDF. | Processing completes; progress and final result are shown. |
| B-02 | FileReader cancellation | Select a larger valid PDF and press Cancel while the file is being read. | Read is aborted; UI returns to idle; no native task remains registered. |
| B-03 | Native decode cancellation | Press Cancel during native base64 decoding/preparation. | UI returns promptly; task cancellation is observed; decode worker remains bounded. |
| B-04 | Cancel and immediate retry | Cancel during preparation and immediately select the same file again. | No overlapping unbounded decoder work; a clear retry/busy message appears if the single decode slot is still occupied. |
| B-05 | Malformed and empty files | Upload malformed PDF bytes and an empty file. | Clear validation error; no hang, crash, or stale busy indicator. |
| B-06 | Encrypted PDF | Upload a password-protected PDF. | Unsupported/password-required behavior is explicit; no false successful extraction. |
| B-07 | Size limit | Test just below and above the documented upload limit. | Below-limit behavior is bounded; above-limit input is rejected before excessive allocation. |
| B-08 | Repeated task IDs | Exercise duplicate active task IDs through automated/native tests. | Duplicate registration is rejected; registry cleanup permits a later valid retry. |
| B-09 | Cancellation cleanup | Cancel, wait for worker completion, then run a new PDF task. | No stale cancellation token or registry entry affects the new task. |

**Important limitation:** Tokio `spawn_blocking` work that has already started cannot be forcibly aborted. Cancellation can stop awaiting the result, but the bounded worker may finish in the background. B-03/B-04 must verify the real resource and retry behavior rather than assuming immediate CPU termination.

## C. Screenshot capture and attachment

| ID | Scenario | Procedure | Expected result |
|---|---|---|---|
| C-01 | Capture supported image | Capture a permitted test screenshot and attach it. | Preview is visible and the attachment is associated with the intended message. |
| C-02 | Capture permission denied | Deny the relevant OS permission, then retry. | Clear permission guidance; no crash or infinite spinner. |
| C-03 | Capture cancelled | Cancel the OS picker/capture flow. | UI returns to idle without a phantom attachment. |
| C-04 | Unsupported or oversized image | Attach a synthetic unsupported/oversized image. | Clear validation message; app remains responsive. |
| C-05 | Attachment send failure | Simulate an unavailable model/runtime or controlled request failure. | Failure is visible; attachment state is not falsely marked as processed. |
| C-06 | Privacy check | Inspect diagnostic output for the test flow. | No image bytes, document contents, secrets, or private paths are logged by default. |

## D. Navigation, sublinks, and UI freeze recovery

| ID | Scenario | Procedure | Expected result |
|---|---|---|---|
| D-01 | Primary navigation | Open every primary sidebar destination once. | Correct view opens; no blank page or uncaught exception. |
| D-02 | Nested links | Open every visible sublink from each primary view. | Correct destination opens; browser history/back behavior remains coherent. |
| D-03 | Rapid navigation | Switch destinations repeatedly while a read-only task is running. | UI remains responsive; no stale view update or duplicate task. |
| D-04 | Long-running task | Run a bounded local task and observe progress. | UI remains interactive; progress is meaningful; cancellation is available where supported. |
| D-05 | Repeated error recovery | Trigger the same safe validation error repeatedly, then run a valid task. | Error state clears appropriately; valid task succeeds. |
| D-06 | Narrow/mobile viewport | Test the supported responsive/mobile web experience at narrow widths. | Content remains reachable; controls do not overlap or become inaccessible. |
| D-07 | Screenshot/send from navigation | Navigate away and back during a permitted screenshot attachment flow. | No stale or duplicated attachment; no unhandled rejection. |

## E. Privacy, history, and local-first behavior

| ID | Scenario | Procedure | Expected result |
|---|---|---|---|
| E-01 | History opt-out | Disable persistent history and complete a test task. | New result is not persisted to history. |
| E-02 | History opt-in | Enable history and complete a test task. | Only the documented result metadata/content is stored under the selected policy. |
| E-03 | History load partial failure | Simulate history-entry load failure while settings are readable. | Settings remain available; failure is surfaced without silently changing the user's preference. |
| E-04 | Cache scope | Repeat an explicitly permitted local search. | Reuse occurs only within the documented user/workspace scope and freshness policy. |
| E-05 | Cloud consent boundary | Test with cloud use disabled. | No cloud request is made; local-only behavior is accurately described. |

## F. Cross-platform execution record

Run applicable cases on each claimed platform and minimum supported OS baseline. A successful hosted build is not an install/launch result.

| Platform | OS version/build | Architecture | Installer | Cases run | Result | Evidence reference |
|---|---|---|---|---|---|---|
| Windows |  |  | MSI / NSIS |  | NOT RUN |  |
| macOS |  |  | DMG |  | NOT RUN |  |
| Linux |  |  | DEB / AppImage |  | NOT RUN |  |
| Mobile web / narrow viewport | Browser + OS |  | N/A |  | NOT RUN |  |

## Release gate

Do not mark this matrix complete until the test record is filled by a human tester on actual devices. Any failure affecting data integrity, privacy, cancellation, application stability, or permission boundaries blocks broader distribution until fixed and retested. Unsigned CI preview installers must remain clearly labelled as previews.
