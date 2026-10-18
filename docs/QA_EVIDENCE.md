# Verified0.12.0 package and preserved runtime checkpoint

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
| Native Linux client transport/audio | apps/desktop/src/{client,audio,lib,ui,main}.rs; apps/desktop/style.css |3a285e0/05951fd |28 desktop tests (26 run,2 opt-in); strict Clippy `--all-targets --features native-qa`; fmt; owned-null-sink audio test; real WebKitGTK window against isolated fixture server/account/media | PASS for Linux library/queue playback | Packaging, secure persistence, favorites/playlists UI, offline/background | main/audio/client/UI |
| MPRIS2 observer | apps/desktop/src/mpris.rs |05951fd | Ignored real-session test under private `dbus-run-session` after bounding update/command channels: properties and play/pause/next/previous/seek/volume forwarding | PASS for Linux session bus | External controller/session hardware smoke | main |
| Native favorites | apps/desktop/src/{client,ui}.rs; examples/native-ui-qa.{rs,js} | 0.12 local | Owner favorite/unfavorite API, filtered Favorites view, unfavorite re-read to empty, Library restore, queue-state sync;28 desktop tests (26 run,2 opt-in) and233 workspace tests | PASS for Linux native scope | Packaged/background/offline remain | main |
| Native playlist lifecycle | apps/desktop/src/{client,ui}.rs; scripts/verify_native_client.py; examples/native-ui-qa.{rs,js} | 0.12 local + current debug server | Fixture creates a real two-entry playlist; native list/detail, create→rename→cancel/confirm-delete, fixture survives, entry add/reorder/restore/remove, three-entry streamable playback, shared queue2 entries, Library return;25/27 desktop tests | PASS for Linux native scope | Entry editing/folders/albums/artists remain | main |
| Native album/artist browse | apps/desktop/src/{client,ui}.rs; examples/native-ui-qa.{rs,js} | 0.12 local + current debug server | Real album groups/detail (`Real Fixtures · Vela Test`), Play album native audio, real artist groups/detail, Play artist native audio, Library return;25/27 desktop tests | PASS for Linux native scope | No editing/edition identity | main |
| Web sleep timer | apps/web/src/main.rs; tests/browser/music.spec.mjs | 0.12 local | Root countdown 5/15/30/60, real60s audio still playing before expiry, paused+status at zero, cancel/restart stays playing; Chromium41/41+Firefox41/41 | PASS for browser scope | No OS-level timer/native sleep | main |
| Native queue/favorite parity | apps/desktop/src/ui.rs; examples/native-ui-qa.{rs,js} | 0.12 local | Move-down follows current occurrence, move-up restores, Clear-upcoming keeps only current, player favorite API+queue sync;25/27 desktop tests | PASS for Linux native scope | No drag/drop | main |
| 0.12.1 DSP patch release/runtime | Cargo.toml/lock; target/releases/vela-0.12.1-*; target/runtime/vela-0.12.1; scripts/install-user-service.py | `9aa062a` | Package verify; exact Chromium40 pass1 skip1.8m, Firefox40 pass1 skip2.2m; private0600 backup restore-verified; graceful stop, installer handover, health0.12.1, PID354876,0 restarts; counts/integrity unchanged | PASS for Linux local runtime/package | No external publication; spectral-1 rows retained; native0.12.0 package not repackaged | release/main |
| Local DSP tempo spectral-2 | crates/media/src/analysis.rs; crates/domain/src/audio_analysis.rs; apps/web/src/radio.rs; tests/browser/music.spec.mjs | 0.12 local | Normalized spectral-flux onset envelope, onset-interval median BPM, bounded confidence, extractor version bump;20 analysis tests incl real synthetic120 BPM click train and None on steady/silence/impulse; Chromium Radio shows spectral-2 + Estimated tempo + disclaimer; full Chromium40 passed,1 live-import skip in2.0m | PASS for bounded descriptive tempo/UI/radio regression | No calibrated beat/chroma; Firefox not rerun; real-music tempo accuracy unmeasured | media/domain/web |
| Native public peer playback | apps/desktop/src/remote.rs,ui.rs; scripts/verify_native_client.py; examples/native-ui-qa.{rs,js} | 0.12 local | Second isolated Vela instance: canonical public origin, explicit CC-BY stream publication, two isolated peers, save/remove origins in isolated private0600 config, unified search returns both origins, unreachable saved peer returns browse-only cached metadata with no play action, second-origin public playback, remote previous/next across both origins, credential-free;26/28 desktop tests | PASS for loopback debug transport | Internet TLS/two-host validation and remote pagination NOT_RUN | main |
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
| Home/root playback/exclusions | apps/web/src/personal_mixes.rs; tests/browser/music.spec.mjs |902dd47 |Chromium37; G07 rendered actual rotation/erase; PC01 real playback/hide/undo/restore/export/catalog race; PC02 held read cancellation | PASS for source run | Preserve exact package two-engine gates / main |
| SQL performance | media/src/recommendations.rs; examples/benchmark_recommendations.rs | SHA256 in final resource JSON |10k/100k/1m actual release + baseline projection/plan equality; raw samples/hardware/scope in PERFORMANCE | PASS within recorded scope | Tied timestamps/dense history and calibrated quality / main |
| Visual/platform scope | tests/browser screenshots; PLATFORM_MATRIX | Final screenshots pending | Source shots inspected; corrected capture for CSS transition settling; Docker daemon and in-app browser absent | PASS for recorded browser scope | Native/Safari/screen-reader remain unverified / main |

Preserved failed runs remain FAILED, not passing evidence. Original20tracks and20analysis
rows were backed up/restored; frozen0.9 runtime restored separately from0.10 candidate.
Private defaults were unchanged. Full native clients and public federation remain NOT_RUN.

# Current mandate evidence

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
