# Core protection and release integrity

**Status: implementation policy and maintainer checklist. This document does not mean repository rules or signing infrastructure have already been configured.**

## Protected source

- Keep `main` as the only release integration branch.
- Require pull requests for changes to `main`; prohibit force pushes and branch deletion.
- Require the full CI suite and a resolved conversation before merge.
- Require an independent reviewer for security-sensitive changes when one is available. A contributor must not approve their own pull request.
- Protect `.github/workflows/`, `CODEOWNERS`, Tauri capabilities/configuration, native IPC, policy/router code, filesystem and PDF processing, credential/history encryption, updater configuration and cloud gateway code.
- Restrict GitHub Actions write permissions to the minimum required. Pin third-party actions to reviewed commit SHAs.

## Change alerts

The `security-change-watch.yml` workflow inspects changed paths on pull requests. It is a review alarm only: it does not determine whether a change is malicious, enforce approval, or protect the default branch by itself.

## Release integrity

Before any stable release:

1. Generate the release signing key outside GitHub source control; protect it in a dedicated secret-management system and limit access.
2. Configure the corresponding public key in the Tauri updater configuration. The current repository configuration has an empty updater public-key field; therefore signed updater delivery is **not ready**.
3. Require signature verification for updates and test rejection of unsigned, altered, wrong-key and rollback artifacts.
4. Produce SHA-256 checksums, an SBOM and build provenance for each release.
5. Publish source commit, build workflow/run, target OS/architecture, signing status and checksums with each artifact.
6. Keep unsigned preview installers clearly labelled as previews and do not present them as trusted stable releases.
7. Keep a recovery path for failed updates; never silently replace a file whose integrity check fails.

## Installed application integrity

A future integrity verifier should compare protected installed components against a signed release manifest. It should report the version, manifest identity, verification time and any changed/missing files. On a mismatch it should warn, block unsafe update/execution paths where practical, preserve diagnostic evidence and offer a verified reinstall/recovery path. Hashes without a trusted signature or protected reference manifest are not sufficient.

## Repository settings that must be configured by an administrator

The GitHub integration used for this change can edit repository files and create pull requests, but does not expose branch-protection/ruleset administration. An administrator must configure the `main` ruleset in GitHub:

- Require a pull request before merging.
- Require the CI and security checks appropriate to the repository.
- Block force pushes and branch deletion.
- Restrict direct pushes to `main`.
- Require Code Owner review only when a second eligible independent reviewer exists.
- Restrict who can change repository rules, Actions secrets, environments and release permissions.

Do not claim these settings are active until their GitHub configuration has been inspected.

## Release gates

The desktop prototype remains pre-alpha. CI success is not physical-device testing, a penetration test, proof of OS-wide isolation, or proof that an installed binary is untampered. Do not process sensitive production data or enable broad system permissions until the documented release gates are met.
