#!/usr/bin/env bash
set -euo pipefail
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
cd "$(dirname "$0")/.."
profile="${VELA_WEB_PROFILE:-release}"
wasm_cli="${VELA_WASM_BINDGEN:-wasm-bindgen}"
if ! command -v "$wasm_cli" >/dev/null 2>&1; then
  echo "Install wasm-bindgen-cli version 0.2.129 (matching Cargo.lock), or set VELA_WASM_BINDGEN to its executable." >&2
  exit 1
fi
if [[ "$profile" == "release" ]]; then
  cargo build --locked -p vela-web --target wasm32-unknown-unknown --release
  wasm_dir=release
elif [[ "$profile" == "dev" ]]; then
  cargo build --locked -p vela-web --target wasm32-unknown-unknown
  wasm_dir=debug
else
  echo 'VELA_WEB_PROFILE must be release or dev' >&2
  exit 1
fi
mkdir -p dist/pkg
"$wasm_cli" "target/wasm32-unknown-unknown/$wasm_dir/vela-web.wasm" --target web --out-dir dist/pkg --out-name vela_web
cp apps/web/index.html apps/web/style.css apps/web/now-playing.css apps/web/bootstrap.js apps/web/player.js apps/web/vela.svg dist/
mkdir -p dist/assets/fonts
cp apps/web/assets/fonts/manrope-latin.woff2 apps/web/assets/fonts/manrope-cyrillic.woff2 apps/web/assets/fonts/OFL.txt dist/assets/fonts/
mkdir -p dist/assets/brand
cp apps/web/assets/brand/vela-coast.png apps/web/assets/brand/vela-wave.png dist/assets/brand/
echo 'Web assets built in dist/'
