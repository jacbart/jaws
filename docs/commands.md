# Command Reference

<!-- AUTO-GENERATED: Run `scripts/generate-docs.sh` to update -->

This page documents all `jaws` commands. The help text is extracted directly
from the CLI to ensure accuracy.

## Table of Contents

- [Top-level Options](#top-level-options)
- [Secret Operations](#secret-operations)
  - [`jaws pull`](#jaws-pull)
  - [`jaws push`](#jaws-push)
  - [`jaws create`](#jaws-create)
  - [`jaws delete`](#jaws-delete)
  - [`jaws list`](#jaws-list)
  - [`jaws sync`](#jaws-sync)
- [Remote Secret Sharing](#remote-secret-sharing)
  - [`jaws serve`](#jaws-serve)
  - [`jaws connect`](#jaws-connect)
  - [`jaws disconnect`](#jaws-disconnect)
- [Version Control](#version-control)
  - [`jaws log`](#jaws-log)
  - [`jaws rollback`](#jaws-rollback)
- [Archive Operations](#archive-operations)
  - [`jaws export`](#jaws-export)
  - [`jaws import`](#jaws-import)
- [Configuration](#configuration)
  - [`jaws config`](#jaws-config)
  - [`jaws config init`](#jaws-config-init)
  - [`jaws config get`](#jaws-config-get)
  - [`jaws config set`](#jaws-config-set)
  - [`jaws config provider`](#jaws-config-provider)
  - [`jaws config clear-cache`](#jaws-config-clear-cache)
- [Maintenance](#maintenance)
  - [`jaws clean`](#jaws-clean)
  - [`jaws version`](#jaws-version)

---

## Top-level Options

```text
A CLI tool for managing secrets

Usage: jaws [OPTIONS] [COMMAND]

Commands:
  pull        Pull secrets from your secrets manager
  push        Push secrets to your secrets manager
  delete      Delete a secret (prompts for scope: local, remote, or both)
  sync        Refresh the local cache of remote secrets
  list        List all known secrets (one per line, for scripting)
  rollback    Rollback a secret to a previous version (local or remote)
  export      Export and encrypt the secrets directory to a .barrel file
  import      Import and decrypt a .barrel archive
  config      Manage configuration (shows current config if no subcommand provided)
  create      Create a new secret (uses default_provider from config, or prompts for provider)
  log         Show operation log or version history for a specific secret
  clean       Clear local cache and secrets
  serve       Start the jaws secret sharing server (gRPC + mTLS)
  connect     Connect to a remote jaws server
  disconnect  Disconnect from a remote jaws server
  version     Print version information
  help        Print this message or the help of the given subcommand(s)

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -h, --help           Print help
```

## Secret Operations

### jaws pull

```text
Pull secrets from your secrets manager

Usage: jaws pull [OPTIONS] [SECRET_NAME]

Arguments:
  [SECRET_NAME]  Secret reference: PROVIDER://SECRET_NAME (e.g., jaws://my-secret, aws-dev://db-pass) If default_provider is set in config, the prefix can be omitted. If not provided, opens TUI selector

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -e, --edit           Open secrets in editor after downloading
  -p, --print          Print secret value to stdout (for use in scripts). Requires secret_name
  -i, --inject <FILE>  Inject secrets into a template file. Replaces {{PROVIDER://SECRET}} patterns
  -o, --output <FILE>  Output file for inject mode (default: stdout)
  -h, --help           Print help
```

### jaws push

```text
Push secrets to your secrets manager

Usage: jaws push [OPTIONS] [SECRET_NAME]

Arguments:
  [SECRET_NAME]  Name of the secret to push (optional - if not provided, shows TUI with modified secrets)

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -e, --edit           Open secrets in editor before pushing
  -h, --help           Print help
```

### jaws create

```text
Create a new secret (uses default_provider from config, or prompts for provider)

Usage: jaws create [OPTIONS] [NAME]

Arguments:
  [NAME]  Name for the secret (optional - if not provided, prompts interactively)

Options:
  -c, --config <PATH>              Path to config file (overrides default search paths)
  -d, --description <DESCRIPTION>  Optional description
  -f, --file <FILE>                Read value from file instead of editor
  -h, --help                       Print help
```

### jaws delete

```text
Delete a secret (prompts for scope: local, remote, or both)

Usage: jaws delete [OPTIONS] [SECRET_NAME]

Arguments:
  [SECRET_NAME]
          Name of the secret to delete (optional - if not provided, opens TUI selector)

Options:
  -c, --config <PATH>
          Path to config file (overrides default search paths)

  -s, --scope <SCOPE>
          Delete scope: local, remote, or both (if not provided, prompts interactively)

          Possible values:
          - local:  Delete only local cached files
          - remote: Delete only from the remote provider
          - both:   Delete from both local cache and remote provider

  -f, --force
          Force delete without recovery period (for remote deletions)

  -h, --help
          Print help (see a summary with '-h')
```

### jaws list

```text
List all known secrets (one per line, for scripting)

Usage: jaws list [OPTIONS]

Options:
  -c, --config <PATH>        Path to config file (overrides default search paths)
  -p, --provider <PROVIDER>  Filter by provider (e.g., "jaws", "aws-dev")
  -l, --local                Show only locally downloaded secrets
  -h, --help                 Print help
```

### jaws sync

```text
Refresh the local cache of remote secrets

Usage: jaws sync [OPTIONS]

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -h, --help           Print help
```

## Remote Secret Sharing

### jaws serve

```text
Start the jaws secret sharing server (gRPC + mTLS)

Usage: jaws serve [OPTIONS]

Options:
  -b, --bind <BIND>           Address to bind to (e.g., "0.0.0.0:9643") [default: 0.0.0.0:9643]
  -c, --config <PATH>         Path to config file (overrides default search paths)
  -n, --name <NAME>           Server name (used as provider prefix on clients, e.g., "myserver")
      --generate-token        Generate a new enrollment token and exit (requires prior 'jaws serve' run)
      --ca-cert <PATH>        Path to custom CA certificate (PEM). Uses built-in CA by default
      --ca-key <PATH>         Path to custom CA private key (PEM)
      --server-cert <PATH>    Path to custom server certificate (PEM)
      --server-key <PATH>     Path to custom server private key (PEM)
      --revoke <CLIENT_NAME>  Revoke a client's access by name
      --list-clients          List enrolled clients
  -h, --help                  Print help
```

### jaws connect

```text
Connect to a remote jaws server

Usage: jaws connect [OPTIONS] --token <TOKEN> <URL>

Arguments:
  <URL>  Server URL (e.g., "https://10.0.0.5:9643")

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -t, --token <TOKEN>  Enrollment token from the server operator
  -n, --name <NAME>    Name for this server connection (defaults to server-provided name)
  -h, --help           Print help
```

### jaws disconnect

```text
Disconnect from a remote jaws server

Usage: jaws disconnect [OPTIONS] <NAME>

Arguments:
  <NAME>  Server name to disconnect from

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -h, --help           Print help
```

## Version Control

### jaws log

```text
Show operation log or version history for a specific secret

Usage: jaws log [OPTIONS] [SECRET_NAME]

Arguments:
  [SECRET_NAME]  Secret reference (e.g., "my-secret", "aws://db-pass"). If provided, shows version history for that secret. If omitted, shows the global operation log

Options:
  -c, --config <PATH>        Path to config file (overrides default search paths)
  -n, --limit <LIMIT>        Maximum number of entries to show
  -p, --provider <PROVIDER>  Filter by provider (global log only)
  -v, --verbose              Show full details including file hashes (version history only)
  -h, --help                 Print help
```

### jaws rollback

```text
Rollback a secret to a previous version (local or remote)

Usage: jaws rollback [OPTIONS] [SECRET_NAME]

Arguments:
  [SECRET_NAME]  Name of the secret to rollback

Options:
  -c, --config <PATH>            Path to config file (overrides default search paths)
  -v, --version <VERSION>        Version number to rollback to for local rollback (optional - shows version selector)
  -e, --edit                     Open the rolled back secret in editor (local rollback only)
  -r, --remote                   Rollback on the remote provider instead of locally
      --version-id <VERSION_ID>  Version ID for remote rollback (provider-specific, e.g., AWS version ID)
  -h, --help                     Print help
```

## Archive Operations

### jaws export

```text
Export and encrypt the secrets directory to a .barrel file

Usage: jaws export [OPTIONS]

Options:
  -c, --config <PATH>    Path to config file (overrides default search paths)
  -K, --ssh-key <PATH>   Encrypt to an SSH public key file instead of passphrase
  -o, --output <OUTPUT>  Output path for the archive (default: ./jaws.barrel)
      --delete           Delete secrets directory after successful export
  -h, --help             Print help
```

### jaws import

```text
Import and decrypt a .barrel archive

Usage: jaws import [OPTIONS] <ARCHIVE>

Arguments:
  <ARCHIVE>  Path to the .barrel archive file

Options:
  -c, --config <PATH>   Path to config file (overrides default search paths)
  -K, --ssh-key <PATH>  Decrypt with an SSH private key file instead of passphrase
      --delete          Delete archive after successful import
  -h, --help            Print help
```

## Configuration

### jaws config

```text
Manage configuration (shows current config if no subcommand provided)

Usage: jaws config [OPTIONS] [COMMAND]

Commands:
  init         Initialize a new config file (interactive by default)
  get          Get a specific configuration value
  clear-cache  Clear cached credentials from the OS keychain
  set          Set a configuration value
  provider     Manage providers (lists providers if no subcommand given)
  help         Print this message or the help of the given subcommand(s)

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -h, --help           Print help
```

### jaws config init

```text
Initialize a new config file (interactive by default)

Usage: jaws config init [OPTIONS]

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
      --path <PATH>    Path where to create the config file (default: ./jaws.hcl)
      --overwrite      Overwrite existing config file if it exists
  -m, --minimal        Generate minimal config without interactive prompts
  -h, --help           Print help
```

### jaws config get

```text
Get a specific configuration value

Usage: jaws config get [OPTIONS] <KEY>

Arguments:
  <KEY>  Setting key (e.g., "editor", "secrets_path", "cache_ttl")

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -h, --help           Print help
```

### jaws config set

```text
Set a configuration value

Usage: jaws config set [OPTIONS] <KEY> <VALUE>

Arguments:
  <KEY>    Setting key
  <VALUE>  New value

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -h, --help           Print help
```

### jaws config provider

```text
Manage providers (lists providers if no subcommand given)

Usage: jaws config provider [OPTIONS] [COMMAND]

Commands:
  add     Add a new provider (interactive discovery)
  remove  Remove a provider from the config
  help    Print this message or the help of the given subcommand(s)

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -h, --help           Print help
```

### jaws config clear-cache

```text
Clear cached credentials from the OS keychain

Usage: jaws config clear-cache [OPTIONS]

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -h, --help           Print help
```

## Maintenance

### jaws clean

```text
Clear local cache and secrets

Usage: jaws clean [OPTIONS]

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -f, --force          Delete without confirmation (dangerous for local jaws secrets)
      --dry-run        Show what would be deleted without actually deleting
      --keep-local     Keep local jaws secrets, only delete cached remote secrets
  -h, --help           Print help
```

### jaws version

```text
Print version information

Usage: jaws version [OPTIONS]

Options:
  -c, --config <PATH>  Path to config file (overrides default search paths)
  -h, --help           Print help
```

---

See also:
- [Getting Started](getting-started.md) — common workflows
- [Configuration](configuration.md) — `jaws.hcl` format and providers
- [Remote Sharing](remote-sharing.md) — `jaws serve` and `jaws connect`
