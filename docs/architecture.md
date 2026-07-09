# Architecture

## Table of Contents

- [Overview](#overview)
- [Provider Model](#provider-model)
- [gRPC Service](#grpc-service)
- [Database Schema](#database-schema)
- [PKI](#pki)
- [Project Structure](#project-structure)

---

## Overview

JAWS is structured as a Rust library (`src/lib.rs`) with a CLI binary (`src/main.rs`). The core abstraction is the [`SecretManager`](https://docs.rs/jaws/latest/jaws/secrets/trait.SecretManager.html) trait — an object-safe interface implemented by all secret providers.

```rust
pub trait SecretManager: Send + Sync {
    async fn get_secret(&self, name: &str) -> Result<String, JawsError>;
    async fn list_all(&self) -> Result<Vec<String>, JawsError>;
    async fn create(&self, name: &str, value: &str, description: Option<&str>) -> Result<String, JawsError>;
    async fn update(&self, name: &str, value: &str) -> Result<String, JawsError>;
    async fn delete(&self, name: &str, force: bool) -> Result<(), JawsError>;
    async fn rollback(&self, name: &str, version_id: Option<&str>) -> Result<String, JawsError>;
    fn supports_rollback(&self) -> bool;
    fn supports_remote_history(&self) -> bool;
    fn id(&self) -> &str;
    fn kind(&self) -> &str;
}
```

This trait is used as `Box<dyn SecretManager>` (aliased as `Provider`) throughout the codebase. Adding a new provider means implementing this trait and registering it in `detect_providers()`.

---

## Provider Model

Providers are detected at runtime based on the configuration file (`jaws.hcl`):

| Provider | Kind | Backend |
|----------|------|---------|
| AWS Secrets Manager | `aws` | AWS SDK |
| GCP Secret Manager | `gcp` | google-cloud-secretmanager-v1 |
| 1Password | `onepassword` | 1Password Rust SDK |
| Bitwarden | `bw` | bitwarden crate |
| Local (jaws) | `jaws` | Filesystem + SQLite |
| Remote | `jaws` (prefixed) | gRPC/mTLS proxy |

The `RemoteProvider` (`src/client/`) implements `SecretManager` by proxying all operations over gRPC to a remote `jaws serve` instance. Remote providers are prefixed with the server name: `myserver/aws-prod`.

---

## gRPC Service

The gRPC service is defined in `proto/jaws.proto` and generated with `tonic-prost-build` at compile time.

**RPCs:**

| RPC | Auth Required | Description |
|-----|---------------|-------------|
| `Enroll` | No | Token + CSR → signed client cert + CA cert |
| `Ping` | No | Health check / discovery |
| `ListProviders` | Yes | Available providers with capabilities |
| `ListSecrets` | Yes | Streaming list of secret names |
| `GetSecret` | Yes | Retrieve secret value |
| `CreateSecret` | Yes | Create new secret |
| `UpdateSecret` | Yes | Update existing secret |
| `DeleteSecret` | Yes | Delete secret |
| `RollbackSecret` | Yes | Rollback to previous version |

Authentication is enforced at the application layer after extracting the client identity from the mTLS handshake. The revocation database is queried on every authenticated request; database errors fail closed (deny the request).

---

## Database Schema

JAWS uses SQLite with `rusqlite` (bundled). The schema is versioned and auto-migrates on startup.

**Current schema version:** 5

**Tables:**

| Table | Purpose |
|-------|---------|
| `schema_version` | Migration tracking |
| `providers` | Registered provider metadata |
| `secrets` | Secret metadata (name, provider_id, description) |
| `downloads` | Version history (version number, filename, hash, timestamp) |
| `credentials` | Encrypted provider auth tokens |
| `operations` | Audit log of all operations |
| `clients` | Enrolled remote clients (name, fingerprint, cert PEM, revoked flag) |
| `enrollment_tokens` | One-time enrollment tokens (token, created, expires, used) |

Migrations are handled in `src/db/schema.rs`. Each version has an associated migration function.

---

## PKI

Certificate management uses the `rcgen` crate.

**Certificate lifetimes:**

| Type | Validity |
|------|----------|
| CA | 10 years |
| Server | 1 year |
| Client | 1 year |

**Process:**

1. `jaws serve` generates or loads CA + server cert
2. Client generates CSR + keypair locally
3. Client connects with unverified TLS (fingerprint captured)
4. Client sends token + CSR to `Enroll` RPC
5. Server signs CSR, returns client cert + CA cert
6. Client pins the CA cert for all future mTLS connections

Certificate revocation is tracked in the SQLite `clients` table, checked on every authenticated request.

---

## Project Structure

```
src/
├── main.rs          # CLI entry point, command routing
├── lib.rs           # Library exports, crate documentation
├── archive.rs       # age-based encryption for .barrel exports
├── credentials.rs   # Credential encryption/decryption
├── keychain.rs      # OS keychain integration (TTL cache)
├── cli/             # Clap argument and command definitions
├── client/          # RemoteProvider, mTLS connection, enrollment
├── commands/        # Command handlers (pull, push, serve, etc.)
├── config/          # HCL parsing, provider discovery
├── db/              # SQLite schema, migrations, repository
├── secrets/         # Secret providers
│   └── providers/   # AWS, GCP, 1Password, Bitwarden, local
├── server/          # gRPC server (mTLS, PKI, enrollment, service)
└── utils/           # Shared utilities
proto/
└── jaws.proto       # gRPC service definition
nix/
├── hm-module.nix    # Home Manager module
├── cross-build.nix  # Cross-compilation derivation
└── onepassword-sdk.nix  # 1Password SDK fetcher
scripts/
├── demo.sh          # Demo GIF generation
├── demo.tape        # VHS tape script
├── release.sh       # Release automation
└── generate-docs.sh # Command reference generator
```

---

See also:
- [Development](development.md) — building, testing, contributing
- [Security](security.md) — threat model, mTLS, PKI details
- [Remote Sharing](remote-sharing.md) — server/client setup
