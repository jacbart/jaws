# Security

## Table of Contents

- [Threat Model](#threat-model)
- [mTLS Authentication](#mtls-authentication)
- [Certificate Authority](#certificate-authority)
- [Enrollment Security](#enrollment-security)
- [Client Revocation](#client-revocation)
- [Secret Handling](#secret-handling)
- [Dependency Security](#dependency-security)
- [Best Practices](#best-practices)

---

## Threat Model

JAWS is designed to protect secrets in transit and at rest with the following threat model:

**In scope:**
- Unauthorized access to secrets via the remote sharing server
- Man-in-the-middle attacks on the gRPC connection
- Compromised client certificates
- Local secret storage on shared/multi-user systems

**Out of scope:**
- Compromise of the underlying cloud provider (AWS, GCP, etc.)
- Keyloggers or compromised local machines
- Social engineering of enrollment tokens
- Physical access to unlocked machines

**Design philosophy:**
- Fail closed — if authentication or the database is unavailable, requests are denied
- Defense in depth — mTLS at the transport layer + application-layer authorization
- Minimal trust — clients verify the server fingerprint before trusting the CA

---

## mTLS Authentication

All gRPC connections between clients and servers use mutual TLS:

1. **Server certificate** — signed by the server's CA, presented on every connection
2. **Client certificate** — signed by the same CA, presented after enrollment
3. **CA verification** — both sides verify the peer's certificate against the shared CA
4. **Revocation check** — on every request, the server checks if the client certificate fingerprint is marked revoked in its local database

The TLS layer uses `client_auth_optional(true)` to allow unauthenticated clients to reach the `Enroll` RPC. All other RPCs require a valid, non-revoked client certificate. If the revocation database is unavailable, the server fails closed and denies all authenticated requests.

---

## Certificate Authority

### Auto-Generated CA

On first `jaws serve` run, JAWS generates:
- A self-signed CA certificate (10-year validity)
- A CA private key stored in `~/.jaws/server/ca-key.pem`

The CA key has owner-only file permissions (`0600`).

### CA Key Protection

The CA private key is the root of trust. Anyone with access to `ca-key.pem` can sign arbitrary client certificates. Protect it:

- Run the server on a dedicated, hardened host
- Restrict file permissions (handled automatically)
- Back up the CA key securely — losing it requires re-enrolling all clients
- Consider using custom certificates for production deployments

### Custom CA

For production or shared infrastructure, provide your own CA:

```bash
jaws serve --ca-cert ca.pem --ca-key ca-key.pem ...
```

This integrates with existing PKI infrastructure and keeps the CA key separate from the server host.

---

## Enrollment Security

### One-Time Tokens

Enrollment tokens are:
- UUIDv4 format
- Single-use
- Expire after 15 minutes
- Written to `~/.jaws/server/enrollment.token` with owner-only permissions

Tokens are not logged to stderr (to avoid capture by centralized logging). Server startup only prints the token file path.

### Fingerprint Verification

During `jaws connect`, the client captures the server's certificate fingerprint and either:
- Prompts the user to confirm interactively (default)
- Verifies against a pre-shared `--fingerprint` value (automated)

This prevents silent man-in-the-middle attacks during the TOFU bootstrap. An attacker impersonating the server will have a different certificate fingerprint, which the user must explicitly accept.

---

## Client Revocation

Revoke a compromised client immediately:

```bash
jaws serve --revoke compromised-client
```

The server maintains a revocation list in its SQLite database. On every authenticated request:
1. Extract the client certificate fingerprint from the TLS handshake
2. Query the database: is this fingerprint valid?
3. If revoked (or DB error), deny the request

Revocation is immediate — no CRL distribution delay, no OCSP round-trips.

---

## Secret Handling

### At Rest

- Local secrets are stored as plain files in `secrets_path` (default: `~/.jaws/secrets/`)
- The secrets directory should have restricted permissions (handled automatically)
- The SQLite database tracks versions but does not store secret values
- Provider credentials are encrypted with age (passphrase or SSH key) and cached in the OS keychain

### In Transit

- Remote sharing uses gRPC over TLS 1.3 with mutual authentication
- Secret values are never logged
- No caching on the server — secrets are fetched from providers on every request

---

## Dependency Security

JAWS uses `cargo audit` and `cargo deny` to monitor dependency vulnerabilities and license compliance.

### Auditing

```bash
# Check for known vulnerabilities
cargo audit

# Full policy check (advisories, licenses, bans, sources)
cargo deny check
```

Configuration lives in `deny.toml` at the project root. Unfixable upstream advisories are documented with ignore entries:

| Advisory | Crate | Status |
|----------|-------|--------|
| RUSTSEC-2023-0071 (rsa Marvin Attack) | `age`, `bitwarden-crypto` | No upstream fix available |
| RUSTSEC-2024-0370 (proc-macro-error) | `knuffel` | Unmaintained, no alternative |
| RUSTSEC-2026-0173 (proc-macro-error2) | `bitwarden`, `age` | Upstream dependency |

### TLS Stack

The gRPC transport uses `rustls 0.23` with `aws-lc-rs` as the cryptographic backend. AWS SDK crates are configured with `default-features = false` to avoid pulling in the legacy `rustls 0.21` stack.

---

## Best Practices

1. **Use fingerprint verification** — always verify the server fingerprint during `jaws connect`, especially on untrusted networks
2. **Rotate enrollment tokens** — generate new tokens for each client enrollment, don't reuse
3. **Revoke promptly** — revoke client certificates immediately when a machine is decommissioned or compromised
4. **Custom certs for production** — use organizational CA certificates instead of auto-generated for long-lived deployments
5. **Restrict secrets directory** — ensure `~/.jaws/` has appropriate permissions (handled automatically, but verify on shared systems)
6. **Keep CA key safe** — back up `ca-key.pem` and store it offline; losing it breaks all client trust

---

See also:
- [Remote Sharing](remote-sharing.md) — server setup, enrollment, client management
- [Configuration](configuration.md) — provider authentication, environment variables
- [Troubleshooting](TROUBLESHOOTING.md) — permission issues, connection problems
