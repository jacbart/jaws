# JAWS

Just A Working Secretsmanager

A CLI tool and library for managing secrets from multiple providers (AWS Secrets Manager, GCP Secret Manager, 1Password, Bitwarden, and local storage) with local version tracking and secure remote secret sharing.

![Version](https://img.shields.io/badge/version-1.5.1-blue)
![License](https://img.shields.io/badge/license-MPL--2.0-blue)

## Demo

### Creating Secrets

![jaws create demo](assets/demo-create.gif)

### Pulling & Template Injection

![jaws pull demo](assets/demo-pull.gif)

### Operation Log & Version History

![jaws log demo](assets/demo-log.gif)

## Features

- **Multi-provider support** — AWS, GCP, 1Password, Bitwarden, and local storage
- **Remote secret sharing** — `jaws serve` and `jaws connect` with mTLS
- **Git-like workflow** — `pull`, `push`, `rollback`
- **Local version tracking** — full history with rollback support
- **Template injection** — inject secrets into config files
- **Script-friendly** — `--print` for shell integration
- **Encrypted export/import** — passphrase or SSH key encryption
- **TUI picker** — interactive fuzzy finder
- **Library support** — use as a Rust library

## Quick Install

```bash
# Nix (recommended)
nix run github:jacbart/jaws

# Or install to profile
nix profile install github:jacbart/jaws

# Cargo
cargo install --path .
```

Cross-compiled binaries: see [Nix docs](docs/nix.md#cross-compiled-binaries).

## Quick Start

```bash
# Initialize configuration
jaws config init

# Pull secrets (interactive picker) — materializes them as plain files
# under .secrets/secrets/{provider_id}/{name}
jaws pull

# Edit any secret with your favourite editor — it's just a file.
$EDITOR .secrets/secrets/jaws/my-secret

# Create a new local secret by writing a file:
echo "value" > .secrets/secrets/jaws/another-secret

# Record local edits in the DB + .versions/ archive (no remote upload):
jaws save

# Inspect what's new / modified / unpushed:
jaws status

# Upload local edits to their remote providers:
jaws push
```

See [Getting Started](docs/getting-started.md) for the full walkthrough.

## Documentation

- [Getting Started](docs/getting-started.md) — first-time setup, secrets workflow
- [Configuration](docs/configuration.md) — KDL format, all provider types
- [Commands](docs/commands.md) — complete CLI reference (auto-generated)
- [Remote Sharing](docs/remote-sharing.md) — `jaws serve` and `jaws connect`
- [Security](docs/security.md) — threat model, mTLS, best practices
- [Nix](docs/nix.md) — flake, overlay, Home Manager module
- [Development](docs/development.md) — building, testing, cross-compilation
- [Architecture](docs/architecture.md) — codebase structure for contributors
- [Troubleshooting](docs/TROUBLESHOOTING.md) — common issues and fixes

## License

[MPL 2.0](LICENSE)
