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

## B. PDF upload, sidecar and cancellation

Use synthetic PDFs, including a valid small PDF, a valid near-limit PDF, an empty file, a malformed file, and an encrypted file. Record actual file sizes; do not use real personal documents.
**Build prerequisite:** these cases exercise the combined PDF sidecar and cancellation work in PR #117. Until that PR is merged into the tested build, mark the new cases **BLOCKED**. CI unit tests are not a substitute for installed-package tests.

| ID | Scenario | Procedure | Expected result |
|---|---|---|---|
| B-01 | Normal sidecar upload | Install the platform preview package; process a small synthetic valid PDF. | The packaged `ujani-pdf-worker` is found beside the app, starts successfully, and returns extracted text; the local model then summarises it. No document is uploaded to a cloud service. |
| B-02 | FileReader cancellation | Select a larger valid PDF and press Cancel while the file is being read. | The browser reader aborts; UI returns to idle; no native decoder or sidecar starts. |
| B-02a | Pre-registration cancellation handoff | Run native regression `cancellation_in_ipc_handoff_is_applied_on_registration`; verify it consumes the pending cancellation when the native task registers. | The task token is cancelled before expensive preparation begins and the pending entry is consumed. This deterministic test does not claim that a normal UI click can interleave between FileReader completion and the synchronous IPC invocation. |
| B-03 | Decoder worker lifecycle | Run `cancelled_pdf_worker_retains_decoder_permit_until_exit`. The test drops the async waiter while the blocking decoder worker is held, then releases that worker. | The decoder permit remains held until the blocking worker exits; a second decoder is rejected while the first is active. |
| B-04 | Sidecar parser cancellation | Start processing a synthetic PDF, then cancel during extraction on each installed OS build. | The parent drops/kills the sidecar child; UI returns to idle; no orphan worker remains. Record OS-specific behaviour and any failure to terminate. |
| B-05 | Malformed and empty files | Upload malformed PDF bytes and an empty file. | Clear validation error; no hang, crash, or stale busy indicator. |
| B-06 | Encrypted PDF | Upload a password-protected PDF. | Unsupported/password-required behaviour is explicit; no false successful extraction. |
| B-07 | Input and worker-output limits | First, upload files just below and above the 10 MiB application input limit and confirm the oversized upload is rejected. Separately, invoke the packaged `ujani-pdf-worker` directly with a deterministic synthetic PDF fixture whose extracted text exceeds 1 MiB. Capture stdout and the process exit status; require exit status 0, output byte length no greater than 1 MiB, valid UTF-8, and an exact match against the independently generated expected truncated fixture text (not merely a non-empty or size-bounded file). Record the fixture generator/version and expected-output digest. Do not exercise the worker output cap through the normal upload UI: that path currently rejects extracted text above its 256 KiB application limit first. | Oversized application input is rejected before excessive allocation. The direct worker run succeeds and emits exactly the expected UTF-8-safe truncated text within 1 MiB. Record both checks independently. |
| B-08 | Identifier and duplicate registration | Run `registration_preserves_resource_queue_identifier_validation`, `pre_registration_cancellation_is_bounded_and_pdf_only` and `duplicate_active_pdf_registration_is_rejected_and_fresh_id_allows_retry`. | Unsupported identifiers are rejected, pending cancellations remain bounded, and duplicate active registrations are rejected. |
| B-09 | Late cancellation and same-file retry | Run `late_cancellation_is_rejected_after_pdf_registration_finishes`. For the installed-app portion, use a test-only instrumented build or debugger/OS process tracing to capture timestamped evidence for the PDF worker's decoder-entry/start event, the cancel request, and worker exit/reap (include the task correlation ID and child PID, but no document content or private paths). Trigger Cancel only after decoder entry is observed; confirm cancellation precedes worker exit, then wait for exit/permit release and select the same synthetic file again. Capture the retry's successful completion. If decoder-entry and cancellation ordering cannot be evidenced, mark this case **BLOCKED**, not PASS. | A late cancel returns inactive for the finished ID and creates no stale pending state. Evidence proves cancellation occurred while decoding was active; the child exits and is reaped, the decoder permit is released only after exit, and the same-file retry completes successfully. |
| B-10 | Missing sidecar / missing OS control | In a disposable test build, temporarily remove the worker or test a platform where the required OS control is unavailable. | UJANI fails closed with a clear repair/unsupported message; it does not fall back to in-process parsing or cloud upload. Restore the worker before further tests. |

**Important limitations:** Tokio `spawn_blocking` work that has already started cannot be forcibly aborted; its decoder permit remains held until the worker exits. The PDF parser itself now runs in a separate child process with resource controls and kill-on-drop. Those controls differ by OS and are not a claim of complete production sandboxing: Windows Job Objects provide process/resource limits but do not by themselves deny network or file access. Scanned-PDF OCR remains unsupported. Mark every real-device case NOT RUN until it has actually been performed.
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
