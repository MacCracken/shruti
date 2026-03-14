#!/bin/bash
# Shruti Version Bump Script
# CalVer: YYYY.M.D or YYYY.M.D-N for same-day patches
# Usage: ./bump-version.sh <new_version>
# Example: ./bump-version.sh 2026.3.14
# Example: ./bump-version.sh 2026.3.14-1

set -e

if [ -z "$1" ]; then
    echo "Current version: $(cat VERSION)"
    echo "Usage: $0 <new_version>"
    echo "Example: $0 2026.3.14"
    echo "Example: $0 2026.3.14-1  (same-day patch)"
    exit 1
fi

NEW_VERSION="$1"
OLD_VERSION=$(cat VERSION | tr -d '[:space:]')

# Cargo.toml only supports SemVer — strip the -N patch suffix
CARGO_VERSION="${NEW_VERSION%-*}"

echo "Bumping version: $OLD_VERSION -> $NEW_VERSION"
echo "Cargo version:   $CARGO_VERSION"

# Update VERSION file (full calver with patch)
echo "$NEW_VERSION" > VERSION

# Update all Cargo.toml files
OLD_CARGO="${OLD_VERSION%-*}"
for toml in Cargo.toml crates/*/Cargo.toml; do
    sed -i "s/^version = \"$OLD_CARGO\"/version = \"$CARGO_VERSION\"/" "$toml"
done

echo ""
echo "Updated files:"
echo "  VERSION          -> $NEW_VERSION"
echo "  Cargo.toml (all) -> $CARGO_VERSION"
echo ""
echo "Don't forget to update CHANGELOG.md!"
