# Remote Secret Sharing

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Server Setup](#server-setup)
- [Client Enrollment](#client-enrollment)
- [Fingerprint Verification](#fingerprint-verification)
- [Managing Clients](#managing-clients)
- [Using Remote Providers](#using-remote-providers)
- [Custom Certificates](#custom-certificates)
- [Disconnecting](#disconnecting)

---

## Overview

JAWS includes a built-in gRPC server with mutual TLS (mTLS) authentication for securely sharing secrets across machines. The server exposes all configured providers to enrolled clients.

Key properties:

- **mTLS authentication** — both server and client present certificates signed by the server's CA
- **Built-in PKI** — CA and certificates auto-generated on first `jaws serve` run
- **One-time enrollment tokens** — UUID tokens expire in 15 minutes, single-use
- **Certificate pinning** — clients trust only the specific server CA received during enrollment
- **Client revocation** — `jaws serve --revoke <name>` immediately blocks a client
- **Audit logging** — all remote operations logged with client identity
- **No secret caching** — secrets fetched from providers on-demand per request

---

## Architecture

```
┌──────────────────┐        mTLS (gRPC/HTTP2)        ┌──────────────────┐
│   jaws client    │ ◄──────────────────────────────► │   jaws serve     │
│                  │                                   │                  │
│ RemoteProvider   │  ── pull/push/list/create/del ─► │ SecretManager    │
│ (per server      │                                   │ providers[]      │
│  provider)       │                                   │ (aws, gcp, op…)  │
└──────────────────┘                                   └──────────────────┘
```

- The **server** runs `jaws serve` and exposes its configured providers over gRPC
- **Clients** enroll via `jaws connect` using a one-time enrollment token
- After enrollment, remote providers appear transparently as `servername/provider`
- All operations use the same syntax as local providers

---

## Server Setup

Start the server:

```bash
jaws serve -n myserver
```

On first run, this generates:
- A self-signed CA (valid 10 years) in `~/.jaws/server/`
- A server certificate (valid 1 year) with SANs for localhost and the system hostname
- An enrollment token written to `~/.jaws/server/enrollment.token`

The server prints:

```
=== Enrollment Token ===
  Token written to: ~/.jaws/server/enrollment.token
  (restricted to owner-only access)

  Clients can connect with:
    jaws connect https://SERVER_IP:9643 --token $(cat ~/.jaws/server/enrollment.token)

  Token expires in 15 minutes. Generate a new one with:
    jaws serve --generate-token
========================
```

Generate a new token without restarting:

```bash
jaws serve --generate-token
```

Bind to a specific address (default is `0.0.0.0:9643`):

```bash
jaws serve -b 192.168.1.10:9643
```

---

## Client Enrollment

Enroll with a server:

```bash
jaws connect https://10.0.0.5:9643 --token <TOKEN>
```

This:
1. Generates a local keypair and CSR
2. Connects to the server with temporary unverified TLS
3. Sends the enrollment token and CSR
4. Receives a signed client certificate and the CA certificate
5. Saves certificates to `~/.config/jaws/clients/<servername>/`
6. Adds the server entry to `jaws.kdl`

After enrollment, discover remote providers:

```bash
jaws sync
```

---

## Fingerprint Verification

During enrollment, the client displays the SHA-256 fingerprint of the server's certificate:

```
Server certificate fingerprint: SHA256:abc123...
Trust this server? [y/N]
```

You must explicitly confirm before proceeding. This protects against man-in-the-middle attacks during the initial TOFU (Trust On First Use) handshake.

For automated/scripted enrollment, provide the expected fingerprint:

```bash
jaws connect https://10.0.0.5:9643 \
  --token <TOKEN> \
  --fingerprint abc123...
```

If the fingerprint does not match, enrollment aborts immediately.

---

## Managing Clients

List enrolled clients:

```bash
jaws serve --list-clients
```

Revoke a client:

```bash
jaws serve --revoke badclient
```

Revoked clients are immediately blocked on their next request. The server validates client certificates against a local SQLite database on every request.

---

## Using Remote Providers

After `jaws sync`, remote providers appear as `servername/provider-id`:

```bash
# Pull a secret through the server
jaws pull myserver/aws-prod://db-password -p

# Use in templates
jaws pull -i .env.tpl -o .env
# where .env.tpl contains: DB_PASS={{myserver/aws-prod://db-password}}

# List remote secrets
jaws list --provider myserver/aws-prod
```

All standard operations (pull, push, create, delete, rollback) work transparently through remote providers.

---

## Custom Certificates

By default, `jaws serve` generates its own CA. You can provide your own:

```bash
jaws serve \
  --ca-cert /path/to/ca.pem \
  --ca-key /path/to/ca-key.pem \
  --server-cert /path/to/server.pem \
  --server-key /path/to/server-key.pem
```

This is useful for:
- Using an existing organizational CA
- Running the server in environments where auto-generated CAs are not trusted
- Integrating with existing certificate management infrastructure

---

## Disconnecting

Remove a server connection and delete client certificates:

```bash
jaws disconnect myserver
```

This removes the server from `jaws.kdl` and deletes the certificate directory.

---

See also:
- [Security](security.md) — threat model, mTLS details, CA handling
- [Commands](commands.md) — `jaws serve`, `jaws connect`, `jaws disconnect`
- [Getting Started](getting-started.md) — first-time setup
