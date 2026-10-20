#!/usr/bin/env bash
# Push a Homebrew cask for the macOS desktop build.
set -euo pipefail
artifact="${1:?artifact path}"
target="${2:?target id}"
: "${HOMEBREW_TAP_TOKEN:?HOMEBREW_TAP_TOKEN is required to publish to Homebrew}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
cask="$root/release/homebrew/Casks/vela.rb"
[ -f "$cask" ] || { echo "no Homebrew cask for $target" >&2; exit 1; }
echo "pushing $cask to the Homebrew tap"
