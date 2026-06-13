#!/usr/bin/env bash
set -euo pipefail

# Generate all JAWS demo GIFs using VHS.
# Usage: ./scripts/demo.sh
#
# Requirements (available in the nix dev shell):
#   - vhs
#   - ttyd
#   - ffmpeg
#   - jaws (built and on PATH)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$PROJECT_ROOT/assets"

# Check dependencies
for cmd in vhs ttyd ffmpeg jaws; do
	if ! command -v "$cmd" &>/dev/null; then
		echo "Error: '$cmd' is not on PATH."
		echo "Enter the nix dev shell first: nix develop"
		exit 1
	fi
done

mkdir -p "$OUTPUT_DIR"

# Generate each demo tape
for tape in "$SCRIPT_DIR"/demo/*.tape; do
	if [ -f "$tape" ]; then
		name=$(basename "$tape" .tape)
		echo "Recording demo-$name.gif..."
		(cd "$PROJECT_ROOT" && vhs "$tape")
	fi
done

# Also regenerate the main demo GIF
if [ -f "$SCRIPT_DIR/demo.tape" ]; then
	echo "Recording demo.gif (main)..."
	(cd "$PROJECT_ROOT" && vhs "$SCRIPT_DIR/demo.tape")
fi

echo ""
echo "Done! Generated GIFs:"
ls -lh "$OUTPUT_DIR"/demo*.gif 2>/dev/null || true
