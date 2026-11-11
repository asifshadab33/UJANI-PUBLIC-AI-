# UJANI Public AI — Android alpha tester pack

**Status:** internal smoke-test build only; not a production release.  
**Build:** Android ARM64 universal debug APK  
**Workflow run:** [UJANI Android native compatibility check #42919034089](https://github.com/asifshadab33/UJANI-PUBLIC-AI-/actions/runs/42919034089)  
**Artifact:** `ujani-android-arm64-debug-apk` (GitHub Actions artifact; expires 2026-11-24)  
**Build commit:** `f56cf0fcc2046ff43712276dcc339804535e96ac`  
**Build branch:** `hardening/ollama-pull-progress-bounds-20261110`

> **Provenance warning:** this build is from a hardening branch that has diverged from `main` (five commits ahead and three behind at the time this pack was prepared). It is suitable only for controlled smoke testing and feedback. Do not describe it as a build of the current main branch, a production release, or a security-certified build. Rebuild from the final release commit before wider distribution.

## Who should test

Use a small group of Android ARM64 testers with different device models, Android versions, storage capacity and network conditions. Test only on devices whose owners have agreed to participate. Do not use personal, confidential, customer or regulated documents.

## Install

1. Open the workflow link above while signed in to GitHub with access to the repository.
2. In the run's Artifacts section, download `ujani-android-arm64-debug-apk` and extract the ZIP.
3. Confirm the APK filename and SHA-256 against the `android-apk.sha256` file included in the artifact, if present.
4. Install the universal debug APK on an ARM64 Android test device. Android may require a one-time approval to install an app from the browser/file manager; only proceed if the tester understands this is a debug build from the repository owner.
5. If Android blocks installation, reports an incompatible package, or the app fails to launch, stop and record the exact device model, Android version and error. Do not bypass device security controls.

## Smoke-test checklist

For each test, record **Pass / Fail / Not tested**, device model, Android version, app build commit, steps, expected result, actual result, and attach a screenshot or screen recording only after removing personal information.

- [ ] APK installs and launches; app remains responsive for 10 minutes.
- [ ] Navigate every visible primary screen and sublink; record freezes, blank screens and crashes.
- [ ] Use Android Back navigation; confirm it does not unexpectedly exit or trap the app.
- [ ] Rotate the device and background/foreground the app; note lost state or crashes.
- [ ] Deny optional permissions; confirm the app explains the limitation and remains usable where possible.
- [ ] Grant only the permissions needed for a test; verify permission prompts are understandable.
- [ ] Test with Wi-Fi on, Wi-Fi off and intermittent connectivity; confirm no unexplained network activity or UI hang.
- [ ] If local Ollama/model features are available in this build, test only against a local test instance and a non-sensitive sample prompt. Confirm model/runtime status is visible and errors are understandable.
- [ ] Try a small, non-sensitive sample text/PDF only if the corresponding feature is present. Record size, processing time and any freeze; do not use confidential files.
- [ ] Cancel any long-running operation; verify the UI returns to a usable state.
- [ ] Force-stop and relaunch; note whether expected local settings/history persist.
- [ ] Check that the app does not claim an operation succeeded when it failed or was cancelled.

## Do not test / do not assume

- Do not upload confidential or personal data.
- Do not connect production accounts, company networks or production Ollama endpoints.
- Do not enable remote execution, shell access, unattended background compute or firewall/security changes.
- Do not treat this debug APK as signed production software or proof of privacy/security certification.
- Do not assume every feature shown in the product roadmap is implemented in this build.

## Feedback format

Send one report per issue:

```text
Title:
Device / Android version:
Build commit:
Steps to reproduce:
Expected:
Actual:
Frequency (once / sometimes / every time):
Impact (blocker / major / minor):
Screenshot or sanitized log:
```

Report crashes, data loss, permission surprises, unexpected network activity and hangs as blockers. Include the workflow run link and do not publish sensitive logs publicly.

## Release gate

This alpha is for usability and compatibility feedback only. Wider tester distribution waits for a fresh artifact built from the selected release commit, verified artifact provenance and hashes, resolved dependency audit findings or documented risk disposition, and owner-approved release notes.
