#!/usr/bin/env bash
# Generate docs/commands.md from `jaws --help` output.
# Run this script after adding new CLI commands or flags.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT="$PROJECT_ROOT/docs/commands.md"

# Find the jaws binary
JAWS_BIN=""
if [ -f "$PROJECT_ROOT/target/release/jaws" ]; then
    JAWS_BIN="$PROJECT_ROOT/target/release/jaws"
elif [ -f "$PROJECT_ROOT/target/debug/jaws" ]; then
    JAWS_BIN="$PROJECT_ROOT/target/debug/jaws"
else
    echo "No jaws binary found. Building first..."
    (cd "$PROJECT_ROOT" && cargo build --quiet)
    JAWS_BIN="$PROJECT_ROOT/target/debug/jaws"
fi

help_output() {
    "$JAWS_BIN" "$@" --help 2>/dev/null || true
}

cat > "$OUTPUT" << 'HEADER'
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

HEADER

# Top-level help
echo '```text' >> "$OUTPUT"
help_output >> "$OUTPUT"
echo '```' >> "$OUTPUT"
echo "" >> "$OUTPUT"

# Helper to emit a section
emit_cmd() {
    local cmd="$1"
    local title="$2"
    echo "### $title" >> "$OUTPUT"
    echo "" >> "$OUTPUT"
    echo '```text' >> "$OUTPUT"
    help_output $cmd >> "$OUTPUT"
    echo '```' >> "$OUTPUT"
    echo "" >> "$OUTPUT"
}

echo "## Secret Operations" >> "$OUTPUT"
echo "" >> "$OUTPUT"
emit_cmd "pull" "jaws pull"
emit_cmd "push" "jaws push"
emit_cmd "create" "jaws create"
emit_cmd "delete" "jaws delete"
emit_cmd "list" "jaws list"
emit_cmd "sync" "jaws sync"

echo "## Remote Secret Sharing" >> "$OUTPUT"
echo "" >> "$OUTPUT"
emit_cmd "serve" "jaws serve"
emit_cmd "connect" "jaws connect"
emit_cmd "disconnect" "jaws disconnect"

echo "## Version Control" >> "$OUTPUT"
echo "" >> "$OUTPUT"
emit_cmd "log" "jaws log"
emit_cmd "rollback" "jaws rollback"

echo "## Archive Operations" >> "$OUTPUT"
echo "" >> "$OUTPUT"
emit_cmd "export" "jaws export"
emit_cmd "import" "jaws import"

echo "## Configuration" >> "$OUTPUT"
echo "" >> "$OUTPUT"
emit_cmd "config" "jaws config"
emit_cmd "config init" "jaws config init"
emit_cmd "config get" "jaws config get"
emit_cmd "config set" "jaws config set"
emit_cmd "config provider" "jaws config provider"
emit_cmd "config clear-cache" "jaws config clear-cache"

echo "## Maintenance" >> "$OUTPUT"
echo "" >> "$OUTPUT"
emit_cmd "clean" "jaws clean"
emit_cmd "version" "jaws version"

# Footer
cat >> "$OUTPUT" << 'FOOTER'
---

See also:
- [Getting Started](getting-started.md) — common workflows
- [Configuration](configuration.md) — `jaws.kdl` format and providers
- [Remote Sharing](remote-sharing.md) — `jaws serve` and `jaws connect`
FOOTER

echo "Generated $OUTPUT"
