#!/usr/bin/env bash
# Publish the AUR package recipe.
#
# The recipe and the source tarball are produced by scripts/release_packages.py
# next to the artifact. This step pushes them to the AUR git repository with the
# configured SSH key.
set -euo pipefail
artifact="${1:?artifact path}"
target="${2:?target id}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
recipes="$root/target/release-artifacts/aur"
: "${AUR_SSH_KEY:?AUR_SSH_KEY is required to publish to the AUR}"
: "${AUR_REPO:?AUR_REPO is required to publish to the AUR}"
[ -f "$recipes/PKGBUILD" ] || { echo "PKGBUILD is missing; run scripts/release_packages.py" >&2; exit 1; }
key="$(mktemp)"
trap 'rm -f "$key"' EXIT
printf '%s' "$AUR_SSH_KEY" > "$key"
chmod 600 "$key"
GIT_SSH_COMMAND="ssh -i $key -o StrictHostKeyChecking=accept-new" \
  git clone --depth 1 "aur@aur.archlinux.org:$AUR_REPO" "$recipes/.push"
cp "$recipes/PKGBUILD" "$recipes/.push/"
cd "$recipes/.push"
git add PKGBUILD
git -c user.name="vela-release" -c user.email="release@example.invalid" commit -m "vela-desktop $target"
GIT_SSH_COMMAND="ssh -i $key -o StrictHostKeyChecking=accept-new" git push
echo "published $target to the AUR"
