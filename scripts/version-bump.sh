#!/bin/bash
# Bump the version across all crates, VERSION file, and roadmap header.
#
# Usage:
#   ./scripts/version-bump.sh 2026.3.20
#
# Updates:
#   - VERSION file
#   - Root Cargo.toml [package] version
#   - All crates/*/Cargo.toml [package] versions
#   - docs/development/roadmap.md version header

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 2026.3.20"
    exit 1
fi

NEW_VERSION="$1"
OLD_VERSION=$(cat VERSION)

if [ "$NEW_VERSION" = "$OLD_VERSION" ]; then
    echo "Already at version $NEW_VERSION"
    exit 0
fi

echo "Bumping $OLD_VERSION → $NEW_VERSION"

# VERSION file
echo "$NEW_VERSION" > VERSION

# All Cargo.toml files (match only the [package] version line)
for toml in Cargo.toml crates/*/Cargo.toml; do
    if [ -f "$toml" ]; then
        sed -i "s/^version = \"$OLD_VERSION\"/version = \"$NEW_VERSION\"/" "$toml"
    fi
done

# Roadmap header
if [ -f docs/development/roadmap.md ]; then
    sed -i "s/> \*\*Version\*\*: $OLD_VERSION/> **Version**: $NEW_VERSION/" docs/development/roadmap.md
fi

# Verify
echo ""
echo "Updated files:"
echo "  VERSION: $(cat VERSION)"
grep -r "version = \"$NEW_VERSION\"" Cargo.toml crates/*/Cargo.toml | sed 's/^/  /'
echo ""
echo "Run 'cargo check' to verify."
