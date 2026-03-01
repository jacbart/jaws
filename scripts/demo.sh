#!/usr/bin/env bash
set -euo pipefail

# Generate the jaws demo GIF using VHS.
# Usage: ./scripts/demo.sh
#
# Requirements (available in the nix dev shell):
#   - vhs
#   - ttyd
#   - ffmpeg
#   - jaws (built and on PATH)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TAPE_FILE="$SCRIPT_DIR/demo.tape"
OUTPUT_DIR="$PROJECT_ROOT/assets"

# Check dependencies
for cmd in vhs ttyd ffmpeg jaws; do
	if ! command -v "$cmd" &>/dev/null; then
		echo "Error: '$cmd' is not on PATH."
		echo "Enter the nix dev shell first: nix develop"
		exit 1
	fi
done

if [ ! -f "$TAPE_FILE" ]; then
	echo "Error: Tape file not found at $TAPE_FILE"
	exit 1
fi

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

echo "Recording demo GIF..."
echo "  Tape:   $TAPE_FILE"
echo "  Output: $OUTPUT_DIR/demo.gif"
echo ""

(cd "$PROJECT_ROOT" && vhs "$TAPE_FILE")

if [ -f "$OUTPUT_DIR/demo.gif" ]; then
	SIZE=$(du -h "$OUTPUT_DIR/demo.gif" | cut -f1)
	echo ""
	echo "Done! Generated $OUTPUT_DIR/demo.gif ($SIZE)"
else
	echo ""
	echo "Error: GIF was not generated. Check vhs output above."
	exit 1
fi
