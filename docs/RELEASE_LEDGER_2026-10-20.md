# Release ledger — 2026-10-20 source checkpoint

Status: **source and local-artifact evidence**, not a public deployment or a new
version. Frozen runtime `0.12.6` and the original `data/` remain the live baseline.

## Source

- `main` at `ba0209d` (TR11 docs) over `e4c425d` (TR10 docs) over `4ceafdc`
  (TR09 dismiss fix) — the package was built from `4ceafdc`.
- `git diff --stat 4ceafdc..HEAD -- . ':!docs'` is empty: every commit after the
  packaged commit is documentation only, so the artifact's behavior matches HEAD.
- Live runtime: `vela-local-development.service`, MainPID `185315`, `NRestarts 0`,
  `/health` `{"status":"ok","version":"0.12.6"}`, SQLite schema12, original `data/`
  (424 MiB) not modified by TR10/TR11 runs (no files newer than the run start).

## Desktop artifact

| Field | Value |
| --- | --- |
| Path | `target/desktop-releases/vela-desktop-0.12.6-linux-x86_64-unknown-linux-gnu.tar.gz` |
| Size / SHA256 | 5,323,519 B / `58da18db0f0af4986ab1069840682f2e673f3024f5137fe25241820c00a367ac` |
| Version / BUILD | `0.12.6` / `4ceafdc482db1789159e6bae53f7015769b9b700` |
| Manifest | `BUILD.txt`, `LICENSE`, `bin/vela-desktop`, `.desktop`, scalable icon, README |
| Launch verification | `scripts/verify_desktop_package.py --launch` exit0, real `Vela Music` window, normal close, 0 temporary originals, `ldd` clean (`target/checks/tr10-package-verify.json`) |
| Preserved predecessor | `target/desktop-releases/previous/…tar.gz`, SHA256 `7abe3982073470d36e4680b56845aa6858a0c21b3c86f2007862ba9ba16d6063`, BUILD `7fe1ca0` |

The version number is intentionally unchanged, so two `0.12.6` desktop archives
exist; `BUILD.txt` commit is the discriminator. The packaged binary contains both
embedded Manrope data-URI faces; the frozen server runtime was not upgraded.

## Assets and build outputs

| Artifact | SHA256 |
| --- | --- |
| `apps/web/assets/fonts/manrope-latin.woff2` | `2436b9c2b6fe71529530bb1e5b0139d9a9af81b9507bc1b4e09113d3119b5315` |
| `apps/web/assets/fonts/manrope-cyrillic.woff2` | `d00bd0cdc67a28c770972d97c6a5f85bbed7f66c99541135d0ec967d11e77e87` |
| `apps/web/assets/brand/vela-coast.png` | `3e9fe7014f03512f64e43601a16f749d0ec3e683023c782579f34623be4732c0` |
| `apps/web/assets/brand/vela-wave.png` | `ff27654ab61791306dca4bae75c2fdf8aba38adc607da7ce90c2fce0d2453db8` |
| `apps/web/style.css` (= `dist/style.css`) | `df1b3d48aa11740f3ee833d6340cf46ff6e955a5c008d3e9763b7d6a942abf1e` |
| `apps/web/player.js` (= `dist/player.js`) | `8cf420538671cdb9f45c7e76a7029ad3993e1bf96b76516b553e6fe62c8ad55a` |
| `dist/pkg/vela_web_bg.wasm` (release, wasm-bindgen 0.2.129) | `9d9251092581ff7ea6b8a137203a8fd652800bdcb1cf0a3b68c728061a72371f` |

`dist/style.css` and `dist/player.js` are byte-identical to source; the release WASM
hash matches the TR11 browser/native runs. Fonts are self-hosted in both clients and
covered by `apps/web/assets/fonts/OFL.txt`.

## Executed checks (summary)

- Format and strict Clippy (workspace host, WASM, native QA): PASS.
- `cargo test --locked --workspace --no-fail-fast`: **345 passed / 0 failed / 3
  ignored** (opt-in audio/MPRIS/offline measurement).
- Chromium and Firefox: **56 passed / 1 skipped / 0 failed** each; the skip is the
  opt-in live network import.
- Native X11 peer/session QA: **260 checks PASS**, one AudioEngine, one owned null-sink
  stream, zero temporary originals; Wayland `--window-only`: **5 checks PASS**.
- Measured startup/HTTP/browser timing and visual width sweep recorded in
  `docs/QA_EVIDENCE.md` (single-host lower bounds, not capacity claims).

## Handoff and remaining work

NOT_RUN / unverified: physical/hardware audio output, screen readers/orca, physical
media keys, WebKit/Safari, Windows/macOS/Android/iOS, Docker, Internet/public-CA
federation, full Wayland resize/peer/offline, packaged-runtime rerun, real touch and
OS keyboard hardware, localization, Better Design VALID receipts, public deployment or
push. TR10 fonts are source/packaged only for Linux desktop; the live web/server
runtime remains the frozen `0.12.6` build from `7fe1ca0`.

A future release should bump the version before rebuilding the desktop archive,
regenerate this ledger, rerun TR11, and only then publish with explicit authorization.
