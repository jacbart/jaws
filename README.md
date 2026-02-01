# JAWS

Just A Working Secretsmanager

A CLI tool and library for managing secrets from multiple providers (AWS Secrets Manager, 1Password, and local storage) with local version tracking.

## Features

- **Multi-provider support** - AWS Secrets Manager, 1Password, and local "jaws" secrets
- **Git-like workflow** - `jaws pull`, `jaws push`, familiar commands
- **Local version tracking** - Full history of downloaded secrets with rollback support
- **Template injection** - Inject secrets into config files with `--inject`
- **Script-friendly** - Print secrets to stdout with `--print` for shell scripts
- **Encrypted export/import** - Archive secrets with passphrase or SSH key encryption
- **TUI picker** - Interactive fuzzy finder for secret selection
- **Library support** - Use as a Rust library in your own projects

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Generate a config file (interactive mode discovers providers)
jaws config generate --interactive

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

### Local Operations

| Command | Description |
|---------|-------------|
| `jaws` | Open TUI to select and edit downloaded secrets |
| `jaws pull [SECRET]` | Download secrets from providers |
| `jaws pull -p SECRET` | Print secret value to stdout (for scripts) |
| `jaws pull -i TPL -o OUT` | Inject secrets into a template file |
| `jaws push` | Upload changed secrets to providers |
| `jaws create NAME` | Create a new local secret |
| `jaws delete` | Delete a local secret and all its versions |
| `jaws list` | List all known secrets (one per line) |
| `jaws history` | View local version history |
| `jaws rollback` | Rollback to a previous local version |
| `jaws log` | Show operation log |
| `jaws clean` | Clear local cache and secrets |

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
| `jaws import FILE` | Import and decrypt a `.barrel` archive |

### Configuration

| Command | Description |
|---------|-------------|
| `jaws config generate` | Generate a new config file |
| `jaws config generate -i` | Interactive config generation with provider discovery |
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
    default_provider "jaws"  // Optional: allows omitting provider prefix
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
API_KEY={{jaws://api-key}}
```

Inject secrets:
```bash
# Output to stdout
jaws pull -i .env.tpl

# Output to file
jaws pull -i .env.tpl -o .env.prod
```

### Local Secrets

Create and manage secrets that stay local (not synced to any provider):

```bash
# Create a local secret
jaws create my-local-secret

# Create from a file
jaws create my-cert -f ./certificate.pem

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
│   └── providers/   # AWS, 1Password, local
└── utils/           # Utilities
```

## Roadmap

- [ ] `jaws serve` - Self-hostable secrets management API
- [ ] Remote version history from providers
- [ ] Hardware key encryption support
- [ ] Secret rotation scheduling

## License

MIT
