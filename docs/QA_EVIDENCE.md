# Verified0.12.0 package and preserved runtime checkpoint

## Current-source interaction/Hyprland repair — executed2026-10-19

Baseline91867f2, workspace0.12.6; source/debug evidence, not a runtime/package upgrade.
Exact changes/reproduction: [interaction review](UX_INTERACTION_REVIEW_2026-10-19.md).

| Check | Executed evidence | Scope/status |
| --- | --- | --- |
| UX01 | Real server/PCM upload, seek7s, Library/Home/current queue Pause/Play, same audio source and retained clock | PASS Chromium/Firefox; no mocked audio |
| Full browser regression | Chromium43 pass/1 live-import skip2.3m; Firefox43 pass/1 skip2.7m; DESIGN01 responsive/theme, PLAY/RES/NET/NP/privacy/auth retained | `target/checks/ux-toggle-{chromium,firefox}-final/`; browser sources unchanged by later native-only follow-up |
| Native X11 fixture |89 checks,10 complete PNGs,640/1280/1440/1728; pending click/row/queue/playlist continuity, two-origin TLS, mode capabilities, server-stopped offline actual EOF repeat;0 leftover originals | PASS `target/checks/ux-modes-native-x11/report.json`; WebKitGTK/Rodio/null sink |
| Actual native Wayland |5 checks, unresized normal Hyprland tiling, exact owned PID/xwayland=false; frameless controls, seek/row/source continuity/no overflow;1278×1030 DPR2→2556×2060 PNG;0 leftovers | PASS `target/checks/ux-modes-native-wayland/report.json`; distinct subset, not full peers/offline/physical output |
| Better Design | Two final scoped comprehension PASS; DOM web0critical/0serious/10moderate, native0critical/0serious/5moderate | `.better-design/ux-review.json`; inherited moderate spacing remains; native390 unsupported |
| Rust/tooling |293 pass/0fail/3ignored; fmt; strict workspace/native-qa/wasm Clippy; native debug/example/dev WASM builds;22 Python tests; JS syntax/CLI bounds | `target/checks/ux-modes-*.log`, `ux-toggle-web-build-final.log`; ignored/device checks not claimed |

G01–G24 mapping: reinforces G05 persistent player/queue; G18 Linux-client subset and
G23 responsive/theme/input subset remain PARTIAL. No screen-reader, physical sound,
mobile, device handoff, rooms or all-platform certification. Data/schema/runtime
unchanged. Initial failed browser/capture runs are retained/explained, not counted
as final passing evidence.

## Current-source Better Design UI — executed2026-10-19

Source baseline0170ec5, workspace0.12.6; separate from the historical packages below.
Exact changes/commands: [design review](DESIGN_REVIEW_2026-10-19.md).

| Check | Executed evidence | Scope/status |
| --- | --- | --- |
| DESIGN01 | Real isolated Rust server; committed FLAC+generated PCM upload; same authoritative browser audio/source through Home→Library; pause; sidebar group geometry and Settings reachability at1440×720; selected rendered text/primary-control contrast≥4.5 and fonts≥12; player/seek targets≥44; no horizontal overflow; real Home/Library/Playlists screenshots in both themes | PASS in Chromium and Firefox; web320/390/1440/1728; no global contrast/screen-reader claim |
| Full browser regression | Chromium42 pass/1 live-import skip2.3m; Firefox42 pass/1 live-import skip2.6m | `target/checks/design-contrast-{chromium,firefox}-final/run.log`; existing security/media/request-order tests retained |
| Strengthened DESIGN01 contrast | Every visible match of the selected selectors, including current/ordinary tracks;38 readings/theme/engine at320/390/1440/1728; dark minimum6.714:1, light4.536:1 | Separate Chromium5.8s/Firefox7.2s PASS; `target/checks/design-contrast-all-rows-{chromium,firefox}/`; this is not an all-screen WCAG audit |
| Native real-window fixture |74 checks,10 complete screenshots, actual DOM widths640/1280/1440/1728, navigation/source/position continuity, queue/favorite/playlist/folder/group/public peer/offline flows,0 abandoned originals | `target/checks/design-contrast-native-final-evidence/report.json`; PASS; X11/XWayland/WebKitGTK/Rodio null sink, not physical sound/native-Wayland certification |
| Better Design review |3 text comprehension PASS; web0 critical/0 serious/10 moderate, native0 critical/0 serious/4 moderate spacing findings | `.better-design/review.json`; original own tokens, UI/UX/review rules; native390 unsupported scope, inherited moderate spacing retained |
| Rust/tooling |291 passed/0 failed/3 ignored; fmt; strict workspace and WASM Clippy; native-qa example Clippy; native/WASM dev builds;22 Python tests; JS syntax | Exact logs under `target/checks/design-contrast-*`; builds/platform evidence remain distinct |

G01–G24 mapping: this increment reinforces G05 browser queue/player continuity and
the320px/theme/keyboard subset of G23. G23 remains PARTIAL: no screen-reader,
localization or complete reduced-motion/platform certification was added. Native
and public-catalog checks retain their separately documented scope; no handoff,
rooms, mobile or whole-product completion follows. No schema/data/runtime migration
was needed. Initial failed harness/session/selector/capture runs are preserved and
their corrections explained in the review.

Documentation reconciliation2026-10-19 (docs-only): current deployed server baseline
is the frozen0.12.6/schema12 runtime and the current native package is0.12.6; the
0.12.0–0.12.5 sections are dated historical checkpoints, not current state. Golden
G14/G15 are corrected to PARTIAL below (exact tested subset stated); see
[CURRENT_STATE.md](CURRENT_STATE.md) for the source/deployed/design split. No check
was executed for this reconciliation.

Source `d3bde0c`; archived package SHA256 above. Exact extracted0.12.0 binary/assets
served the original data directory after a verified backup/isolated restore; the
service is healthy with schema11 and all prior rows/media intact.

| Feature | Exact paths | Verified source | Executed evidence | Result | Next | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| Local package/defaults | scripts/package-release.sh; target/releases/vela-0.12.0-*.tar.gz | d3bde0c | Extracted startup/setup/schema11/assets/SIGTERM;19 frozen hash matches; preserved earlier release checksums | PASS | Docker/ARM64 unverified | main |
| Exact packaged browser regression | apps/web; tests/browser | d3bde0c package | Chromium41/41 1.9m + Firefox41/41 2.4m with licensed live import; sleep/playlist/queue/history/DSP/radio/publication regressions | PASS | Safari/mobile NOT_RUN | web/main |
| Backup/restore before upgrade | scripts/backup.py; target/upgrade-backups/before-0.12.0-20261018T161750Z.tar.gz | d3bde0c |0600 440MB backup; isolated restore with30 table count equality, media/artwork SHA, quick_check/FK0 | PASS | Encryption/offsite not claimed | main |
| Runtime switch | target/runtime/vela-0.12.0; enabled user unit; original data/ | d3bde0c | Graceful stop old PID; unit switched; health0.12.0/PID267247/0restarts; schema11/1user/20tracks/20analysis/0playlists; served hashes match | PASS | Boot/login behavior unverified | main/devops |
| Shutdown/worker recovery | scripts/verify_shutdown.py | d3bde0c package |10 clean SIGTERM/restarts exit0; import/scanner/analysis/listening worker DB failures stopped correctly | PASS | No device/Docker claim | main |

# Verified native Linux and web source slices0.12

Source slices: native `3a285e0`/hardening `05951fd`, web queue/playlist `6421f3d`.
The frozen0.11.0 server runtime and original data were not replaced; no0.12 package
was installed.

| Feature | Exact paths | Verified source | Executed evidence | Result | Next | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| Native Linux client transport/audio | apps/desktop/src/{client,audio,lib,ui,main}.rs; apps/desktop/style.css |3a285e0/05951fd |30 desktop tests (28 run,2 opt-in); strict Clippy `--all-targets --features native-qa`; fmt; owned-null-sink audio test; real WebKitGTK window against isolated fixture server/account/media | PASS for Linux library/queue playback | Packaging, secure persistence, favorites/playlists UI, offline/background | main/audio/client/UI |
| MPRIS2 observer | apps/desktop/src/mpris.rs |05951fd | Ignored real-session test under private `dbus-run-session` after bounding update/command channels: properties and play/pause/next/previous/seek/volume forwarding | PASS for Linux session bus | External controller/session hardware smoke | main |
| Native favorites | apps/desktop/src/{client,ui}.rs; examples/native-ui-qa.{rs,js} | 0.12 local | Owner favorite/unfavorite API, filtered Favorites view, unfavorite re-read to empty, Library restore, queue-state sync;30 desktop tests (28 run,2 opt-in) and233 workspace tests | PASS for Linux native scope | Packaged/background/offline remain | main |
| Native playlist lifecycle | apps/desktop/src/{client,ui}.rs; scripts/verify_native_client.py; examples/native-ui-qa.{rs,js} | 0.12 local + current debug server | Fixture creates a real two-entry playlist; native list/detail, create→rename→cancel/confirm-delete, fixture survives, entry add/reorder/restore/remove, three-entry streamable playback, shared queue2 entries, Library return;25/27 desktop tests | PASS for Linux native scope | Entry editing/folders/albums/artists remain | main |
| Native album/artist browse | apps/desktop/src/{client,ui}.rs; examples/native-ui-qa.{rs,js} | 0.12 local + current debug server | Real album groups/detail (`Real Fixtures · Vela Test`), Play album native audio, real artist groups/detail, Play artist native audio, Library return;25/27 desktop tests | PASS for Linux native scope | No editing/edition identity | main |
| Web sleep timer | apps/web/src/main.rs; tests/browser/music.spec.mjs | 0.12 local | Root countdown 5/15/30/60, real60s audio still playing before expiry, paused+status at zero, cancel/restart stays playing; Chromium41/41+Firefox41/41 | PASS for browser scope | No OS-level timer/native sleep | main |
| Native queue/favorite parity | apps/desktop/src/ui.rs; examples/native-ui-qa.{rs,js} | 0.12 local | Move-down follows current occurrence, move-up restores, Clear-upcoming keeps only current, player favorite API+queue sync;25/27 desktop tests | PASS for Linux native scope | No drag/drop | main |
| 0.12.6 session/offline integrity release/runtime | Cargo.toml/lock; target/releases/vela-0.12.6-*; target/runtime/vela-0.12.6; scripts/install-user-service.py | `7fe1ca0` | Package verify; exact Chromium41 pass1 skip1.9m, Firefox41 pass1 skip2.2m; backup restore-verified schema12; installer handover, health0.12.6,PID544183,0 restarts; counts intact; native0.12.6 SHA7abe39…; session/offline/integrity features included | PASS for Linux local runtime/package | No external publication; non-Linux not implied | release/main |
| 0.12.5 spectral-4/native folders release/runtime | Cargo.toml/lock; target/releases/vela-0.12.5-*; target/runtime/vela-0.12.5; scripts/install-user-service.py | `3eedf1b` | Package verify; exact Chromium41 pass1 skip1.9m, Firefox41 pass1 skip2.2m; backup restore-verified schema12; installer handover, health0.12.5,PID498859,0 restarts; counts intact; native0.12.5 SHAc6bd29… | PASS for Linux local runtime/package | No external publication; non-Linux not implied | release/main |
| 0.12.4 playlist folders schema12 release/runtime | Cargo.toml/lock; target/releases/vela-0.12.4-*; target/runtime/vela-0.12.4; scripts/install-user-service.py | `16034b9` | Package verify; exact Chromium41 pass1 skip1.9m; Firefox41 pass1 skip2.3m on clean rerun (first run flaked sleep timer); backup restore-verified schema11; explicit new-runtime migration to schema12, then installer handover; health0.12.4,PID463860,0 restarts; counts intact | PASS for Linux local runtime/package | No external publication; native folder UI absent; non-Linux not implied | release/main |
| 0.12.3 privacy/offline patch release/runtime | Cargo.toml/lock; target/releases/vela-0.12.3-*; target/runtime/vela-0.12.3; scripts/install-user-service.py | `6cd407c` | Package verify; exact Chromium40 pass1 skip1.9m, Firefox40 pass1 skip2.2m; private0600 backup restore-verified; graceful handover, health0.12.3, PID432696,0 restarts; counts/integrity unchanged; native0.12.3 SHA13d75a… | PASS for Linux local runtime/package | No external publication; spectral-1/2 rows retained; non-Linux not implied | release/main |
| 0.12.2 spectral-3 patch release/runtime | Cargo.toml/lock; target/releases/vela-0.12.2-*; target/runtime/vela-0.12.2; scripts/install-user-service.py | `e9d1954` | Package verify; exact Chromium40 pass1 skip1.9m, Firefox40 pass1 skip2.2m; private0600 backup restore-verified; graceful stop/installer handover, health0.12.2, PID382555,0 restarts; counts/integrity unchanged | PASS for Linux local runtime/package | No external publication; spectral-1/2 rows retained; native/mobile outside Linux not implied | release/main |
| 0.12.1 DSP patch release/runtime | Cargo.toml/lock; target/releases/vela-0.12.1-*; target/runtime/vela-0.12.1; scripts/install-user-service.py | `9aa062a` | Package verify; exact Chromium40 pass1 skip1.8m, Firefox40 pass1 skip2.2m; private0600 backup restore-verified; graceful stop, installer handover, health0.12.1, PID354876,0 restarts; counts/integrity unchanged | PASS for Linux local runtime/package | No external publication; spectral-1 rows retained; native0.12.0 package not repackaged | release/main |
| Local DSP tempo/chroma/key spectral-4 | crates/media/src/analysis.rs; crates/domain/src/audio_analysis.rs; apps/web/src/radio.rs; tests/browser/music.spec.mjs | 0.12 local | Normalized spectral-flux onset envelope, onset-interval median BPM, bounded confidence, extractor version bumps;22 analysis tests incl120 BPM click train, A440/C#4/two-tone chroma, C-major/A-minor Krumhansl triads and None on single-pitch/silence; Chromium Radio shows spectral-4 + Estimated tempo + Chroma pitch classes + Estimated key + heuristic disclaimer; full Chromium41 passed,1 live-import skip in2.2m | PASS for bounded descriptive tempo/UI/radio regression | No calibrated beat/chroma; Firefox not rerun; real-music tempo accuracy unmeasured | media/domain/web |
| Native TLS federation | apps/desktop/src/remote.rs; scripts/verify_native_client.py | 0.12.5 source | One-day local CA + IP-SAN cert; Python TLS terminator proxies the second peer; native reqwest validates the chain for catalog/stream;3 TLS connections; debug-only CA/transport overrides absent from release strings;28/30 desktop tests | PASS for loopback TLS certificate validation | Internet DNS/public CA/two physical hosts NOT_RUN | desktop/main |
| Native public peer playback | apps/desktop/src/remote.rs,ui.rs; scripts/verify_native_client.py; examples/native-ui-qa.{rs,js} | 0.12 local | Second isolated Vela instance: canonical public origin, explicit CC-BY stream publication, two isolated peers, save/remove origins in isolated private0600 config, 21-record public peer first page20 then preview cursor Load more appends the final record; unified search per-peer cursor Load more appends its 21st result with no duplicate IDs, unified search returns both origins, unreachable saved peer returns browse-only cached metadata with age, no play action and an explicit Retry live that remains cached when still unreachable, second-origin public playback, remote previous/next across both origins, credential-free;28/30 desktop tests | PASS for loopback debug transport | Internet TLS/two-host validation NOT_RUN | main |
| Native playlist folders | apps/desktop/src/{client,ui}.rs; examples/native-ui-qa.{rs,js} | 0.12.4 source | Owner folder create/filter/move/rename/delete via real APIs; unfiled keeper excluded by filter; delete unfiles and restores grouping;28/30 desktop tests | PASS for Linux native source | Source newer than packaged0.12.4 desktop archive; no folder drag/drop | desktop |
| Offline lifecycle/quota repair | apps/desktop/src/{offline,ui}.rs | `e225b63` | AUD-03/07/08/12: ticket+checking/Cancel before work, spawn_blocking same-fd BLAKE3 with per-block cancel, non-current removal remaps queue/order, per-await gate/epoch/user/origin guards, manifest cap/unique staging/quota+replacement delta/recoverable publish; desktop56 pass3 ignored; real native offline/server-stopped QA `target/checks/aud-offline-p1/` | PASS for source+native window basic flow | Cross-process save, disk-full/power-loss, explicit checking/Cancel harness assertions NOT_RUN | desktop |
| Desktop saved-session P0 repair | apps/desktop/src/{session_store,ui}.rs; scripts/verify_native_client.py | `3094754` | AUD-01/02/04/11: off-thread bounded keyring deletion after audio stop; transient/5xx/403 preserve item, only401 deletes; attempt generations; byte-bounded/kill+reap/serialized helper; redacted Debug; desktop43 pass2 ignored; real two-process Secret Service save/resume/sign-out QA `target/checks/aud-session-p0/` | PASS for source+Linux Secret Service | Not deployed to live0.12.6; no hung-real-D-Bus repro | desktop |
| History snapshot reads | crates/core/src/listening.rs | `48b2fa4` | AUD-13: history/export use one deferred snapshot, no BEGIN IMMEDIATE/prune on GET; hourly worker prunes; core26+7, API31; red/green 2.52ms vs10.03s SQLITE_BUSY under open writer | PASS for source | Retention worker delay not measured live; preferences/devices GETs still write | core |
| Native hardening review fixes | apps/desktop/src/{offline,session_store,ui}.rs; crates/core/src/playlists.rs | working slice | Keyring singleton/verified deletion/malformed cleanup; same-fd BLAKE3 offline verification + failed-manifest cleanup; transactional folder-owned move; two-process resume + server-stopped offline QA;28/30 desktop tests,31 API tests | PASS for reviewed fixes | Independent review produced no findings before stop; self-review fixes recorded; loopback/mobile limits unchanged | desktop/core |
| Firefox full suite (TR04a baseline) | tests/browser/music.spec boot helper + IN01 Search path | pending | Boot-state wait (setup OR sign-in) and fresh Search instead of stale Library; full Firefox49 passed/1 skipped/0 failed3.2m; targeted IN01 Chromium pass; full Chromium49/1/0 before test-only change | PASS Firefox suite | WebKit/Safari/native/package NOT_RUN | web/QA |
| Cross-engine browser suite | apps/web/style.css range focus; tests/browser | `62d2fab` | Explicit range :focus ring for Firefox; full Chromium49 pass1 skip0 fail, full Firefox49 pass1 skip0 fail3.1m, targeted SCALE both; web30/API33/desktop113 | PASS Chromium+Firefox | WebKit/Safari/native package/PT matrix NOT_RUN | web |
| Web playlist artwork | apps/web/src/{experience,main}.rs; PlaylistSummary.artwork_id | `93138a1` | Home and Playlists cards render first authorized playlist artwork via CatalogCover with fallback; full Chromium49 pass1 skip0 fail; web/API/desktop gates pass | PASS web playlist art | Firefox full after this change49/1/0; native detail QA pass; package NOT_RUN | web |
| TR04b/TR07a web + native playlist art | apps/web/{home_editorial,queue_drawer,main,style}; apps/desktop/src/ui.rs | `df0290a`,`e876449`,`7aa0ae0` | Home editorial/My Wave and right queue drawer project one root player; playlist detail/list real artwork; Chrome full49 pass1 skip0 fail, web30, API33, desktop113 | PASS web + native source | Firefox/manual PT widths/package/native matrix for last header NOT_RUN | web/desktop |
| Playlist artwork projection | crates/domain/src/lib.rs; crates/core/src/playlists.rs | `2c5318f` | First authorized playlist entry artwork exposed as PlaylistSummary.artwork_id via owned/list/search correlated query with serde default; core26 + API33 pass | PASS backend | Client playlist cards not wired; external metadata-only grants unchanged | core/server |
| Native actual-next preview | apps/desktop/src/{ui.rs,playback reuse}, style.css | `a2b4f94` | Queue shows order.projection() explicit-next or first shuffle-deck pending with reason label; no mutation/play; native X11 peer QA pass | PASS native preview | Drag/insertion/cross-device NOT_RUN | desktop |
| Native Wayland window | scripts/verify_native_client.py --backend wayland --window-only; apps/desktop | working tree | No GTK titlebar, integrated controls reachable, row pause/resume retains source+position, no horizontal overflow, backend=wayland, zero temp originals | PASS Wayland window-only | Full Wayland resize/peer/offline/package NOT_RUN | desktop |
| Native paused queue restore | apps/desktop/src/{queue_store,ui}.rs | `350d72a` | Private versioned snapshot; restore only same origin+account when queue empty, paused/no autoplay, no notice override; save on transitions/sign-out; one AudioEngine; native X11 peer/session QA pass | PASS native restore | Position resume starts at0; cross-device/handoff/TR07 NOT_RUN | desktop |
| Native storage controls | apps/desktop/src/{media_cache,ui}.rs | `ab7bb19` | Usage/clear only published cache pairs; Settings Storage panel count/size/Refresh/2-step clear; offline/staging untouched; unit test + native X11 peer/session QA; desktop tests pass | PASS native storage | Download manager/priority/TR07 drawer/Wayland NOT_RUN | desktop |
| TR05a native detail rows | apps/desktop/src/ui.rs | `8294802` | Album/artist detail uses .native-track grid: start, real art, title/artist, album, duration, row no; play_owned_context unchanged; native X11 peer/session QA pass, desktop111/3 ignored | PASS native detail | Library/search restyle, TR05 web, PT06/07/10, package NOT_RUN | desktop |
| TR04a Home filters | apps/web/{experience,icons,style}; prototype-transfer.spec | `1b70b7a` | Header+3-tab tablist over real DTOs; editorial honest absent (IMAGE_BLOCKED); real shelf<=6 borderless art; real Play new additions; playlists grid/empty; no fake/stale content; one root player; full Chromium49 pass1 skip0 fail, web27 | PASS web TR04a | Editorial/wave images, My Wave visual, Firefox/native/package NOT_RUN | web |
| TR03 web player | apps/web/src/main.rs; style.css; prototype-transfer.spec | `456dec3` | One root element;51px art/title/artist/album/quality; transport+44px seek/aria; source/queue/volume/mute; responsive88/mini; targeted TR01/02/03 3/3, DESIGN01/UX01 2/2, web27 | PASS web TR03 targeted | Full suite one login-limiter fail; Firefox/PT matrix/package NOT_RUN | web |
| TR03 native player | apps/desktop/src/ui.rs; style.css; native QA | `4a29a59` | 88px band:48px artwork, honest metadata, transport44px hit boxes/32px face, seek10px tabular+aria, Source/Queue/Mute/volume, mute restore; one AudioEngine; source/queue/position survive Queue/theme/scale; native X11 QA204 checks/14 screenshots incl owned null-sink audio, desktop108/3 ignored | PASS native TR03 | Web TR03 source-generation8→10 on keyboard seek unresolved; TR07 drawer/Wayland/package/other widths NOT_RUN | desktop |
| TR01/TR02 shell foundation | apps/web/{style.css,index.html,src/*}; apps/desktop/{style.css,src/ui.rs}; apps/server/src/lib.rs; scripts/build-web.sh; font assets | `d687026`,`6269ff0`,`d0e5584` | Namespaced roles, self-hosted Manrope+allowlist/copy, web216/75/88 one-offset shell targeted Chromium DESIGN01/UX01/TR01/TR02 4/4 across13 widths+80/150% zoom; native X11 153 checks 216/75/88, single scroller, 640 destinations, playback continuity | PASS targeted web + native X11 shell | Home/player/topbar search/mobile/TR09/Wayland/packaging/native font NOT_RUN | web/desktop/server |
| Native authorized artwork | apps/desktop/src/{client,artwork,artwork_cache,ui}.rs | `0f9f12b` | ART01-08/10 source: authenticated HEAD+GET,8MiB/MIME/magic, denial, private0600 cache atomic/200/64MiB, bounded store, data-URL covers+fallback; desktop96 pass3 ignored incl real-server auth/cache reuse; ART01 real 2288x937: 640x498 data-image covers in library+player, square cover, sign-out clears, re-sign-in restores; ART02 PARTIAL (favorites+queue visual only; album/detail/search NOT_RUN); ART11 PARTIAL (2288 dpr1 dark + 1280 light queue; high-DPI/light player NOT_RUN) (`target/checks/artwork-visual-2288/`) | PASS ART01, PARTIAL ART02/ART11 | Denied/foreign visual, long-scroll memory, offline/MPRIS art NOT_RUN | desktop |
| Native settings/app scale | apps/desktop/src/{presentation,ui}.rs; style.css | pending | SCALE01/03/07/08/12/SET01/04/13 source: private versioned device store0700/0600,80-150+theme, first-frame apply, Settings/−+/slider/reset/repair, no audio/queue touch; desktop108 pass3 ignored; native peer/session QA pass | PASS for source+native flow | Visual/HiDPI/640/Orca/disk-full NOT_RUN | desktop |
| Web app scale/appearance | apps/web/src/{preferences,settings,main}.rs; style.css; tests/browser | `1618e63` | SCALE01/02/07/09: versioned device prefs80-150+theme, boot apply, Settings panel/reset/status, one CSS zoom, malformed recovery, no tokens; web27 tests, targeted Chromium SCALE01 pass4.2s | PASS for Chromium targeted | Full suite/Firefox/screen-reader/native scale NOT_RUN | web |
| Native saved session | apps/desktop/src/{session_store,client,ui}.rs; scripts/verify_native_client.py | 0.12.5 source | Opt-in checkbox stores opaque token/origin in unique QA Secret Service item; second native process resumes via `/api/me` without password; sign-out clears item; envelope has no password/username/CSRF;28/30 desktop tests | PASS for Linux Secret Service | Windows/macOS/mobile stores, biometrics NOT_RUN | desktop/main |
| Native offline copy/playback | apps/desktop/src/{offline,ui,client}.rs; apps/desktop/examples/native-ui-qa.{rs,js}; scripts/verify_native_client.py | `7609a76`+working slice | Explicit download of two tracks via validated originals; account-scoped0600 audio+v2 BLAKE3 manifests checked before playback and by the startup sweep; harness stops real fixture server, health probe fails, first offline original plays to advancing media, Next/Previous move within the newest-saved offline queue, confirmed removal leaves one pair;28/30 desktop tests | PASS for Linux one-track offline playback | No offline queue/removal/sync, mobile/background | desktop/main |
| Native portable package0.12.6 | scripts/package-desktop-linux.sh; scripts/verify_desktop_package.py; apps/desktop | `7fe1ca0` | Exact manifest/modes, `--version`=0.12.6, ldd, styled sign-in launch, normal close exit0, zero temp files; SHA256 `7abe39…`; includes folder UI, offline v2 BLAKE3 manifests + sweep, cache age/Retry live, pagination, opt-in session resume | PASS for portable tar/launch | deb/AppImage/signature/physical audio NOT_RUN | main |
| Native portable package0.12.1 | scripts/package-desktop-linux.sh; scripts/verify_desktop_package.py; apps/desktop | `a3cb52c` | Exact manifest/modes, `--version`=0.12.1, ldd no missing, real GUI sign-in launch+styled screenshot, normal close exit0, zero temp files; SHA256 `67850b30…` | PASS for portable tar/launch | deb/AppImage/signature/physical audio NOT_RUN | main |
| Native portable package | scripts/package-desktop-linux.sh; scripts/verify_desktop_package.py; apps/desktop |692c083 app/current scripts | Exact manifest/modes, `--version`=0.12.0, ldd no missing, real GUI sign-in launch+styled screenshot, normal close exit0, zero temp files; SHA256 4d6c67bb… | PASS for portable tar/launch | deb/AppImage/signature/physical audio NOT_RUN | main |
| Responsive native window/source lifecycle | apps/desktop/examples/native-ui-qa.{rs,js}; apps/desktop/{style.css,src/client.rs,src/lib.rs} |05951fd |15 named checks at1280×840 and640×700: real WAV/FLAC/MP3 playback, pause/seek, navigation continuity, duplicate queue, reorder, element bounds, no overflow, sign-out; zero temporary files during held transfer/after exit; close-while-playing variant passed | PASS for recorded Linux scope | Physical output, screen reader, packaged window | main/QA |
| Failure found and fixed | apps/desktop/style.css; src/client.rs |05951fd | First compact run reproduced queue grid rows overflowing640px; independent review reproduced a named temp file during preparation. Element-level assertion and unnamed `O_TMPFILE` source retained; complete reruns passed | FAILED then PASSED | Preserve failed evidence | main/reviewer |
| Owner playlist rename/delete + queue truthfulness | crates/domain/src/lib.rs; crates/core/src/playlists.rs; apps/server/src/lib.rs; apps/server/tests/api.rs; apps/web/src/{main,client}.rs; tests/browser/music.spec.mjs |6421f3d | Real TCP owner/editor/outsider/CSRF/unknown/stale/replay/share-revocation/track-preservation test; full workspace233 passed/0 failed/2 opt-in; strict native+WASM Clippy; fmt;22 Python; Chromium40/40 in2.5m and Firefox40/40 in3.3m with live licensed import | PASS for source web/API scope | Folders/smart rules/invitations/media grants; exact package rerun | main/web |

Evidence paths are under ignored `target/checks/playlist-*` and `native-0.12*`;
`native-0.12-window2` preserves the failed compact run. These are Linux window/audio
and source-browser tests, not mobile background, Apple/Windows, installer, exact
packaged-asset or physical-audio certification.

# Verified0.11 package/runtime checkpoint

Source/application `fe0484d62f1a2ae33a8c25ee901d4ed6b9568f06`.

| Feature | Exact paths | Verified source | Executed evidence | Result | Next | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| Explicit publication/grants/revocation/peer policy | crates/domain/src/network.rs; crates/core/src/network.rs; crates/media/src/publications.rs; apps/server/src/network.rs; migrations/0011_explicit_publications.sql | fe0484d | Rust200; strict native+WASM; API owner/CSRF/CAS; two real independent origins + hostile/slow peer; DNS/redirect/capacity and legacy10 preservation | PASS for profile1 | Full signed federation/adapters remain unfinished | main/media/QA |
| Actual Dioxus Network/root-player workflows | apps/web/src/network.rs; tests/browser/music.spec.mjs | fe0484d exact package | Chromium39/Firefox39 with licensed live import, PN01/PN02, all prior regressions; screenshots/physical abort/actual Range/revoke | PASS for web scope | Native/Safari/screen-reader/Internet TLS NOT_RUN | web/main |
| Frozen release integrity/defaults/recovery | scripts/package-release.sh; scripts/verify_package.py; scripts/verify_shutdown.py | fe0484d | Extracted schema11/default-private/assets/SIGTERM;19 matching binary/static hashes; retained release SHA checks;20 restarts+4faults | PASS | Docker/ARM64 platform validation unavailable | main |
| Original data and persistent supervision | scripts/backup.py; scripts/install-user-service.py; tests/test_user_service.py; data/ | fe0484d | Fresh stopped backup/isolated restore/all old SQL rows+media preserved; original10→11;22Python; actual unit installation/recheck, enabled/PID78464/0restarts | PASS | No boot/logout/native-audio inference | devops/main |

All exact commands/evidence paths, unexecuted scope and current original-data
counts are in PROGRESS. One listening preference/device record existed before
the final backup; private consent values are identical before/after migration,
with0sessions. New public catalog remains disabled with0publications. G01/G24
are verified to this original-data upgrade scope; G11/G14/G15/G22 remain the
specific read-only/rights subsets below, not full federation golden acceptance.

# Public catalog0.11 source integration

Historical source gate before the exact package checks above.200 Rust tests, strict native/all-target+WASM Clippy, release native/WASM,
fmt,22 Python, JS syntax/diff and locked dependency policy passed.20 real clean
restarts and4 independent worker faults passed. `target/checks/publication-*` holds
commands/logs; the failed NUL projection run is preserved separately from corrected
whole-workspace evidence. Source Chromium39/39 passed in2.5m, licensed live import; screenshot/evidence paths
are in PROGRESS. No native/public-TLS/ActivityPub compatibility claim.

| Feature | Exact paths | Latest verified source | Evidence command | Result | Next | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| Origin/private publication/independent grants | core/network.rs; media/publications.rs; domain/network.rs; migration0011; server/network.rs | fe0484d | cargo test --locked --workspace -- --test-threads=2:200 | PASS | Exact package/regressions | main/media |
| Two independent origins + hostile peer | server/src/network/tests.rs | fe0484d | Same Rust run: actual B HTTP preview calls actual A HTTP, Range/HEAD/revoke, slow/oversized/spoofed identity/grants/IDs | PASS for read-only profile | Standards signatures/replay/blocks remain NOT_RUN | main |
| Controlled DNS/redirects and stream capacity | imports/egress.rs; server/network/tests.rs | fe0484d | Real two-listener no-dial redirect controls, barrier cancellation retaining4DNS slots,8 public bodies drop/reserved owner capacity | PASS | Public load measurements | main/reviewers |
| Network UI/source guards | web/network.rs; tests/browser/music.spec.mjs | fe0484d | Chromium39/39 with live import, PN01/PN02; main inspected Network1440dark/320light, no overflow | PASS for source Chromium | Exact packaged two-engine runs | web/main |
| Same-build persistent user service | scripts/install-user-service.py; tests/test_user_service.py; docs/USER_SERVICE.md | fe0484d + frozen902dd47 runtime |22 Python incl13 installer; actual --replace-transient --install and --dry-run recheck | PASS | Preserve pinned upgrade process | devops/main |

Independent readonly QA signed off the implemented permission/projection/peer/DNS/
UI boundaries and checked retained logs. No release-blocking defect found; exact
package/two-engine acceptance still follows. Private playlist sharing remains
metadata-only even though separate public publication now has explicit audio grants.

Latest mandate mapping: G01/G24 additive10→11 legacy-row and backup safeguards;
G11 independent origin/catalog + permitted original public stream subset; G14
bounded hostile/slow/SSRF/metadata subset (forged actors/signature replay NOT_RUN);
G15 removal/stream-revocation subset (no federated recommendation/cache);
G22 declaration/provenance/unknown-rights/private defaults subset. PN01/PN02 are
new named acceptance; preserve all EXP/PLAY/PLNAV/RES/NET/AUTH/NP/PC/history/DSP
regressions. No entire golden scenario becomes green from a partial subset.

# Verified0.10 package/runtime checkpoint

Application/source 902dd47f9dca2d9b6f405cd50c21bf4dab5a73d8. Exact extracted binary passed37Chromium +37Firefox
with live licensed import; static bytes match frozen package before/after runs.
Screenshots8 size/theme combinations per engine; main inspected desktop/phone.
Packaged20clean restarts +4worker DB faults, default/schema10/assets/SIGTERM,
all preserved release checksums and9frozen hashes pass. Fresh stopped0.9backup
restored with every SQL row/media hash; original20tracks/assets and20analysis
rows preserved after9→10, quick_check ok/FK0, no auto consent/exclusions.
Current supervised loopback0.10 PID34675 uses original data,0restarts.
Source gates176Rust/strict native+WASM/fmt/9Python/JS/deny pass. Evidence paths
and exact limits are in PROGRESS; no native/Safari/screen-reader/Docker claim.

# Personal discovery0.10 source evidence

Source/application902dd47; final archive/runtime evidence is recorded above and in PROGRESS.

| Feature | Exact paths | Verified source | Executed evidence | Status | Next / owner |
| --- | --- | --- | --- | --- | --- |
| Private mix/service/feedback | crates/media/src/recommendations.rs; apps/server/src/recommendations.rs; migrations/0010_recommendation_exclusions.sql |902dd47 |176 Rust tests, strict native/all-target+WASM Clippy, real TCP, legacy9upgrade, owner/consent/retention/concurrency | PASS | Preserve gates / main |
| Web peer preview cursor | apps/server/src/network.rs; crates/domain/src/network.rs; apps/web/src/network.rs | 0.12.6 source | PreviewPeer.cursor validated, UUID-checked, forwarded to the peer catalog URI and checked for continuation; fixture test records the outbound cursor and rejects non-UUID; web Load more revalidates origin/dedupes by ID; Chromium41 pass1 skip2.0m; Firefox41 pass1 skip in three clean runs (2.6m/2.9m pre-fix, 2.9m at HEAD3a3f463) — still non-regression only (no successful-preview browser scenario); network lib tests4, API31 | PASS for source/Chromium/Firefox non-regression | Not deployed to live0.12.6 runtime; no browser success-path scenario (server fixture covers cursor) | server/web |
| Playlist folders | migrations/0012_playlist_folders.sql; crates/{domain,core}/src; apps/server/src/lib.rs; apps/web/src/main.rs; tests/browser/music.spec.mjs | 0.12.3 source | Owner-scoped folder list/create/CAS rename+delete, CAS playlist move, unfiling bumps playlist revisions; API folder test; Chromium42 scenarios (41 pass1 skip2.2m) and Firefox42 (41 pass1 skip2.6m) incl create/move/filter/rename/delete at320px | PASS for source/Chromium | Not deployed to live0.12.3 schema11; native folder UI absent | core/server/web |
| Listening device grants/revocation | crates/core/src/listening.rs; crates/domain/src/listening.rs; apps/server/src/listening.rs; apps/web/src/listening.rs; tests/browser/music.spec.mjs | 0.12.2 source | Owner-scoped GET devices (counts/last activity), DELETE with privacy-revision CAS, cascade sessions, revision bump, stale409/unknown404; G07 seeds a second real grant, lists it, revokes with confirmation at320px, and history/DSP/export erasure still pass; Chromium40 passed1 skip2.4m | PASS for Chromium source run | Firefox not rerun; packaged runtime not upgraded | core/server/web |
| Home/root playback/exclusions | apps/web/src/personal_mixes.rs; tests/browser/music.spec.mjs |902dd47 |Chromium37; G07 rendered actual rotation/erase; PC01 real playback/hide/undo/restore/export/catalog race; PC02 held read cancellation | PASS for source run | Preserve exact package two-engine gates / main |
| SQL performance | media/src/recommendations.rs; examples/benchmark_recommendations.rs | SHA256 in final resource JSON |10k/100k/1m actual release + baseline projection/plan equality; raw samples/hardware/scope in PERFORMANCE | PASS within recorded scope | Tied timestamps/dense history and calibrated quality / main |
| Visual/platform scope | tests/browser screenshots; PLATFORM_MATRIX | Final screenshots pending | Source shots inspected; corrected capture for CSS transition settling; Docker daemon and in-app browser absent | PASS for recorded browser scope | Native/Safari/screen-reader remain unverified / main |

Preserved failed runs remain FAILED, not passing evidence. Original20tracks and20analysis
rows were backed up/restored; frozen0.9 runtime restored separately from0.10 candidate.
Private defaults were unchanged. Full native clients and public federation remain NOT_RUN.

# Current mandate evidence

## Golden-scenario coverage map (2026-10-18 checkpoint; G14/G15 reconciled2026-10-19)

Mapping of all24 golden scenarios to executed evidence and honest status. Historical
EXP/PLAY/PLNAV/RES/NET/AUTH/NP regression IDs and their browser scenarios are the
executable substitutes where a golden cannot run on this host.

- G01 migration/rollback: EXECUTED (0.10→0.12.6 chain; per-step backup/restore/counts).
- G02 web upload/FLAC/artwork/playlist/stream: EXECUTED (browser suite + API tests).
- G03 URL import/cancellation/SSRF: EXECUTED (jobs + malicious URL + importer tests).
- G04 personalized Home/rotation: EXECUTED for owned signals; **G10-style fabricated
  "Daily Mix" is intentionally absent** — onboarding/discovery is honest, not fake.
- G05 queue/mini-fullscreen/Now Playing races: EXECUTED (EXP/PLAY/NP browser scenarios).
- G06 concurrent playlist revisions/view-only/revocation: EXECUTED (browser + API).
- G07 private history consent/exactly-once/export/delete/erasure: EXECUTED (G07).
- G08 DSP on sine/silence/click/licensed songs: EXECUTED for synthetic fixtures and
  real FLAC/MP3 decode; STFT/RMS/tempo/key descriptors with version/confidence.
- G09 track radio from analyzed local tracks: EXECUTED (measured/tag/fallback reasons).
- G10 onboarding + meaningful "Daily Mix": PARTIAL — real qualified listening drives
  rotation/discovery; there is no scheduled/branded mix; not claimed as equal parity.
- G11 A↔B federation opt-in/discovery/playable public track: EXECUTED for two isolated
  instances (native+web) on loopback; EXECUTED for TLS via debug CA; **Internet/public
  DNS/two physical hosts NOT_RUN.**
- G12 cross-server search origin/availability/partial results: PARTIAL — unified native
  search with per-peer cursor pagination and cached fallback exists; merged global
  cursor and progressive partial-results UI are absent.
- G13 cross-server playlist + B disconnect status: NOT_RUN (design checkpoint; folders
  and metadata are local-only; remote playlist references are not created).
- G14 forged peer/replay/SSRF/huge object abuse: **PARTIAL — executed subset only**
  (reconciled2026-10-19): public-DNS/redirect validation, SSRF address rejection,
  oversized/128KiB body and capacity limits, hostile/slow peer, metadata-only
  projection, spoofed origin/ID and grant mismatch, revocation, via network/security
  tests and PN01/PN02. **Absent/NOT_RUN:** HTTP-signature actor authentication, forged
  actor identity and signed-request replay (no signature/replay protocol exists), and
  Internet public-CA/two-host validation. Not full G14 acceptance.
- G15 federated public recommendation/revocation/no cache leak: **PARTIAL — executed
  subset only** (reconciled2026-10-19): explicit publication revocation → catalog/
  stream404 and public-first/private-default behavior (publications/two-node tests;
  web revoke flow). **Absent/NOT_RUN:** a federated/public recommendation model (none
  exists) and cross-node cache-leak/tombstone validation; native cached peer metadata
  is browse-only with no play action and no background refresh.
- G16 Android APK on device/emulator: BLOCKED (no Android SDK/NDK/Gradle on host).
- G17 iOS simulator/device: BLOCKED (no macOS/Xcode).
- G18 desktop packages on each platform in CI/physical: PARTIAL — Linux tar + WebKitGTK
  launch EXECUTED per release; Windows/macOS installers BLOCKED; remote CI NOT_RUN.
- G19 queue/device handoff phone↔desktop: NOT_RUN (no second platform).
- G20 collaborative room/participants/latency: NOT_RUN (collaborator grants tested).
- G21 100k benchmark/audio-usable-under-load: EXECUTED for metadata/rotation benchmarks
  and document scope; a fresh 100k multi-server run NOT_RUN on this host.
- G22 licensed provenance/uncertain-rights-private/takedown: EXECUTED (publications,
  revocation, invalidation paths).
- G23 keyboard/screen reader/Reduced Motion/localization/320px/theme: PARTIAL —
  keyboard/Reduced Motion/theme/320px executed; localization and screen-reader runs
  NOT_RUN.
- G24 update/backup restore/graceful shutdown/resume/failure recovery: EXECUTED per
  release (backup restore, SIGTERM, restart, failure-recovery tests).

Summary of honest gaps: G10 full parity, G12 merged cursor/partial UI, G13 cross-server
playlists, G14 forged-actor/signature-replay acceptance, G15 federated recommendation/
cache-leak validation, G16/G17/G19 platform evidence, G21 fresh 100k multi-server run,
G23 localization/screen-reader. Everything else is EXECUTED to the evidence scope
stated above, not beyond it.

The latest master mandate has24 golden scenarios. Historical G01–G14 numbers in older docs/tests belong to the earlier UX brief and do not automatically satisfy the new meanings. Preserve named regression IDs EXP/PLAY/PLNAV/RES/NET/AUTH/NP. New history uses G07-private-history, DSP G08-audio-analysis and radio G09-track-radio.

| Feature | Exact paths | Latest verified source | Executed evidence | Status | Next / owner |
| --- | --- | --- | --- | --- | --- |
| Baseline regression | workspace; tests/browser/music.spec.mjs | f6cc845 / app ac31d74 | cargo test --locked --workspace -- --test-threads=2:110; real Chromium33 with licensed live import | PASS | Repeat after integration / main |
| Data safeguard | scripts/backup.py; data; target/upgrade-backups | f6cc845 | New before-listening-20261017T085256Z.tar.gz restored in disposable dir; schema7,1user,20tracks/assets,1artwork,0playlists; quick_check ok, FK0; listening-backup.json | PASS | Preserve backup and frozenruntime / main |
| Private history service/API | core/src/listening.rs; domain/src/listening.rs; migration0008; server/src/listening.rs |bc50564 | Full workspace157; real TCP consent/CAS/dedup/ownership/retention/export/delete; final Chromium35 + Firefox35 including actual playback/RepeatOne/erasure/stale export | PASS | Preserve regressions, add personal ranking / main+core+web |
| Audible clock/outbox/UI | web/src/audible.rs,listening.rs,browser_player.rs,radio.rs |bc50564 | Strict native/WASM Clippy, final real-browser playback/muting/history-to-radio, bounded queue/session checks | PASS | Device registry/offline remain separate / main+web |
| DSP/worker/radio | media/src/analysis.rs,analysis_service.rs,discovery.rs; migration0009; server/src/analysis.rs |bc50564 |10 extractor/9 worker/7 radio tests + real TCP worker+originals+ACL + browser G08/G09;20 benchmark runs, immutable hashes | PASS | Personal recurring mixes and scale/quality evaluation / main+DSP+core |
| Process lifecycle | server/src/main.rs; scripts/verify_shutdown.py |bc50564 |20 real SIGTERM/restarts; four independent worker DB failures stop process | PASS | Preserve regression / main |
| Browser tool availability | Browser skill runtime | This session | getForUrl returned unavailable; documented browsers.list returned[] | BLOCKED in-app | Existing isolated Playwright harness is available / main |
| Native/federation | PLATFORM_MATRIX; FEDERATION_PROTOCOL; NETWORK_RIGHTS | No implementation | Host/tool probes only; no two-instance or native audio test | NOT_RUN | Implement independent milestones; no completion claim / main |

Logs remain in ignored target/checks/listening-*. Do not delete target: it also holds original private backups, release archives and active frozen runtime snapshots. Final commands/counts/source commit/runtime and remaining limitations are recorded in PROGRESS; preserved failed attempts are not passing runs.


Exact local package/runtime checkpoint: source bc50564979a9bcead7ed6667d6ae673df374e265; frozen extracted binary passed another35 Chromium/35 Firefox scenarios, setup/schema9/assets/SIGTERM and all release checksums. Original data was backed up/restored/logically compared, then upgraded in place without reset:1 user,20 tracks/assets,1 artwork,0 playlists; integrity/FK pass. Supervised loopback0.9.0 PID114613, eight served asset hashes match, no auto consent/analysis. Evidence: listening-upgrade-{backup,runtime}.json and listening-upgrade.log. Reinspect runtime on resume.
