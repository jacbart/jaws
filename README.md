# JAWS

Just A Working Secretsmanager

A CLI tool and library for managing secrets from multiple providers (AWS Secrets Manager, 1Password, Bitwarden, and local storage) with local version tracking.

## Features

- **Multi-provider support** - AWS Secrets Manager, 1Password, Bitwarden, and local "jaws" secrets
- **Git-like workflow** - `jaws pull`, `jaws push`, familiar commands
- **Local version tracking** - Full history of downloaded secrets with rollback support
- **Template injection** - Inject secrets into config files with `--inject`
- **Script-friendly** - Print secrets to stdout with `--print` for shell scripts
- **Encrypted export/import** - Archive secrets with passphrase or SSH key encryption
- **TUI picker** - Interactive fuzzy finder for secret selection
- **Library support** - Use as a Rust library in your own projects

## Installation

### Using Cargo

```bash
cargo install --path .
```

### Using Nix

```bash
# Build and run directly
nix run github:jacbart/jaws

# Install to profile
nix profile install github:jacbart/jaws

# Or add to your flake inputs
{
  inputs.jaws.url = "github:jacbart/jaws";
}
```

### Cross-Compiled Binaries

Pre-built binaries for multiple platforms can be built using the Nix flake:

```bash
# Build for specific target
nix build .#jaws-x86_64-linux      # Intel/AMD Linux
nix build .#jaws-aarch64-linux     # ARM64 Linux (AWS Graviton, etc.)
nix build .#jaws-x86_64-darwin     # Intel Mac
nix build .#jaws-aarch64-darwin    # Apple Silicon Mac

# Build native (optimal for current platform)
nix build .#default
```

## Quick Start

```bash
# Generate a config file (interactive mode discovers providers)
jaws config init

# Or generate a minimal template without prompts
jaws config init --minimal

# Pull secrets from your providers (opens TUI picker)
jaws pull

# Pull a specific secret
jaws pull aws://my-secret

# Edit and push changes back
jaws push

# View local version history
jaws history

# Rollback to a previous version
jaws rollback
```

## Commands

### Secret Operations

| Command                   | Description                                    |
| ------------------------- | ---------------------------------------------- |
| `jaws`                    | Open TUI to select and edit downloaded secrets |
| `jaws pull [SECRET]`      | Download secrets from providers                |
| `jaws pull -p SECRET`     | Print secret value to stdout (for scripts)     |
| `jaws pull -i TPL -o OUT` | Inject secrets into a template file            |
| `jaws push`               | Upload changed secrets to providers            |
| `jaws create [NAME]`      | Create a new secret (local or remote)          |
| `jaws delete [SECRET]`    | Delete a secret (prompts for scope)            |
| `jaws delete -s remote`   | Delete a secret from the remote provider only  |
| `jaws list`               | List all known secrets (one per line)          |
| `jaws sync`               | Refresh local cache of remote secrets          |

### History & Rollback

| Command                  | Description                                    |
| ------------------------ | ---------------------------------------------- |
| `jaws history`           | View local version history                     |
| `jaws history --remote`  | View remote provider version history           |
| `jaws rollback`          | Rollback to a previous local version           |
| `jaws rollback --remote` | Rollback to a previous version on the provider |
| `jaws log`               | Show operation log                             |

### Archive Operations

| Command            | Description                                    |
| ------------------ | ---------------------------------------------- |
| `jaws export`      | Export and encrypt secrets to a `.barrel` file |
| `jaws import FILE` | Import and decrypt a `.barrel` archive         |

### Maintenance

| Command                         | Description                                       |
| ------------------------------- | ------------------------------------------------- |
| `jaws clean`                    | Clear local cache and secrets                     |
| `jaws version`                  | Print version information                         |

### Configuration

| Command                         | Description                                       |
| ------------------------------- | ------------------------------------------------- |
| `jaws config`                   | Show current configuration and providers          |
| `jaws config init`              | Interactive config generation with auto-discovery |
| `jaws config init --minimal`    | Generate a minimal config template                |
| `jaws config get <key>`         | Get a specific config value                       |
| `jaws config set <key> <value>` | Set a config value                                |

## Configuration

JAWS uses a `jaws.kdl` config file ([KDL](https://kdl.dev/) format):

```kdl
// jaws configuration file
// cache_ttl is in seconds (default: 900 = 15 minutes)

defaults editor="nvim" secrets_path="./.secrets" cache_ttl=900 default_provider="jaws"

// AWS with auto-discovery of all profiles
provider "aws" kind="aws" {
    profile "all"
}

// Or a specific AWS profile
provider "aws-prod" kind="aws" {
    profile "production"
    region "us-east-1"
}

// 1Password with auto-discovery of all vaults
provider "op" kind="onepassword" {
    vault "all"
}

// Or a specific 1Password vault
provider "op-dev" kind="onepassword" {
    vault "abc123"
}

// Bitwarden Secrets Manager
provider "bw-myproject" kind="bw" {
    vault "project-uuid-here"
    organization "org-uuid-here"
    token-env "BWS_ACCESS_TOKEN"
}
```

### AWS Setup

Ensure your AWS credentials are configured in `~/.aws/credentials` and you have appropriate IAM permissions for Secrets Manager.

### 1Password Setup

Set the `OP_SERVICE_ACCOUNT_TOKEN` environment variable with your 1Password service account token.

### Bitwarden Setup

Set the `BWS_ACCESS_TOKEN` environment variable with your Bitwarden Secrets Manager access token.

## Usage Examples

### Scripting with `--print`

```bash
# Get a secret value for use in scripts
export DB_PASSWORD=$(jaws pull aws://prod/db-password -p)

# Use in a command
mysql -u admin -p$(jaws pull aws://mysql-pass -p) mydb
```

### Template Injection with `--inject`

Create a template file (e.g., `.env.tpl`):

```
DATABASE_URL=postgres://user:{{aws://db-password}}@localhost/mydb
API_KEY={{jaws://api-key || 'default_value' }}
```

Inject secrets:

```bash
# Output to stdout
jaws pull -i .env.tpl

# Output to file
jaws pull -i .env.tpl -o .env.prod
```

### Creating Secrets

You can create secrets locally or directly in a remote provider:

```bash
# Create a local secret (default provider)
jaws create my-local-secret

# Create a secret in AWS
jaws create aws://my-new-secret

# Create from a file
jaws create my-cert -f ./certificate.pem
```

### Local Management

List and manage secrets:

```bash
# List all secrets including local ones
jaws list --provider jaws
```

### Clean Up

```bash
# See what would be deleted
jaws clean --dry-run

# Delete remote caches but keep local jaws secrets
jaws clean --keep-local

# Full cleanup (with confirmation for local secrets)
jaws clean
```

## Export/Import

Securely archive your secrets directory:

```bash
# Export with passphrase
jaws export

# Export with SSH public key
jaws export -K ~/.ssh/id_ed25519.pub

# Export to specific file
jaws export -o backup.barrel

# Import with passphrase
jaws import ./jaws.barrel

# Import with SSH private key
jaws import ./jaws.barrel -K ~/.ssh/id_ed25519
```

## Library Usage

JAWS can be used as a Rust library:

```rust
use jaws::{Config, detect_providers};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let providers = detect_providers(&config).await?;

    for provider in &providers {
        println!("Provider: {} ({})", provider.id(), provider.kind());
    }

    Ok(())
}
```

## Project Structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library exports
├── archive.rs       # Encryption/archiving
├── cli/             # CLI definitions
├── commands/        # Command handlers
├── config/          # Configuration
├── db/              # SQLite database
├── secrets/         # Secret providers
│   └── providers/   # AWS, 1Password, Bitwarden, local
└── utils/           # Utilities
```

## Development

### Dev Shell

Enter the development environment with all dependencies:

```bash
nix develop
```

### Building from Source

```bash
# Native build
cargo build --release

# Or using Nix
nix build
```

### Cross-Compilation

The project supports cross-compilation using `cargo-zigbuild` for Linux targets and native cargo for Darwin targets:

```bash
# Build all cross-compiled binaries
./scripts/release.sh --build-only

# Full release process (updates version, builds all targets, creates tag)
./scripts/release.sh 1.3.0
```

Release binaries are output to `dist/`:

```
dist/
├── jaws-x86_64-linux.tar.gz
├── jaws-aarch64-linux.tar.gz
├── jaws-x86_64-darwin.tar.gz
└── jaws-aarch64-darwin.tar.gz
```

## License

MPL 2.0
