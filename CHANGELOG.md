# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **BREAKING: config format switched from KDL to HCL.** The config file is now `jaws.hcl` (was `jaws.kdl`); `jaws.kdl` files are no longer read from any search path and must be converted by hand. Motivation: the `knuffel` KDL parser is unmaintained and only supports KDL v1, so any file reformatted by `kdlfmt` (which emits KDL v2) failed to parse. Parsing now uses the actively maintained `hcl-rs` crate with strict unknown-field rejection. The Home Manager module now generates `~/.config/jaws/jaws.hcl`. Bonus fixes: `max_versions` and `force_cli` were silently dropped when saving the config, and values containing quotes/backslashes were not escaped — both fixed by the new serializer.

### Added

- **Folder-first workflow.** Open `secrets_path/secrets/{provider_id}/{name}` in any text editor to read or edit a secret, or drop a new file there to create one. `jaws save` reconciles the working dir with the local SQLite DB, hashing each file and archiving prior contents under `.versions/{provider_id}/{name}/v{N}`. Every provider (jaws, aws, gcp, onepassword, bitwarden) uses the same edit surface.
- `jaws save [name]` — local-only reconciliation. Scans the working dir (or a single secret), creates new download rows for new/changed files, leaves remote providers' rows as `pushed_at = NULL` for later upload.
- `jaws status` — git-like view of working dir vs DB: new / modified / unpushed / orphan.

### Changed

- **On-disk layout.** Secrets are no longer stored as flat `{name}_{hash}_{version}` files at the root of `secrets_path`. New layout:
  - `secrets/{provider_id}/{name}` — user-editable working copy (current value)
  - `.versions/{provider_id}/{name}/v{N}` — per-version archive
- `jaws push` now runs `save` first, then uploads every download row with `pushed_at IS NULL` to its remote provider. Conflict detection: if the remote drifted since the last successful push, push aborts with a clear remediation hint.
- Database schema migrated to v6: `downloads.pushed_at TIMESTAMP NULL` distinguishes "saved locally" from "uploaded to remote"; new indexes on `(provider_id, display_name)` and partial `pushed_at IS NULL`.
- One-shot auto-migration on first run relocates legacy `{name}_{hash}_{version}` files into the new layout — idempotent.
- Restructured documentation into focused guides under `docs/`
- Auto-generated command reference from `--help` output
- Upgraded `tonic` 0.12 → 0.14, `prost` 0.13 → 0.14, `tonic-build` → `tonic-prost-build`
- Pinned `time` crate to 0.3.47 to resolve `aws-smithy-types` build conflict
- Removed deprecated `ClientProjectsExt` and `ClientSecretsExt` imports from Bitwarden provider

### Security

- Fail-closed on database errors during client certificate validation
- Server certificate fingerprint verification during enrollment
- Enrollment tokens written to restricted file instead of logs
- Fixed `rustls-webpki` vulnerabilities (RUSTSEC-2026-0098, RUSTSEC-2026-0099, RUSTSEC-2026-0104) by disabling legacy TLS in AWS SDK crates
- Added `cargo-deny` for dependency license and advisory enforcement
- Added `deny.toml` with documented advisory ignores for unfixable upstream issues

## [1.4.0] - 2026-03-01

### Added

- Initial GCP Secret Manager provider
- File permission checks for sensitive PKI material

### Changed

- Improved error messages throughout the CLI
- Reduced dependency footprint

### Fixed

- Rollback handling for edge cases
- Delete and create command consistency

## [1.3.1] - 2026-02-25

### Changed

- Updated Bitwarden SDK to v2

## [1.3.0] - 2026-02-24

### Added

- Interactive config generation with provider discovery
- Credential encryption with passphrase or SSH key
- Session caching with OS keychain integration

## [1.2.7] - 2026-02-22

### Changed

- Updated dependencies and Cargo.lock
- Improved Home Manager module

## [1.2.6] - 2026-02-08

### Added

- `jaws version` command

### Changed

- Unified secret command operations with TUI integration
- Simplified config command layout
- Improved version control display

## [1.2.5] - 2026-02-07

### Changed

- Updated `ff` dependency to 1.0.10

## [1.2.4] - 2026-02-07

### Fixed

- TUI flickering during rapid updates
- AWS provider now correctly uses profile from config

## [1.2.3] - 2026-02-06

### Fixed

- 1Password push command handling

## [1.2.2] - 2026-02-04

### Added

- `||` operator and default values for template injection

### Changed

- Updated README with injection examples

## [1.2.1] - 2026-02-02

### Added

- Bitwarden Secrets Manager support
- Release automation script
- Home Manager Nix module

### Fixed

- Config generation tool edge cases

## [1.2.0] - 2026-02-01

### Added

- Bitwarden provider (initial support)
- Local secret history and caching
- Export and import with age encryption
- Additional config file locations
- `jaws` local provider for self-hosted secrets

### Changed

- Reworked file organization (local actions to `JawsSecretsManager`)
- Removed `--editor` and `--secrets-path` CLI flags in favor of config file
- Simplified config command structure

### Fixed

- Nix build date formatting
- Security dependency update (golang.org/x/net)
- Config location detection
- Tilde expansion in secrets path

## [1.0.8] - 2024-03-26

### Fixed

- Build errors in release mode

## [1.0.7] - 2024-03-26

### Changed

- Dependency updates

## [1.0.6] - 2024-03-26

### Changed

- Internal build script updates

## [1.0.5] - 2024-03-26

### Changed

- Migrated from private project to public repository

## [0.1.3] - 2022-07-16

### Fixed

- Config creation when existing config is broken

### Changed

- New subcommand aliases
- Raw version flag support
- Third-party library updates

## [0.1.2] - 2022-07-16

### Added

- Curlable install script

## [0.1.1] - 2022-07-16

### Added

- `version` command

### Fixed

- Editor flag now opens files correctly

## [0.1.0] - 2022-07-16

### Added

- Initial project setup
- Config create command
- AWS Secrets Manager provider
- Secret download and print
- Nested folder organization

[Unreleased]: https://github.com/jacbart/jaws/compare/v1.4.0...HEAD
[1.4.0]: https://github.com/jacbart/jaws/compare/v1.3.1...v1.4.0
[1.3.1]: https://github.com/jacbart/jaws/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/jacbart/jaws/compare/v1.2.7...v1.3.0
[1.2.7]: https://github.com/jacbart/jaws/compare/v1.2.6...v1.2.7
[1.2.6]: https://github.com/jacbart/jaws/compare/v1.2.5...v1.2.6
[1.2.5]: https://github.com/jacbart/jaws/compare/v1.2.4...v1.2.5
[1.2.4]: https://github.com/jacbart/jaws/compare/v1.2.3...v1.2.4
[1.2.3]: https://github.com/jacbart/jaws/compare/v1.2.2...v1.2.3
[1.2.2]: https://github.com/jacbart/jaws/compare/v1.2.1...v1.2.2
[1.2.1]: https://github.com/jacbart/jaws/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/jacbart/jaws/compare/v1.0.8...v1.2.0
[1.0.8]: https://github.com/jacbart/jaws/compare/v1.0.7...v1.0.8
[1.0.7]: https://github.com/jacbart/jaws/compare/v1.0.6...v1.0.7
[1.0.6]: https://github.com/jacbart/jaws/compare/v1.0.5...v1.0.6
[1.0.5]: https://github.com/jacbart/jaws/compare/v0.1.3...v1.0.5
[0.1.3]: https://github.com/jacbart/jaws/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/jacbart/jaws/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/jacbart/jaws/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/jacbart/jaws/releases/tag/v0.1.0
