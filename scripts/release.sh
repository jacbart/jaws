#!/usr/bin/env bash
set -e

# Helper script to prepare a new release locally using Nix
# Usage: ./scripts/release.sh [version]
# Example: ./scripts/release.sh 1.3.0

if [ -z "$1" ]; then
  echo "Usage: $0 <version>"
  exit 1
fi

VERSION=$1
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

# 3. Verify Build with Nix
echo "Verifying build with Nix..."
# We need to make sure the flake uses the source with the updated Cargo.toml
# Nix flakes usually grab the current git HEAD, but since we haven't committed yet,
# we might need to rely on 'path:.' behavior or add the file to git intent.
git add Cargo.toml Cargo.lock

if nix build .#default --no-link; then
  echo "Nix build successful."
else
  echo "Nix build failed. Aborting release."
  git reset HEAD Cargo.toml Cargo.lock
  git checkout Cargo.toml Cargo.lock
  exit 1
fi

# 4. Generate Changelog (optional)
if command -v git-cliff &>/dev/null; then
  echo "Generating changelog..."
  git cliff --tag "$TAG" >CHANGELOG.md
  git add CHANGELOG.md
else
  echo "git-cliff not found, skipping changelog."
fi

# 5. Commit and Tag
echo "Committing and tagging..."
git commit -m "chore(release): prepare for $TAG"
git tag -a "$TAG" -m "Release $TAG"

echo "Done! Ready to push."
echo "Run: git push && git push --tags"
