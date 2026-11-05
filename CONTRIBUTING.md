# Contributing

Thanks for helping build UJANI Public AI.

## Before opening a pull request

1. Read [README.md](README.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [SECURITY.md](SECURITY.md).
2. Open or find an issue describing the problem or proposed change.
3. Keep pull requests focused and explain user impact, security implications and testing.

## Development

The initial core is a Rust workspace:

```sh
cargo fmt --all -- --check
cargo test --workspace
```

Do not add a runtime dependency without explaining why it is needed, its licence and maintenance status.

## Contribution expectations

- Never introduce silent device-resource use or undisclosed network transfers.
- Do not add unrestricted shell execution.
- Keep tool interfaces typed and least-privileged.
- Add tests for policy decisions and failure cases.
- Document platform limitations rather than claiming universal support.
- Do not commit credentials, private datasets, model weights or user files.

By submitting a contribution, you agree it is provided under the repository's GPL-3.0 licensing terms.
