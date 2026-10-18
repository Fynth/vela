#!/usr/bin/env bash
# Build a portable local Linux desktop review package. Never publishes anything.
set -euo pipefail
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
cd "$(dirname "$0")/.."
version="$(cargo metadata --locked --no-deps --format-version=1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="vela-desktop"))')"
host_triple="$(rustc -vV | sed -n 's/^host: //p')"
if [[ -n "${CARGO_BUILD_TARGET:-}" && "$CARGO_BUILD_TARGET" != "$host_triple" ]]; then
  echo 'Package on the native build host; cross-target packaging needs separate validation.' >&2
  exit 1
fi
if [[ -n "$(git status --porcelain)" ]]; then
  echo 'Commit the verified checkpoint before packaging the desktop binary.' >&2
  exit 1
fi
cargo build --locked --release -p vela-desktop
archive="target/desktop-releases/vela-desktop-$version-linux-$host_triple.tar.gz"
if [[ -e "$archive" || -e "$archive.sha256" ]]; then
  echo 'Desktop artifacts already exist; preserve or move them before rebuilding.' >&2
  exit 1
fi
stage="$(mktemp -d /tmp/vela-desktop-package-XXXXXX)"
trap 'rm -rf "$stage"' EXIT
install -Dm755 target/release/vela-desktop "$stage/bin/vela-desktop"
install -Dm644 apps/desktop/vela-desktop.desktop "$stage/share/applications/vela-desktop.desktop"
install -Dm644 apps/web/vela.svg "$stage/share/icons/hicolor/scalable/apps/vela-desktop.svg"
install -Dm644 apps/desktop/README.md "$stage/share/doc/vela-desktop/README.md"
install -Dm644 LICENSE "$stage/LICENSE"
commit="$(git rev-parse HEAD)"
printf 'version=%s\ntarget=%s\ncommit=%s\nruntime=GTK3, WebKitGTK 4.1, ALSA, libxdo\n' "$version" "$host_triple" "$commit" > "$stage/BUILD.txt"
mkdir -p target/desktop-releases
tar -C "$stage" -czf "$archive" .
sha256sum "$archive" > "$archive.sha256"
chmod 0644 "$archive" "$archive.sha256"
echo "Created local desktop review package: $archive"
