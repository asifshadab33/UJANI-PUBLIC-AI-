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
**Build prerequisite:** B-02a, B-03 and B-04 exercise the cancellation handoff and decoder lifecycle implemented in PR #72. Until that PR is merged into the tested build, mark these cases **BLOCKED**. Also mark them BLOCKED if the build lacks the pending-cancellation handoff, single-decoder admission guard, or decode lifecycle events described below.

| ID | Scenario | Procedure | Expected result |
|---|---|---|---|
| B-01 | Normal upload | Upload and process a small valid PDF. | Processing completes; progress and final result are shown. |
| B-02 | FileReader cancellation | Select a larger valid PDF and press Cancel while the file is being read. | Read is aborted; UI returns to idle; no native task remains registered. |
| B-02a | FileReader-to-native cancellation handoff | Run the native regression `pre_registration_cancellation_is_consumed_by_pdf_registration`; manually click Cancel while FileReader is still active, then retry the same file. The FileReader completion and IPC invocation run in one promise continuation, so do not claim a normal UI click can interleave between those synchronous steps. | The native test proves a pre-registration PDF cancellation is consumed by the task token at registration. The manual UI test proves FileReader aborts, no decode starts, the UI returns to idle, and same-file retry works. No stale task or cancellation state remains. |
| B-03 | Native decode cancellation | Run the deterministic native regression `cancellation_keeps_decoder_permit_until_blocking_worker_exits` in the Tauri test suite. The test holds a blocking worker after start, cancels the awaiting task, then releases the worker. | The awaiting command reports cancellation while the worker remains alive and bounded; the worker exits only after release. The decoder permit remains held until worker exit. This proves native cancellation/worker-lifecycle behavior; real-device cancellation remains NOT RUN. |
| B-04 | Cancel and immediate retry | Run the same deterministic native regression `cancellation_keeps_decoder_permit_until_blocking_worker_exits`; it attempts a second worker while the first is held, then retries after release. | The retry is rejected with the bounded-decoder busy response while the first worker holds the permit. After the first worker exits and releases the permit, a subsequent retry succeeds. The test confirms no two decoder workers overlap; real-device rapid retry remains NOT RUN. |
| B-05 | Malformed and empty files | Upload malformed PDF bytes and an empty file. | Clear validation error; no hang, crash, or stale busy indicator. |
| B-06 | Encrypted PDF | Upload a password-protected PDF. | Unsupported/password-required behavior is explicit; no false successful extraction. |
| B-07 | Size limit | Test just below and above the documented upload limit. | Below-limit behavior is bounded; above-limit input is rejected before excessive allocation. |
| B-08 | Repeated task IDs | Exercise duplicate active task IDs through automated/native tests. | Duplicate registration is rejected; registry cleanup permits a later valid retry. |
| B-09 | Cancellation cleanup | Cancel, wait for worker completion, then run a new PDF task. | No stale cancellation token or registry entry affects the new task. |

**Important limitation:** Tokio `spawn_blocking` work that has already started cannot be forcibly aborted. Cancellation can stop awaiting the result, but the bounded worker may finish in the background. B-03/B-04 must verify the real resource and retry behavior rather than assuming immediate CPU termination.

## C. Image inspection and screenshot capability boundary

**Current implementation boundary:** the desktop build supports metadata-only inspection of user-selected PNG, JPEG, GIF, WebP and BMP files, with a 20 MiB input limit. It does **not** implement screen capture, screenshot-to-chat attachment, pixel decoding, OCR or vision inference. Do not mark screenshot-to-chat or capture-permission cases PASS until that separate feature exists.

| ID | Scenario | Procedure | Expected result |
|---|---|---|---|
| C-01 | Image metadata inspection | Select a synthetic supported image under 20 MiB using the image inspector. | Header metadata is returned; no image preview, OCR, chat attachment or model inference is claimed. |
| C-02 | Oversized image | Select a synthetic image larger than 20 MiB. | Clear size rejection; no crash, hang or stale image result. |
| C-03 | Malformed or truncated image | Select synthetic invalid image bytes with a supported extension. | Explicit validation error; no crash or unbounded allocation. |
| C-04 | Same-file retry | Select the same image again after a failed validation. | The same file can be selected again; old error/result state is cleared appropriately. |
| C-05 | Screenshot-to-chat capability | Look for a screen-capture or chat-image attachment path in the current build. | Mark **NOT IMPLEMENTED / BLOCKED**. The current UI has no screenshot capture or chat attachment path; do not report this as a passing test or imply the metadata inspector sends images to a model. |
| C-06 | Capture permission flow | Attempt to exercise OS screen-capture permission prompts. | Mark **NOT APPLICABLE / BLOCKED** until screen capture is implemented; no capture permission flow exists in the current build. |
| C-07 | Privacy check | Inspect diagnostic output for image inspection. | No image bytes, document contents, secrets or private paths are logged by default. |

## D. Navigation, sublinks, and UI freeze recovery

**Mobile scope:** D-06 is a responsive/narrow-viewport check for the current desktop UI only. It is not evidence of a native iOS/Android application or a supported mobile installation target.

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
| E-05 | Cloud consent boundary | Use a harmless synthetic prompt and a controlled, non-production HTTPS test endpoint. With cloud approval disabled, attempt the cloud path. Require OS-level outbound connection monitoring (for example, OS firewall/connection telemetry or packet capture) for the negative case; endpoint access logs alone are insufficient because they cannot detect connections to other hosts. Separately identify loopback traffic to Ollama (127.0.0.1:11434). Use a harmless positive control to confirm the observer sees the controlled endpoint connection. If OS-level outbound monitoring or a controlled endpoint is unavailable, mark BLOCKED. | The unapproved cloud request is rejected before any external connection; OS-level monitoring shows no external connection attributable to the attempt. Local Ollama loopback traffic is distinguished from external egress. An endpoint log with no request, by itself, is not PASS. |

## F. Cross-platform execution record

Run applicable cases on each claimed platform and minimum supported OS baseline. A successful hosted build is not an install/launch result.

| Platform | OS version/build | Architecture | Installer | Cases run | Result | Evidence reference |
|---|---|---|---|---|---|---|
| Windows |  |  | MSI / NSIS |  | NOT RUN |  |
| macOS |  |  | DMG |  | NOT RUN |  |
| Linux |  |  | DEB / AppImage |  | NOT RUN |  |
| Mobile web / narrow viewport | Browser + OS |  | N/A |  | NOT RUN |  |

## Release gate

Do not mark this matrix complete until the test record is filled by a human tester on actual devices. Before broader distribution, every applicable release-critical case for data integrity, privacy, cancellation, application stability, screenshot/OS permissions, cloud egress, and filesystem boundaries must be marked PASS; BLOCKED or NOT RUN is insufficient. Any failure in these areas blocks distribution until fixed and retested. Platform code signing and notarization must be added and tested before broad public distribution. Unsigned CI preview installers may be used only for supervised preview testing and must remain clearly labelled as previews.
