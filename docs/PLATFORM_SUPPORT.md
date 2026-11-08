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

The release workflow builds a draft release, verifies Tauri updater signatures, checksums, SBOM and provenance, and only then publishes. It now explicitly requires Windows publisher-certificate credentials on Windows runners and Apple Developer ID signing/notarization credentials on macOS runners. The final `publish-release` job targets the `production-release` GitHub Environment. **Repository administrators must configure required reviewers and disable self-review for that environment in Settings → Environments; without that repository-side protection, the environment name alone is not an approval gate.** The workflow still does not run minimum-version install/launch/upgrade/uninstall tests. The human approver must review completed platform test evidence before approving publication. Do not create production tags until those tests and signing prerequisites are ready.

Unsigned CI preview artifacts are for controlled evaluation only and must be labelled as such. A green cross-platform build matrix is not equivalent to OS compatibility certification.

## Maintenance

When changing Tauri, WebView2, WebKitGTK, Rust or native dependencies, rerun the minimum-version matrix and update this document. Record exact OS image/version, architecture, installer type, app version and test outcome in release evidence.
