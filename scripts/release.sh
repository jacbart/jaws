#!/usr/bin/env bash
set -e

# Helper script to prepare a new release locally using Nix
# Usage: ./scripts/release.sh [version] [--build-only] [--cross-only]
# Example: ./scripts/release.sh 1.3.0
# Example: ./scripts/release.sh --build-only    # Only build cross-compiled binaries
# Example: ./scripts/release.sh --cross-only    # Only build cross-compiled binaries (alias)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$PROJECT_ROOT/dist"

# Cross-compilation targets
CROSS_TARGETS=(
	"x86_64-linux"
	"aarch64-linux"
	"x86_64-darwin"
	"aarch64-darwin"
)

# Parse arguments
BUILD_ONLY=false
VERSION=""

for arg in "$@"; do
	case $arg in
	--build-only | --cross-only)
		BUILD_ONLY=true
		;;
	*)
		VERSION="$arg"
		;;
	esac
done

# Function to build cross-compiled binaries
build_cross_targets() {
	echo "Building cross-compiled binaries..."
	echo "=================================="

	# Clean and create dist directory
	rm -rf "$DIST_DIR"
	mkdir -p "$DIST_DIR"

	for target in "${CROSS_TARGETS[@]}"; do
		echo ""
		echo "Building for $target..."
		echo "------------------------"

		target_dir="$DIST_DIR/jaws-$target"
		mkdir -p "$target_dir"

		if nix build ".#jaws-$target" --out-link "$target_dir/result"; then
			# Copy binary from nix store to dist directory
			cp -L "$target_dir/result/bin/jaws" "$target_dir/jaws"

			# Copy library if it exists
			if [ -d "$target_dir/result/lib" ]; then
				cp -rL "$target_dir/result/lib" "$target_dir/"
			fi

			# Remove the nix store symlink
			rm "$target_dir/result"

			# Create tarball
			echo "Creating archive for $target..."
			(cd "$DIST_DIR" && tar -czvf "jaws-$target.tar.gz" "jaws-$target")

			echo "Successfully built jaws for $target"
		else
			echo "WARNING: Failed to build for $target"
		fi
	done

	echo ""
	echo "Cross-compilation complete!"
	echo "Binaries available in: $DIST_DIR"
	echo ""
	ls -la "$DIST_DIR"/*.tar.gz 2>/dev/null || echo "No archives created"
}

# If --build-only flag is set, just build and exit
if [ "$BUILD_ONLY" = true ]; then
	build_cross_targets
	exit 0
fi

# Full release process requires version
if [ -z "$VERSION" ]; then
	echo "Usage: $0 <version> [--build-only]"
	echo ""
	echo "Options:"
	echo "  <version>     Version number for the release (e.g., 1.3.0)"
	echo "  --build-only  Only build cross-compiled binaries, skip release process"
	echo "  --cross-only  Alias for --build-only"
	echo ""
	echo "Examples:"
	echo "  $0 1.3.0              # Full release process"
	echo "  $0 --build-only       # Just build cross-compiled binaries"
	exit 1
fi

TAG="v$VERSION"

# Ensure clean working directory
if [ -n "$(git status --porcelain)" ]; then
	echo "Error: Working directory is not clean. Commit changes first."
	exit 1
fi

echo "Preparing release $VERSION..."

# 1. Update Cargo.toml
# Use sed to replace the version line. Assumes standard formatting.
sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# 2. Update Cargo.lock (requires cargo)
echo "Updating Cargo.lock..."
cargo check >/dev/null 2>&1 || true

# 3. Verify Build with Nix (native build)
echo "Verifying native build with Nix..."
# We need to make sure the flake uses the source with the updated Cargo.toml
# Nix flakes usually grab the current git HEAD, but since we haven't committed yet,
# we might need to rely on 'path:.' behavior or add the file to git intent.
git add Cargo.toml Cargo.lock

if nix build .#default --no-link; then
	echo "Native Nix build successful."
else
	echo "Native Nix build failed. Aborting release."
	git reset HEAD Cargo.toml Cargo.lock
	git checkout Cargo.toml Cargo.lock
	exit 1
fi

# 4. Build cross-compiled binaries
echo ""
echo "Building cross-compiled release binaries..."
build_cross_targets

# 5. Generate Changelog (optional)
if command -v git-cliff &>/dev/null; then
	echo "Generating changelog..."
	git cliff --tag "$TAG" >CHANGELOG.md
	git add CHANGELOG.md
else
	echo "git-cliff not found, skipping changelog."
fi

# 6. Commit and Tag
echo "Committing and tagging..."
git commit -m "chore(release): prepare for $TAG"
git tag -a "$TAG" -m "Release $TAG"

echo ""
echo "Done! Ready to push."
echo "Run: git push && git push --tags"
echo ""
echo "Release binaries are in: $DIST_DIR"
