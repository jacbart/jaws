# Getting Started

## Table of Contents

- [Installation](#installation)
- [Initialize Configuration](#initialize-configuration)
- [Your First Secret](#your-first-secret)
- [Pull and Push Workflow](#pull-and-push-workflow)
- [Scripting with Secrets](#scripting-with-secrets)
- [Template Injection](#template-injection)
- [Remote Secret Sharing](#remote-secret-sharing)
- [Export and Import](#export-and-import)
- [Next Steps](#next-steps)

---

## Installation

### Using Nix (recommended)

```bash
# Run without installing
nix run github:jacbart/jaws

# Install to profile
nix profile install github:jacbart/jaws
```

### Using Cargo

```bash
cargo install --path .
```

### Cross-Compiled Binaries

See [Nix documentation](nix.md#cross-compiled-binaries) for pre-built targets.

---

## Initialize Configuration

JAWS uses a `jaws.kdl` configuration file. Generate one interactively:

```bash
jaws config init
```

This walks you through provider discovery (AWS, GCP, 1Password, Bitwarden).

For a minimal template without prompts:

```bash
jaws config init --minimal
```

See [Configuration](configuration.md) for the full KDL reference.

---

## Your First Secret

Create a local secret (stored in `~/.jaws/secrets/` by default):

```bash
jaws create my-first-secret
```

This opens your configured editor. Save and exit to store the secret.

---

## Pull and Push Workflow

Download secrets from providers:

```bash
# Interactive picker (all providers)
jaws pull

# Pull a specific secret
jaws pull aws://prod/db-password
jaws pull gcp://my-project/api-key
```

Edit downloaded secrets:

```bash
# Open TUI to select and edit
jaws

# Or edit a specific secret
jaws push my-secret
```

Push changes back to providers:

```bash
jaws push
```

---

## Scripting with Secrets

Print secret values to stdout for use in scripts:

```bash
export DB_PASSWORD=$(jaws pull aws://prod/db-password -p)
mysql -u admin -p"$DB_PASSWORD" mydb
```

The `-p` flag bypasses the editor and prints the raw value.

---

## Template Injection

Create a template file (e.g., `.env.tpl`):

```
DATABASE_URL=postgres://user:{{aws://db-password}}@localhost/mydb
API_KEY={{gcp://api-key}}
FALLBACK={{jaws://local-secret || 'default_value'}}
```

Inject secrets:

```bash
# Output to stdout
jaws pull -i .env.tpl

# Output to file
jaws pull -i .env.tpl -o .env.prod
```

---

## Remote Secret Sharing

Share secrets across machines with `jaws serve` and `jaws connect`.

**On the server:**

```bash
jaws serve -n myserver
# Token written to: ~/.jaws/server/enrollment.token
```

**On the client:**

```bash
jaws connect https://server-ip:9643 --token $(cat ~/.jaws/server/enrollment.token)
jaws sync
jaws pull myserver/aws-prod://db-password -p
```

See [Remote Sharing](remote-sharing.md) for full details on mTLS, enrollment, and fingerprint verification.

---

## Export and Import

Archive your secrets directory securely:

```bash
# Export with passphrase encryption
jaws export -o backup.barrel

# Export with SSH public key
jaws export -K ~/.ssh/id_ed25519.pub -o backup.barrel

# Import
jaws import backup.barrel
```

---

## Next Steps

- [Configuration](configuration.md) — full KDL reference, all provider types
- [Commands](commands.md) — complete CLI command reference
- [Security](security.md) — threat model, mTLS details, best practices
- [Troubleshooting](TROUBLESHOOTING.md) — common issues and fixes
