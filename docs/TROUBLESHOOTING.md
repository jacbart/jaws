# Troubleshooting

## Table of Contents

- [Database Errors](#database-errors)
- [1Password SDK Loading](#1password-sdk-loading)
- [Permission Denied](#permission-denied)
- [Connection Problems](#connection-problems)
- [Provider Authentication](#provider-authentication)
- [Template Injection](#template-injection)
- [Build Issues](#build-issues)

---

## Database Errors

### `no such column: filename`

Your database was created with an older schema. JAWS auto-migrates on startup, but if you see this error, the migration may have failed.

**Fix:**

```bash
# Remove the database (you'll lose local version history)
rm ~/.jaws/jaws.db

# Next jaws command will recreate with the latest schema
jaws pull
```

**Note:** This only affects local version tracking. Your secrets in remote providers are unaffected.

### Database is locked

Another `jaws` process is holding the database lock.

**Fix:**

```bash
# Find and terminate other jaws processes
pkill -f jaws

# Or wait a few seconds and retry
```

---

## 1Password SDK Loading

### `libop_uniffi_core.so: cannot open shared object file`

The 1Password SDK shared library cannot be found.

**Nix users:** This is handled automatically by the wrapper.

**Cargo users:**

```bash
# The SDK is downloaded automatically on first use.
# If it fails, set the skip flag and provide the path manually:
export ONEPASSWORD_SKIP_DOWNLOAD=1
export ONEPASSWORD_LIB_PATH=/path/to/libop_uniffi_core.so
```

### `1Password provider failed to initialize`

Check that `OP_SERVICE_ACCOUNT_TOKEN` is set and valid:

```bash
echo $OP_SERVICE_ACCOUNT_TOKEN
```

The token needs access to the vault specified in your config.

---

## Permission Denied

### `Permission denied (os error 13)` on secrets directory

JAWS tries to set restrictive permissions (owner-only) on sensitive directories. On some filesystems (network shares, Windows WSL), this may fail.

**Fix:**

```bash
# Manually set permissions
chmod 700 ~/.jaws
chmod 700 ~/.jaws/secrets
chmod 600 ~/.jaws/server/ca-key.pem
chmod 600 ~/.jaws/server/server-key.pem
```

### Cannot read enrollment token

The token file has restricted permissions:

```bash
# Read as the owner
cat ~/.jaws/server/enrollment.token
```

If you need to share the token with another user, use a secure channel (not email or Slack). Tokens expire in 15 minutes anyway.

---

## Connection Problems

### `Failed to connect to server`

1. Verify the server is running: `jaws serve --list-clients` (from the server host)
2. Check firewall rules — port 9643 must be open
3. Verify the URL matches the server's bind address
4. Check that the CA certificate hasn't expired

### `Client certificate required`

You haven't enrolled with the server, or your client certificate expired.

**Fix:**

```bash
# Re-enroll
jaws connect https://server:9643 --token <NEW_TOKEN>

# Or check if your cert is still valid
openssl x509 -in ~/.config/jaws/clients/myserver/client.pem -noout -dates
```

### `Server fingerprint mismatch`

The server's certificate changed. This can happen if:
- The server was reinstalled (new CA generated)
- A man-in-the-middle attack is occurring
- You're connecting to a different server at the same address

**Fix:**

```bash
# Disconnect and re-enroll
jaws disconnect myserver
jaws connect https://server:9643 --token <TOKEN>
```

Verify the fingerprint out-of-band before trusting the new certificate.

---

## Provider Authentication

### AWS: `No credentials found`

Ensure AWS credentials are configured:

```bash
aws configure
# Or set environment variables:
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
```

### GCP: `Failed to authenticate`

Run `gcloud auth application-default login` or set `GOOGLE_APPLICATION_CREDENTIALS`.

### Bitwarden: `Invalid access token`

Verify `BWS_ACCESS_TOKEN` is set and the token has access to the specified organization and project.

---

## Template Injection

### `Secret not found` in template

Template syntax: `{{provider://secret-name}}`

Common mistakes:
- Missing `//` after provider
- Wrong provider ID (check `jaws config provider`)
- Secret doesn't exist in that provider

### Default values not working

Syntax for defaults: `{{provider://secret || 'default'}}`

The `||` operator requires spaces around it. Single or double quotes are accepted.

---

## Build Issues

### `protoc` not found

Install Protocol Buffers:

```bash
# NixOS
nix-shell -p protobuf

# macOS
brew install protobuf

# Ubuntu/Debian
apt-get install protobuf-compiler
```

### `cargo build` fails with OpenSSL errors

Use the Nix dev shell which sets all required environment variables, or:

```bash
export PKG_CONFIG_PATH=$(pkg-config --variable pc_path pkg-config)
export OPENSSL_DIR=/usr
```

### Cross-compilation fails

Use Nix for cross-compilation — it handles toolchains automatically:

```bash
nix build .#jaws-aarch64-linux
```

Manual cross-compilation requires `cargo-zigbuild` and target-specific linkers.

---

See also:
- [Getting Started](getting-started.md) — basic usage
- [Configuration](configuration.md) — provider setup
- [Security](security.md) — certificate and authentication details
- [Remote Sharing](remote-sharing.md) — server/client troubleshooting
