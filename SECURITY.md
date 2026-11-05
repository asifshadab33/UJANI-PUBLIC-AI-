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
