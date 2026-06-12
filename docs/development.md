# Development

## Table of Contents

- [Dev Shell](#dev-shell)
- [Building from Source](#building-from-source)
- [Running Tests](#running-tests)
- [Demo GIF Generation](#demo-gif-generation)
- [Cross-Compilation](#cross-compilation)
- [Release Process](#release-process)
- [Documentation Generation](#documentation-generation)

---

## Dev Shell

Enter the development environment with all dependencies:

```bash
nix develop
```

This provides:
- Rust toolchain (with `rust-src`, `rust-analyzer`)
- `bacon` (auto-rebuild on save)
- `lldb` (debugger)
- `cargo-zigbuild` (cross-compilation)
- `protobuf` (gRPC code generation)
- `vhs`, `ttyd`, `ffmpeg` (demo recording)

Environment variables for OpenSSL and the 1Password SDK are pre-configured.

---

## Building from Source

### Native build

```bash
cargo build --release
```

### Using Nix

```bash
nix build
```

### Development build with auto-reload

```bash
bacon
```

---

## Running Tests

```bash
cargo test
```

The test suite covers:
- Provider trait implementations
- Database migrations and queries
- Configuration parsing (KDL)
- Archive encryption/decryption
- PKI certificate generation

---

## Demo GIF Generation

Regenerate the demo GIF (requires the nix dev shell):

```bash
./scripts/demo.sh
```

The tape file at `scripts/demo.tape` defines the recorded session. Edit it to change what the demo shows.

Requirements:
- `vhs` — terminal recorder
- `ttyd` — terminal sharing daemon
- `ffmpeg` — video encoding
- `jaws` binary in `$PATH`

---

## Cross-Compilation

The project supports cross-compilation using `cargo-zigbuild` for Linux targets and native cargo for Darwin targets:

```bash
# Build all cross-compiled binaries
./scripts/release.sh --build-only

# Full release process (bumps version, builds all targets, creates tag)
./scripts/release.sh 1.5.0
```

Supported targets:

| Target | Command |
|--------|---------|
| x86_64 Linux | `nix build .#jaws-x86_64-linux` |
| aarch64 Linux | `nix build .#jaws-aarch64-linux` |
| x86_64 Darwin | `nix build .#jaws-x86_64-darwin` |
| aarch64 Darwin | `nix build .#jaws-aarch64-darwin` |

Release binaries are output to `dist/`:

```
dist/
├── jaws-x86_64-linux.tar.gz
├── jaws-aarch64-linux.tar.gz
├── jaws-x86_64-darwin.tar.gz
└── jaws-aarch64-darwin.tar.gz
```

---

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run `./scripts/generate-docs.sh` to refresh command reference
4. Run `./scripts/release.sh <VERSION>`:
   - Verifies Nix native build
   - Cross-compiles for all targets
   - Generates changelog with `git-cliff`
   - Commits and tags

---

## Documentation Generation

Auto-generate the command reference from `--help` output:

```bash
./scripts/generate-docs.sh
```

This updates `docs/commands.md` with the current CLI flags and options. Run it before releases or when adding new commands.

---

See also:
- [Nix](nix.md) — flake, overlay, Home Manager module
- [Architecture](architecture.md) — codebase structure for contributors
