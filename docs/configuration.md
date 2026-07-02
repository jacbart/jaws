# Configuration

## Table of Contents

- [Config File Location](#config-file-location)
- [KDL Format](#kdl-format)
- [Defaults](#defaults)
- [Providers](#providers)
  - [AWS](#aws)
  - [GCP](#gcp)
  - [1Password](#1password)
  - [Bitwarden](#bitwarden)
  - [Local (jaws)](#local-jaws)
- [Remote Servers](#remote-servers)
- [Environment Variables](#environment-variables)

---

## Config File Location

JAWS looks for `jaws.kdl` in this order:

1. Path specified by `--config` CLI flag
2. `JAWS_CONFIG_PATH` environment variable
3. `./jaws.kdl` (current directory)
4. `~/.config/jaws/jaws.kdl`
5. `~/.jaws/jaws.kdl`

Use `jaws config` to see which file is currently loaded.

---

## KDL Format

JAWS uses [KDL](https://kdl.dev/) — a human-friendly document language similar to JSON but with a cleaner syntax.

```kdl
// jaws.kdl — full example

defaults editor="nvim" secrets_path="~/.jaws/secrets" cache_ttl=900 default_provider="jaws"

provider "aws-prod" kind="aws" {
    profile "production"
    region "us-east-1"
}

provider "op-team" kind="onepassword" {
    vault "abc123"
}

provider "bw-project" kind="bw" {
    vault "project-uuid"
    organization "org-uuid"
    token-env "BWS_ACCESS_TOKEN"
}

provider "gcp-prod" kind="gcp" {
    project "my-gcp-project-id"
}

server "myserver" url="https://10.0.0.5:9643" {
    ca-cert "~/.config/jaws/clients/myserver/ca.pem"
    client-cert "~/.config/jaws/clients/myserver/client.pem"
    client-key "~/.config/jaws/clients/myserver/client-key.pem"
}
```

---

## Defaults

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `editor` | string | `"$EDITOR"` or `"vi"` | Editor for secret editing |
| `secrets_path` | string | `"~/.jaws/secrets"` | Local secret storage directory |
| `cache_ttl` | integer | `900` | Credential cache TTL in seconds |
| `default_provider` | string | `"jaws"` | Provider used when no prefix given |
| `max_versions` | integer | none | Maximum local versions to keep per secret |

---

## Providers

### AWS

Requires AWS credentials in `~/.aws/credentials` with Secrets Manager permissions.

```kdl
// Auto-discover all profiles
provider "aws" kind="aws" {
    profile "all"
}

// Specific profile and region
provider "aws-prod" kind="aws" {
    profile "production"
    region "us-east-1"
}
```

### GCP

Requires [Application Default Credentials](https://cloud.google.com/docs/authentication/application-default-credentials).

```bash
# Local development
gcloud auth application-default login
```

```kdl
provider "gcp-prod" kind="gcp" {
    project "my-gcp-project-id"
}
```

The project ID can be auto-discovered during `jaws config init` from `GOOGLE_CLOUD_PROJECT` or the active `gcloud` config.

### 1Password

JAWS supports two authentication methods for 1Password:

1. **Service Account Token** (SDK backend) — requires `OP_SERVICE_ACCOUNT_TOKEN` environment variable
2. **Desktop App Integration** (CLI backend) — uses biometric auth via 1Password desktop app

By default, JAWS will use the SDK backend if `OP_SERVICE_ACCOUNT_TOKEN` is set, otherwise it falls back to the CLI backend with biometric authentication.

```kdl
// Auto-discover all vaults
provider "op" kind="onepassword" {
    vault "all"
}

// Specific vault
provider "op-dev" kind="onepassword" {
    vault "abc123"
}

// Force CLI backend even when SDK token is available
provider "op-biometric" kind="onepassword" {
    vault "abc123"
    force_cli true
}
```

#### Performance Notes

- **SDK backend**: Faster (~100ms per API call), no process spawn overhead
- **CLI backend**: Slower (~500ms per API call), requires biometric authentication
- Both backends use concurrent requests (up to 10 at a time) to improve performance
- Large vaults (100+ items) may take 10-60 seconds to sync due to API limitations

The CLI backend requires:
- 1Password desktop app running
- "Integrate with 1Password CLI" enabled in Settings > Developer
- `op` CLI installed (available in `nix develop` shell)

### Bitwarden

Requires `BWS_ACCESS_TOKEN` environment variable with a Secrets Manager access token.

```kdl
provider "bw-myproject" kind="bw" {
    vault "project-uuid-here"
    organization "org-uuid-here"
    token-env "BWS_ACCESS_TOKEN"
}
```

### Local (jaws)

The `jaws` provider is always available — no configuration needed.

#### Layout

Under `secrets_path`:

```
{secrets_path}/
├── jaws.db                           # SQLite metadata
├── secrets/                          # user-editable working copies
│   └── {provider_id}/{name}
└── .versions/                        # immutable per-version archive
    └── {provider_id}/{name}/v{N}
```

- `secrets/{provider_id}/{name}` — open in any text editor; this file is the source of truth for the current value.
- `.versions/{provider_id}/{name}/v{N}` — every prior version, preserved verbatim. Used by `jaws rollback` and `jaws log`.

The same layout is used for **every** provider — the working file is the editing surface for AWS, GCP, 1Password and Bitwarden secrets too. `jaws pull` materializes the remote value to the working file; `jaws save` records local edits in the DB without uploading; `jaws push` uploads any unsynced edits.

#### Creating a secret by dropping a file

```sh
echo "my-value" > .secrets/secrets/jaws/my-new-secret
jaws save                  # registers it
```

---

## Remote Servers

Server entries are added automatically by `jaws connect`. You can also add them manually:

```kdl
server "myserver" url="https://10.0.0.5:9643" {
    ca-cert "~/.config/jaws/clients/myserver/ca.pem"
    client-cert "~/.config/jaws/clients/myserver/client.pem"
    client-key "~/.config/jaws/clients/myserver/client-key.pem"
}
```

Remote providers appear as `servername/provider-id` (e.g., `myserver/aws-prod`).

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `JAWS_CONFIG_PATH` | Override config file path |
| `OP_SERVICE_ACCOUNT_TOKEN` | 1Password service account token |
| `BWS_ACCESS_TOKEN` | Bitwarden Secrets Manager token |
| `GOOGLE_APPLICATION_CREDENTIALS` | GCP service account key file |

---

See also:
- [Getting Started](getting-started.md) — first-time setup walkthrough
- [Nix](nix.md) — declarative configuration with Home Manager
- [Remote Sharing](remote-sharing.md) — server/client setup
