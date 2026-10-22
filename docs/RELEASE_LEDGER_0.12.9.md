# Release ledger — 0.12.9

Tagged source: `e091a5ef43d7b5b9ebde92a9cd493dcbd3d965be` (`v0.12.9`).
Published: 2026-10-22. Release run
[39795757802](https://github.com/Fynth/vela/actions/runs/39795757802) — every job
that the tag needed reported `success`: `quality / rust`, `quality / dependencies`,
`packages / Linux tar.gz, deb, AppImage and AUR`, `packages / Android arm64 APK`,
`packages / Build AUR recipe using makepkg`, `verify`, `github-release`, `aur`.

## Verification performed on this host

The nine artifacts were downloaded from the public release and each SHA-256 in
`SHA256SUMS` was checked against the downloaded bytes; all nine matched. The AUR
RPC endpoint reports `vela-music-bin` at version `0.12.9-1`, last modified
2026-10-22T15:01:48Z.

## Published artifacts

| Artifact | SHA-256 |
| --- | --- |
| `vela-0.12.9-android-arm64.apk` | `201371c948d224db64e9e226fd6262a4e7dfa72ef8db63572da02a6851de54ad` |
| `vela-desktop-0.12.9-linux-x86_64-local-review.tar.gz` | `bc041eb70930e4609b225a2e87d9cfdb3400a1fd7e9859d1cf0ae7afa12157c2` |
| `vela-desktop-0.12.9-windows-x86_64-winget.zip` | `eadb6f6d1580e10b2a6ba0b5a87753ea035a9929728a8f638aef79b584c04b15` |
| `vela-desktop-0.12.9-windows-x86_64.exe` | `e95ae9f03a81510f7794912632b3a5e2e96577d6b820b2c89f9147a622d7d644` |
| `vela-desktop-0.12.9-x86_64.AppImage` | `58268e0538d4362be09917676650ba2be530c6dc78ab9a3fe2556714d773d0a1` |
| `vela-desktop_0.12.9_amd64.deb` | `f0bfa6189540aa73389358b3cca49a1b91f5dae87941e2f3945ec3e23ba2821e` |
| `vela-music-bin-0.12.9-x86_64.tar.gz` | `f661155c2708a6ba2b981aacb4e39c1edb925b32def0e92280616ea1f81cbf1f` |
| `vela-server-0.12.9-linux-x86_64-local-review.tar.gz` | `34522711fc01f0e307111cc2173bf5452f739991dc34cdc6017785250508cbf8` |
| `vela-web-0.12.9-linux-x86_64-local-review.tar.gz` | `5c0ca5f3e7da9901d0d30d4868904ceef704a6faa4165ef400e587b0a0d6ac83` |

## What changed since 0.12.8

- DR-04: the dense desktop composition. Every interactive target reaches 44px,
  the transport is a tiled band, the section heading is 20px, and the layout
  follows the width the content really has at app sizes 80/100/150.
- DR-05: readable empty states. The light-theme title contrast moved from
  1.114:1 to 15.71:1 and the subtitle from 1.622:1 to 7.65:1; one feedback
  stack replaces the overlapping surfaces.
- DR-07: a keyboard reader can drive the queue drawer. The focus ring moved
  from 1.315:1 to 5.74:1 light and 12.54:1 dark, the timer leaves the live
  region, and Escape returns focus to the trigger.
- The audit round that rejected the first slice: the duplicated duration
  formatter, the wrong `closer` comment, the root receipt that arrived with
  its own text, and the two documentation tables that did not match the
  shipped CSS are all fixed.
- `fix(web)`: the keyboard ring survives the query container the desktop
  composition needs.

## Checks behind this release, as run on this host

`cargo fmt --all -- --check` exit 0; strict workspace clippy exit 0; workspace
tests 380 passed, 0 failed, 3 ignored; release WASM and `scripts/build-web.sh`
exit 0; the full Chromium suite 68 passed, 1 skipped; the full Firefox suite
68 passed, 1 skipped. The dist files were byte-compared with their sources.

## Not run

Screen-reader output, forced-colors mode, 200% browser zoom with 150% app scale
in the native WebKitGTK shell, physical audio output, and the Docker job are
not covered by this host. The release workflow's own `quality / docker` job is
the only Docker evidence, and it is the workflow's, not this host's.
