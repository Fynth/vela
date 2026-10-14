#!/usr/bin/env bash
# Local review artifacts only; never pushes or publishes anything.
set -euo pipefail
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
cd "$(dirname "$0")/.."
version=0.1.0
host_triple="$(rustc -vV | sed -n 's/^host: //p')"
if [[ -n "${CARGO_BUILD_TARGET:-}" && "$CARGO_BUILD_TARGET" != "$host_triple" ]]; then
  echo 'Package on the native build host; cross-target packaging needs separate validation.' >&2
  exit 1
fi
cargo build --locked --release -p vela-server
scripts/build-web.sh
python3 scripts/generate_notices.py
mkdir -p target/releases
archive="target/releases/vela-$version-$host_triple.tar.gz"
source_archive="target/releases/vela-$version-source.tar.gz"
if [[ -e "$archive" || -e "$source_archive" ]]; then
  echo 'Release artifacts already exist; preserve or move them before building a new checkpoint.' >&2
  exit 1
fi
package_dir="$(mktemp -d /tmp/vela-package-XXXXXX)"
trap 'rm -rf "$package_dir"' EXIT
cp target/release/vela-server "$package_dir/"
cp -R dist "$package_dir/dist"
cp -R docs "$package_dir/docs"
cp -R deploy "$package_dir/deploy"
cp LICENSE README.md "$package_dir/"
mkdir "$package_dir/scripts"
cp scripts/backup.py "$package_dir/scripts/"
# A commit is required so source and binary can be compared against a checkpoint.
if [[ -n "$(git status --porcelain)" ]]; then
  echo 'Commit the verified checkpoint before packaging its source archive.' >&2
  exit 1
fi
commit="$(git rev-parse HEAD)"
printf 'version=%s\ntarget=%s\ncommit=%s\n' "$version" "$host_triple" "$commit" > "$package_dir/BUILD.txt"
git archive --format=tar.gz --output="$source_archive" HEAD
tar -C "$package_dir" -czf "$archive" .
sha256sum "$archive" "$source_archive" > target/releases/SHA256SUMS
echo "Created local review artifacts in target/releases (commit $commit)."
