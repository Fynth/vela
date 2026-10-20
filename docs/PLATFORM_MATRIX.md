# Platform evidence

Latest source/debug repair2026-10-19 (baseline91867f2), separate from the package
matrix below: Chromium43 pass/1skip and Firefox43 pass/1skip; Linux X11 native
fixture89 checks/10 screenshots; actual native Wayland Hyprland normally tiled
fixture5 checks/1 screenshot, verified own PID/xwayland=false and DOM×DPR PNG size.
FUNCTIONAL_TESTED applies to the stated interaction/window subset only. Both native
runs use owned null sinks; HARDWARE_VERIFIED is not claimed. See
[interaction review](UX_INTERACTION_REVIEW_2026-10-19.md) and QA_EVIDENCE. No platform
package, frozen runtime or other-OS status below was upgraded by this source pass.

TR10 increment2026-10-20 (source `4ceafdc`; typography/QA `47fb518`, clippy `d314a2c`):
`apps/desktop/style.css` self-hosts the approved Manrope Latin+Cyrillic woff2 subsets
as two base64 `data:font/woff2` `@font-face` rules (weight200 800,
`font-display:swap`, per-subset `unicode-range`); `:root` leads with Manrope over the
unchanged system fallbacks, with no CDN, asset protocol, extra package file or network
font request. The real X11 window QA passed260 checks/15 screenshots: typography (both
subsets loaded, Cyrillic sample rendered with Manrope metrics), frameless window
controls plus drag region, app scale80-150 on the real stepper with clamping, and
exactly one `AudioEngine` output stream on the owned null sink. The portable package
was rebuilt from `4ceafdc` (version0.12.6, SHA256 `58da18db…`) and passed
`verify_desktop_package.py --launch` with a real window, exit0 and0 temporary
originals; the previous0.12.6 archive from `7fe1ca0` (SHA256 `7abe39…`) is preserved
under `target/desktop-releases/previous/`. Format/unit checks at `4ceafdc`: fmt and
strict Clippy pass, and116 desktop tests run with113 passed/3 opt-in ignored.
HARDWARE_VERIFIED, physical media keys, screen-reader certification, Internet/two-host
and every other platform remain NOT_RUN.

Updated2026-10-18; reconciled2026-10-19 (docs-only). A native server executable is not a desktop music client. Status applies only to the stated scope. COMPILES, RUNS, FUNCTIONAL_TESTED and HARDWARE_VERIFIED require separate evidence; RELEASE_READY is not inferred from any one of them.

Current deployed baseline is the frozen0.12.6 server runtime (schema12) and the0.12.6 native package built from `7fe1ca0`; the canonical0.12.6 desktop tar was later rebuilt from source `4ceafdc` for TR10 while the deployed package is preserved under `target/desktop-releases/previous/`; earlier0.12.x rows quoted in historical documents are evidence for those releases, not the current state. Current source HEAD `b0738cd` contains the TR10–TR12 evidence, the AUD-05/06 native offline work and the AUD-09 measured 100k performance fix (source schema13, deployed runtime still schema12); the live runtime remains frozen `7fe1ca0`, and the older `05b4c37` audit is historical — see [CURRENT_STATE.md](CURRENT_STATE.md) and [RELEASE_LEDGER](RELEASE_LEDGER_2026-10-20.md). New analysis jobs use `vela-pcm-spectral-4`; retained rows keep their recorded extractor version (the live20rows are `spectral-1`).

| Target | Actual implementation | Latest established evidence | Status / remaining work |
| --- | --- | --- | --- |
| Linux x86_64 server | Axum/SQLite/native binary | Frozen0.12.6 source `7fe1ca0`/enabled user runtime `target/runtime/vela-0.12.6`, schema12, original1user/20tracks/20assets/20analysis/0playlists/0folders/0publications, `vela-pcm-spectral-4` for new analysis jobs (retained rows are `spectral-1`), verified pre-upgrade backup/restore; read-only2026-10-19 health0.12.6, migration12/quick_check ok | FUNCTIONAL_TESTED for documented server scope |
| Web Chromium / Firefox Linux | Dioxus WASM, one HTMLMediaElement, Media Session |0.12.6 exact package:41 scenarios per engine (Chromium41 pass1 skip1.9m, Firefox41 pass1 skip2.2m); spectral-4 tempo/chroma/key disclosure, sleep timer/queue/playlist/folders, history/DSP/radio/publication/player regressions; assets match frozen package. Source preview-cursor `Load more` is not deployed and has no successful-preview browser scenario | FUNCTIONAL_TESTED for packaged scenarios; physical sound/background/Safari unverified |
| Local DSP / track radio | Symphonia0.6.1/RustFFT6.4.1, spectral-4 |0.12 local source:22 analysis tests incl chroma/Krumhansl key, full media tests, Chromium G08/G09 and the0.12.6 full41-pass+1 skip exact-package suite | FUNCTIONAL_TESTED for bounded descriptive analysis/radio; no calibrated beat/key/chord, real-music tempo/key accuracy or native audio implied |
| Linux desktop client | Dioxus0.7.10 + Rodio/CPAL0.22.2 + MPRIS2; apps/desktop |0.12.6 native source/package `7fe1ca0`:30 desktop tests (28 run,2 opt-in) + owned-null-sink audio + private bounded-channel D-Bus MPRIS tests; portable tar SHA256 `7abe39…`; real WebKitGTK window1280×840/640×700 against isolated fixture server/media; play/pause/seek/queue/search/favorite-unfavorite/Favorites view/album+artist browse/playlist create/rename/delete/entry add+reorder+remove+playback, folders create/move/filter/rename/delete (web+native), private offline copies with queue Previous/Next and confirmed removal, played with server stopped, queue down/clear-upcoming, player favorite, unified search across two public peers (one HTTPS loopback test-CA transport) + origin-labelled remote next/previous + saved peers + private browse-only cached fallback with age/Retry live + preview and per-peer search cursor pagination (debug loopback QA)/theme/logout/reconnect; unnamed0600 source files (zero directory entries during transfer/exit), opt-in Secret Service session token (explicit resume/sign-out deletion); TR10 source `4ceafdc` (`47fb518`/`d314a2c`): embedded Manrope data-URI faces + X11 native QA260 checks/15 screenshots (typography subsets/Cyrillic metrics, frameless window controls+drag region, app scale80-150, one AudioEngine output stream); rebuilt portable0.12.6 tar SHA256 `58da18db…`/BUILD `4ceafdc`, `verify_desktop_package.py --launch` PASS (real window, exit0,0 temporary originals), prior0.12.6 tar `7abe39…` preserved under `previous/` | FUNCTIONAL_TESTED for native library/queue/public playback/cached fallback and the portable0.12.6 tar package launch; no deb/AppImage/Flatpak/signature, Windows/macOS/mobile secure stores, background offline sync/refresh, automatic cache refresh/global cursor, Internet public-CA/two-host or physical-output audit; TR10 package is source `4ceafdc` at version0.12.6; not a `.deb`/AppImage/Flatpak/signature or hardware-audio certification, and the0.12.6 version number is shared with the preserved `7fe1ca0` archive |
| Windows desktop | None | No Windows target/runner available | NOT_STARTED; native engine/SMTC and Windows build/runtime required |
| macOS desktop | None | No Apple host/Xcode | BLOCKED for Apple compilation/runtime; shared-contract work remains possible |
| Android | None | adb, sdkmanager, gradle absent from PATH; no connected-device verification | NOT_STARTED; SDK/NDK/toolchain, Dioxus native bridge, Media3 service/audio focus/offline sources required |
| iOS | None | xcodebuild absent; Linux host, no simulator/device/Apple signing environment | BLOCKED for compilation/device validation; AVAudioSession/AVFoundation/remote-command bridge still required |
| ARM64 server | Deployment target only | Not compiled/run in this session | NOT_STARTED validation |
| Docker | Recipe present | docker info fails: daemon socket absent | BLOCKED runtime validation; no image success claimed |

Installed Rust targets are x86_64-unknown-linux-gnu and wasm32-unknown-unknown. No mobile/native client has been silently substituted with a browser WebView. The0.12.6 Linux desktop client is a real Dioxus/WebKitGTK window with a native Rodio/CPAL output thread and MPRIS observer; a WebView was not used for audio, and the web player remains a separate target. Android tooling/NDK/Gradle and Apple/Xcode are absent, so those targets remain NOT_STARTED/BLOCKED rather than inferred from Linux. Native next work is fixing the audited offline/keyring lifecycle defects and a real cold-start-without-server path (AUD-01–AUD-08), then Android Media3 foreground service and Apple AVFoundation on their real toolchains; Linux packaging, session persistence, offline copies and folders are implemented to the0.12.6 scope above. Audio must outlive UI route/lifecycle changes; Linux window evidence does not certify mobile background audio. Android's [Media3 ExoPlayer documentation](https://developer.android.com/media/media3/exoplayer) was checked during this session; no Android version or hardware compatibility is claimed without a build.
