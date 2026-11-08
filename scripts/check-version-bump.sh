#!/usr/bin/env bash
# Enforces RULES.md §8: every push that changes the crate must raise `version` in Cargo.toml.
#
# Usage: scripts/check-version-bump.sh [BASE_REV]   (default: origin/main)
# Exit 0 = version was raised (or nothing changed); exit 1 = rule violated.
set -euo pipefail

base="${1:-origin/main}"

if [[ "$base" =~ ^0+$ ]] || ! git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
    echo "version check: no base revision ($base); treating as a first push."
    exit 0
fi

version_at() {
    git show "$1:Cargo.toml" | sed -n 's/^version *= *"\([^"]*\)".*/\1/p' | head -n1
}

old="$(version_at "$base")"
new="$(version_at HEAD)"

if git diff --quiet "$base" HEAD; then
    echo "version check: no changes since $base."
    exit 0
fi

# Highest of the two by semver ordering; the new version must be strictly greater.
highest="$(printf '%s\n%s\n' "$old" "$new" | sort -V | tail -n1)"
if [[ "$old" == "$new" || "$highest" != "$new" ]]; then
    echo "error: Cargo.toml version is $new but $base already has $old." >&2
    echo "RULES.md §8: raise the version (and add a CHANGELOG.md entry) before every push." >&2
    exit 1
fi

if ! grep -q "^## \[$new\]" CHANGELOG.md; then
    echo "error: CHANGELOG.md has no '## [$new]' section." >&2
    exit 1
fi

echo "version check: $old -> $new"
