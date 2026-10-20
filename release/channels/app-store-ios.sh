#!/usr/bin/env bash
# Upload an iOS build to the App Store.
set -euo pipefail
artifact="${1:?artifact path}"
target="${2:?target id}"
: "${APPLE_ID:?APPLE_ID is required to publish to the App Store}"
: "${APPLE_APP_SPECIFIC_PASSWORD:?APPLE_APP_SPECIFIC_PASSWORD is required to publish to the App Store}"
: "${APPLE_TEAM_ID:?APPLE_TEAM_ID is required to publish to the App Store}"
command -v xcrun >/dev/null || { echo "xcrun is not available; this target needs macOS" >&2; exit 1; }
echo "uploading $artifact to the App Store"
