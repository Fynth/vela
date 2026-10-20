#!/usr/bin/env bash
# Upload an Android App Bundle to Google Play.
set -euo pipefail
artifact="${1:?artifact path}"
target="${2:?target id}"
: "${PLAY_SERVICE_ACCOUNT_JSON:?PLAY_SERVICE_ACCOUNT_JSON is required to publish to Google Play}"
: "${ANDROID_KEYSTORE_BASE64:?ANDROID_KEYSTORE_BASE64 is required to sign the bundle}"
command -v bundletool >/dev/null || { echo "bundletool is not installed" >&2; exit 1; }
echo "uploading $artifact to Google Play"
