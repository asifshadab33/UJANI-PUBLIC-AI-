# UJANI Public AI desktop — platform support policy

**Status: proposed support baseline; not a compatibility certification.** The project remains pre-alpha until the minimum-version install/launch tests below pass on the named systems.

## Installer formats currently configured

| OS | Configured artifacts | CI build host |
|---|---|---|
| Windows | MSI, NSIS installer (EXE) | `windows-latest` |
| macOS | DMG, APP bundle | `macos-latest` |
| Linux | DEB, AppImage | `ubuntu-24.04` |

A successful build on a current CI runner proves that the package can be produced; it does not prove compatibility with every older OS release or Linux distribution.

## Proposed initial support baseline

| Platform | Initial target | Runtime requirements / caveats |
|---|---|---|
| Windows x64 | Windows 10 1803+ and Windows 11 | Microsoft Edge WebView2 Runtime. Verify installer bootstrap/fallback when WebView2 is absent or outdated. |
| macOS | macOS 10.15+; Intel and Apple Silicon only after separate smoke tests | Native WKWebView. Set and test an explicit minimum OS version before claiming support. |
| Linux x64 | Ubuntu 24.04 initially; expand only to distributions that pass tests | Compatible GTK and WebKitGTK 4.1 system libraries. DEB and AppImage do not imply universal distro compatibility. |

These are conservative project targets, not a statement that all versions have already been tested. Do not advertise Windows 7/8/8.1, all macOS releases, all Linux distributions, ARM64 Windows, or Linux ARM64 without dedicated build and runtime validation.

## User requirements vs developer requirements

End users installing a finished package should not need Node.js, npm, Rust or C++ build tools. Those are build/development prerequisites.

The desktop shell and local inference are separate resource profiles. Do not publish a minimum RAM/CPU/disk number until measured on a supported baseline. Running a local model through Ollama requires additional RAM, storage and compatible model/runtime resources; the desktop application alone must not imply that a local model is bundled.

## Release gates

Before a public production release:

1. Run clean install, launch, upgrade, uninstall and interrupted-install recovery tests on the oldest claimed OS version.
2. Test Windows MSI and NSIS independently, including WebView2 present, missing and outdated scenarios.
3. Test macOS Intel and Apple Silicon separately; configure signing and notarization.
4. Test Ubuntu 24.04 DEB and AppImage on clean images; record missing-library diagnostics. Add another distro only after repeatable smoke tests.
5. Verify updater signatures, release manifest, SHA-256 checksums and rollback/recovery behaviour.
6. Sign Windows binaries and notarize macOS packages. Tauri updater signatures are not a substitute for OS publisher signing.
7. Attach build provenance and an SBOM; publish only after all required platform jobs pass.

### What the current release workflow enforces

The release workflow verifies Tauri updater signatures before publication. On Windows it imports the PFX certificate, configures Tauri's Authenticode thumbprint, SHA-256 digest and timestamp URL, then verifies the MSI and NSIS EXE signatures against the imported certificate before the build job succeeds. Configure the repository variable `WINDOWS_TIMESTAMP_URL` to the timestamp service specified by the certificate issuer. Required Windows secrets are `WINDOWS_CERTIFICATE` (base64 PFX), `WINDOWS_CERTIFICATE_PASSWORD`; required macOS secrets are `APPLE_CERTIFICATE` (base64 .p12), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_KEYCHAIN_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` (app-specific password), and `APPLE_TEAM_ID`. Keep all certificate/private-key material in GitHub secrets only; never commit it. On macOS, the workflow imports the base64 .p12 into a temporary keychain, verifies the configured Developer ID Application identity, passes the signing/notarization credentials to the Tauri build action, verifies the app signature with codesign and Gatekeeper, and validates the stapled DMG ticket. The Apple signing identity must be a Developer ID Application certificate; notarization requires an eligible paid Apple Developer membership. The `prepare-release-evidence` job runs after all platform build jobs but before the `production-release` approval gate. It downloads the still-draft release, verifies Tauri updater signatures, generates SHA-256 checksums and runs `sha256sum -c SHA256SUMS` against the release-assets directory, generates an SPDX JSON SBOM, checks its basic JSON structure, creates a GitHub build-provenance attestation, and uploads the installer-size report, checksum manifest and SBOM to the draft release. This makes the evidence available for inspection before approval. The checksum check detects local artifact/manifest mismatch but does not establish authenticity; the workflow does not independently verify the provenance attestation or validate the SBOM against a complete SPDX schema. Reviewers must verify the provenance attestation, inspect the SBOM, and check macOS `codesign`/notarization results before approving publication. The final `publish-release` job targets the `production-release` GitHub Environment and verifies that the evidence assets are present before changing the release from draft to published. **Repository administrators must configure required reviewers and disable self-review for that environment in Settings → Environments; without that repository-side protection, the environment name alone is not an approval gate.** The workflow does not ingest or verify a manual minimum-OS install/launch/upgrade/uninstall test record. The human approver must review the completed test matrix and all manual verification evidence before approving publication. Keep the release in draft and do not approve `production-release` if any evidence is missing or incomplete.

Unsigned CI preview artifacts are for controlled evaluation only and must be labelled as such. A green cross-platform build matrix is not equivalent to OS compatibility certification. A valid Windows Authenticode signature verifies publisher identity and artifact integrity; it does not guarantee SmartScreen reputation or suppress first-download warnings.

## Current verification record — 2026-11-08

The platform matrix remains a **proposed baseline**, not a certified support list. The human device-test record is maintained in [`docs/PUBLIC_AI_MANUAL_SMOKE_TEST_MATRIX.md`](PUBLIC_AI_MANUAL_SMOKE_TEST_MATRIX.md); its Windows, macOS, Linux and mobile/narrow-viewport rows remain **NOT RUN** until a tester records the exact OS build, architecture, installer, cases and evidence. Hosted CI and unsigned preview artifacts do not satisfy these install/launch gates. The release workflow does not ingest manual test results automatically: keep releases in draft and do not approve the `production-release` environment until the applicable manual evidence, Windows Authenticode verification, and Apple signature/notarization evidence have been independently reviewed. If required environment reviewers are not configured, do not treat the environment name as a publication safeguard.

## Maintenance

When changing Tauri, WebView2, WebKitGTK, Rust or native dependencies, rerun the minimum-version matrix and update this document. Record exact OS image/version, architecture, installer type, app version and test outcome in release evidence.


## Legal and customer-facing status

This matrix is a proposed technical support policy, not a compatibility certification, warranty, SLA or promise of support. A package build does not prove that a customer device or operating-system version is supported. Users must verify the exact environment and retain independent backups. Read [the UJANI Public AI legal and operational notice](LEGAL-DISCLAIMER.md) and the applicable [GPLv3 licence](../LICENSE). This notice is informational and does not exclude any rights or liabilities that applicable law does not permit to be excluded.
