# Security Policy

## Status

UJANI Public AI is pre-alpha. Do not use it to process sensitive production data or grant it broad system permissions.

## Reporting a vulnerability

Please do not publish exploitable details in a public issue. Use GitHub's private vulnerability reporting feature if enabled for this repository, or contact the maintainers through a verified private channel listed on the repository profile.

Include:
- affected commit/version and operating system;
- concise impact and reproduction steps;
- proof-of-concept that avoids accessing real user data;
- suggested mitigation, if known.

## Maintainer response and disclosure process

The project is intended to remain freely usable by ordinary users. Security maintenance is therefore part of the upstream project, not a paid prerequisite for access to the core application.

For each credible report, maintainers should:
1. Acknowledge receipt when reasonably possible, without promising a fixed response-time SLA.
2. Reproduce and assess the issue, affected versions/platforms, exploitability and user impact.
3. Coordinate a fix and disclosure timeline with the reporter; avoid publishing exploit details before users have a reasonable opportunity to update.
4. Publish a security advisory and patched release when disclosure is appropriate, including affected versions, mitigations and upgrade guidance.
5. Add a regression test or other durable check where practical, and review whether the issue affects both Public AI and shared/Enterprise components.

The pre-alpha project may not yet have a staffed security-response team or guaranteed support window. Users and downstream distributors should not assume that every historical build receives backports. Companies may independently provide paid maintenance, patched distributions, support or response services, but those services must not be represented as a condition for obtaining the freely distributed upstream core.

Release maintainers should protect signing keys, publish checksums and provenance where available, and never describe an unsigned preview artifact as a verified signed release.

## Automated security checks

- GitHub Actions CI runs Rust tests, native Tauri checks on Windows/macOS/Linux, and the React TypeScript build.
- CodeQL scans Rust and JavaScript/TypeScript on pushes, pull requests, and a weekly schedule.
- Dependabot checks Rust dependencies and GitHub Actions for updates weekly. Review and test dependency update pull requests before merging.

## Security requirements

- No arbitrary model-generated shell execution.
- Allow-listed tools with strict input/output schemas.
- Least privilege and sandboxing for every adapter.
- Explicit approval for cloud data transfer and consequential actions.
- Local-only mode must block outbound task payloads.
- Secrets must not be written to logs or prompts.
- Signed releases and dependency provenance are required before stable releases.
- Security issues must be assessed before release; do not promise that a feature is secure without evidence.

We will acknowledge reports as soon as reasonably possible and coordinate fixes and disclosure with reporters.
