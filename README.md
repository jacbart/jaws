# JAWS

Just A Working Secretsmanager

A CLI tool for managing secrets from multiple providers (AWS Secrets Manager, 1Password) with local version control powered by [Jujutsu (jj)](https://github.com/martinvonz/jj).

## Features

- **Multi-provider support** - AWS Secrets Manager and 1Password
- **Git-like workflow** - `jaws pull`, `jaws push`, familiar commands
- **Local version control** - Full history tracking with jj (auto-initialized)
- **Encrypted export/import** - Archive secrets with passphrase or SSH key encryption
- **TUI picker** - Interactive fuzzy finder for secret selection

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Generate a config file (interactive mode discovers providers)
jaws config generate --interactive

# Pull secrets from your providers
jaws pull

# Edit and push changes back
jaws push

# View local version history
jaws history

# Restore a previous version
jaws restore
```

## Commands

### Local Operations

| Command | Description |
|---------|-------------|
| `jaws` | Open TUI to select and edit downloaded secrets |
| `jaws pull` | Download secrets from providers |
| `jaws push` | Upload changed secrets to providers |
| `jaws delete` | Delete a local secret and all its versions |
| `jaws history` | View local version history |
| `jaws restore` | Restore a previous local version |
| `jaws undo` | Undo the last VCS operation |
| `jaws log` | Show VCS operation log |
| `jaws diff` | Show diff between operations |

### Remote/Provider Operations

| Command | Description |
|---------|-------------|
| `jaws sync` | Refresh local cache of remote secrets |
| `jaws remote delete` | Delete a secret from the provider |
| `jaws remote rollback` | Rollback to a previous version on the provider |
| `jaws remote history` | View provider version history (not yet implemented) |

### Archive Operations

| Command | Description |
|---------|-------------|
| `jaws export` | Export and encrypt secrets to a `.barrel` file |
| `jaws import` | Import and decrypt a `.barrel` archive |

### Configuration

| Command | Description |
|---------|-------------|
| `jaws config generate` | Generate a new config file |
| `jaws config list` | List all configuration settings |
| `jaws config get <key>` | Get a specific config value |
| `jaws config set <key> <value>` | Set a config value |
| `jaws config providers` | List configured providers |

## Configuration

JAWS uses a `jaws.kdl` config file:

```kdl
defaults {
    editor "nvim"
    secrets_path "./.secrets"
    cache_ttl 900
}

providers {
    // AWS with auto-discovery of all profiles
    aws id="aws" profile="all"
    
    // Or specific AWS profile
    aws id="aws-prod" profile="production" region="us-east-1"
    
    // 1Password with auto-discovery of all vaults
    onepassword id="op" vault="all"
    
    // Or specific 1Password vault
    onepassword id="op-dev" vault="abc123"
}
```

### AWS Setup

Ensure your AWS credentials are configured in `~/.aws/credentials` and you have appropriate IAM permissions for Secrets Manager.

### 1Password Setup

Set the `OP_SERVICE_ACCOUNT_TOKEN` environment variable with your 1Password service account token.

## Version Control

JAWS automatically tracks all secret changes using [Jujutsu (jj)](https://github.com/martinvonz/jj), a Git-compatible VCS. This happens transparently - no setup required.

- Every `pull`, `push`, and `restore` creates a commit
- Use `jaws log` to view operation history
- Use `jaws undo` to revert the last operation
- Use `jaws history <secret>` to see version history for a specific secret

## Export/Import

Securely archive your secrets directory:

```bash
# Export with passphrase
jaws export

# Export with SSH public key
jaws export -K ~/.ssh/id_ed25519.pub

# Import
jaws import ./jaws.barrel
jaws import ./jaws.barrel -K ~/.ssh/id_ed25519
```

## Roadmap

- [ ] `jaws serve` - Self-hostable secrets management API
- [ ] Remote version history from providers
- [ ] Hardware key encryption support

## License

MIT
