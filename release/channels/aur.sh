#!/usr/bin/env bash
# Push only a makepkg-validated recipe. Release assets must already exist.
set -euo pipefail
recipes="${1:?directory containing PKGBUILD and .SRCINFO}"
package="${2:?AUR package name}"
: "${AUR_SSH_KEY:?Add the AUR account SSH private key as GitHub secret AUR_SSH_KEY}"
: "${AUR_REPO:=vela-music-bin}"
: "${AUR_KNOWN_HOSTS:?Set AUR_KNOWN_HOSTS to the verified aur.archlinux.org SSH host key}"
[ "$package" = vela-music-bin ] && [ "$AUR_REPO" = vela-music-bin ] || { echo 'Unexpected AUR package name' >&2; exit 1; }
recipes="$(realpath "$recipes")"
test -s "$recipes/PKGBUILD"
test -s "$recipes/.SRCINFO"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
chmod 700 "$work"
printf '%s\n' "$AUR_SSH_KEY" > "$work/key"
printf '%s\n' "$AUR_KNOWN_HOSTS" > "$work/known_hosts"
chmod 600 "$work/key" "$work/known_hosts"
export GIT_SSH_COMMAND="ssh -i $work/key -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$work/known_hosts"
git clone "ssh://aur@aur.archlinux.org/vela-music-bin.git" "$work/repo"
cp "$recipes/PKGBUILD" "$recipes/.SRCINFO" "$work/repo/"
git -C "$work/repo" add PKGBUILD .SRCINFO
if git -C "$work/repo" diff --cached --quiet; then
  echo 'AUR already contains this recipe'
  exit 0
fi
git -C "$work/repo" -c user.name=Fynth -c user.email=release@users.noreply.github.com commit -m "Update vela-music-bin"
git -C "$work/repo" push origin HEAD:master
