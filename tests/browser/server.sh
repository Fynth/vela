#!/usr/bin/env bash
set -euo pipefail
cd ../..
fixture_dir="$(mktemp -d /tmp/vela-browser-XXXXXX)"
server_pid=''
cleanup() {
  if [[ -n "$server_pid" ]]; then kill -TERM "$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true; fi
  rm -rf "$fixture_dir"
}
trap cleanup EXIT
trap 'exit 0' TERM INT
binary="${VELA_TEST_BINARY:-target/debug/vela-server}"
mkdir -p "$fixture_dir/data" "$fixture_dir/music/album"
cp crates/media/tests/fixtures/signal.flac "$fixture_dir/music/album/"
cp crates/media/tests/fixtures/beacon.mp3 "$fixture_dir/music/"
scan_roots="$(python3 -c 'import json,sys; print(json.dumps([{"id":"fixture","name":"Browser music folder","path":sys.argv[1]}]))' "$fixture_dir/music")"
VELA_BIND=127.0.0.1:3217 VELA_DATA_DIR="$fixture_dir/data" VELA_WEB_DIR=dist VELA_SCAN_ROOTS="$scan_roots" "$binary" &
server_pid=$!
wait "$server_pid"
