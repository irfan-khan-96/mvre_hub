#!/usr/bin/env bash
set -euo pipefail

if ! command -v git >/dev/null 2>&1; then
  echo "git is required"
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required"
  exit 1
fi

if [ $# -ne 1 ]; then
  echo "Usage: scripts/release.sh vX.Y.Z"
  exit 1
fi

TAG="$1"

chmod +x "$0"

echo "Building release for $TAG"
cargo build --release

echo "Packaging"
mkdir -p dist
tar -czf dist/mvre-hub-linux-x86_64.tar.gz -C target/release mvre-hub
sha256sum dist/mvre-hub-linux-x86_64.tar.gz > dist/mvre-hub-linux-x86_64.sha256

echo "Tagging and pushing"
git tag "$TAG"
git push --tags

echo "Done. GitHub Actions will publish the release assets."
