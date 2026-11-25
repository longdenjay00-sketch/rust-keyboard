#!/usr/bin/env bash
# bump-version.sh — Semantic version manager for rust-keyboard
# Usage:
#   ./bump-version.sh [patch|minor|major|<X.Y.Z>] [--no-deb] [--push]
set -euo pipefail

cd "$(dirname "$0")"

CUR_VER="$(awk -F'"' '/^version *=/ {print $2; exit}' Cargo.toml)"

if [[ $# -eq 0 ]]; then
  echo "Current version: v${CUR_VER}"
  echo "Git tags:"
  git tag -l -n1
  echo ""
  echo "Usage: ./bump-version.sh <patch|minor|major|X.Y.Z> [--no-deb] [--push]"
  exit 0
fi

TARGET="$1"
shift

BUILD_DEB=1
DO_PUSH=0
for arg in "$@"; do
  case "$arg" in
    --no-deb) BUILD_DEB=0 ;;
    --push)   DO_PUSH=1 ;;
    *) echo "Unknown option: $arg" >&2; exit 1 ;;
  esac
done

IFS='.' read -r MAJOR MINOR PATCH <<< "$CUR_VER"
case "$TARGET" in
  patch) NEW_VER="${MAJOR}.${MINOR}.$((PATCH + 1))" ;;
  minor) NEW_VER="${MAJOR}.$((MINOR + 1)).0" ;;
  major) NEW_VER="$((MAJOR + 1)).0.0" ;;
  [0-9]*.[0-9]*.[0-9]*) NEW_VER="$TARGET" ;;
  *)
    echo "Invalid version target: '$TARGET' (use patch, minor, major, or X.Y.Z)" >&2
    exit 1
    ;;
esac

echo "[*] Bumping rust-keyboard: v${CUR_VER} -> v${NEW_VER}"

# 1. Update Cargo.toml
sed -i "0,/^version = \".*\"/s//version = \"${NEW_VER}\"/" Cargo.toml

# 2. Verify build & tests (updates Cargo.lock)
cargo test --quiet
cargo build --release --quiet

# 3. Prepend CHANGELOG.md section if not already present
TODAY="$(date -u +%Y-%m-%d)"
if ! grep -q "## \[${NEW_VER}\]" CHANGELOG.md 2>/dev/null; then
  awk -v ver="$NEW_VER" -v dt="$TODAY" '
    /^## \[/ && !inserted {
      printf "## [%s] - %s\n\n### Changed\n- Version bump to v%s.\n\n---\n\n", ver, dt, ver
      inserted=1
    }
    { print }
  ' CHANGELOG.md > CHANGELOG.md.tmp && mv CHANGELOG.md.tmp CHANGELOG.md
fi

# 4. Optionally rebuild .deb package
if [[ "$BUILD_DEB" -eq 1 ]]; then
  ./build-deb.sh
fi

# 5. Atomic commit & annotated tag
git add Cargo.toml Cargo.lock CHANGELOG.md spec/ android/
git commit -m "Release rust-keyboard v${NEW_VER}" -m "Bump version from v${CUR_VER} to v${NEW_VER} and update release artifacts."
git tag -a "v${NEW_VER}" -m "rust-keyboard v${NEW_VER}"

echo "[✓] Committed and tagged v${NEW_VER}"

if [[ "$DO_PUSH" -eq 1 ]]; then
  git push origin main --follow-tags
  echo "[✓] Pushed main and v${NEW_VER} to origin"
fi
