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
VELA_BIND=127.0.0.1:3217 VELA_DATA_DIR="$fixture_dir" VELA_WEB_DIR=dist "$binary" &
server_pid=$!
wait "$server_pid"
