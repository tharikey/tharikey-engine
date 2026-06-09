#!/usr/bin/env bash
# Guided release version bump for the tharikey-engine workspace.
#
#   1. verifies your local `main` is clean and in sync with origin/main
#   2. prompts for the new version (shows the current one)
#   3. branches `release/<version>` off main and bumps the single [workspace.package] version
#   4. hands back to you to review → commit → push → open a PR
#
# It does NOT commit or tag. On merge to main, CI (.github/workflows/release.yml) creates the tag and
# publishes. Run from anywhere in the repo:  scripts/bump-version.sh
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# 1. preflight — clean tree + main in sync with origin/main
if [ -n "$(git status --porcelain)" ]; then
  echo "✗ working tree has uncommitted changes — commit or stash first." >&2
  exit 1
fi
echo "• fetching origin/main…"
git fetch -q origin main
if ! git rev-parse --verify -q main >/dev/null; then
  echo "✗ no local 'main' branch — run: git checkout main" >&2
  exit 1
fi
if [ "$(git rev-parse main)" != "$(git rev-parse origin/main)" ]; then
  echo "✗ local main is not in sync with origin/main — run: git checkout main && git pull" >&2
  exit 1
fi
echo "✓ main is clean and in sync with origin/main"

# 2. prompt for the new version
CURRENT=$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)
echo "current version: $CURRENT"
read -rp "new version (X.Y.Z): " VERSION
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "✗ '$VERSION' is not a valid X.Y.Z version." >&2
  exit 1
fi
if [ "$VERSION" = "$CURRENT" ]; then
  echo "✗ $VERSION is already the current version." >&2
  exit 1
fi
if git rev-parse --verify -q "v$VERSION" >/dev/null; then
  echo "✗ tag v$VERSION already exists." >&2
  exit 1
fi

# 3. branch off main + bump the single workspace version (crates inherit via version.workspace = true)
BRANCH="release/$VERSION"
git checkout -q -b "$BRANCH" main
sed -i.bak -E "s/^version = \"$CURRENT\"\$/version = \"$VERSION\"/" Cargo.toml && rm -f Cargo.toml.bak
cargo update -q --workspace   # sync Cargo.lock with the new workspace version
echo "✓ bumped $CURRENT → $VERSION on branch $BRANCH"

# 4. hand back to the user
cat <<EOF

Next:
  1. Add a '## $VERSION' entry to docs/changelog.md (curated — released, user-facing changes).
  2. Review:  git diff
  3. Commit:  git add -A && git commit -m "release: v$VERSION"
  4. Push/PR: git push -u origin $BRANCH && gh pr create --base main --fill

On merge to main, CI tags v$VERSION and publishes @tharikey/engine.
EOF
