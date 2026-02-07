#!/usr/bin/env zsh

set -euo pipefail

cd "$(dirname "$0")"

if ! command -v npm >/dev/null 2>&1; then
  echo "Error: npm is required but not found."
  exit 1
fi

if ! command -v npx >/dev/null 2>&1; then
  echo "Error: npx is required but not found."
  exit 1
fi

old_version="$(node -p "require('./package.json').version")"
new_version="$(npm version patch --no-git-tag-version)"
new_version="${new_version#v}"

echo "Version bumped: ${old_version} -> ${new_version}"

npx vsce package --allow-package-all-secrets
