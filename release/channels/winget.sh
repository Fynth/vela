#!/usr/bin/env bash
# Submit a winget manifest for the Windows desktop build.
set -euo pipefail
artifact="${1:?artifact path}"
target="${2:?target id}"
: "${WINGET_TOKEN:?WINGET_TOKEN is required to publish to winget}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
manifests="$root/release/winget/$target"
[ -d "$manifests" ] || { echo "no winget manifest for $target" >&2; exit 1; }
echo "submitting $target to winget with the configured token"
