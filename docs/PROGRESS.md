# Verified progress

## DR-05 readable states and one feedback stack — 2026-10-21

The Home editorial and My Wave cards, the queue drawer and the idle player now
speak one English dictionary. The empty editorial card states «Choose music for
this space», the settled wave states «No suitable tracks yet», a pending wave
states «Loading your mix…», the idle player states «Nothing playing / Choose a
track», and the queue states «NOW PLAYING», «Saved queue» and «No tracks
queued. Add tracks from your library.». Real playlist, track and artist names
are never translated; only the system's own labels are.

The empty editorial and wave cards had almost white text on a white card in
the light theme (measured 1.114:1 for the title and 1.622:1 for the subtitle
before this change). Both empty variants now use the semantic text roles —
`--vela-text` for the title and eyebrow, `--vela-muted-text` for the subtitle,
note and wave copy, `--vela-danger` for the error — while the image-backed
card keeps its own light-on-artwork colours and its shade.

The root's three feedback signals — the recovery status, the Undo receipt and
the ordinary receipt — are projected into one `.feedback-stack` next to the
persistent player, outside the scrolling catalog. The DOM order is recovery,
Undo receipt, ordinary receipt, so a real failure is never hidden under a
receipt and the Undo is never lost when a general notice arrives. The stack is
a projection, not a new store: every signal and every handler stayed where it
was, and the old fixed `right/bottom` rules of the moved instances (including
the specific `.content-inner > .alert.success`) are gone. The stack scrolls
itself when it is taller than the space above the player, and it never takes
focus on its own.

The library toolbar and the sidebar account line now use the server's own
counts with the English noun that follows the count: «2 tracks · 2 albums · 1
artist» and «1 private track». An unknown statistics read states nothing at
all; no count is derived from the number of visible rows and no zero is
invented.

The Now Playing recovery summary and its Retry became one compact wrap row
(`flex-wrap`, 8px gap, 12px padding, `p` at `flex: 1 1 240px`, the action at a
44px minimum). The artwork's height reserve was re-derived from the dialog's
real height budget, so a normal track name with a genuine recovery status shows
its Retry in full at 1284×856 without scrolling, while the transport, the
volume and the artwork keep their own sizes.

Executed on this change: `cargo fmt --all`; strict `cargo clippy -D warnings`
for `vela-web`, `vela-server`, `vela-core`, `vela-domain`, `vela-media`,
`vela-imports` and `vela-scanner` with `--all-targets`; the release WASM build
for `vela-web`; `scripts/build-web.sh` (wasm-bindgen 0.2.129) into `dist/`;
the workspace test run for the same crates; and the Chromium and Firefox
Playwright suites. The package also adds a permanent browser journey,
`tests/browser/dr05-states.spec.mjs`, that owns its server and data and asserts
the live DOM's own contrast for the empty cards in both themes, the three
feedback signals in one column, their zero intersection and ≥8px gap at
1284×856/390×844/360×800/320×700, the stack's zero intersection with the
player and the mobile navigation, and the 44px height of every action in the
stack. The GTK development packages and `pkg-config` are now installed on
this host, so `cargo clippy --locked --workspace --all-targets -- -D warnings`
and `cargo test --locked --workspace -- --test-threads=2` both complete for
the whole workspace, `apps/desktop` included: clippy exit 0, and every test
result `ok` with 0 failed. The native client's WebKitGTK rendering and its
physical audio were not run — this host has no X server and no working
PulseAudio server. Measured browser evidence and its
exact numbers are in [QA_EVIDENCE](QA_EVIDENCE.md); the remaining limits are in
[KNOWN_LIMITATIONS](KNOWN_LIMITATIONS.md).

## Honest empty library state on Home — 2026-10-21

The Home page no longer promises collections that do not exist. On a confirmed
empty library the «ПОДБОРКА» / «ПЕРСОНАЛЬНЫЙ МИКС» editorial pair and the
«Your next listening chapter» personal-mixes section are not rendered at all,
and the page shows one state instead: a heading, one explanation line, and the
single existing «Add music» action (the root's real upload/import dialog). No
mixes, track counts, recommendation promises or invented percentages remain
on an empty library.

Real import progress comes from the existing owner-scoped `/api/jobs` endpoint
(`kind=import&state=active`, bounded to 5 rows) through
`jobs::read_import_activity`. Only `queued` / `downloading` / `processing`
rows are projected, and the byte counters are the server's own numbers: a task
whose total is unknown or zero keeps `None`, so the page never renders a
made-up percentage. The read polls every 2 s only while the server still
reports a running import and stops as soon as none is left; the page's abort
signal cancels it physically on unmount.

The library state is tri-state (`None` = unknown, `Some(true)` = empty,
`Some(false)` = has music) and is derived from the server's `/api/stats`
count. `None` never paints an empty library and never hides a real
collection: Home falls back to its own settled bounded reads, and personal
mixes keep their normal read until the server confirmed an empty library.

Executed on this change: `cargo fmt --all`; strict `cargo clippy -D warnings`
for `vela-web` (host and `wasm32-unknown-unknown`, `--all-targets`) and for
`vela-server`/`vela-domain`; `cargo test -p vela-web` → 45 passed, 0 failed
(39 pre-existing + 6 new job-projection tests); `cargo build --locked -p
vela-web --target wasm32-unknown-unknown --release`; and the full web asset
build via `scripts/build-web.sh` (wasm-bindgen 0.2.129) into `dist/`.
Browser, native and Docker validation were NOT run for this change.
`apps/desktop` could not be checked on this host at the time of that change:
`pkg-config` and the GTK development packages were absent, so its `glib-sys`
build script failed before compiling. The GTK development packages are
installed now, and the workspace clippy and test commands complete for
`apps/desktop` as well — see the DR-05 section above.

## Distribution CI/CD 0.12.8 — 2026-10-20

Published [v0.12.8](https://github.com/Fynth/vela/releases/tag/v0.12.8) from tagged source
`eb0fb8cd059e2da03ef94abe9265d4a4d3505c63` at 2026-10-20 19:21:09 UTC.
[Release workflow 39479453423](https://github.com/Fynth/vela/actions/runs/39479453423) completed successfully: quality, Linux,
Windows, Android, real Arch makepkg, Docker image build, aggregate verification,
GitHub publication and AUR publication. The public release has nine payloads and
21 assets including SHA256 sidecars, the consolidated report, checksums and PKGBUILD.
EXE/ZIP, AppImage, DEB, signed arm64 APK and desktop/server/web/AUR tar.gz are real
built artifacts. See [release ledger](RELEASE_LEDGER_0.12.8.md) for exact hashes.

[AUR vela-music-bin 0.12.8-1](https://aur.archlinux.org/packages/vela-music-bin) was published at recipe commit
`10c82b3c5810d9b6a28a8497433d2e861468c887`. An anonymous public clone downloaded the
published source tar, passed SHA256 and built with `makepkg --noconfirm --nodeps`;
the packaged executable printed `vela-desktop 0.12.8`. After the AUR metadata snapshot
updated, `yay -Si vela-music-bin` returned the correct version and maintainer.
Installation command: `yay -S vela-music-bin`. No host installation was performed.

Executed quality checks: fmt, strict workspace/all-target, WASM and native-QA
Clippy, release WASM build, actionlint 1.7.12, 374 Rust tests (3 opt-in ignored,
including 33 passing real API integration tests) and 38 Python tests. In final
CI, Chromium and Firefox each passed 60 tests with one pre-existing opt-in live
Internet import skip (4.3m/5.7m). Extracted optimized server/assets/schema 13 and
shutdown/worker-fault checks passed. Linux deb/AppImage and native Windows binaries
passed CLI smoke; the Arch build passed linkage/CLI checks. APK v2/v3 signatures,
identity/API 26 minimum, non-debuggable/backup-off/cleartext-off policies passed.
Docker image build passed; Docker runtime, new native GUI/physical audio,
Windows GUI and Android device/background validation remain NOT_RUN for this release.
The pre-existing AUD-20 maintenance warning remains informational in CI.

The v0.12.7 tag is preserved: its release gate rejected a packaging unit test
that depended on a pre-existing release executable. The corrected test supplies
its own fixture; no failed check was bypassed and no prior tag was rewritten.
Firefox CI now uses an owned PulseAudio null sink. Its real playback exposed a
10s SQLite writer wait in listening preferences/devices GETs; these now use a
deferred read snapshot and retention-filtered counts without pruning, insertion
or writer reservation. Existing writer/query-only/retention tests cover all four
listening read paths. TR03 observes genuine EOF for its short fixture and separately
checks unchanged source and advancing time through favorites on the 30s original.

Reusable quality/package workflows retain artifacts on main/PR/manual runs;
version-matching pushed tags publish only after all gates and then update AUR.
Pinned prebuilt wasm-bindgen and retained compiler caches avoid duplicated work;
optimized extracted-server checks stay in the package job. See
[DISTRIBUTION](DISTRIBUTION.md) and [ADR0025](DECISIONS/0025-release-artifact-gates.md).
Dedicated AUR and durable Android signing identities are configured through
repository secrets; private backups remain ignored under `target/private-release-keys`
(directory 0700/files 0600). The AUR host key was independently verified; the
owner-authorized profile update preserved existing public keys. Credentials and
cookies stayed in process memory. Frozen runtime, original data and target
checkpoints were not upgraded or deleted; this is no new live-health observation.

## AUD-05 cold offline launch — fixed (2026-10-20)

`3aeee09`: a private non-secret profile (canonical origin+account only,0600, atomic,
generation-guarded, cleared on sign-out) lets a cold start open a read/play-only
Offline library when the server is unreachable/expired or no session exists. Startup
reads profile+copies locally, attempts the existing saved-session resume once when it
names that origin, and otherwise/after failure shows real saved tracks with Play via
offline::prepare and the single AudioEngine. A real sign-in wins; one automatic startup resume occurs only when a profile
with real copies names the same canonical origin as the saved session, as documented;
nothing autoplays or retries; saving/removing/favorites/search/playlists/server routes stay unavailable.
Native QA `--test-offline-launch` (server stopped) asserts offline rows, one source and
advancing position, no server controls, normal close,0 temp originals. Desktop124
passed/3 ignored; combined X11 260 + offline26 + resume6 PASS. Hardware/SR/Wayland
full/other OS remain NOT_RUN.

## Queue snapshot canonical origin — fixed (2026-10-20)

The queue snapshot previously stored and compared the raw origin string, so a queue
saved by a fresh login typed without the trailing `/` could fail to restore after a
canonical resume (`…/`). `QueueSnapshot::new` now stores
`client::canonical_origin(origin)`, `load` compares the same canonical value, and
`validate` rejects non-canonical/empty/foreign forms. Unit coverage proves
`https://a.example.org` and `https://a.example.org/` are one identity. This closes the
adjacent FIX-04 step-7 gap reported during AUD-06; cold offline launch (AUD-05) remains
open.

## AUD-06 origin-scoped offline namespace — fixed (2026-10-20)

`a913574`: offline copies now live under
`$XDG_DATA_HOME/vela/offline/<blake3(canonical origin)>/<account>/` and every manifest
records the canonical origin. A manifest whose origin/account does not match the
requested namespace is ignored, so two origins with identical UUIDs cannot share a
copy or grant. Legacy account-only files are preserved byte-for-byte but never
reinterpreted as a current-origin copy. UI calls pass `account.origin`; the native
harness matches the new depth. Desktop tests118 passed/3 ignored (offline module15
passed), strict clippy/fmt PASS, full native X11 peer/session QA260 checks PASS with
the new layout and tampered-copy integrity evidence. AUD-05 cold offline launch
remains open.

## AUD-09 measured stats/search cost — optimized (2026-10-20)

`87b7acd`: measured on an isolated 100k-owner fixture with the release binary
(same host, isolated temp data, real migrations and setup API):

| Endpoint | Before | After |
| --- | --- | --- |
| `/api/stats` p50 (8 clients) | 531ms | **119ms** |
| `/api/search?q=Signal` p50 (8 clients) | 138ms | **121ms** |
| `/api/search` p50 (1 client) | 22ms | 20ms |
| `/api/artists?limit=50` p50 | 30.9ms | 27.0ms |

Root causes, each isolated by SQL-level timing on the same fixture:
- the stats aggregate read the wide owner track row and the asset row for all
  100k rows (320-480ms). Migration 0013 adds covering
  `tracks(owner_id, media_asset_id, duration_ms)` and `media_assets(id, size_bytes)`;
  the same aggregate is now index-only (~26ms) and the three groups run on
  separate pool connections.
- the album visibility probe (one EXISTS per album) cost ~50ms for 8,334 albums;
  `tracks(owner_id, album_id)` answers it directly (~10ms).
- search built album/artist summaries for every match before LIMIT; it now
  matches bounded identities first and builds summaries for those rows only,
  and the three catalog reads run concurrently.

Search identity is unchanged: the returned track ids/titles match the
authoritative FTS selection exactly on the 100k fixture. Verified: fmt, strict
workspace clippy, workspace tests (25 suites ok), server API 33/33, media suites,
full Chromium 56/1/0, full Firefox 56/1/0. Migration 0013 was applied to an
isolated copy of the live data directory (schema 12->13, `quick_check ok`, 0 FK
violations, all three indexes present); the live runtime and `data/` were not
touched. Write amplification was measured as FIX-07 requires: inserting 2,000 metadata
tracks into the 100k fixture took 5.1ms with the three new indexes and 4.0ms without
them (+1.1ms), and the database grew 4.5MiB (202.4->206.9MiB). Only read-path
indexes were added, so the write cost is one extra B-tree per insert.

Remaining known cost: 8-client concurrency is bounded by the 4-connection
SQLite pool, so per-endpoint RPS stays a client-bound lower bound; no capacity
claim is made.

## AUD-10/AUD-11 reconciliation and keyring verification (2026-10-20)

AUD-10: the documents that previously contradicted the source now match it.
KNOWN_LIMITATIONS, SPOTIFY_PARITY_MATRIX, PLATFORM_MATRIX, NATIVE_CLIENT_CONTRACTS,
CONTRACTS, DESIGN_SYSTEM, PRODUCT_SPEC, PERFORMANCE and CURRENT_STATE now describe
the delivered TR07 menus, the Settings hub instead of the removed More sheet, the
origin-scoped offline namespace, the cold offline launch and the measured AUD-09
costs; dated historical entries stay historical and point at their successors.
AUD-11: the keyring subprocess risk is verified closed in current source by 13
`session_store` tests — `SavedSession` prints `<redacted>`, helper stdout is bounded
before buffering (`MAX_SECRET + 1`), a hung helper is killed and reaped at the
deadline, invocations are serialized, and a physical cancellation is never treated
as cancelling the external process. No token logging path was found in the audit
and none was introduced. AUD-12/13 were fixed in earlier source; AUD-14–AUD-20
remain open in the dated audit record.

## TR12 release ledger / asset verification — 2026-10-20

Created [RELEASE_LEDGER_2026-10-20.md](RELEASE_LEDGER_2026-10-20.md). Package source
`4ceafdc` and HEAD `ba0209d` differ only in docs, so the launch-verified archive
(SHA256 `58da18db…`, BUILD `4ceafdc`) matches the shipped source behavior. Verified
byte-identical `dist/style.css`/`dist/player.js`, release WASM
`9d925109…`, both Manrope subsets/brand PNG hashes, and the package launch result.
The frozen `0.12.6` runtime (PID185315, 0 restarts, health ok) and `data/` were not
modified; the previous `7fe1ca0` desktop tar is preserved under `previous/`. Handoff
lists every remaining platform/hardware gap and the version-collision caveat.

## TR11 integration/regression matrix2026-10-20 — executed, no source change

Baseline `e4c425d` on `main`; the only diff from this task is this section and the
TR11 section/row in [QA_EVIDENCE](QA_EVIDENCE.md). No source, Cargo/lock, test, asset,
migration, package or runtime change, and no commit were made. Host: Ryzen7
8845HS,16 logical CPUs,14GiB visible RAM, kernel7.2.10-1-cachyos, cargo/rustc1.99.0,
Python3.14.8, node26.11.1, Playwright1.64.0, Chromium156.0.8078.4, Firefox157.0.
Every run used isolated temporary data, random loopback ports and each suite's own
fixture accounts on the real login limiter. The frozen0.12.6 runtime
(`vela-local-development.service`, MainPID185315,0 restarts, `/health` ok) and the
original `data/` were not opened or modified.

Executed results, all under ignored `target/checks/tr11/`:

- **Rust** — `cargo fmt --all -- --check` exit0. Strict Clippy exit0 for
  `--workspace --all-targets`, for `-p vela-web --target wasm32-unknown-unknown` and
  for `-p vela-desktop --all-targets --features native-qa` (the last two re-run after
  a cache-only0.4s pass so the crates were really checked: `07-clippy-wasm-fresh.log`,
  `08-clippy-native-qa-fresh.log`). `cargo test --locked --workspace --no-fail-fast`
  gave **345 passed,0 failed,3 ignored** in42s: core33 (26 lib+7 `tests/jobs.rs`),
  desktop113/3 ignored, domain1, imports20, media98 (63 lib+catalog6+filters8+ingest8+
  metadata4+publications9), scanner10 (2+`tests/scanning.rs`8), server40 (7 lib+
  `tests/api.rs`33), web30, doc-tests0. The3 ignored are the opt-in owned-Pulse-null-sink
  audio test, the private-D-Bus MPRIS test and an explicit offline measurement.
- **WASM/asset build** — pinned CLI `wasm-bindgen 0.2.129`
  (`target/tools/wasm-bindgen-0.2.129-x86_64-unknown-linux-musl/`, same version as
  `Cargo.lock`). `VELA_WEB_PROFILE=dev scripts/build-web.sh` produced
  20,070,213 B dev WASM (sha256`3e1f17c0…`); `VELA_WEB_PROFILE=release` compiled for
  real38.5s and produced3,529,914 B (sha256`9d925109…`), which is what the browser and
  native runs loaded.
- **Browser** — full Chromium **56 passed/1 skipped/0 failed in3.0m**, full Firefox
  **56 passed/1 skipped/0 failed in3.4m** (57 declared: music44, prototype-transfer11,
  design2), against real isolated Rust servers and the current `dist`. The single skip
  is `music.spec.mjs:308 direct public downloadable audio URL imports…`, gated by
  `test.skip(!process.env.VELA_LIVE_IMPORT_TEST)`; no other test is skipped and none
  was weakened. Security/player/history/publication/scale/sleep-timer regressions kept.
- **Visual review** — read-only `target/checks/tr11/capture_visual.js` on real isolated
  servers measured320/390/768/1024/1284/1440/1728 in both engines: document and body
  overflow0, exactly one central scroller`#main-scroll`,1 audio element, player band
  120px(mobile)/88px(desktop), topbar63/75, content scroller ending exactly at the
  player top, mobile nav only at≤760 and never below the player, sidebar offset applied
  once and0 page errors.28 PNGs were captured; the320 Home and1284 Library images were
  opened and inspected (mobile Home at320 shows the mini-player+nav reserve; desktop1284
  Library shows216/75/88). The suites' own TR09 evidence records
  the reserve arithmetic:190=120+70 at320/390/671/760 and88 from761 up, scroller bottom
  = player top at all10 viewports.
- **Measured performance/resources** — `measure_startup.py` (bounded, private temp data)
  on the release binary: spawn→`/health` **21.89–28.57ms**, setup15.0ms, first
  `/api/me`0.35–0.41ms, first `/api/tracks`0.38–0.45ms, warm restart21.4–21.9ms,
  RSS12.8MiB→34.3MiB, SIGTERM3.1–7.1ms exit0, pre-restart cookie still valid; the debug
  binary measured330–368ms cold and25.6→49.9MiB RSS. `scripts/benchmark_http.py
  --threads8 --duration5 --tracks20` against a release server rebuilt from this source
  (sha256`346b8ea2…`): `/health`44,201 requests/5s p50 0.874 p95 1.379 p99 1.634ms,
  `/api/me`43,892 p50 0.882, `/api/tracks?limit=50`36,162 p50 1.069,
  `/api/search?q=Bench`29,751 p50 1.334, `/api/albums`41,545 p50 0.929, 0 errors.
  Chromium navigation timing: TTFB1.3–1.8ms, FCP120–164ms, app ready152–191ms,
  WASM fetch+compile4.6–7.1ms, route click→content56/42/34ms for Library/Albums/Home.
  These are single-host, single-client lower bounds, not capacity claims.
- **Native** — X11 wrapper (`--test-peer --test-session`, current debug server):
  **260 checks PASS**,15 PNGs (1280×840/1440×900/1728×900/640×700), **one AudioEngine
  with1 output stream on the owned `vela_native_test_null` sink and zero HTML media
  elements**,0 temporary originals after exit,3 TLS peer connections, Secret-Service
  session resumed without password and cleared after sign-out, v2 BLAKE3 offline copy.
  Wayland `--window-only`: **5 checks PASS**,1 PNG2556×2060 (viewport1278×1030 DPR2),
  normally tiled without resize requests,0 temporary originals. Wayland stays a
  window-only subset; no full Wayland resize/peer/offline run was performed.

Corrections retained, not hidden: the first WASM/native-qa Clippy invocations were
cargo-cache hits that checked no crate, and the first HTTP benchmark used a stale
release server artifact; both were re-run properly and the superseded logs remain. Two
harness runs of my own read-only scripts were discarded before the recorded ones (a
wrong Home heading selector produced one15s timeout, and the first visual pass reported
raw scroller counts including the sidebar). Git stays clean apart from these two
documentation files.

Honest gaps: physical/hardware audio output, screen reader/Orca, WebKit/Safari,
Android/iOS, Windows/macOS, Docker/other OS, Internet/public-CA federation, full
Wayland, packaged-runtime rerun, real touch/keyboard hardware and Better Design VALID
receipts are **NOT_RUN**. Next: TR12 package/asset verification, source-versus-artifact
ledger and the coherent documentation commit.

## Approved prototype transfer specification 2026-10-19 — documentation only

Prepared the English [reference transfer package](PLANS/2026-10-19/PROTOTYPE_TRANSFER/README.md)
and [portable DeepSeek prompt](PLANS/2026-10-19/PROTOTYPE_TRANSFER/START_HERE_DEEPSEEK.md)
at source anchor `4f16a32`: nine Markdown documents, 13 TR delivery packages and 48 PT
acceptance scenarios. It specifies the selected prototype across shell/Home/catalog/
playlists/player/queue/dialogs/Settings/mobile/native, with source mapping, role
collision migration, original asset/font hashes and a context-resumption protocol.

Inspected actual repository contracts/DTOs/client/style/build/tests and the prototype's
public HTML/CSS/eight linked JS chunks as data. Retrieved and visually opened the three
original PNGs; retrieved two WOFF2 subsets and inspected the upstream Manrope OFL.
Recorded eight numerical opaque-color contrast calculations. Better Design URL
extraction/guidance/review rules and scoped copy checks were used; its merged token
roles were reconciled with the explicitly dark published CSS rather than copied blindly.

Interactive browser unavailable: runtime selection returned no browser and discovery
returned an empty list. Standard web open failed; direct HTTPS source inspection
succeeded. No screenshot/click/focus/playback/native test was executed in this task.
Spacing returned needs-capture, not a completed measurement. Exact document/check/
preservation evidence is in the reference audit; application PT cases remain NOT_RUN.

No application code/CSS/dependency/schema/data/runtime changes, new app benchmarks,
deployment, publishing or delegation. Existing skeleton/artwork/appearance code is
preserved. The new design is a target, not a shipped UI. Future implementation must
receive its own assignment, preserve existing audio/privacy contracts and use the
non-destructive TR/PT sequence with real source-versus-artifact evidence.

## Interactive product playbook 2026-10-19 — documentation only

Prepared the English [Interaction Playbook](PLANS/2026-10-19/PRODUCT_EXPANSION/INTERACTION_PLAYBOOK/README.md):
9 Markdown documents, 96 proposed IX interaction briefs, 16 detailed product recipes,
11 dependency-ordered IN packages and a copyable implementation prompt. It supplements
the320-entry product catalog without counting overlap as new implemented features.
Core scope: geometry-matched skeleton/shimmer/static loading, right-click/More/
keyboard/touch actions, searchable playlist picker, receipts/Undo, queue manipulation,
mix previews, offline readiness and privacy-safe collaborative flows.

Inspected existing loading/actions/artwork/appearance and contracts at `b619253`
with pre-existing dirty native appearance work. During preparation, independent
`eb36048`, `9c1ba4f` and `8c339e3` committed native Settings and scoped artwork QA.
Reconciled the plan with actual 80–150% web/native settings and implemented native
artwork; their tests remain historical and are not rerun by this documentation task.

Loaded Better Design UI/UX/review guidance and checked selected WAI/MDN primary
references. Three scoped proposed-copy comprehension checks PASS: loading status,
queue receipt and playlist-picker introduction. These are copy checks, not visual/
DOM/platform approval. LOAD18/MENU22/ACT16/MOT12/FLOW12 rows specify future tests;
none is marked executed here. Document link/ID/table/preservation results are in
the playbook README.

No application-code/CSS/dependency/schema/data/runtime changes, application test
runs, new benchmarks, deployment, publishing or delegation performed by this task.
The pre-existing prompt.txt remains unread/unstaged. Next authorized implementation:
IN00 baseline, then IN01 truthful loading + IN02 single-item context actions, IN03
playlist picker/receipts and continuous IN10 accessibility/real-UI review.

## Product/settings/native expansion specification2026-10-19 — documentation only

Prepared the English [Product Expansion plan](PLANS/2026-10-19/PRODUCT_EXPANSION/README.md)
at source anchor6fa2346, incorporating the later native-client request. It contains
320 feature implementation briefs,120 proposed setting contracts,27 dependency-ordered
EP packages,30 user journeys, detailed Settings/scale architecture, ART/SCALE/SET
acceptance, platform audio/cache/prefetch/optimization delivery and general/native
agent prompts. These are proposals, not shipped features.

Inspected actual source/settings/config/cache/artwork/auth/tests and current dated
docs; surveyed selected primary product/platform/HTTP/accessibility sources and
loaded Better Design principles/review rules. Native ui.rs renders neutral covers
despite existing artwork_id and the authorized server art route; the plan prioritizes
the missing native image pipeline rather than inventing a tag-extraction failure.
Existing complete-file cache is retained; actual-next prefetch, progressive integrity
and pinned offline copies remain separate contracts.

The three pre-existing dirty native launch/QA files and unrelated prompt.txt are
preserved. No agents, application-code/CSS/asset/dependency/schema/data/runtime changes,
Cargo/browser/native test runs, new benchmark numbers, deployment or external publication
were performed. Earlier verification remains historical. Only document integrity/
preservation checks and scoped proposed-copy review apply; package README records limits.

Next implementation, after a new implementation assignment: native authorized artwork
plus typed device Settings/scale; then native restore/origin-safe offline, actual-next
preparation and measured native/platform completion in the specified dependency order.

## Row transport and frameless Hyprland repair2026-10-19

Evolved actual baseline91867f2 after the newer Better Design implementation request.
Original mint design, root audio owners and auth/cancellation boundaries remain.
No user data, schema, dependencies, Cargo.lock, frozen runtime, release archives or
Hyprland configuration changed. Initially untracked unrelated `prompt.txt` was not
read/staged/changed. No agents spawned. See [review](UX_INTERACTION_REVIEW_2026-10-19.md)
and [ADR0024](DECISIONS/0024-row-transport-and-linux-window.md).

Implemented current-row pause/resume without source/seek/order reset; distinct
duplicate occurrence selection; accurate requested-state names/icons; separate
context Play for albums/playlists/radio/mixes; native pending intent so a second click
prevents late autoplay; native per-entry group/playlist playback; route-independent
public Next/Previous; saved-source queue dispatch; retry/clear state repairs; honest
public/offline mode availability. Offline repeat-one retains its verified local
descriptor/domain at EOF. Shared Linux production/QA builder disables GTK decorations
and integrates window controls/drag region. Non-Linux retains system decorations.

Executed final evidence:

- fmt/diff whitespace PASS; strict workspace/all-target, native-qa/all-target and
  wasm32 web Clippy PASS. Workspace **293 passed,0 failed,3 ignored** in
  `target/checks/ux-modes-rust-tests-final.log`;22 Python tests PASS; JS syntax and
  native QA CLI bounds PASS. Native desktop/example debug build PASS
  (`ux-modes-native-build.log`), pinned dev WASM build PASS
  (`ux-toggle-web-build-final.log`). No release/Docker/mobile result is inferred.
- Full Chromium **43 passed/1 live-import skip in2.3m**, Firefox
  **43 passed/1 live-import skip in2.7m**, against real isolated Rust servers/current
  dev assets. UX01 plays PCM, seeks7s and toggles Library/Home/current queue rows
  while checking actual source/time. Existing PLAY/RES/NET/NP/auth/privacy/DSP/radio/
  sharing tests remain. Logs/screens `target/checks/ux-toggle-{chromium,firefox}-final/`.
  Browser source hashes remain unchanged by the later native-only mode follow-up.
- Native X11/WebKitGTK/Rodio **89 checks PASS**,10 complete PNGs with exact dimensions,
  actual640/1280/1440/1728 widths and0 abandoned originals. Includes pending double
  click, seek/row/queue/playlist-entry continuity, two-origin TLS/public transport
  after navigation and actual offline repeat-one EOF while server stopped.
  `target/checks/ux-modes-native-x11/report.json`.
- Actual **native Wayland**, normally tiled in Hyprland without resize/float requests:
  **5 checks PASS**,1 complete PNG, viewport1278×1030/DPR2→2556×2060, exact owning PID
  and xwayland=false verified. Frameless controls, seek/row source continuity and
  no overflow checked. `target/checks/ux-modes-native-wayland/report.json`.
  Both native runs use owned null sinks, not physical audio/hardware validation.
- Better Design principles/review rules loaded. Final scoped transport-guidance and
  window-action comprehension PASS after simplifying offline copy. Initial bundled
  copy check mixed mutually exclusive Play/Pause and unrelated window actions; it
  is not claimed as whole-screen approval. Actual DOM measurement completed: web
 390/1440/1728 themes **0 critical/0 serious/10 moderate**; four native X11 states plus
  tiled Wayland **0 critical/0 serious/5 moderate**. Native390 is outside its640px
  minimum; web320/390 checks are separate. Inherited off-scale values/web2px volume
  gap drift remain. `.better-design/ux-review.json` records scope. In-app Browser
  and review-ui-code unavailable; existing Playwright CLI/real WebKitGTK used.

Intermediate failures were fixed and preserved: NET02 now explicitly appends/selects
a different duplicate rather than expecting current-row restart; Radio's primary
Play now starts its context. Initial Wayland captures failed after Dioxus mutated
GDK_BACKEND following GTK startup; QA now independently checks actual compositor
protocol. Earlier artifacts remain under target/checks. Fingerprints:
`target/checks/ux-source-manifest.json`.

Final read-only runtime check: frozen service active0.12.6/MainPID1178/NRestarts0,
health OK. Initially inspected desktopPID115855 was not terminated by this task and
was absent at final inspection. Launch rebuilt `target/debug/vela-desktop` for the
new frame. Frozen server/web assets were not upgraded. No prefetch/handoff/mobile/
whole-product completion follows. Next: native artwork, stable duplicate-entry
provenance/native queue restore, then bounded progressive sources/prefetch/device work.

## Better Design contrast and listening-layout implementation2026-10-19

The newer explicit UI implementation request supersedes the earlier MD-only task.
Evolved the existing Dioxus web/Linux desktop from baseline0170ec5. No app data,
schema, authentication/source contracts, Cargo dependencies, frozen runtime or
release archives changed; the pre-existing untracked `prompt.txt` is untouched.

Implemented brighter original mint dark/light tokens, readable primary/secondary
text, visible controls and44px main targets; native shell grid with one sidebar
inset/content scroll/player row; explicit176px action column; compact create
disclosures and dismissible notices; concise route titles, real Library Play action
in the heading and removal of the oversized decorative banner; borderless artwork
shelves and scrolling web navigation. Playback still has the existing single root
owner. Native artwork/progressive preparation/prefetch/handoff remain separate work.

Executed evidence:

- `cargo fmt --all -- --check`, strict workspace/all-target Clippy, strict wasm32
  web Clippy, explicit native-qa example Clippy, native desktop/example builds and
  pinned wasm-bindgen dev web build PASS. `cargo test --locked --workspace --
  --test-threads=2`: **291 passed,0 failed,3 ignored**.22 Python tooling tests PASS;
  JS syntax and diff whitespace PASS. Logs: `target/checks/design-contrast-*.log`;
  final Rust log `design-contrast-rust-tests-final.log`. No release build/packaging
  or Docker/platform result is inferred from these debug builds.
- Final Chromium **42 passed/1 live-import skip in2.3m** and Firefox
  **42 passed/1 live-import skip in2.6m**, each against an isolated real Rust
  server/current dev WASM assets. Upload/artwork/decode, navigation continuity,
  queue/recovery/dialog/favorites/playlist concurrency, private history/recommendation,
  network authorization and stale-session regressions remain covered. DESIGN01 owns
  its own temporary server/session to avoid security tests' intentionally revoked
  cookies and per-IP login budget; production policies were not weakened.
  Logs/screens: `target/checks/design-contrast-{chromium,firefox}-final/`.
  Final DESIGN01 was strengthened to measure all visible matches of its selected
  selectors, not only the first row: separate Chromium5.8s/Firefox7.2s reruns PASS,
  38 measurements/theme/engine over320/390/1440/1728px, minima6.714:1 dark and
  4.536:1 light. Exact reports/screens:
  `target/checks/design-contrast-all-rows-{chromium,firefox}/`.
- Actual native WebKitGTK/Rodio fixture with `--test-peer`: **74 checks PASS**,
  ten complete PNG captures,1440/1728 requested widths match actual DOM widths,
  compact640 layout/queue actions and1280 normal library tested; no temporary
  originals remain. Source generation/advancing position survive navigation;
  private favorites/playlist/folder/album/artist/public peer/offline flows pass.
  `target/checks/design-contrast-native-final-evidence/`. Capture code now validates
  PNG dimensions; earlier cropped captures and a refused-width run are preserved
  as intermediate failures, not misreported as full native screenshots.
- Better Design loaded original tokens, UI/UX principles and review rules; three
  scoped comprehension checks PASS. Final real DOM spacing: web at390/1440/1728
  has **0 critical/0 serious/10 moderate**; native640/1440/1728 has
  **0 critical/0 serious/4 moderate**. Native390 is outside the supported fixture
  scope. Moderate inherited spacing deviations remain documented. No tool score
  implies global visual, screen-reader or WCAG certification.

See [review](DESIGN_REVIEW_2026-10-19.md),
[tokens](DESIGN_SYSTEM.md), [ADR0023](DECISIONS/0023-contrast-and-listening-layout.md)
and `.better-design/review.json`. Source fingerprints:
`target/checks/design-contrast-source-manifest.json`.

Checkpoint: source/debug binary `target/debug/vela-desktop` and `dist/` are rebuilt;
the frozen local service remains active0.12.6/MainPID1178/NRestarts0 at the final
read-only check. No runtime upgrade or external deployment occurred. Native QA is
X11/XWayland in an owned floated Hyprland window with a null sink; physical sound,
native-Wayland tiling, mobile/other desktop OS, screen readers and Safari are NOT_RUN.
Next: separately authorized native artwork/cache contract and wider accessibility
coverage, then Player Reboot progressive/prefetch/device work. Do not mark the full
product mandate complete from this UI increment.

## Player Reboot specification2026-10-19 — documentation only

Prepared [Player Reboot](PLANS/2026-10-19/PLAYER_REBOOT/README.md) in response to two
user screenshots and the explicit instruction to write only detailed Markdown.
Source inspection anchor: `10fe946dac88b0f94e3178946838f62b0363b799`; initial tree clean.
No subagents, application code/CSS/assets, Cargo files, migrations, user data,
credentials, existing backups, runtime upgrades or public deployment were changed.

Confirmed by code inspection: desktop `Client::prepare` receives/verifies an entire
original before `AudioEngine::load`; normal selections do not consult a reusable
catalog byte cache; current `prepared` is a source lease, not queue prefetch. The
server already supports authorized HEAD/Range/validators but uses no-store. Window
layout combines fixed panels and centered/max-width content with sidebar padding;
the native QA harness forces X11. The screenshots alone do not identify the exact
Hyprland strip cause, and the user's installed desktop executable is unidentified.
The currently observed user service uses the preserved frozen 0.12.6 server; newer
session/offline source repairs are recorded separately and not assumed installed.

The 16-file package specifies window diagnosis, cache/progressive integrity tradeoffs,
next preparation, device sync/handoff, independent optional Vela ID, rooms, native
Android/iOS/desktop, continuous colorful music UX, detailed screen states, 240 feature
proposals, W00–W13 delivery dependencies, future acceptance and data/privacy migration.
Primary standards/platform sources are linked with their verification limits.

No new Cargo/browser/native/audio/mobile/performance/restore checks were run; this
entry is not application verification and does not supersede prior dated evidence.
Only documentation structure/links/feature enumeration and diff hygiene are checked
for this deliverable. Next implementation remains W00 build/runtime/data safeguards,
then normal Hyprland geometry and reusable authorized media sources; implementation
is outside this user's current MD-only request.

Documentation checks: PASS for 16 new Markdown files, relative-link targets, table
column consistency, whitespace and 240 ordered unique F-IDs in 16 groups of 15;
`git diff --check` passed. These checks validate the plan artifact only.

## Documentation reconciliation2026-10-19 — docs-only, no verification

Created [CURRENT_STATE.md](CURRENT_STATE.md) as the source/deployed/design index and
corrected stale statements without deleting dated evidence: KNOWN_LIMITATIONS (native
offline/Secret Service/folders and the AUD-20 dependency gate), PLATFORM_MATRIX
(0.12.6 current vs0.12.2 historical, spectral-4,0.12.6 native package), QA_EVIDENCE
(G14/G15 PARTIAL with the exact tested subset), RESILIENT_SCALE_PLAN (personal SQLite
profile vs opt-in large hub), ROADMAP, SECURITY, NATIVE_CLIENT_CONTRACTS and the
parity ledger. No code/schema/runtime/test/package was changed and no check was
executed for this pass; this entry is a documentation record, not evidence. The dated
package/run logs remain valid for their own dates, and the audit package under
`docs/PLANS/2026-10-19/` is untouched.

## Offline routing follow-ups — source verified (2026-10-19, `03b2523`)

Fixed the four regressions observed during the offline repair: natural EOF during
offline playback now advances the saved-copy queue (Repeat::One keeps the replay
path) instead of routing to server prepare; MPRIS Next/Previous dispatch through the
offline-aware handlers; `start_offline_at` no longer early-returns outside
Library/Favorites; `cancel_all` cancels `folder_gate`. Desktop56 passed/3 ignored and
strict native Clippy pass. Natural-EOF behavior is covered by inspection/compilation,
not a new real-window EOF assertion.

## AUD-03/07/08/12 offline repair — source+native verified (2026-10-19)

`e225b63` moves offline prepare off the UI thread: a ticket/checking state is shown,
`offline::prepare` runs in spawn_blocking, the same opened descriptor is hashed and
decoded, cancellation is checked between64KiB blocks, and late/cancelled results
drop the lease without writes. Removing a non-current saved copy remaps queue/order
and keeps the active source/position; only removing the current occurrence stops
playback. Every offline await re-checks gate generation, account epoch, user id,
origin, cancellation and window lifetime. Manifests are capped before allocation;
staging names are unique/0600; quota counts staging/orphan bytes and only the
replacement delta; failed publication restores the previous healthy pair.

Verified: fmt, strict workspace+WASM Clippy, desktop56 passed/3 ignored, and the real
native window/server-stopped QA (`target/checks/aud-offline-p1/`) covering offline
listing, queue advance/return, removal of the other copy and Secret Service resume.
Still NOT_RUN: two-process concurrent save, disk-full/power-loss boundaries, explicit
checking/Cancel harness assertions. Newly observed but unfixed: offline EOF routes to
server prepare, MPRIS next/prev bypass offline routing, offline start limited to
Library/Favorites, folder_gate not canceled by cancel_all.

## AUD-20/21 dependency assessment — read-only (2026-10-19)

A subagent traced the advisory gate: mpris-server0.10.0 is the only consumer of
zbus5.19.0 and pins `zbus ^5.14`; zbus6.0.0 (2026-10-16) requires zbus_utils/macros
crates absent from the local registry, so `cargo update --offline --dry-run` cannot
resolve. No fix applied, no ignore added; the FAIL stays visible with a review
trigger. The same pass ran `cargo audit --no-fetch` and found a **compiled**
`glib0.18.5` RUSTSEC-2024-0429 via gtk/tray-icon/dioxus, which the pinned
cargo-deny advisories check does not report (AUD-21); rsa0.9.10 and rand0.7.3 are
lockfile-only/not compiled. Recorded in SECURITY/CURRENT_STATE/KNOWN_LIMITATIONS;
no implementation or dependency change is claimed.

## AUD repair slice: session lifecycle + history snapshots — source verified (2026-10-19)

Three subagents worked in disjoint zones; docs reconciliation plus two code repairs
were integrated and verified.

- AUD-01/02/04/11 (`3094754`): sign-out clears private state/stops audio before an
  off-thread, deadline-bounded Secret Service deletion; only a genuine 401 deletes a
  saved item (network/timeout/5xx/403 preserve it with Retry); attempt generations
  prevent superseded saves/clears; secret-tool is byte-bounded, kill+reaped and
  serialized; SavedSession Debug is redacted. Desktop43 pass/2 ignored; the real
  two-process Secret Service save/resume/sign-out QA passed
  (`target/checks/aud-session-p0/`).
- AUD-13 (`48b2fa4`): owner history/export read one deferred snapshot and never prune
  or reserve a writer; the hourly retention worker owns physical deletion. Core26+7,
  API31; red/green showed 2.52ms success under an open writer vs 10.03s
  SQLITE_BUSY on the old path.
- Docs reconciliation in this worktree adds `CURRENT_STATE.md`, fixes stale
  limitations/parity/matrix claims and records AUD-20 dependency FAIL honestly.

## Planning-only source audit2026-10-19 — application unchanged

At user request, inspected clean main05b4c37/version0.12.6, current code/contracts,
known limitations, benchmark artifacts and frozen original-data runtime. Created
[a detailed audit and planning package](PLANS/2026-10-19/README.md):20 identified
defects/risks/gaps, ordered repair regressions, product roadmap, implementation
recipes, screen/state UX, original Listening Atlas visual system and42 prospective
work packages. No tasks were delegated and no application feature/schema/runtime
was changed. Proposed fixes/design are not marked verified implementations.

Executed again: fmt, strict workspace/all-target Clippy and WASM Clippy passed;
workspace244 tests passed/0failed/2ignored; Python22 passed. Evidence:
`target/checks/audit-20261019/`. Hardware-dependent ignored tests, full browser/native
journeys and release builds were not rerun. Read-only health0.12.6/unit active
PID544183/0restarts; SQLx migration12, quick_check ok/FK0, original1account/20tracks/
20assets/0playlists/0publications retained. `user_version=0` is unused by these SQLx
migrations. Browser discovery returned none; isolated Playwright rendered the
separate static design board in6views, with local fonts,0JS errors/no document
horizontal overflow and12 passing normal-text contrast pairs. Concept captures
are not application E2E or accessibility certification. Original runtime/packages/
backups remain untouched. New repair findings are still outstanding.

Fresh dependency check FAILED on3INFO Unmaintained advisories for
zvariant/zbus_names/zvariant_derive through MPRIS/ZBus5 (AUD-20); bans/licenses/sources
passed. No advisory ignore or dependency code was changed. This supersedes prior
dependency-policy PASS claims for the current advisory database, not the dated logs.

## Bounded onset-tempo DSP spectral-2 — verified (2026-10-18, `966505d`)

The local analysis extractor is now `vela-pcm-spectral-2`. It measures a normalized
half-wave-rectified spectral-flux envelope, detects onsets at least twice the mean
with a refractory gap, filters inter-onset intervals to60–200 BPM, and estimates BPM
from the median interval. Confidence is onset-interval regularity scaled by interval
support. At least64 windows,4 onsets and3 intervals are required; flat, too-short or
irregular material returns no tempo. Chroma remains absent and no beat annotation is
claimed. Existing immutable spectral-1 caches are not reinterpreted.

Evidence:20 analysis tests (including an8-second120 BPM click train measuring within
115–125 BPM with bounded confidence, and steady/silent/impulse material remaining
None), full `vela-media` tests, strict media Clippy. The browser Radio measurements
now display Extractor `vela-pcm-spectral-2`, an "Estimated tempo" row and the honest
onset-regularity disclaimer. Full Chromium run with the rebuilt web assets:40 passed,
1 live-import skip in2.0m, including G08/G09, G07 and PC01–02. Screenshot:
`target/checks/dsp-tempo-v2/radio-1440-dark.png` (Drift is a steady tone, so the row
correctly shows "Not established"). No schema, server HTTP, native or original-data
runtime change. The live runtime was subsequently upgraded to0.12.1; existing
analysis rows remain spectral-1 until explicitly reanalyzed.

## Native offline startup integrity sweep — verified (2026-10-18, `2706f4b`)

After sign-in/resume, a bounded blocking sweep re-hashes up to100 saved offline copies
against their v2 BLAKE3 manifests. Corrupt rows are labelled "Integrity check failed"
and their Play offline control is disabled; bytes are retained for explicit removal or
re-save. Legacy v1 length-only rows are reported as unverified rather than healthy.
This is integrity maintenance, not content/entitlement sync.

Real-window verification tampers one byte of the remaining saved copy after the full
journey, restarts the primary server, resumes the saved session in a fresh native
process, and confirms the startup sweep labels the row and disables its Play button;
the session then signs out and keyring deletion is verified. Unit test covers the sweep
report. Evidence:`target/checks/native-offline-sweep/`;28/30 desktop tests with both
opt-ins, strict Clippy/fmt. No schema/server/web/original-data/runtime change.

## Native/offline/folder hardening — verified (2026-10-18, `8d7d565`)

Self-review of the newest slices found and fixed concrete issues:
- Secret Service save now clears the fixed service attribute first, preventing duplicate
  stale tokens; explicit sign-out waits for deletion and surfaces a visible error if the
  item remains; malformed startup items are cleared.
- Offline playback opens the audio once, hashes that descriptor, rewinds and decodes the
  same descriptor, removing the path-reopen TOCTOU; failed manifest writes remove the
  copied audio; removing the currently playing copy clears queue/order state.
- Playlist folder moves now validate folder ownership and playlist CAS inside one
  IMMEDIATE transaction instead of a check-then-update race.

Evidence: two-process Secret Service save/resume/delete and server-stopped offline queue
with v2 digest manifest (`target/checks/native-hardening2/`);28/30 desktop tests with
both opt-ins,31 API tests, strict workspace+WASM Clippy/fmt. No schema/server/web or
original-data/runtime change. An independent subagent review was started but produced no findings
before it was stopped; the self-review fixes above are the recorded result.

## Native offline BLAKE3 manifests — source verified (2026-10-18, `eb68780`)

Offline copies now write v2 manifests carrying the BLAKE3 digest of the copied device
bytes. Play offline re-hashes the file and refuses a same-size tampered copy before
creating a decoder; legacy v1 length-only manifests remain readable and gain a digest on
the next save. Quota/account scoping/0600 modes are unchanged.

Evidence: a unit save/load test asserts a64-hex digest, flips one byte without changing
length, and verifies prepare refuses the copy; the real server-stopped WebKitGTK journey
downloads two tracks, checks v2 digest manifests and plays/Next/Previous/removes with the
server down. Evidence:`target/checks/native-offline-hash2/`;28/30 desktop tests with both
opt-ins, strict Clippy/fmt. Source postdates0.12.5; no schema/server/web/runtime change.

## Web peer preview cursor pagination — source verified (2026-10-18, `e3b6168`)

The web Network preview now supports bounded page continuation: `PreviewPeer` gained an
optional UUID cursor the server validates on every page **and forwards to the peer's
`/api/public/catalog?...&cursor=...` request**. (canonical ID, empty/oversized
rejected), the web Load more button revalidates the same canonical origin, dedupes rows
by track ID and disables at exhaustion. Existing per-page invariants (ID ordering,
next_cursor==last item, per-track text/duration/rights checks) are unchanged.

A server fixture test now records the outbound peer URI and asserts page2 contains the
validated cursor (and that a non-UUID cursor is rejected), closing the gap where the
cursor was validated but never forwarded. Note: the Chromium suite does not exercise a
successful public preview (PN02 covers rejection paths), so Chromium's green result is
non-regression only for this success path.

Evidence: server network lib tests4 (including cursor forwarding) and API31 pass; strict
workspace+WASM Clippy/fmt; full Chromium41 passed+1 live-import skip in2.0m and
Firefox41+1 skip in three clean runs (159s/175s pre-fix and 177s at HEAD `3a3f463`
after the server cursor-forwarding fix) from `target/checks/preview-cursor-firefox/`.
Source postdates the live0.12.6 runtime; the next patch will deploy it. No schema or
original-data change.

## Phase 0 follow-up: 100k HTTP catalog + artist grouping fix — verified (2026-10-19, `1a6bf32`)

Extended `scripts/benchmark_http.py` with `--seed-tracks N`: one isolated server
lifetime runs real migrations/setup, stops gracefully, inserts metadata-only rows in
1,000-row transactions, restarts on the same port and requires the pre-restart cookie
to authenticate. Verified at 100,020 tracks / 8,334 albums / 1,001 artists (seed
~40-42s, DB ~200MB, FTS integrity/quick/FK checks, cleanup asserted).

Measured release 100k (8 Python threads, client-bound; 0 errors): `/api/tracks` p50
2.5ms, `/api/albums` 5.8ms, `/api/health` 1.0ms, `/api/search?q=Signal` 156ms,
`/api/stats` 665ms. `/api/artists?limit=50` was 44-58s due to a planner choice in the
two-arm `ARTIST_TRACKS` union; forcing `tracks_catalog_artist` plus
`albums CROSS JOIN tracks` (albums-first) reduced the release endpoint to **29.8ms**
on the same 100k catalog. Evidence: `target/checks/phase0/http-bench-100k{,-fixed3}.json`,
PERFORMANCE.md. `/api/stats` was then reduced from 665ms to **490ms** by the same index/join
pinning (`http-bench-100k-stats.json`); its ~300ms owner-wide COUNT/SUM join remains
and needs a covering index or incremental counters. Broad search (~156ms) is the
next read target. No schema, live-runtime or original-data change.

## Phase 0 measurement baseline — verified (2026-10-18, `72d9860`)

Executed the first plan phase with reproducible, isolated numbers: 100k metadata
(first page p50 0.100ms, favorites p50 1.53ms), FTS rare/absent with the adaptive
bounded plan (0.100/0.015ms p50 vs 7.75/7.97ms baseline), and a new stdlib HTTP read
harness (`scripts/benchmark_http.py`) that starts a fresh server, creates a throwaway
account, uploads 20 synthetic WAVs and measures five read endpoints with 8 client
threads. Raw JSON/logs under `target/checks/phase0/`; full scope and caveats appended
to PERFORMANCE.md. The HTTP figures are Python-client-bound and explicitly not a
server capacity claim. No live data/runtime access; no schema change.

## 0.12.6 session/offline integrity handover — verified (2026-10-18, `4a4351e`)

`target/releases/vela-0.12.6-{x86_64-unknown-linux-gnu,source}.tar.gz` and
`vela-0.12.6-SHA256SUMS` were built from commit `7fe1ca0` (native
`a02cb38e1f9a38288cefa9a55d0206c6c25fcab518d6c3dcd1bfb184ebf633bf`, source
`e31f817279b2356748d3f341f6aeec13d1632c1baf45967a88a59dab83c2d865`) and extracted to
`target/runtime/vela-0.12.6/` (schema12). Exact-package Chromium passed41+1 live-import
skip in1.9m and Firefox41+1 skip in2.2m, covering opt-in session resume, the offline
integrity sweep, folder lifecycle, spectral-4 Radio and player/publication regressions.

A portable native0.12.6 archive
(`target/desktop-releases/vela-desktop-0.12.6-linux-x86_64-unknown-linux-gnu.tar.gz`,
SHA256 `7abe3982073470d36e4680b56845aa6858a0c21b3c86f2007862ba9ba16d6063`, BUILD commit
`7fe1ca0`) passed exact manifest/modes, `vela-desktop 0.12.6`, ldd, styled sign-in
launch, normal close exit0 and zero temporary originals. It includes the session resume
flow, v2 offline manifests with startup sweep, folder UI and all prior native features.

TR10 (2026-10-20) rebuilt that canonical0.12.6 desktop archive from source `4ceafdc`
(SHA256 `58da18db0f0af4986ab1069840682f2e673f3024f5137fe25241820c00a367ac`,
5,323,519 B): both Manrope subsets are embedded as data-URI faces,260-check native X11
QA passed (fonts/Cyrillic metrics, frameless controls, scale80-150, one AudioEngine),
and `verify_desktop_package.py --launch` passed with a real window, exit0 and0
temporary originals. The prior `7fe1ca0` archive (`7abe3982…`) is preserved under
`target/desktop-releases/previous/`; the live server runtime remains `7fe1ca0`.

`before-0.12.6-20261018T204840Z.tar.gz` (440464582 bytes,0600) was created while the
live DB was schema12 and restore-verified in an empty isolated directory (integrity/FK
ok, schema12,20 tracks). The0.12.5 unit file was preserved at0600. The service was
stopped gracefully and the installer validated/installed0.12.6 without migration.

Post-handover: health0.12.6; MainPID544183 active/enabled,0 restarts; integrity/FK ok;
schema12; counts unchanged (1user/20tracks/20assets/20analysis/1artwork/0playlists/
0folders). Rollback remains the preserved0.12.5 runtime/unit and this backup.

## Native opt-in Secret Service session — source verified (2026-10-18, `2e12554`)

Native sign-in now offers an explicit Remember-this-session checkbox (default off). When
enabled, only the opaque64-hex session token and canonical origin are written through
stdin to the freedesktop Secret Service; password, username, CSRF token and history are
never stored. The sign-in screen shows Continue saved session only when an item exists;
resume calls `/api/me` with the stored cookie and rebuilds CSRF from that response. No
automatic passwordless login occurs. Sign-out deletes the item; expired/invalid saved
sessions are deleted with a visible error. Release uses a fixed service name; QA
overrides it only in debug.

Two-process QA: the full journey opts in at its final sign-in, Python verifies the
Secret Service envelope contains origin+64-hex token and no password/username/CSRF; a
second native process resumes via the explicit button, loads the private library,
signs out and Python confirms the item is gone. Evidence:
`target/checks/native-session/`;28/30 desktop tests with both opt-ins, strict
Clippy/fmt. Source postdates the preserved0.12.5 desktop archive; Windows/macOS/mobile
secure stores and biometrics remain absent.

## Native TLS federation — source verified (2026-10-18, `e605c83`)

Native peer transport now supports a debug-only HTTPS loopback override with a test CA,
so the real reqwest client performs certificate-chain/SNI/handshake validation for
public catalog, search, cached fallback and stream preparation. The isolated harness
generates a one-day local CA and IP-SAN server certificate, runs a TLS terminator in
front of the second Vela peer and records accepted TLS connections. Release builds
exclude both the CA and transport overrides; a release example contains neither
environment string.

The WebKitGTK journey completed unified search, public playback and cache/pagination/
offline checks through the TLS-terminated peer with3 recorded TLS connections. Evidence:
`target/checks/native-tls3/`;27/29 desktop tests with opt-ins, strict Clippy/fmt, and a
release example string check (0 occurrences of both override env names). This is
loopback TLS only; Internet DNS, public CA, two physical hosts and deployment hardening
remain NOT_RUN. No schema, server HTTP, web or original-data/runtime behavior change.

## 0.12.5 spectral-4/native folders handover — verified (2026-10-18, `b8b9ef5`)

`target/releases/vela-0.12.5-{x86_64-unknown-linux-gnu,source}.tar.gz` and
`vela-0.12.5-SHA256SUMS` were built from commit `3eedf1b` (native
`12eb12cf2e577a7da32b0efec307140bf3b67b035e77d50c728bf61f59da93d6`, source
`42cd778037f136735de6f6d59d474e90494b922f0ded52180e61bedf2f1fad26`) and extracted to
`target/runtime/vela-0.12.5/` (schema12). Exact-package Chromium passed41+1 skip in1.9m
and Firefox41+1 skip in2.2m, covering spectral-4 key and the web folder lifecycle.

A portable native0.12.5 archive
(`target/desktop-releases/vela-desktop-0.12.5-linux-x86_64-unknown-linux-gnu.tar.gz`,
SHA256 `c6bd29810be2da28faecfa02bc90a0b56b10b52a89add77905385aaa5349d3e7`, BUILD
commit `3eedf1b`) passed exact manifest/modes, `vela-desktop 0.12.5`, ldd, styled
sign-in launch, normal close exit0 and zero temporary originals. It includes native
folder create/filter/move/rename/delete plus offline/cache/pagination features.

`before-0.12.5-20261018T194334Z.tar.gz` (440464629 bytes,0600) was restore-verified in
an isolated empty directory (integrity/FK ok, schema12,20 tracks). Since schema was
already12, no separate migration was needed. The0.12.4 unit file was preserved at0600;
the service was stopped gracefully and the installer validated/installed0.12.5.

Post-handover: health0.12.5; MainPID498859 active/enabled,0 restarts; integrity/FK ok;
schema12; counts unchanged (1user/20tracks/20assets/20analysis/1artwork/0playlists/
0folders). Rollback remains the preserved0.12.4 runtime/unit and backup.

## DSP heuristic key spectral-4 — source verified (2026-10-18, `3f97662`)

`vela-pcm-spectral-4` adds a bounded Krumhansl–Kessler key estimate from normalized
chroma. At least three pitch classes must exceed5%; all24 major/minor rotations are
Pearson-correlated, the best is labelled and confidence is the clamped margin over the
next-best profile. Single pitch classes, silence and missing chroma return no key.
Spectral-3 caches are retained unchanged.

Unit evidence: synthetic C-major→`C major` and A-minor→`A minor` triads, A440 single
tone→None, finite/bounded descriptors;22 analysis tests and full media tests. Web Radio
shows Estimated key with the heuristic margin and disclaimer. Full Chromium41 passed+1
live-import skip in2.2m. Shipped in the verified0.12.5 runtime. No schema, HTTP, native or original-data change.

## Native playlist folders — source verified (2026-10-18, `9bfdb39`)

The native Playlists view now uses the real folder APIs: create, per-folder filtering,
CAS rename/delete and CAS Move-to-folder from playlist detail. Deleting a folder
unfiles its playlists and restores them under All playlists; an unfiled keeper is
excluded while a folder filter is active. Errors/busy states remain visible; no folder
drag/drop or cross-account grouping is claimed.

The isolated WebKitGTK journey creates a folder and an unfiled playlist, moves the
fixture playlist into the folder, checks exclusion under the filter, renames and
deletes the folder, and verifies both playlists return unfiled. Evidence:
`target/checks/native-folders2/`;27/29 desktop tests plus both opt-ins, strict
Clippy/fmt. Shipped in the verified0.12.5 portable desktop archive. No schema/server/web/original-data change.

## 0.12.4 playlist folders schema12 handover — verified (2026-10-18, `e5ef26a`)

`target/releases/vela-0.12.4-{x86_64-unknown-linux-gnu,source}.tar.gz` and
`vela-0.12.4-SHA256SUMS` were built from commit `16034b9` (native
`78616c0957767096d5d8e31d7f7871c2cb1a55eb2f507b3b7c0cc5468b5e9687`, source
`4faefc31f5cdf27094c3fe0fd947f5f94d41151128a51a0185d0bd93fa784916`) and extracted to
`target/runtime/vela-0.12.4/` (schema12). `verify_package.py` passed. Exact-package
Chromium passed41+1 live-import skip in1.9m; Firefox passed41+1 skip in2.3m on a clean
rerun after its first run hit the known sleep-timer pause flake and stopped the suite.
Both include the real playlist-folder lifecycle.

A portable native0.12.4 archive was built
(`target/desktop-releases/vela-desktop-0.12.4-linux-x86_64-unknown-linux-gnu.tar.gz`,
SHA256 `978a31f9356df3ce3e79c0de61e434f7fde0f64aae49bfcaf900f4fd83891228`, BUILD
commit `16034b9`) and passed exact manifest/modes, `vela-desktop 0.12.4`, ldd, styled
sign-in launch, normal close exit0 and zero temporary originals.

`before-0.12.4-20261018T192134Z.tar.gz` (440464314 bytes,0600) was created while the
live DB was still schema11 and restore-verified in an empty isolated directory
(integrity/FK ok, schema11,0 playlists). The installer correctly refused to migrate
implicitly. With the old service stopped, the new runtime was started once on
127.0.0.1:3211 against the backed-up data, applied schema12, reported health0.12.4 and
exited0 on SIGTERM. The installer then validated and installed the unit without touching
media. The0.12.3 unit file was preserved at0600.

Post-handover: health0.12.4; MainPID463860 active/enabled,0 restarts; integrity/FK ok;
schema12; counts unchanged (1user/20tracks/20assets/20analysis/1artwork/0playlists/
0folders). Rollback remains the preserved0.12.3 runtime/unit and the schema11 backup
(forward schema migration means rollback requires restoring that backup).

## Playlist folders — source verified (2026-10-18, `168c95f`)

Playlist folders add owner-scoped grouping metadata without changing playlist tracks,
shares or media rights. Migration0012 creates `playlist_folders` plus
`playlists.folder_id` (`ON DELETE SET NULL`) and an owner/folder index. Core/HTTP support
list/create, CAS rename/delete and CAS move of an owned playlist; deleting a folder
unfiles contained playlists and bumps their revisions, while stale revisions conflict
and foreign/missing IDs are404. Folders cap100/owner with case-insensitive unique names.

Web Playlists exposes folder creation, filter chips, selected-folder rename, confirmed
delete and a per-playlist Move-to-folder control. The API test covers duplicate names,
owner isolation, move/CAS conflicts, folder rename/delete, unfiling and revision bumps.
Chromium42 scenarios (41 pass1 live-import skip in2.2m) and Firefox42 (41 pass1
skip in2.6m) include a real folder lifecycle at320px. The runtime was subsequently upgraded to0.12.4/schema12; native folder UI is not
implemented (the additive summary field is ignored).
Evidence:`target/checks/browser-folders-chromium.log`,
`tests/browser/test-results/playlist-folders-phone.png`.

## 0.12.3 privacy/offline runtime handover — verified (2026-10-18, `e462f87`)

`target/releases/vela-0.12.3-{x86_64-unknown-linux-gnu,source}.tar.gz` and
`vela-0.12.3-SHA256SUMS` were built from commit `6cd407c` (native
`925fafc957ac0bc784aa49834b73b0a9eff557594341045a6ca02430f462d68f`, source
`c6f516950271100a1e3d43329e7bbf8412a6fa8eb237a7b0a4dedbb1937f2d8f`) and extracted to
`target/runtime/vela-0.12.3/` (schema11). `verify_package.py` passed. Exact-package
Chromium and Firefox each passed40 scenarios with1 live-import skip (1.9m/2.2m),
including listening-device listing/revocation, spectral-3 chroma, G07 and player/
publication regressions.

A portable native0.12.3 archive was built
(`target/desktop-releases/vela-desktop-0.12.3-linux-x86_64-unknown-linux-gnu.tar.gz`,
SHA256 `13d75a236da331fa56cd90e49df6aff828614df7088270278c03338595811593`, BUILD
commit `6cd407c`) and passed exact manifest/modes, `vela-desktop 0.12.3`, ldd, styled
sign-in launch, normal close exit0 and zero temporary originals. It includes private
offline copies/queue/removal, cached metadata age/Retry live and public pagination.

`before-0.12.3-20261018T185213Z.tar.gz` (440464320 bytes,0600) was created and
restore-verified into an empty isolated directory: integrity/FK ok and1user/20tracks/
20assets/20analysis/1artwork/0playlists. The0.12.2 unit file was preserved at0600. The
service was stopped gracefully, the installer validated the new runtime, and the unit
handed over without touching `data/`.

Post-upgrade: health0.12.3; MainPID432696 active/enabled,0 restarts; integrity/FK ok;
counts unchanged. Existing spectral-1/2 rows remain; new jobs use spectral-3. Rollback
remains the preserved0.12.2 runtime/archive/unit and this backup.

## Listening-device listing and revocation — verified (2026-10-18, `ade8969`)

Settings now lists the owner's consented listening devices with session counts,
enabled state and last activity (bounded32). Each row supports inline-confirmed
revocation guarded by the current privacy revision; the server deletes that owner/device
grant and its cascaded sessions, increments the privacy revision and returns the new
list. Stale revisions return409, unknown/foreign devices404, and held exports are
invalidated by the revision change. Bulk deletion remains the existing delete-all-and-
disable path.

Core/service+HTTP test covers two real devices, counts/activity, stale409, revoke,
cascade/export with only the remaining device and repeat404. G07 extended with a real
second device grant through the API, Settings listing, 320px layout, confirmation and
revocation while history/DSP/export/erasure checks remain green. Evidence:
`target/checks/listening-devices/` (320px privacy screenshot), Chromium40 passed+1
live-import skip in2.4m, API30 tests, web20 tests, strict core/server/web Clippy/fmt.
Shipped in the verified0.12.4 runtime. No schema
migration, original-data rewrite or native change.

## Native offline queue + removal0.12.2 — verified (2026-10-18, `b0bf085`)

Offline playback now uses all saved copies in newest-saved order: Play offline builds
the local queue at the selected copy, Previous/Next step only within it, and controls
disable at the edges. Each saved copy has inline-confirmed removal; removing the
currently playing copy stops offline playback. Removal deletes only the account-scoped
device audio+manifest, never the server original.

The real WebKitGTK fixture downloads two tracks, stops the actual server, verifies the
no-store health probe fails, plays the first saved copy, moves Next then Previous
through the two-copy offline queue, removes one copy with confirmation while the server
is still down, and leaves exactly one0600 audio+manifest pair. Evidence:
`target/checks/native-offline-queue2/`;27/29 desktop tests plus both opt-ins, strict
Clippy/fmt. No automatic/background offline sync, bulk storage manager, mobile/background
or entitlement refresh. Shipped in the verified0.12.3 portable desktop archive. No
schema, server HTTP, web or original-data/runtime change.

## Native explicit offline copies0.12.2 — verified (2026-10-18, `46bd390`)

The native Library now offers an explicit per-track Save offline action. It downloads
through the existing validated authorized-original path (exact size/BLAKE3 ETag,
250MiB cap), then copies the completed unnamed original into an account-scoped private
cache: `$XDG_DATA_HOME/vela/offline/<account>/` with versioned JSON manifests,0700
directories,0600 files, private temp+rename, and100-track/2GiB bounds. (Superseded
by the origin-scoped namespace of `a913574`; this entry is the historical0.12.2
record.) Malformed,
size-mismatched and orphaned entries are ignored; no credentials, URLs or history are
stored.

Saved copies appear in the Library and Play offline decodes the local file through the
single native engine; previous/next are disabled in offline mode. The real-window
fixture downloads a track, **stops the actual fixture server**, verifies a no-store
health probe fails, plays the saved original to advancing media with context
`Offline · <title>`, and confirms exactly one audio+manifest pair at0600. Sign-out
clears in-memory state but files remain account-scoped. Evidence:
`target/checks/native-offline3/`;27/29 desktop tests plus both opt-ins, strict
Clippy/fmt. No offline queue/removal/sync, mobile/background or entitlement refresh.
Source postdates the preserved0.12.2 desktop archive. No schema, server HTTP, web or
original-data/runtime change.

## Native cached-peer age + Retry live0.12.2 — verified (2026-10-18, `7609a76`)

Cached public metadata now carries its fetch time into the search result. The
browse-only label discloses a bounded age (`just now`/minutes/hours/days) and each
cached peer offers an explicit Retry live action. A retry re-queries only that
canonical origin with the current query; a successful answer replaces the cached
result with live metadata, while failure keeps the browse-only cached rows and never
reenables playback. There is no background refresh.

The real WebKitGTK journey seeds a one-hour-old cache for an unreachable saved origin,
asserts the age and Retry live control, runs the retry, and verifies it remains
browse-only when the origin is still unreachable; live peers, pagination, cached
fallback and remote queue still pass. Evidence:`target/checks/native-cache-retry/`;
26/28 desktop tests plus both opt-ins, strict Clippy/fmt. Source postdates the
preserved0.12.2 desktop archive; next native package will include it. No schema,
server HTTP, web or original-data/runtime change.

## 0.12.2 spectral-3 + pagination runtime handover — verified (2026-10-18, `12a1f76`)

`target/releases/vela-0.12.2-{x86_64-unknown-linux-gnu,source}.tar.gz` and
`vela-0.12.2-SHA256SUMS` were built from commit `e9d1954` (native
`90668c13587346aa2d23e7d673a5f8310c271b66d48ffc1b5c305e7f1b36a658`, source
`94a05522872f1b2ac9edb094712b54407ae2f85b6153910080a46066718dc35a`) and extracted to
`target/runtime/vela-0.12.2/` (schema11). `verify_package.py` passed. Exact-package
Chromium and Firefox each passed40 scenarios with1 live-import skip (1.9m/2.2m),
including spectral-3 chroma, G07 history, PC01–02, publications and player
regressions.

A portable native archive was also built at0.12.2
(`target/desktop-releases/vela-desktop-0.12.2-linux-x86_64-unknown-linux-gnu.tar.gz`,
SHA256 `a5527b9364e7c79aa3c6a4a2bd44471a20ae75138cb3b7bae80c858804df007d`, BUILD
commit `e9d1954`) and passed exact manifest/modes, `vela-desktop 0.12.2`, ldd, styled
sign-in launch, normal close exit0 and zero temporary originals. It includes cached
browse-only metadata and preview/per-peer cursor pagination.

Before handover, `before-0.12.2-20261018T181050Z.tar.gz` (440464320 bytes,0600) was
created and restore-verified into an empty isolated directory: integrity/FK ok and
1 user/20 tracks/20 assets/20 analysis/1 artwork/0 playlists. The0.12.1 unit file was
preserved at0600. The service was stopped gracefully, the new runtime validated by the
installer, and the unit handed over without touching `data/`.

Post-upgrade: health0.12.2; MainPID382555 active/enabled,0 restarts; integrity/FK ok;
counts unchanged. Existing spectral-1/2 analysis rows remain; new jobs use spectral-3.
Rollback remains the preserved0.12.1 runtime/archive/unit and this backup.

## DSP chroma spectral-3 — verified (2026-10-18, `2be441f`)

`vela-pcm-spectral-3` adds chroma: accumulated55Hz–5kHz power is folded to12
equal-tempered pitch classes by nearest semitone relative to A4=440Hz and normalized
to sum1; there is no tuning, chord or key estimate. Existing spectral-2 caches are
not reinterpreted. Web Radio measurements now show up to3 pitch classes and the honest
nearest-semitone disclaimer.

Evidence:21 analysis tests, including A440→A(96%), C#4→C#, a two-tone A/C# mix, plus
silence/invalid-descriptor guards; server analysis API test derives the expected class
from the measured centroid. Full Chromium:40 passed,1 live-import skip in2.1m with
`vela-pcm-spectral-3` and the chroma row. Screenshot:
`target/checks/dsp-chroma/radio-1440-dark.png`. Strict media/server/web Clippy, fmt,
WASM Clippy. The live runtime was subsequently upgraded to0.12.2, which serves spectral-3; existing
spectral-1/2 rows are retained. No schema, HTTP or native change.

## Native cross-peer search pagination0.12.1 — verified (2026-10-18, `5560eb1`)

Each saved-peer search result now carries its own validated catalog cursor. A
per-origin Load more follows only that peer's `next_cursor`, rechecks the exact
origin/cursor generation, appends only unseen IDs and removes the control when
exhausted; cached or failed peers have no cursor. There is no merged global cursor
and no automatic cache refresh.

The 21-record peer fixture now matches the unified `Vela` query. The real WebKitGTK
journey sees20 peer results, loads the21st through that peer's own control, verifies
no duplicate IDs across both origins, verifies the control disappears, and still
runs cached fallback, public playback and remote Previous/Next. Evidence:
`target/checks/native-cross-peer-pagination/`;26/28 desktop tests plus both opt-ins,
strict Clippy/fmt. Shipped in the verified0.12.2 portable desktop archive. No schema, server HTTP, web or original-data runtime change.

## Native public preview cursor pagination0.12.1 — verified (2026-10-18, `2bf1936`)

The native Public preview now follows a peer's validated public-catalog `next_cursor`
through a bounded Load more control (20 records/page). Each page revalidates the
canonical origin, server identity, per-track IDs/text/duration and `next_cursor`
equality with the last item; appended rows are deduplicated by ID and the control
disappears when the cursor is exhausted. Unified saved-peer search remains capped at
its first20 results per peer.

The isolated peer now publishes21 distinct lawful stream fixtures. The real WebKitGTK
journey sees exactly20 rows plus Load more, requests the next page, appends exactly
one remaining row with no duplicates, and verifies Load more disappears; saved/cached
peer behavior, public playback and remote Previous/Next still pass. Evidence:
`target/checks/native-public-pagination/`;26/28 desktop tests plus both opt-ins,
strict Clippy/fmt. This source slice is newer than the preserved0.12.1 desktop archive
and will ship in the next native package. No schema, server HTTP, web or original-data
runtime change.

## Native browse-only public metadata cache0.12.1 — verified (2026-10-18, `a3cb52c`; package `d7c0efb`)

A successful native public catalog fetch is now cached as bounded public metadata
under `$XDG_CACHE_HOME/vela/public-catalog` (fallback `~/.cache`): versioned JSON,
canonical origin echo, at most50 rows,128KiB, atomic private0600 files, directory0700
and at most64 files. When a saved peer is unreachable, unified search returns its
cached rows labelled `Cached metadata · stream unverified`; cached rows have **no play
button** and never authorize streaming. Cache failures are silent and never replace a
live response.

Evidence: cache round-trip/permission/exact-origin/version unit test; two isolated
peers plus a third preseeded unreachable saved origin; the real WebKitGTK journey
returns both live origins and the browse-only cached row, keeps saved origins at three
(including the cache origin), and still plays/Previous/Next across the live peers.
Evidence:`target/checks/native-cached-peers3/`;26/28 desktop tests plus both opt-ins,
strict Clippy/fmt. The portable desktop archive was rebuilt as0.12.1
(`target/desktop-releases/vela-desktop-0.12.1-linux-x86_64-unknown-linux-gnu.tar.gz`,
SHA256 `67850b30d3affabbbd59ce16cc26cf53d84a538c2a58d6a4efb68d3329de694f`, BUILD
commit `a3cb52c`) and `verify_desktop_package.py --launch` passed exact
manifest/modes, `vela-desktop 0.12.1`, ldd, styled sign-in window screenshot, normal
close exit0 and zero temporary originals. No schema, server HTTP, web or
original-data/runtime change.

## 0.12.1 spectral-2 patch release + preserved-data runtime handover — verified (2026-10-18, `f2c9fdb`)

`target/releases/vela-0.12.1-{x86_64-unknown-linux-gnu,source}.tar.gz` and
`vela-0.12.1-SHA256SUMS` were built from commit `9aa062a` (native
`d88a2dec844153906197e82d250ae2849deb5e99afb8e77ef66551629fbe0b90`, source
`e5249f5df75631e462fe7c6d399758e7afa28e14add911199f314cb2a36b0765`) and extracted to
`target/runtime/vela-0.12.1/` (27MB, schema11). `scripts/verify_package.py` passed.
Exact-package Chromium and Firefox each passed40 scenarios with1 live-import skip
(1.8m/2.2m), including G08/G09 spectral-2 Radio, G07 history, PC01–02, auth,
publication and player regressions.

Before handover, a fresh private0600 backup
`target/upgrade-backups/before-0.12.1-20261018T174417Z.tar.gz` (440464312 bytes) was
created and restore-verified into an isolated empty directory: SQLite integrity/FK
ok,1 user/20 tracks/20 assets/20 analysis/1 artwork/0 playlists. The old unit file was
preserved at `target/upgrade-backups/vela-local-development-0.12.0.service` (0600).
The service was stopped gracefully, the new runtime validated by
`install-user-service.py`, and the unit handed to build0.12.1 without touching
`data/`.

Post-upgrade: health reports0.12.1; MainPID354876 active/enabled with0 restarts;
SQLite integrity/FK ok; counts unchanged (1 user/20 tracks/20 assets/20 analysis/
1 artwork/0 playlists). Existing spectral-1 analysis rows are retained and are not
reinterpreted; spectral-2 applies to new analysis jobs. No schema migration or
original media write occurred. Rollback remains the preserved0.12.0 runtime/archive,
old unit file and pre-upgrade backup.

## Portable Linux desktop package0.12 — verified launch smoke (2026-10-18)

`scripts/package-desktop-linux.sh` built a portable x86_64 Linux desktop archive from
the verified native client (app source commit `692c083d0d64f641cbcd65dd08de0ab9b5a96232`):
`target/desktop-releases/vela-desktop-0.12.0-linux-x86_64-unknown-linux-gnu.tar.gz`
SHA256 `4d6c67bb154aa566cf7f19bf9f65de477c86512685fc891a324abc711b85cbad`
(4.77MB). It contains the release binary, desktop entry, scalable icon, README,
LICENSE and BUILD.txt; no external publication or installation occurred.

`scripts/verify_desktop_package.py --launch` extracted the archive to a disposable
directory, checked the exact file manifest/modes, ran the packaged binary's
`--version` (`vela-desktop 0.12.0`), verified no missing shared libraries with `ldd`,
launched the real packaged GUI against an unreachable origin so only the sign-in view
appears, waited for styles, captured `target/checks/desktop-package-screens/vela-desktop-package-launch.png`,
closed the window normally and observed exit0 with **zero temporary originals**.
The package is a portable tar archive, not a `.deb`/AppImage/Flatpak, signed artifact
or app-store build; the smoke does not play audio, access user data or certify
physical sound/screen readers. Release build is now covered by the CI build step
(remote CI still unexecuted). Evidence:
`target/checks/desktop-package-{2,verify}.log` and the preserved pre-fix archive under
`target/checks/desktop-package-warning-artifact/`.

## Native playlist add-from-library0.12 — verified (2026-10-18, `c49f95c`)

The native playlist detail now fetches up to50 owner-authorized library tracks when a
playlist opens and offers a bounded picker plus Add selected track. The mutation uses
the existing revisioned `POST /api/playlists/{id}/items` API with busy/generation and
selected-playlist guards; success updates the detail, revision and list item count.
The two-instance real-window journey adds a third owned track, verifies three entries,
plays the three-track queue, then reorders/restores/removes entries. Evidence:
`target/checks/native-playlist-add3/`,25/27 desktop tests plus both opt-ins, strict
Clippy/fmt. No schema, server, web or original-data/runtime change.

## Native origin-labelled remote queue0.12 — verified (2026-10-18, `d14f8b0`)

Remote playback now owns a separate in-memory queue of origin-labelled public
results. Playing a unified-search result builds that queue from all stream-authorized
results, and Previous/Next move across it, including across independently run peers.
Starting local playback clears the remote queue; remote EOF keeps the ended item and
queue visible for replay/stepping but releases prepared audio; MPRIS reports stopped/
no metadata; no remote item enters private history or persistence. Switching to remote
resets the local order/current and stops local audio so an in-flight local EOF/advance
cannot replace the public selection.

The two-peer window journey plays the second origin's result, Previous moves to the
first origin's result, Next returns to the second, and Library restores the private
catalog. Evidence:`target/checks/native-remote-queue18/`; desktop25/27 tests plus both
opt-ins, strict Clippy/fmt, zero temporary leftovers. No schema, server HTTP, web or
original-data/runtime change.

## Native unified saved-peer search0.12 — verified (2026-10-18, `6418ba2`)

The native Public view now searches up to eight saved canonical peers concurrently
through each peer's public catalog `q` endpoint, with independent five-second
timeouts, per-origin error reporting, at most20 results per peer, and no credential
forwarding. Results are labelled with their advertised origin and any stream-authorized
result plays through the single native engine.

Two isolated Vela peers were published on distinct canonical origins with different
tracks. The real-window journey saves both origins, searches them together with
`Vela`, receives track rows from both origins, captures unified results, plays the
second origin's public track natively and verifies the origin context. Evidence:
`target/checks/native-unified-search11/` (including `native-public-search-dark.png`),
desktop25/27 tests plus both opt-ins, strict Clippy/fmt. No schema, server HTTP, web
or original-data runtime change.

## Native saved public peers0.12 — verified (2026-10-18, `db893b7`)

The native Public view can now save/remove at most32 canonical public origins in a
versioned JSON file under `$XDG_CONFIG_HOME/vela/peers.json` (fallback
`~/.config/vela`). The store is atomic and private0600, rejects credentials,
non-canonical/plain-HTTP non-loopback origins, malformed/oversized/unsupported files
and stores no server metadata, tokens or history. The two-instance QA saves the
peer, removes it, re-saves it, and verifies the isolated config contains exactly the
canonical origin with0600 permissions; public playback continues through the native
engine. Evidence:`target/checks/native-saved-peers3/`; desktop25/27 tests plus both
opt-ins separately, strict Clippy/fmt. No schema, server or original-data change.

## Verified0.12.0 package and preserved original-data runtime upgrade (2026-10-18)

Package source `d3bde0c8e1c93324cf8628d8def8ba02e824bac4`. Local review artifacts
`target/releases/vela-0.12.0-{x86_64-unknown-linux-gnu,source}.tar.gz` and
`vela-0.12.0-SHA256SUMS` (native `e023288f13e57a7ae2e632365e0e5b5696318792d787204e06f47c49678e1e0e`,
source `e735c692939a6bc49af153502e2ae6ef458e82b57a2703710e504e15005c6c7b`);
never externally published. `verify_package.py` passed extracted startup/default
setup/schema11/HTML/WASM/CSS/SIGTERM. The exact archive was extracted to new
`target/runtime/vela-0.12.0/`; all19 frozen binary/static files match the archive and
the built release/WASM assets (`release-0.12-frozen-package-match.json`), including
the packaged notices file.

Immediately before the upgrade, the owned persistent0.11 unit/PID78464 was verified
and stopped normally. Fresh private0600
`target/upgrade-backups/before-0.12.0-20261018T161750Z.tar.gz` (440464330 bytes) was
created with SQLite backup semantics and restored into a disposable directory; all30
existing non-ledger tables had identical row counts, both databases passed
quick_check/FK0, and restore verified every referenced media SHA (20 media/1
artwork). Original data was never restored/reset/deleted. No schema migration was
needed (schema11 unchanged).

The unit now runs the frozen0.12.0 binary/assets with the ORIGINAL data directory.
Health reports0.12.0; MainPID267247 active with0 restarts; counts remain1 user/20
tracks/20 assets/1 artwork/0 playlists/20 analysis rows; quick_check ok/FK0. Served
HTML/WASM/style hashes equal the frozen runtime files. The installer `--dry-run`
validated build0.12.0/schema11/19 frozen assets and the upgraded unit without writing
anything; its same-build guard is why the actual cross-build switch used the documented
backup/stop/unit-switch/start path. Ten real SIGTERM/restarts exited0 and four
independent import/scanner/analysis/listening worker DB failures stopped correctly
(`release-0.12-shutdown.log`).

Exact-package browser gates: Chromium41/41 in1.9m and Firefox41/41 in2.4m against the
extracted binary and packaged web assets with `VELA_LIVE_IMPORT_TEST=1`, including
history/DSP/radio/publication/playlist/sleep/player regressions. Logs:
`release-0.12-{chromium,firefox,package,package-verify,service-recheck,backup}.log`;
screens preserved under `target/checks/release-0.12-{chromium,firefox}-screens`.
Preserve the0.11 runtime, releases, backups and all earlier evidence. Six established
metadata-only notice gaps remain; Docker/ARM64/native desktop packages, Internet TLS,
mobile/Apple hardware and a real federated two-host deployment remain NOT_RUN.

## Native public peer playback0.12 — verified local two-instance slice (2026-10-18, `6bf1824`)

The Linux native client can now enter an independently run Vela server's canonical
origin, fetch only its explicitly published public catalog, and play a track whose
owner granted public streaming. `apps/desktop/src/remote.rs` uses a credential-free,
no-redirect/no-proxy client; it validates the advertised origin/server identity, IDs,
text and duration bounds, and grants; requires bounded content length plus a quoted
BLAKE3 ETag; prepares an unnamed private0600 original; and loads it through the one
native engine. Remote mode does not mutate or advance the private queue, is cleared
when local playback starts, and MPRIS reports stopped/no metadata rather than
attributing public audio to a private track. Private history and credentials never
leave the client.

The two-instance QA starts a second isolated Vela server, enables its public catalog
with a canonical advertised origin, uploads and publishes the stream fixture with an
explicit CC-BY grant, then the native client fetches the catalog, plays the public
track and shows the public origin context. Because both instances are loopback HTTP,
a **debug-only** exact canonical→loopback transport override is used; release builds
ignore it and the advertised identity is still validated. This is not Internet TLS
or two-host validation. Evidence:`target/checks/native-public-peer5/`
(`native-public-dark.png`), with fmt, strict native+WASM Clippy and23/25 desktop
tests. No schema, server HTTP, web app or original-data/runtime change.

## Web sleep timer0.12 — verified browser increment (2026-10-18, `49b7d8c`)

A root-owned sleep timer is available from the playback queue: 5/15/30/60 minutes,
a visible countdown, and explicit cancel. It uses a generation-guarded one-second
task and the existing single `HTMLMediaElement`; at zero it pauses through
`BrowserPlayer`, disables itself and reports the pause. Navigation does not cancel
it, account sign-out clears it, and an old task cannot pause a replaced timer.

Real-browser evidence: Chromium41/41 and Firefox41/41 passed with the licensed live
import. The new scenario uploads a real60s PCM WAV, starts playback, sets5 minutes,
fast-forwards4 minutes (still playing), then past expiry (paused plus status), then
restarts, sets15 minutes, cancels, advances20 minutes and remains playing. The
Firefox run produced `test-results/sleep-timer.png`, preserved at
`target/checks/sleep-timer-firefox-screens/`. Logs:
`sleep-timer-{chromium-final,firefox}.log`; strict native/WASM Clippy, fmt and233
workspace tests pass. No schema, server HTTP, storage or original-data change.

## Queue truthfulness and owner playlist lifecycle0.12 — verified source (2026-10-18)

Application/source checkpoint `6421f3d` (native hardening `05951fd`). Continued the
same dirty0.12 worktree without schema migration or original-data changes; the
frozen0.11 runtime PID78464 stayed active/healthy on127.0.0.1:3210 with its original
data. A bounded web contributor implemented queue move-down/Clear-upcoming/first-page
truthfulness in `apps/web/src/main.rs`; main owned the domain/core/server contracts,
playlist UI, integration, compilers, tests and evidence. An independent read-only
native review had found a real temporary-source leak and other medium issues; those
were fixed and re-verified in`05951fd` (see the native section below).

### Implemented

Owner-only `PATCH /api/playlists/{id}` accepts a closed
`UpdatePlaylist{revision,mutation_id,name}`, applies revision CAS and mutation-id
idempotency, validates a trimmed1–120-character control-free name and audits
`playlist.edit`. Owner-only `DELETE /api/playlists/{id}?revision=&mutation_id=`
cascades entries, collaborator grants, share links and mutation records, audits
`playlist.delete`, and leaves owned tracks/assets/stream rights untouched; later
detail/share reads and repeated deletes return404. Unknown rename fields are400;
editors/viewers/outsiders get404; a missing CSRF header is403. Playlist detail now
shows owner rename/Save and inline Delete→Confirm/Cancel plus move-down, refreshing
the guarded same view and list only on success. The web `mutate` helper now supports
PATCH (the first real-browser run caught that it silently used POST).

Queue rows gained occurrence-aware move-down; “Clear upcoming” keeps rows0..=cursor
and removes only the suffix back-to-front through supported `PlaybackOrder::remove`,
so the playing occurrence/audio and already-played history survive. Remove-current
has an honest disabled explanation, and “Back to first page” is disabled on page one
using the incoming cursor that produced the visible page.

### Executed gates and defects

- New real-TCP test covers owner rename, trimmed revision/idempotent replay,
  conflicting identity reuse, stale revision409, unknown-field400, editor/outsider
 404, missing-CSRF403, share-link revocation, delete CAS, repeated-delete404 and
  unchanged track list/stream (`target/checks/playlist-lifecycle-api.log`).
- `cargo fmt --all -- --check`; strict `cargo clippy --locked --workspace
  --all-targets -- -D warnings`; strict wasm32 web Clippy; `cargo test --locked
  --workspace -- --test-threads=2`:233 passed,0 failed,2 explicitly ignored opt-ins
  (owned-null-sink audio, private D-Bus MPRIS). Python tooling:22 passed. JS syntax
  checks passed. Logs:`playlist-{workspace-tests,workspace-clippy,wasm-clippy,
  python}.log`.
- The release web build failed first because `wasm-bindgen` was not on PATH; the
  preserved pinned `target/tools/wasm-bindgen-0.2.129` was used explicitly.
- The first Chromium run (40 tests) stopped after test2 because rename returned
  “The request was refused.”; trace showed the shared web `mutate` helper lacked
  PATCH and sent POST. After the fix, **Chromium40/40 passed in2.5m** and
  **Firefox40/40 passed in3.3m**, both with `VELA_LIVE_IMPORT_TEST=1`, including the
  new playlist lifecycle and queue assertions plus every prior publication/
  history/DSP/radio/player regression. Logs:`playlist-{chromium2,firefox}.log`;
  Firefox screenshots are preserved in`target/checks/playlist-firefox-final-screens`
  and main visually inspected the lifecycle/list view.
- Native hardening re-verification:25/27 desktop tests with both opt-ins executed,
  real WebKitGTK window15/15 plus zero temporary directory entries, and bounded
  MPRIS session test passed (`native-0.12b-*`, native-0.12b-window).
- First-party review resolved the reported25-vs17 test-count concern by enumerating
  the actual25 tests (8 audio,6 client,3 MPRIS,8 shared playback); the docs'23-run
  claim was correct.

### Scope and next work

FUNCTIONAL_TESTED for the source web/API slice. No exact0.12 package was produced or
installed at that time; the later0.12.0 package checkpoint below now serves the
original data. No Docker, Safari/WebKit, screen
reader, physical-audio or mobile-hardware validation follows. Playlists still lack
folders, smart rules, invitations/media grants, owner transfer and drag/drop. Next
autonomous slices: package/verify0.12 only after deciding whether the native app
belongs in the server package; native favorites/playlist surfaces; saved peer
catalog adapters and genuinely playable authorized remote streaming; then scheduled
recommendation refresh. Preserve the0.11 runtime, all backups/releases/evidence and
this exact checkpoint; re-inspect process/data on resume.

Resume commands on this host (do not clean target or replace data/):
```bash
git status --short && git log --oneline -6
curl -sS http://127.0.0.1:3210/health
CARGO_BUILD_JOBS=2 cargo test --locked --workspace -- --test-threads=2
CARGO_BUILD_JOBS=2 cargo clippy --locked --workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=2 cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings
CARGO_BUILD_JOBS=2 cargo build --locked -p vela-server
CARGO_BUILD_JOBS=2 cargo build --locked -p vela-desktop --example native-ui-qa --features native-qa
python3 scripts/verify_native_client.py --evidence target/checks/native-resume-$(date +%s) --test-audio-engine --test-peer
CARGO_BUILD_JOBS=2 VELA_WASM_BINDGEN="$PWD/target/tools/wasm-bindgen-0.2.129-x86_64-unknown-linux-musl/wasm-bindgen" scripts/build-web.sh
(cd tests/browser && npx playwright test --browser=chromium)   # separate fresh server/data
```
The live unit is `vela-local-development.service`, now running the frozen
`target/runtime/vela-0.12.0/` binary/assets with the ORIGINAL `data/`; re-inspect the
actual process/unit/data rather than trusting this paragraph. Playwright cleans `tests/browser/test-results` at each invocation:
copy screenshots to `target/checks/` immediately after a run. The native QA opens a
real window and creates/removes an owned Pulse null sink; never run it against the
default output. `wasm-bindgen` is not on PATH; use the pinned target/tools binary.


## Native Linux client0.12.0 — verified source, original runtime preserved (2026-10-18)

Application/source checkpoint `3a285e0`. Started from the dirty post-0.11 worktree
(workspace already0.12.0 with an uncommitted eighth app), inspected the actual git
tree, running service, data and toolchains, and continued rather than recreating.
There is no schema, migration, server HTTP or original-data change in this slice.
The verified0.11.0 user runtime PID78464 remained active/healthy on127.0.0.1:3210
with its original `data/` throughout; media and runtime/release/backup snapshots
were not modified. No new backup was needed because this client writes no server
data, and the existing private0.11 upgrade backup remains preserved.

`apps/desktop` is a real Dioxus0.7.10 Linux application, not a WebView audio
substitute. `client.rs` owns an in-memory cookie/CSRF transport with canonical
HTTPS/loopback origins, TLS, no redirects/proxies, bounded requests and exact
size+BLAKE3 ETag checks on one owner-authorized original. `audio.rs` owns one
supervised Rodio/CPAL output thread with a bounded command channel and generation/
source guards; prepared originals are private0600 temp files capped at250MiB,
unlinked before decode on Linux and released on replacement/failure/sign-out/exit.
`playback.rs` is reused by path from the web app so occurrence/duplicate/shuffle
semantics do not fork. `mpris.rs` observes immutable snapshots and forwards typed
commands on its own thread without credentials/HTTP/audio ownership. `ui.rs` covers
sign-in, Library, Queue, transport, theme and honest preparing/cancel/retry states;
sign-out stops native audio and clears private state. No native listening events are
sent, so consent/history is not bypassed or inferred.

### Executed gates and defects

- `cargo fmt --all -- --check`; strict `cargo clippy --locked -p vela-desktop
  --all-targets --features native-qa -- -D warnings`; `cargo test --locked -p
  vela-desktop -- --test-threads=2`:23 passed,2 explicitly ignored opt-ins. Logs:
  `target/checks/native-0.12-{fmt,clippy,tests}.log`.
- The ignored real audio test passed under a script-created owned Pulse null sink
  (`native-0.12-window4/audio-unit.log`); the ignored MPRIS test passed under a
  private `dbus-run-session` (`native-0.12-mpris-session.log`).
- `scripts/verify_native_client.py` ran the actual WebKitGTK window against a fresh
  isolated server/account and three real uploaded fixtures (30s WAV, FLAC, MP3),
  with the user's default output untouched.15 named checks passed: no HTML/iframe
  substitute, real native clock, navigation continuity, paused stability/seek,
  light theme, search preserving playback, duplicate queue, reorder/removal,
  640×700 viewport with element-level queue/control/player bounds, no document
  overflow, queue clear, MP3 decode and sign-out cleanup; zero temporary originals
  remained. Evidence:`target/checks/native-0.12-window4/{report.json,*.png}`.
  The close-while-playing variant also passed with zero leftover originals
  (`native-0.12-close-playing/`).
- The first compact run reproduced a real responsive defect: queue grid rows
  overflowed640px and clipped their remove controls. The assertion was strengthened
  from document scrollWidth to element rectangles, `.native-queue-list` was
  constrained with `minmax(0,1fr)`, and the complete run passed. The failed report/
  screenshot remain in `target/checks/native-0.12-window2/`.
- The window harness initially reported a false compact failure because it floated
  the Hyprland surface with an invalid action (`set`) and the tiled X11 window
  ignored resize requests; `enable` is the documented Hyprland action. This was a
  test-driver defect, not application behavior, and is fixed in the committed QA
  example.
- Follow-up `05951fd` integrated the independent read-only review. The accepted HIGH
  finding was real: the first adapter created a named temporary file while the
  transfer was in progress, so closing the window during “Preparing” could leave a
  partial private original. Preparation now creates an unnamed0600 file from the
  first byte (`O_TMPFILE`/create-and-unlink), the transfer test asserts zero
  directory entries while a held download is active, and decoding still reopens the
  inode through procfd. MPRIS update/command channels are bounded64/32; a dead audio
  worker stops advertising playable transport and offers an explicit same-account
  reconnect; the QA harness requires `PULSE_SINK` plus an ALSA config routed to the
  script-owned null sink; the resize-error UTF-8 slice panic, username length
  mismatch and weaker startup-origin check were fixed. Re-verified: fmt, strict
  Clippy,25/27 desktop tests with both opt-ins executed, real WebKitGTK window15/15
  and zero temporary entries after exit, and the bounded-channel MPRIS session test
  (`target/checks/native-0.12b-*`, `native-0.12b-window/`). The review's reported
  17-test count was reconciled against the actual25 enumerated tests.
- Native favorites follow-up `358dd6f`: the desktop client now has a Favorites navigation
  view backed by the existing owner-only `favorites=true` catalog read, per-row
  favorite/unfavorite with a feedback gate and busy-track id, queue-state
  synchronization, and a filtered-page re-read after unfavorite. A stale search
  click during the new flow reproduced a real stale-error retention; successful
  catalog loads now clear the prior error, and the QA waits for the rendered query
  state instead of an old row. Real-window named checks pass for favorite→Favorites
  (exactly one row)→unfavorite→empty→Library restores three tracks, with screenshot
  `target/checks/native-favorites-window9/native-favorites-dark.png`; desktop23/25
  tests (both opt-ins separately), strict Clippy, and233 workspace tests pass.
- Native read-only playlists follow-up `6e6dafb`: the desktop client now reads the real
  `GET /api/playlists`/`GET /api/playlists/{id}` APIs, shows list/detail metadata and
  per-entry `can_stream`, and queues only the stream-authorized subset under a
  playlist context. The fixture creates a two-entry playlist through the real API;
  the window test verifies list→detail→Play playlist→native playback→shared queue
  two entries→Library return (`target/checks/native-playlists-window/`, screenshot
  `native-playlists-dark.png`).
- Native playlist lifecycle follow-up `f54188a`: owners can create, CAS-rename and
  confirm/cancel-delete playlists through the same bounded client (current0.12 debug
  server), with busy/error/guard handling and list/detail updates. The window journey
  creates `Native draft`, renames it to `Native renamed`, verifies cancel preserves
  the detail, confirms deletion removes the card/detail, keeps the fixture playlist,
  then plays its two entries. Evidence:
  `target/checks/native-playlist-edit-window3/` and
  `native-playlist-edit-{clippy,tests}.log`. Entry add/reorder, folders,
  albums/artists and packaging remain native gaps.
- Native playlist entry editing follow-up `5d2b774`: the detail view now supports revisioned
  move-up/move-down and remove through the real order/item APIs, with busy/generation
  and selected-playlist guards and list count/detail refresh. The window journey
  swaps the two fixture entries, restores order, removes one entry and verifies
  list/detail counts (`target/checks/native-playlist-editing/`). Adding library
  tracks to a playlist, folders/albums/artists editing and packaging remain gaps.
- Native queue/favorite parity follow-up `60915e6`: queue rows now support move-down and a
  Clear-upcoming action that preserves the current occurrence and earlier rows, while
  the player bar exposes the current track's favorite state through the same owner
  API and queue synchronization. The window journey verifies move-down/current,
  move-up restore, Clear-upcoming to one row and a real player favorite mutation
  (`target/checks/native-queue-favorite3/`).
- Native album/artist browse follow-up `1634f1d`: the desktop client reads the owner-scoped
  album/artist list, summary and track APIs, renders real grouped lists/detail and
  plays the authorized group tracks through the same native engine and queue. The
  window journey verifies album groups/detail (`Real Fixtures · Vela Test`), Play
  album audio, artist groups/detail, Play artist audio and Library return
  (`target/checks/native-browse-window3/`, `native-albums-dark.png`). Album/artist
  editing, playlist entry editing/folders and packaging remain native gaps.
- The same session's DSP packet added9 deterministic numerical tests (module now19:
  pure/two-tone spectra, silence, sub-window, clipping/oversized PCM, determinism,
  click/ZCR, -60dBFS, non-finite bounds) inside `crates/media/src/analysis.rs`; one
  ambiguous float annotation was fixed by main, then `cargo test -p vela-media
  analysis::tests` passed19/19 (`target/checks/dsp-0.12-tests.log`). This is
  test-only and does not claim tempo/chroma/genre/mood inference.

### Honest scope and next work

FUNCTIONAL_TESTED only for the Linux library/queue native slice. There is no
installer/package/signing, secure credential persistence, favorites/playlists/
albums/artists native UI, offline/download manifest, background mobile service,
device handoff or physical-output/screen-reader certification. Windows, macOS,
Android and iOS remain unimplemented/blocked on their real toolchains; Linux
evidence cannot certify them. Native listening history and federated remote sources
are absent. Next autonomous work: integrate a pending web queue/deep-link polish
slice, then the playlist rename/delete lifecycle, then native favorites and a
reviewed session-persistence decision. Preserve the0.11 runtime, all backups/releases
and the native evidence; re-inspect process/data on resume.

## Verified0.11.0 package and preserved original-data upgrade (2026-10-18)

Application/source fe0484d62f1a2ae33a8c25ee901d4ed6b9568f06. Native/source archives
`target/releases/vela-0.11.0-{x86_64-unknown-linux-gnu,source}.tar.gz` and SHA256SUMS
are local review artifacts, never externally published. Extracted default startup,
schema11, HTML/WASM/CSS, closed-public-catalog including malformed filters, and
SIGTERM passed. All retained release manifests passed. Frozen binary SHA256
1224d6fd1d52195028d96e0de592b2717019d981b81d3bdb43e42f3863d00af5;
all19 binary/static files match archive AND built files before/after both browsers
and original upgrade. Highest required GLIBC symbol2.34; only this Linux x86_64
server host is validated. Existing6 metadata-only notice gaps are unchanged.

Exact extracted binary passed Chromium39/39 in2.6m and Firefox39/39 in3.3m, each
with fresh isolated data and licensed live import. PN01/PN02 use generated project
PCM and actual backend/settings/rights/Range/revoke calls; all prior playback,
privacy/history/DSP/radio/recommendation/share/import/search regressions passed.
Source Chromium39 also passed separately in2.5m. Main inspected source Network
1440dark/320light plus packaged Firefox320light and Chromium320dark declaration
screens. Screens cover Network320/1440 both themes and existing player/Home suites
320/768/1440/1920; no horizontal overflow in assertions. These are browser checks,
not physical audio/background/mobile certification. Exact package20clean restarts
and4 worker DB faults passed too.

Immediately before upgrade verified persistent/enabled0.10 unit PID46997 and its
frozen executable. Graceful stop returned0/old PID exited. Fresh private0600
before-0.11.0-20261018T095211Z.tar.gz restored to a separate disposable directory;
every SQL logical row and referenced media hash matched. Original data was never
restored/reset/deleted. Forward10→11 preserved EVERY existing table's logical-row
hash except the intentionally extended SQLx migration ledger, including all20
analysis rows. Counts remain1user/20tracks/20assets/1artwork/0playlists. One preference
and one device row already existed in the stopped schema10 backup; their exact
consent/retention/revision values match after upgrade. Listening sessions and
recommendation exclusions remain0. No installer/upgrade opt-in occurred. New
network_settings is disabled/originnull/revision0 and publications0. Public endpoints
return404; no original music was published. quick_check ok/FK0.

Current runtime PID78464 is active/running with0restarts, non-root persistent and
enabled `vela-local-development.service`, frozen `target/runtime/vela-0.11.0/`,
ORIGINAL `data/`, loopback127.0.0.1:3210. Health0.11.0/setup complete; executable,
eight served core assets and complete19-file frozen/archive hashes verified.
The same-build installer validates the upgraded unit/schema/archives too. Prior
private unit definitions and0.10 package remain retained.0077 umask, on-failure2s,
3starts/60s and stop60s. Starts with the existing user manager, normally login;
reboot-before-login/logout behavior and linger were not configured/tested.

Evidence under target/checks/: publication-{package,package-verify,package-abi,
packaged-chromium,packaged-firefox,packaged-shutdown}.log/json as applicable;
publication-packaged-{chromium,firefox}-screens; publication-frozen-package-match.json;
publication-upgrade{,-backup,-runtime,-consent}.log/json; publication-service-011-recheck.log.
Source200Rust/strict native+WASM/fmt/22Python/JS/deny and failed-then-fixed NUL run
are recorded below. Independent reviewer signed off cross-system security/data/UI
boundaries before packaging. Docker daemon socket remains unavailable; no Docker,
ARM64, native client, Safari, screen-reader or Internet TLS/signature claim.

Next autonomous work: shared platform audio contract plus a complete actual Linux
Dioxus native player/OS media bridge, then Android Media3 service/offline manifests;
Apple compile/device evidence needs an Apple environment. Network still needs
saved origin adapters/blocked peers, standards-based identity/signature/replay and
social delivery, remote queue/search and separately authorized rooms/handoff.
On-demand mixes need scheduled refresh/calibrated quality. Preserve this deployable
checkpoint and all backups/runtime/releases. The whole mandate is not complete;
no work outside an active execution session is implied.

## Public catalog0.11.0 — verified source before packaging (2026-10-18)

Started clean main8abba45/application902dd47, original frozen0.10 unit PID34675,
schema10/quick_check ok/FK0/20 tracks and20 analysis rows. Fresh private0600
before-publication-20261018T062341Z.tar.gz restored into a separate disposable
directory before additive0011; see publication-backup.json. No original music
was published, copied away, reset or deleted. Three actual specialist agents
implemented media policy/tests, Network UI/browser acceptance, and a validated
user-service installer; main owns contracts/schema/HTTP/review/build integration,
max2 compilerjobs. The media agent independently reviewed network security and
implemented a bounded DNS correction; the installer agent added redirect tests.

Source implements stable optional origin identity, explicit owner license/private
provenance declarations, metadata/stream/download grants, retained CAS revocation,
redacted bounded public catalog, authorized Range/HEAD/download and bounded HTTPS
peer preview. Real independent A/B Axum HTTP plus hostile/slow peer tests run under
production-absent exact fixture routing. This is read-only profile1, not ActivityPub,
HTTP signatures, a public directory, native clients or a unified remote queue.
Dioxus Network uses the existing root audio only for private original preview;
remote links reflect actual public grants. Rights/stream/download default false.

Current executed source gates:200 Rust tests, strict native/all-target and WASM
Clippy, fmt, native release and WASM builds;22 Python tests incl13 installer,
JS syntax/diff and locked dependency policy passed.20 real clean SIGTERM/restarts
and4 independent worker infrastructure faults passed. Existing6 metadata-only
license notices remain. Evidence: publication-rust-corrected.log,
publication-final-clippy.log, publication-wasm-clippy.log, publication-native.log,
publication-web.log, publication-final-python.log, publication-deny.log,
publication-shutdown.log and publication-notices.log under target/checks/.

The first full projection test failed because SQLite substr(TEXT) terminates at
embedded NUL. Bounded BLOB extraction + safe UTF8/control projection fixed the
actual bug while preserving the failing assertion and private owner tags; the
corrected complete200-test run passes. Failed publication-final-rust.log is kept.
Review also caught/fixed the exact discovery route's protection allowlist, disabled
malformed-query responses, unrelated/backward peer cursors and real blocking DNS
admission after cancellation. No production private-network escape was added.

Actual same-build local-service handover passed against frozen0.10/original data.
Unit is now persistent/enabled, Transientno, active/running PID46997,0restarts;
private prior configuration retained in the user's unit directory. Install/dry-run/
recheck and schema10 row fingerprints are in publication-service-*. Original data
still has1user/20tracks/assets/20analysis, no consent/events/exclusions/publications.
No root/linger/public bind was enabled. Starts with the existing user manager,
normally login; reboot/logout behavior was not executed or claimed.

Independent QA reviewed cross-system DTO/migration/owner/grant/revoke/peer/DNS/UI
boundaries and found no blocking permission, data-loss or stale-response defect;
its historical private-playlist documentation correction was applied.

Source Chromium39/39 passed in2.5m with the licensed live import, real PN01/PN02,
root playback and all prior privacy/order/recovery regressions. Main inspected
Network desktop dark and320px phone light screenshots; no overflow was observed
in the automated theme/viewport checks. Evidence: publication-chromium.log and
publication-source-chromium-screens. Next commit coherent source, package frozen native/source artifacts,
test exact package Chromium/Firefox and fresh backup/restore before original10→11
upgrade. Keep public catalog and listening consent disabled on original data.
Full signed social federation, saved multi-server playback, native Linux/Android/
Apple clients/offline/handoff/rooms and scheduled/calibrated mixes remain unfinished.
No complete-roadmap or work-outside-active-execution claim is made.

## Verified0.10.0 package and preserved original-data upgrade (2026-10-18)

Application/source checkpoint 902dd47f9dca2d9b6f405cd50c21bf4dab5a73d8. Clean native/source archives target/releases/vela-0.10.0-{x86_64-unknown-linux-gnu,source}.tar.gz and SHA256SUMS are preserved locally, never externally published. Extracted default startup/setup/schema10/HTML/WASM/CSS/SIGTERM checks passed; all preserved prior release manifests passed too. Frozen binary SHA256 d5a3d3828a41182267edf69b6c471afc07d920a5aa44eee63efc1e223b660e8c; all9 binary/static hashes match the archive AND built files before/after both browser runs. Highest required GLIBC symbol2.34; only this Linux x86_64 host runtime is validated. Six established metadata-only notices remain, no new dependency or runtime advisory exception.

Exact extracted executable passed Chromium37/37 in1.7m and Firefox37/37 in2.0m with the licensed live import. Assets served by the test harness are byte-identical to the frozen package, verified again after both runs. Stable snapshots cover320/768/1440/1920 in both themes with no horizontal overflow and square-artwork assertions; main inspected Chromium desktop dark/phone light and Firefox phone light. Animation-disabled viewport capture removes transitional theme colors; it does not simulate native playback. Packaged20 real clean SIGTERM/restarts and four independent worker-DB failures passed too. Logs/screens: personal-packaged-{chromium,firefox}.log and -screens; personal-packaged-shutdown.log; personal-frozen-package-match.json; personal-package-verify.log; personal-package-abi.json. Source gates176 Rust/strict native+WASM/fmt/9Python/JS/dependency policy are recorded below.

Immediately before upgrade, verified owned0.9 user-unit PID21141 and its exact frozen executable. Normal stop returned0 and old process exited. Fresh private0600 before-0.10.0-20261018T044246Z.tar.gz restored into a new disposable directory; EVERY existing logical row and referenced media hash matched. Original data was never restored/reset/deleted. New server migrated9→10; catalog, favorites, playlists, private preferences/history and all20 analysis rows remained byte-for-byte identical at the SQL-row level. Integrity quick_check ok/FK0. Counts remain1 user/20 tracks/20 assets/1 artwork/0 playlists; preferences/devices/listening sessions remain0. Exclusions are0: no collection, analysis or hidden choices were enabled implicitly. Reports: personal-upgrade-backup.json, personal-upgrade-runtime.json and personal-upgrade.log.

Current local runtime PID34675 is active/running with0 restarts, non-root transient vela-local-development.service, frozen target/runtime/vela-0.10.0/ and ORIGINAL data/. GET127.0.0.1:3210/health reports0.10.0; setup remains complete; eight served static hashes match. Restart on-failure2s,3starts/60s, stop60s,0077 umask. Automatic reboot/user-manager startup remains unconfigured for this local review unit; production systemd/Compose recipes are separate. Reinspect process/data on resume; do not infer them from this checkpoint or Git HEAD. Preserve target/runtime/releases/upgrade-backups and failed evidence.

Next autonomous vertical slice: explicit private-by-default origin/publication/license-provenance policy with independent metadata/stream/download grants, then actual two-instance public discovery/stream/revocation tests. Native Linux playback/Dioxus packaging and Android Media3 service remain real implementation work; Apple compilation/device verification requires its environment. Scheduled personal mixes, calibrated preference quality, large tied-time/history-density profiling, device handoff/offline/native/federation are unfinished and not claimed complete. No future background engineering is implied outside active execution.

## Personal mixes0.10.0 — source verification before packaging (2026-10-18)

Started from clean main ffa81a6/application bc50564; evolved the same seven crates. Fresh private0600 before-personal-mixes-20261017T131348Z.tar.gz restored before additive0010. On resume the original transient user unit was absent/inactive (cause unestablished); original data was still schema9/quick_check ok/FK0,1 account/20 tracks/assets/0 playlists,20 analysis rows and no listening consent/events. Fresh personal-resume-20261018T042752Z.tar.gz restored with media hashes before restarting the exact hash-verified frozen0.9.0 binary/assets against original data. Current restored0.9 runtime PID21141/loopback3210 is distinct from the development candidate, no auto reboot startup configured. See personal-resume-runtime.json. No original reset/restore/delete or credential inspection.

### Usable source increment

Home's exploration uses real owned favorites and available validated signal/tag descriptors, plus qualified retained private listens ONLY while current account consent is enabled. Rotation uses actual qualified counts/recency. One read snapshot,500 eligible candidates,12 combined anchors; hidden filters precede selection limits. No retained-disabled history query (adversarial missing-table test), external model, network or decode in ranking. Additive exclusions store desired-state owner preferences with1000 quota/serialized concurrency; real Hide/Undo/confirmed restore/private UUID export change no originals, favorites, playlists, radio or stream ACLs. History erasure and hidden choices stay distinct. New typed API authenticates all access and CSRF/Origin writes. Dioxus uses root audio/queue, actual play/next/last/radio and physically abortable bounded requests with session/privacy/lifetime guards. Data-only sync waits for pending feedback rather than discarding its receipt.

Actual media/web/privacy agents implemented/reviewed independent owned files, main owns DTO/migration/server/builds/tests/docs/integration max2compiler jobs. The original media followup hit usage limit after reporting EXPLAIN findings; resumed SQL and browser specialists completed optimization/test repair with a separate readonly reviewer. No unexecuted contributor check is counted as passing.

### Executed gates and discovered defects

- Final native workspace/all-target and wasm32 web strict Clippy passed; cargo test --locked --workspace -- --test-threads=2:176 passed. Includes15 recommendation SQLite tests, real TCP owner/consent/feedback/stream boundaries, schema9-to10 preserving every existing fixture logical row, concurrent consent/quota races, bounded hydration/source order and2 pure web privacy-context tests. fmt, JS syntax/diff checks,9 Python tests passed. Native release/WASM builds passed; final locked dependency policy passed (known compile-time paste exception only). Logs:personal-final-{clippy,wasm-clippy,rust,native,benchmark-build,fmt,python,deny}.log; web build remains personal-web.log because frontend source is unchanged since that build.
- Real Chromium37/37 passed in1.8m with licensed live import, G07 actual advancing/muted/RepeatOne history and rendered2-listen rotation; held genuine recommendation response across erase workflow; PC01 real exploration playback/root node identity, committed Hide while actual Now Playing favorite state changes, successful Undo/restore/export; PC02 held real snapshots across navigation/dedicated-session logout; all prior player/upload/scanner/sharing/security scenarios. Log personal-final-chromium.log; source-run screenshots personal-final-chromium-screens. Stable final theme/viewport screenshots are prepared for exact packaged Chromium/Firefox reruns; the first captures exposed intermediate CSS theme-transition colors, so final screenshot capture disables CSS animations and uses a viewport at exploration rather than an unreadable full-page thumbnail.
- Failed attempts preserved: initial conditional Icon RSX type mismatch/needless borrow fixed; first PC01 selected a nonexistent Home Favorite track button (33passed/1failed/3unrun); switched to actual root Now Playing favorite control preserving initial state under7s deadline. Second test failure was unroute racing a pending route.fulfill; now wait for its successful response before cleanup. No production guard/deadline/threshold was weakened. Failures:personal-chromium-first-failure, personal-chromium-route-failure and corresponding logs. Readonly review found/fixed earlier ordinary-sync cancellation of unsettled Hide; true privacy changes still invalidate immediately.
- Actual release synthetic-metadata benchmarks at10k/100k/1m,5 samples plus warmup per three cases. Baseline history p50 at1m was10995.440ms, above web10s deadline; source-derived EXPLAIN identified catalog scans with sparse evidence. Bounded source-driven selection/hydration and reused aggregates preserve exact four selector/projection results. Final1m p50 is5.169ms cold,7.123ms favorite,7.282ms history+exclusions. Raw samples/source/binary hashes/resources and scope are in PERFORMANCE and target/checks/personal-{final-,}benchmark*.json. No human relevance, stream capacity, native audio or comparative superiority claim. Equal-timestamp and dense-history cost remain unmeasured.
-20 clean real SIGTERM/restarts and4 worker infrastructure failures passed in personal-shutdown.log (worker code unchanged by SQL followup). Docker info still fails because daemon socket absent; no image/build validation. Browser skill discovery/list returned no browser; isolated Playwright used, no signed-in user profile. Native/Safari/device/screen-reader validation remains unexecuted.

Next: commit coherent source, package immutable native/source archives, test exact extracted binary/assets in both engines, verify another fresh stopped-runtime backup/restore and only then upgrade original data in place. Preserve all target/runtime/releases/backups and failed evidence. Lawful two-node public rights/discovery/revocation and actual Linux/Android/native Apple clients remain separate unfinished work; scheduled/calibrated recommendations remain partial. No entire-roadmap completion or work outside active execution is claimed.

## Verified0.9.0 package and original-data runtime (2026-10-17)

Application/source checkpoint is bc50564979a9bcead7ed6667d6ae673df374e265. Clean native/source archives target/releases/vela-0.9.0-{x86_64-unknown-linux-gnu,source}.tar.gz passed extracted default startup/setup/version/schema9/HTML/WASM/CSS/SIGTERM and SHA256 checks. Every preserved earlier release checksum passed too. Notice generation retains the6 explicitly documented metadata-only gaps; no external publication. Native ABI inspection reports highest GLIBC symbol2.34; runtime evidence covers this x86_64 Linux host only.

Packaging rebuilt the native executable with a different hash from the earlier release-test binary. The exact extracted archive was therefore tested again: Chromium35/35 in2.2m and Firefox35/35 in2.6m, both with the licensed live import and every new history/DSP/radio/privacy case. Logs:listening-packaged-{chromium,firefox}.log; screenshots:listening-packaged-{chromium,firefox}-screens. All9 frozen binary/assets match the package and built files (listening-frozen-package-match.json). Earlier source-release runs remain separate evidence, not a byte-identity assertion.

Immediately before upgrade, verified the owned0.8.1 user unit/PID15481/executable and original schema7 library. Normal systemctl stop returned0 and PID15481 exited. Fresh private0600 backup before-0.9.0-20261017T130231Z.tar.gz restored into a new disposable directory; every logical SQL row and referenced media hash matched. Original data was never restored/reset/deleted. Counts remained1 account,20 tracks,20 assets,1 artwork,0 playlists. See listening-upgrade-backup.json and listening-upgrade.log. The permanent schema7 forward-migration test separately preserves an edited track/favorite and a nonempty old playlist's revision/order.

Current PID114613 is supervised by the non-root transient user unit vela-local-development.service, serves127.0.0.1:3210 from target/runtime/vela-0.9.0/ and uses the ORIGINAL data/ directory. Health/setup report0.9.0/completed. SQLite is schema9, quick_check ok, foreign-key errors0; all existing catalog/playlist table fingerprints are unchanged. Listening preferences/devices/sessions and audio_analysis are all0: upgrade did not enable collection or queue analysis. Served8 static asset hashes and frozen binary match the archive. Unit active/running,0 restarts, on-failure2s/3starts per60s, private0077 umask; stop timeout is now60s to match the supplied deployment recipe. This transient unit has no configured reboot/user-manager auto-start. Reinspect actual process/data on resume rather than infer them from HEAD.

Runtime evidence:listening-upgrade-runtime.json, private backup report/log, listening-before-upgrade.json and frozen hash report. Preserve original data, target/runtime, target/releases and target/upgrade-backups. Later documentation commits do not change frozen source bc50564. No browser/native-device/Safari/Docker/remote CI evidence is invented; platform limits remain in PLATFORM_MATRIX and KNOWN_LIMITATIONS. Next autonomous implementation is real consented history/favorite-based personal ranking and mixes; lawful publication/rights/two-instance federation and native Linux/Android/Apple audio remain unfinished milestones.

## Private listening, bounded DSP and local radio0.9.0 (2026-10-17)

Read the new784-line master mandate in full and inspected actual main f6cc845/application ac31d74, AGENTS/contracts/source/tests, persistent data and supervised frozen0.8.1 PID15481. The two user-supplied mandate/start files are preserved. Baseline rerun:110 Rust tests and33 real Chromium scenarios passed. Original data had1 account,20 tracks/assets,1 artwork,0 playlists, schema7, quick_check ok/FK0. Fresh private0600 before-listening-20261017T085256Z.tar.gz restored separately with referenced media hashes verified (listening-backup.json). No original data reset or production credential inspection occurred.

### Usable increment and ownership

Three actual specialists contributed nonoverlapping files: private_listening_core implemented consented listening and bounded discovery plus readonly security review; dsp_analysis implemented the PCM extractor, durable worker and benchmark; private_listening_web implemented collection/history/settings and most radio integration. The web contributor hit the environment usage limit after saving its work. Main completed/reviewed radio integration and responsive CSS, owned shared DTOs/Cargo/lock/migrations/server transport/supervision/API/browser tests/docs and all compiler scheduling (at most2 jobs). No isolated branch or native-device execution is implied.

Seven existing crates evolve to0.9.0; additive migrations0008/0009 preserve old media/catalog/playlist semantics. Private history starts off globally and per device, validates monotonic cumulative logical sessions/idempotency/ownership, supports7–365day retention, owner-only keyset history and JSON export, and atomically erases/disables/invalidates old batches. The browser measures real advancing unmuted playback against a monotonic clock, preserves a session across pause/recovery and creates another at actual EOF replay. Its account-local pending journal is bounded and clears on opt-out/logout. Home/History show real qualified listens and link them to track radio. Old native/source snapshots are untouched.

Explicit analysis enqueues1–50 actual owned track IDs. One supervised blocking Symphonia0.6.1/RustFFT6.4.1 worker measures bounded PCM RMS/peak/ZCR, phase-safe Hann FFT spectrum/bands and an onset-interval tempo estimate (spectral-2), caches by immutable asset+extractor version, enforces50-owner/128-global current active jobs, recovers/requeues interruptions and retires obsolete unfinished versions. Source files never change. Radio compares measured descriptors or actual tags over at most500 newest other owned candidates, diversifies deterministically and supplies real IDs/reasons. Its Dioxus view shares the root player and exposes real queue/status/measurement/analysis controls. No cloud inference, inferred mood/genre/key/chroma/LUFS, calibrated beat annotation, public catalog or native audio engine is claimed.

### Executed gates and defects

- Final fmt and strict workspace/all-target plus wasm32 web Clippy passed. `CARGO_BUILD_JOBS=2 cargo test --locked --workspace -- --test-threads=2`:157 passed. Includes12 listening service tests, schema7 forward upgrade preserving edited tags/playlist revision/order/favorites,10 DSP tests,9 worker tests,7 radio tests,6 new web policies/formatting and2 new real TCP scenarios. The standalone release-benchmark percentile test also passed (1). Logs:listening-{fmt,final-clippy,final-wasm-clippy,final-rust,benchmark-test}.log.
- Native release server/benchmark and release WASM/static builds passed (listening-release-{native,web}.log). Final Chromium35/35 passed in2.3m, including licensed live import, real8s PCM consent/muted-time/RepeatOne/dedup/history/export/delete and held-export suppression, actual background analysis/radio/queue, private history-to-radio navigation and all previous upload/scanner/metadata/filter/job/share/recovery/expiry/race scenarios. The earlier complete0.9 pre-polish run also passed35/35 in Chromium and35/35 in Firefox. Final polished Firefox35/35 also passed in2.9m (listening-final-firefox.log); Chromium is listening-final-chromium.log. Both include the eight radio size/theme combinations and private-history-to-radio action.
- Final radio screenshots/assertions cover320/768/1440/1920px in both themes. Main inspected actual desktop dark, phone light, private Settings and History desktop images. The first radio images exposed a flexible placeholder; the new square assertion then caught the inherited39px minimum against a38px phone height. Both CSS causes were fixed and the assertion retained. Final images:listening-final-{chromium,firefox}-screens. Browser skill discovery returned no available browser/list[], so the existing isolated Playwright harness was used. No physical sound/Safari/screen-reader/native-device claim.
- Readonly review found and corrected repeat-one/manual EOF replay missing a new logical history session, held export completion after erasure, and old extractor queues blocking current quotas. Initial G07 used insufficient post-mute observation time: the trace recorded3868ms below its4000ms qualification threshold. The test waits for more actual playback while preserving its original threshold assertion. A separate optional live-import run received a real temporary source outage; later full runs passed the unchanged controlled importer. Failure logs/traces remain in listening-first-failure, listening-live-network-failure and listening-artwork-failure; no failed run was counted as passing.
-9 Python tooling tests, JS syntax, diff whitespace, dependency advisory/license/source policy and notice generation passed. Six established metadata-only license gaps remain, no runtime advisory exception added and no external publication. Docker info still fails because its daemon is absent. `verify_shutdown.py --runs 20 --worker-failure` passed20 clean SIGTERM/restarts and deliberate isolated Import/Scanner/Audio analysis/Listening retention DB failures, each stopping the process correctly (listening-shutdown.log).
- Real release PCM benchmark:20 successful measured extractions across1s mono44.1kHz,20s mono48kHz,20s8ch48kHz,120s mono48kHz. p50 was0.949/19.099/132.973/114.081ms; all source hashes unchanged. Whole example process peaked13164KiB RSS. PERFORMANCE records hardware, warm-cache/small-sample limits, source/binary/lock hashes and the Python resource wrapper; time(1) was absent. This is not streaming capacity, server RSS or audible-start evidence.

### Resumable scope

QA_EVIDENCE maps the new mandate's G01–G24 independently of historical UX numbers; parity/product/design/architecture/security/limits/ADRs0013/0014 are updated. PLATFORM_MATRIX records missing Android tools and Apple host/device validation; FEDERATION_PROTOCOL/NETWORK_RIGHTS are explicitly design-only, no publication endpoints. Native desktop/mobile/offline/handoff/federation remain unfinished. Next independent product work is consented personal ranking/feedback/daily mixes, then actual opt-in origin/publication/rights/two-instance tests and native Linux/Android audio contracts/engines. Analysis cancellation/task-center projection, large-library similarity indexing and contention profiling remain honest limits. No full-roadmap completion or work outside active execution is claimed.

At this checkpoint the original supervised0.8.1 remains live while clean0.9 source/package/restore/runtime verification is prepared; the next recorded section must identify the actual installed source/binary/assets/data, not infer them from Git HEAD. Preserve target/runtime, releases, upgrade-backups and original data.


## Shared queued duration0.8.1 (2026-10-17)

After the0.8 Chromium/Firefox checkpoint68da522, inspection found that adding to an empty queue updated BrowserPlayer.saved but left root duration0. The0.8 modal's metadata fallback displayed30s while the bar stayed max1. Added the NP01 root-range assertion and ran the actual0.8 release/WASM:28 passed, NP01 failed with max1 instead of30,4 unexecuted. Log queued-duration-before-fix.log and its trace directory preserve that reproduction.

BrowserPlayer::sync now projects the current saved occurrence's nonnegative metadata duration only while unloaded, after existing owner/session validation. It compares with peek before writing to avoid a duration subscription/redundant updates. Position, pending seek, source/element/session generations, retry intent and PlaybackOrder are unchanged. Loaded browser duration stays authoritative. Removed the modal fallback so both views use the same root value. The readonly reviewer inspected/accepted the exact diff and acceptance extension; main owns all code/builds/tests. Seven workspace packages/lock are0.8.1 with no third-party pin, schema/API/storage change.

Executed patch gates: fmt; strict native/all-target and wasm32 Clippy;110 Rust tests;9 Python tests; native release; release WASM/static build; locked dependency policy; JS syntax/diff checks all passed (target/checks/queued-duration-{fmt,clippy,clippy-wasm,rust,python,native,web,deny}.log). Full real-browser suites with the licensed live import passed Chromium33/33 in1.2m and Firefox33/33 in1.5m (queued-duration-{chromium,firefox}.log). NP01 now requires matching30s bar/modal ranges for a real unloaded queued WAV, paused11s seek with no source, explicit Play at11..14s, then all prior continuous audio/focus/Media Session/duplicate queue/favorite/navigation checks. All prior account/ingest/permission/recovery/race tests remain passing. Screenshots are retained separately in queued-duration-{chromium,firefox}-screens. No native-device, Safari, screen-reader, audio-effects or new benchmark claim.

### Local process recovery during this patch

A subsequent health probe found old tool-owned0.8 PID221832 absent and port3210 closed; its log contained readiness only, so the cause is unknown. Readonly SQLite still had1 account,20 tracks,0 playlists, quick_check ok and0 foreign-key errors. Restarted the exact frozen0.8.0 executable/assets against the original data as a new non-root transient user systemd unit vela-local-development.service. Confirmed PID11418, health0.8.0, loopback3210, active/running and0 restarts (now-playing-supervised-runtime.json / now-playing-supervised-server.log). Restart=on-failure,2s delay,3 starts/60s and20s normal shutdown are verified unit settings. No global package/account/service installation or public exposure. This transient unit is independent of tool process lifetime; automatic reboot/user-manager startup is not configured. Current running PID must always be re-inspected rather than inferred from an old checkpoint.

Docker info was rechecked on this host and again failed because /var/run/docker.sock is absent (queued-duration-docker.log). No Docker build/runtime passed.

### Verified package and current supervised runtime

Source/runtime0.8.1 is ac31d74ef6651dcbc860c5ffa88818d81b7a37c6. Clean native/source archives target/releases/vela-0.8.1-{x86_64-unknown-linux-gnu,source}.tar.gz passed extracted default startup/version/setup/schema7/HTML/WASM/CSS/SIGTERM checks, both hashes and every preserved older manifest (queued-duration-package{,-verify,-checksums}.log; queued-duration-all-release-checksums.log). Notice generation retains6 known metadata-only gaps; no external publication. The exact native archive is extracted to new target/runtime/vela-0.8.1/; six binary/asset hashes match the package and built files (queued-duration-frozen-package-match.json). Older archives/runtime snapshots are untouched.

Verified owned0.8.0 user-unit PID11418 and its frozen executable before normal systemctl --user stop; the command returned0 and the process exited. Fresh private0600 backup target/upgrade-backups/before-0.8.1-20261017T063540Z.tar.gz restored into a new disposable directory, with every logical SQL row identical and every referenced media hash verified. Counts/schema:1 account,20 tracks,20 assets,1 artwork,0 playlists, schema7, quick_check ok,0 foreign-key errors. No original data was reset/restored/deleted, and no production credential was inspected or created. Evidence:queued-duration-upgrade-{prepare.log,backup.json}.

Current PID15481 is managed by the non-root transient user unit vela-local-development.service and serves127.0.0.1:3210 from target/runtime/vela-0.8.1/ against the original data/. Health/setup report0.8.1/completed. Served HTML/WASM/JS/both CSS hashes match the frozen package; post-start counts/integrity equal the verified backup. Unit active/running,0 restarts, bounded on-failure policy; no tool session owns this process. Exact current PID/unit/source/assets are in queued-duration-upgrade-runtime.json and log queued-duration-supervised-server.log. Reinspect this actual unit/executable before future upgrades. Automatic startup after reboot/user-manager restart remains unconfigured; this is a transient local review deployment, separate from deploy/vela.service's operator installation.

Preserve target/runtime, target/releases, target/upgrade-backups and original data; do not cargo clean. New builds cannot replace active frozen files. Later documentation/test commits do not change frozen source ac31d74. Next product slice remains private consent-controlled listening events with idempotency/export/delete, then genuine history/radio/mixes; native/offline/federation and untested targets remain explicit backlog. No public deployment, remote CI execution, unsupported platform, gapless or full-product completion is claimed.


## Chromium/Firefox follow-up for0.8.0 (2026-10-17)

After the verified0.8 local upgrade, broadened P0 browser evidence without changing application code, schema, dependencies or production cache policy. The source/runtime remains2519119; the docs checkpoint31d37dc records its preserved local data. Pinned Playwright1.64 downloaded its Firefox157.0 build1555 into the private tool cache (firefox-install.log). This CachyOS host is not officially supported by Playwright, so its Ubuntu24.04-x64 fallback was used. No global system packages, user browser profile or credentials were accessed.

Two initial full Firefox runs passed24 cases, then NET01 failed with0 HEAD probes (8 cases unexecuted). Traces showed no new stream GET after the fixture's paused reload while its clock continued: buffered-resource reuse bypassed the intended network outage. The earlier internal-retry inference was corrected from that trace. Merely keeping all pre-probe GETs aborted did not exercise an uncached transfer. An attempted filtered diagnostic passed setup but failed its Home prerequisite because Beacon was omitted; this serial suite needs its real earlier fixture scenarios (firefox-network-focus.log). No application regression or passing full-suite claim was inferred from these failed attempts. Original full failure logs/traces are retained in firefox-{first,second}-failure.

Network fixtures now release only their own paused audio buffer with native pause/remove-src/load before reload, preserve the real backend/bookmark and assert the saved position remains9..10s. NET02 also releases its own buffer before the exhaustion stage. Every pre-probe GET genuinely fails, with a positive failure count. NET01 requires exactly one real authorized HEAD, a changed application source generation after that probe starts, genuine resumed playback and the saved seek. NET02 retains cancellation/duplicate occurrence/3-probe exhaustion/modal retry at6s; RES03 retains held-transfer/watchdog behavior. The readonly reviewer accepted the cleanup and attribution guards. This is cold-source streaming recovery, not offline support or production buffer eviction.

Final complete runs against the unchanged0.8 release server/WASM both passed with the licensed live import: Firefox33/33 in1.5m (now-playing-firefox.log) and Chromium33/33 in1.3m (now-playing-chromium-crossbrowser.log). Same native-focus/Media Session/paused inspection/metadata/favorite races/auth/320px+large-text/light-dark/short-desktop scenarios ran in each engine. Screenshots are retained separately in now-playing-firefox-screens and now-playing-crossbrowser-chromium-screens; actual Firefox desktop-dark and phone-light images were visually inspected. Physical sound, stock Firefox variants, Safari/WebKit, screen readers, OS media controls and mobile hardware remain unverified. Existing110 Rust, strict native/WASM checks, package/backup verification remain unchanged application-code evidence from the same session.

CI now installs/runs Chromium and Firefox sequentially against fresh server.sh databases and the native release binary, preserving Chromium artifacts before Firefox. JS syntax, diff whitespace and YAML parse/step-order checks passed (cross-browser-ci-yaml.log); no remote CI/push/publication occurred. README and browser instructions now state33 cases (32 deterministic +1 opt-in), separate engine invocations, and the fixture-state limitation. The parity/limits/design/security/performance docs distinguish the tested headless targets from unsupported platforms. Local0.8 process/data are unchanged; see the runtime section below for PID221832/tool session33671 and private backup.


## Root Now Playing0.8.0 (2026-10-17)

Continued from clean main a62a9ad and the owned frozen0.7.1 runtime/source cbbc467. Re-read conventions/contracts/progress and inspected actual process, health and readonly SQLite counts: schema7,1 account,20 tracks,0 playlists. Re-ran the unmodified baseline:110 Rust tests and30 Chromium scenarios including the licensed live import passed (now-playing-baseline-{rust,browser}.log); baseline images are retained in target/checks/now-playing-baseline-screens. No schema, HTTP API, storage semantics or third-party dependency pin changed. Seven workspace packages/lock are0.8.0.

### Usable increment and review

Root now opens a native Now Playing modal from the current-track button while retaining the same HTMLMediaElement, queue/cursor, source generations and playback coordinator. now_playing.rs only presents actual metadata/context/clock/modes/status and root Actions. Desktop has cover/transport beside original-file properties and a bounded saved-order queue preview; phones use a scrollable full-height view. Real seek/pause/play/previous/next/shuffle/repeat/volume/favorite/retry/queue selection share existing handlers. Preview selects the exact occurrence; View full queue opens existing queue controls. Known duration is available before fetching audio, and small source sizes use B/KiB/MiB/GiB rather than0.0MB. Unsupported lyrics/devices/effects are not exposed.

Favorite mutations use the same authorized root path and update matching library/catalog/queued copies without resetting traversal. Local busy/error feedback additionally checks exact session, modal-open generation and current track+cursor. Closing/reopening, track change, account change and expiry cannot revive old feedback. Native close occurs before unmount for focus return. A small exact-node close bridge handles direct browser close; legacy focus trapping excludes native dialogs. now-playing.css is copied/linked, exactly allowlisted by the server, and checked by the package verifier when referenced. Unrelated CSS/database files remain denied.

Two actual contributors participated: now_playing_css delivered only the new stylesheet; now_playing_review performed readonly design/integration/lifecycle review. Main implemented Rust/JS/server/test/contracts, reviewed/integrated CSS and refined height-aware artwork. Review findings on favorite consistency, artwork identity, asset access and dialog lifecycle were integrated. No contributor ran competing Cargo builds or committed.

### Executed checks

- `cargo fmt --all -- --check`, strict workspace/all-target Clippy and strict wasm32 web Clippy passed (now-playing-{fmt,clippy,clippy-wasm}.log), with CARGO_BUILD_JOBS=2.
- `cargo test --locked --workspace -- --test-threads=2`:110 passed, including the extended real TCP static-asset regression (now-playing-rust.log). Native release and release WASM/static builds passed (now-playing-{native,web}.log).
- Real isolated Chromium against the release native server/WASM:33/33 passed in1.2m with VELA_LIVE_IMPORT_TEST=1 (now-playing-browser.log). NP01 checks no stream request/remount on open, real clock/seek/transport, native inertness/focus/Escape/return, exact duplicate occurrence and captured real Media Session Next/Pause/Play. NP02 holds genuine denied/successful server replies across track changes and close/reopen. NP03 verifies genuine FLAC artwork/metadata, consistent favorites, paused inspection without media fetch,320/768/1440/1920px both themes,720px desktop height and enlarged phone text. NET02 now retries inside the full player at the saved seek; AUTH01 clears it on genuine401. The final account-switch test holds a denied favorite and a private job response across real logout/login. Existing upload/import/scan/search/playlist/order/recovery/security scenarios remain passing.
- The first full run exposed disabled mounted-event support in the lean Dioxus build:25 passed,1 failed,7 unexecuted. The dialog existed but showModal was never called. Main replaced onmounted with a cancelable component-scoped deferred effect, consistent with the working metadata editor. The reviewer confirmed task-drop semantics against pinned Dioxus source. Failure trace/log is retained in target/checks/now-playing-first-failure. A complete33-pass intermediate run and screenshots remain in now-playing-browser-before-polish.log / now-playing-before-polish-screens; final run includes size/duration/short-height refinements.
-9 Python tooling regressions, JS syntax checks, locked cargo-deny policy and notice generation passed (now-playing-{python,deny,notices}.log). cargo-deny is the existing target/tools/0.20.2 binary; the initial plain cargo deny invocation found no installed subcommand and was corrected. Six known metadata-only notice gaps and the documented compile-time advisory exception remain; no external distribution occurred.

Final screenshots are preserved in target/checks/now-playing-final-screens, including10 Now Playing views plus Home/recovery checks. Main visually inspected actual desktop dark, phone light, enlarged-text phone and short-desktop light images. Normal-size seek/transport/volume fit; enlarged text wraps and remains vertically reachable. No proprietary artwork was used. The in-app Browser runtime returned no available browser; its documented list confirmed[] and the existing isolated Chromium suite was used. Docker info failed because /var/run/docker.sock is absent (now-playing-docker.log). Firefox/Safari, screen readers, physical media controls, native clients/mobile/offline/handoff/federation remain unverified or absent as stated in KNOWN_LIMITATIONS. No new performance benchmark claim.

### Verified package and active local checkpoint

Source/runtime0.8.0 is2519119c1d73c829fdd2cda23910226e31fdd218. The clean source/native archives are target/releases/vela-0.8.0-{source,x86_64-unknown-linux-gnu}.tar.gz. Extracted-package default startup, version, setup, schema7, HTML/WASM/new CSS and SIGTERM exit0 passed; both hashes and all preserved earlier release manifests passed (now-playing-package{,-verify,-checksums}.log; now-playing-all-release-checksums.log). No artifact was overwritten/published. Native/source packaging is local Linux x86_64 only.

The exact archive was extracted to new target/runtime/vela-0.8.0/; binary/WASM/index/JS/both CSS files match the archive and built files (now-playing-frozen-package-match.json). Owned0.7.1 PID202665 was sent SIGTERM; its tool session53837 exited0. Private0600 backup target/upgrade-backups/before-0.8.0-20261016T212252Z.tar.gz was restored into a separate disposable directory. All logical SQL rows matched the stable original database, including schema7,1 account,20 tracks,20 assets,1 artwork,0 playlists; quick_check ok and0 foreign-key errors. Restore verified every referenced file hash. Original data was not reset/restored/deleted, and no production credentials were inspected/created. Evidence:now-playing-upgrade-{prepare.log,backup.json}.

PID221832 now serves127.0.0.1:3210 from frozen target/runtime/vela-0.8.0/ using the original data/. Health/setup report0.8.0/completed; served HTML/WASM/JS/both CSS hashes match the package and post-start counts/integrity match the backup (now-playing-upgrade-runtime.json). Tool session33671 and now-playing-dev-server.log identify this persistent owned process for future graceful shutdown. Later docs/tests commits do not change this frozen source/runtime. Preserve all target/runtime, target/releases, target/upgrade-backups content and re-inspect the actual process/library before future upgrades because the user can add music.

Next independent work: broaden browser/accessibility evidence (Firefox install/probe now underway), then implement private consent-controlled/idempotent listening events with export/delete before real Home history/radio/mixes. The parity matrix contains ten current gaps and ADR0012 records the root-view design. No public deployment or native/mobile/offline/federation completion is claimed.


## Restored-clock follow-up0.7.1 (2026-10-16)

The0.7.0 source26eec9459dc6d8ef068d9939cb7d0be3c90b3445 was packaged locally and extracted-package startup/default assets/schema7/SIGTERM exit0 and both SHA256 entries passed (resume-package{,-verify,-checksums}.log). Its native/source archives remain under target/releases; it was not installed over the active0.6.1 server or externally published.

During final review, a sequential Previous on a paused restored occurrence was found to use the empty HTMLMediaElement's0s clock instead of the saved11s position. RES01 was extended to disable restored shuffle, press Previous before loading any source, require the same duplicate row/0s/no media request, then set a pending11s seek and explicitly resume. Running the extended test against the actual0.7.0 binary/WASM reproduced the wrong selected row:22 passed,1 failed,7 skipped. Evidence:resume-restored-previous-sequential-before-fix.log and resume-previous-before-fix/. The earlier shuffle-only variant passed because its first-history boundary independently seeks to0; the sequential branch is now explicit.

Version0.7.1 adds BrowserPlayer::elapsed: use the matching metadata-ready error-free active element clock, otherwise the bookmark. PlaybackUi supplies this to the unchanged order model. No schema/API/storage/dependency pin changed; workspace/lock versions are0.7.1.

Final patch checks: fmt; strict native/all-target and wasm32 Clippy;110 Rust tests; native release; release WASM/static build; locked cargo-deny policy all passed (target/checks/resume-patch-{fmt,clippy,clippy-wasm,rust,native,web,deny}.log). The complete real Chromium suite passed30/30 in59.9s including the new sequential branch, all prior recovery/expiry/playlist/order/navigation/ingest/security checks and licensed live import (resume-patch-browser.log). The9 Python tests passed earlier in this session with unchanged tool code (resume-python.log). Docker info actually failed because /var/run/docker.sock is absent (resume-docker.log); no container/native-client/mobile/other-browser claim follows.

Eleven responsive Home/recovery images are retained in target/checks/resume-patch-final-screens. The phone44px retry/viewport/no-overflow checks passed in both themes; these are Chromium visual/interaction checks, not screen-reader or physical audio certification. The parity matrix now lists ten actual remaining gaps instead of repeating resolved baseline issues. Next work remains root-coordinated Now Playing, then private deduplicated listening events and real discovery.

### Active local checkpoint

The verified0.7.1 source/runtime checkpoint is cbbc467e944a8ca02d323be37bb4830e7eaa3901. Native/source archives target/releases/vela-0.7.1-{x86_64-unknown-linux-gnu,source}.tar.gz were produced from that clean commit. Extracted-package version/default setup/assets/WASM/schema7/SIGTERM exit0 passed; both hashes and every preserved0.1–0.7.0 manifest passed (resume-patch-package{,-verify,-checksums}.log; resume-all-release-checksums.log). No artifact was overwritten or published. Highest required GLIBC remains2.34 on this x86_64 Linux host; no ARM64/other-OS claim.

The exact verified native archive was extracted into the new ignored target/runtime/vela-0.7.1/; native/WASM/JS/CSS hashes match the package and built files (resume-frozen-package-match.json). The owned0.6.1 PID159641 was sent SIGTERM and confirmed stopped. Fresh private0600 backup target/upgrade-backups/before-0.7.1-20261016T203131Z.tar.gz was restored into an isolated disposable directory and compared against the original database: schema7,1 account,20 real tracks,0 playlists, quick_check ok,0 foreign-key errors. No source media/account/library was reset/deleted and no production credential was inspected or created. Evidence:resume-upgrade-backup.json.

PID202665 now serves127.0.0.1:3210 from target/runtime/vela-0.7.1/ using the original data/; health reports0.7.1 and setup remains completed. Served WASM/JS/CSS hashes match the frozen package; database counts/integrity are identical after startup (resume-upgrade-runtime.json). Tool process session53837 and log resume-dev-server.log identify this active local process for future graceful shutdown. Frozen BUILD.txt records source cbbc467; later documentation-only commits do not change the runtime. Preserve all target/runtime, target/releases and target/upgrade-backups content; future builds must not cargo clean/replace the active frozen files. Reinspect the process and counts on resume because the user can add music.

No public deployment, external release, remote CI run, real native/mobile background playback, offline or cross-server/federation implementation is claimed. Next autonomous work: a root-coordinated Now Playing view with real track/queue/context/quality state, keyboard focus and320px dark/light E2E without audio remount; then separately design private listening events/consent/idempotency/export before Home recents and radio/mixes. Current ten-gap priorities and platform limits are in SPOTIFY_PARITY_MATRIX and KNOWN_LIMITATIONS.


## Browser resume and recovery0.7.0 (2026-10-16)

Continued from clean main df24a13, runtime/source e69120c and the frozen0.6.1 loopback server. Seven existing crates remain; all workspace versions are0.7.0. No database migration, storage semantics, server API or third-party dependency pin changed; the existing web-sys dependency enables MediaError. One actual read-only playback reviewer delivered findings; its subsequent implementation turn hit the environment usage limit without delivering a file. Main implemented and integrated all code sequentially.

### Usable increment

- session.rs validates atomic versioned per-account queue/cursor/current-ID/context/position/preferences bookmarks, canonical UUIDs,1000 occurrences,4MiB input/output including JSON escaping, finite seek/volume, modes and text bounds. Legacy structural restore remains bounded/validated. Bookmarks contain metadata/state, no session/CSRF/import/source credentials. They are plaintext local browser data, not an offline download or authorization grant.
- Root App keeps one audio element and the existing pure order model. Reload restores paused and makes no stream request before explicit action. Saved seek applies after matching metadata; a new selection starts at0. Queue reorder/removal preserves active position and duplicate occurrence. Volume/shuffle/repeat and real Home/Search/Library/Album/Artist/Playlist context persist independently per account; transient traversal rebuilds. Playlist-row playback now uses stable item identity in its complete stream-authorized context rather than a one-track replacement.
- browser_player.rs owns session/source/element generations, pending seek, play-promise identity and explicit timer/probe cancellation. Three retry delays, abortable10s authenticated HEAD and an8s progress watchdog recover actual transient/stalled streams at their saved position. Stable progress resets the consecutive-failure budget; exhaustion/manual retry, decode/unavailable/autoplay states are explicit. Pause/selection/logout invalidate pending work, including repeated URLs.401 releases media/private UI and returns to sign-in while keeping the original account bookmark. Successful probes never replace auth. No navigator.onLine reachability assumption.
- Time updates checkpoint about every5s; edits/seek/pause/visibility-hidden/pagehide flush. Browser storage denial is nonfatal. Recovery status uses existing theme tokens above the persistent player and a44px touch action. Queue context is visible without changing navigation or remounting media.

### Executed checks and corrected defect

- fmt, strict native workspace/all-target Clippy and strict wasm32 Clippy passed; logs target/checks/resume-final-{fmt,clippy,clippy-wasm}.log. Native release passed (resume-native.log); release WASM/static build passed using the pinned wasm-bindgen CLI (resume-final-web.log). JS syntax for player.js/music.spec.mjs passed. Locked cargo-deny policy passed (resume-deny.log), preserving the existing compile-time paste exception. Notices regenerated: the same6 known metadata-only notice gaps remain; no external redistribution occurred.
- `CARGO_BUILD_JOBS=2 cargo test --locked --workspace -- --test-threads=2`:110 passed,0 failed. Three new policy tests cover malformed/foreign/oversized/unknown snapshots, escaped serialization limit, count/UUID/current-occurrence bounds, finite seek/context/preferences and fixed retry budget. All prior database/API/permissions/import/scanner/order tests remain. Log:resume-final-rust.log.
- `(cd tests/browser && VELA_TEST_BINARY=target/release/vela-server VELA_LIVE_IMPORT_TEST=1 npx playwright test)`:30 passed,0 failed in the final rerun (resume-final-browser-recheck.log). Seven new real cases RES01–04/NET01–02/AUTH01 exercise generated30s PCM, duplicate/context/preferences restore without autoplay/prefetch, corruption/foreign/4MiB storage rejection, actual failed/stalled transfers, authenticated HEAD recovery near the saved9s seek, paused/new-source cancellation, three-attempt exhaustion/manual retry, full playlist context at its second duplicate and genuine dedicated-session revocation. Actual licensed live import, upload/decode/range/navigation/shuffle/playlist/share/private-race/job regressions passed too. No fabricated catalog or synthetic ended substitute.
- The first28-case run stopped after24 passes: NET01 exposed a WASM panic when a browser timeout invoked the scope-dependent Dioxus spawn outside a component callback, so no recovery HEAD was sent. Browser-adapter futures now use spawn_local with explicit cancellation/session/source checks. Failure log/trace:resume-browser-first.log and resume-first-browser-failure. An intermediate29-case run then passed; final30-case runs include the playlist-context fix and44px phone action. No failed run is represented as green.
- `python3 -m unittest discover -s tests -p 'test_*.py'`:9 passed (resume-python.log). Pre-work private backup before-0.7.0-work-20261016T164541Z.tar.gz restored into a disposable isolated directory: schema7,1 account,20 tracks,0 playlists, quick_check ok,0 foreign-key errors (resume-before-backup.json). Runtime was reinspected immediately before packaging: the same real counts and frozen0.6.1 executable (resume-before-runtime.json).
- Eight Home phone/tablet/desktop/ultrawide dark/light screenshots and three actual exhausted-recovery screenshots were generated again. Desktop and320px recovery dark/light were visually inspected; no horizontal overflow and retry remains in viewport with a44px target. Files are copied to target/checks/resume-final-screens. In-app Browser reported no available browser, so only isolated Playwright Chromium was used. No physical audible output, screen-reader certification, Firefox/Safari, native/mobile/background or Docker runtime claim. Shutdown/worker recovery tools were executed in the earlier0.5 checkpoint; those code paths are unchanged and were not represented as newly run here. No new benchmark/superiority claim.

The0.7.0 local package was verified and retained; the0.7.1 follow-up above is the intended active runtime. Next prioritized autonomous slice: root-coordinated Now Playing and accessible richer track context, then privacy-controlled deduplicated listened-time events before truthful recents/radio/mixes. Native/offline/device handoff/rooms/multi-server/federation remain absent as recorded in the parity matrix.


## Current checkpoint — playlist context fix0.6.1 (2026-10-16)

Continued autonomously after packaging0.6.0. Final review found that the page-name-only playlist guard could accept an old response after leaving and returning to Playlists. The new PLNAV01 browser scenario was first run against the unchanged0.6.0 native/WASM builds and reproduced the defect (expected no detail, received one;19 scenarios passed before the new failure). Log:target/checks/playlist-navigation-before.log; trace/screens are preserved inplaylist-navigation-reproduced-failure. The failure was fixed in actual product code, not removed from the test.

### Implemented and verified

- Root PlaylistView invalidates pending details on navigation and opening another playlist, captures exact session/target/view generation and clears old share links. Returning to the same route cannot revive an old request. All ordinary navigation/catalog/Jobs/upload destinations use the same invalidation contract.
- Post-reorder/delete refresh checks selection context before/after GET and cannot lower the displayed revision. A committed mutation remains committed when its abandoned response is dropped. Collaborator list/form changes and share-link results retain their initiating view; a new collaborator draft is preserved. No API/migration/storage/permission changes; sharing remains metadata-only.
- Full `cargo test --locked --workspace -- --test-threads=2`:107 passed. Final fmt, strict native workspace/all-target Clippy, strict wasm32 Clippy and native/WASM release builds passed. Logs:playlist-navigation-{rust,fmt,clippy-final,clippy-wasm,native,web}.log. JS syntax passed. Locked dependency checks passed with the established paste maintenance exception (playlist-navigation-deny.log); third-party pins are unchanged.
- Full real Chromium run with VELA_TEST_BINARY=target/release/vela-server and VELA_LIVE_IMPORT_TEST=1:23 passed in34.1s (playlist-navigation-browser-final.log; prior23-case run also passed in36.1s). PLNAV01 holds actual detail, deletion-refresh and share POST responses across route/selection changes, confirms the mutation committed, and prevents a stale detail or another playlist's link appearing. An authenticated out-of-band edit plus a genuine stale-revision409/newer refresh also confirms that an older same-view snapshot cannot lower the displayed revision or remove its latest rows. Existing real FLAC/MP3/WAV/Ogg playback, licensed live import, all ingest/filter/job/share/security/browser races and the two new shuffle scenarios remain green. No mocked catalog/share response replaces the backend.
- Eight responsive/theme screenshots were regenerated undertarget/checks/playlist-navigation-final-screens. Prior visual review covers the unchanged shell; current E2E repeats overflow, keyboard and persistent-media checks. Full screen-reader/cross-browser/native validation is still absent. The nine Python tooling tests and20 restart/both worker-failure checks recorded below were executed earlier this session; these scripts/worker code did not change.

### Live local checkpoint and safeguards

Stopped the owned0.6 process gracefully (exit0), made private0600 backup target/upgrade-backups/before-0.6.1-20261016T160646Z.tar.gz and restored it to an isolated disposable fixture. Before/after remain schema7,1 account,20 real tracks,0 playlists, quick_check=ok and0 foreign-key errors. No user library, account or original file was reset/deleted, and production credentials were not inspected or created.

The verified0.6.1 binary and assets were copied to ignored target/runtime/vela-0.6.1/ and are now serving127.0.0.1:3210 with the original data/ directory. Health reports0.6.1. This snapshot is separate from mutable target/release and dist, so later builds do not replace the active local interface. Backup/runtime evidence:playlist-navigation-upgrade-{backup,runtime}.json; log:playlist-navigation-dev-server.log. Preserve target/runtime, target/releases and target/upgrade-backups; do not cargo clean/remove target wholesale. Reinspect the listener and counts before any subsequent upgrade because the user may add music.

The0.6.1 runtime/source checkpointe69120cd0df98a5f08c886557a622f6c5195cec4 is packaged as target/releases/vela-0.6.1-{x86_64-unknown-linux-gnu,source}.tar.gz. Extracted-package default startup, version/first-run HTML/bootstrap/WASM/schema7 and clean SIGTERM passed; both0.6.1 checksums and every preserved0.1–0.6 manifest passed. Packaged binary and WASM SHA256 hashes match the active frozen runtime (playlist-navigation-runtime-package-match.json). Highest required GLIBC symbol remains2.34; runtime evidence covers this x86_64 Linux host. Logs:playlist-navigation-{package,package-verify,checksums}.log. Frozen BUILD.txt records the source checkpoint. Earlier0.1–0.6 artifacts are retained. No external release/CI or public deployment occurred. Docker daemon, Firefox/Safari, native/mobile devices and offline/handoff/federation remain unverified or not implemented as marked in the parity matrix. No new benchmark/performance-superiority claim. Next prioritized autonomous slice remains exact playback-context/position restore and reconnect/session-expiry recovery, then Now Playing and privacy-controlled listening-event foundations for real discovery.


## Playback order0.6.0 (2026-10-16)

Continued from the verified0.5 source3894e4e and documentation863bec3 in the same session, sequentially in the main agent. The user-visible Home/Search/shell increment and ingest/share/job/security workflows are retained. Third-party dependency pins, seven-crate boundaries and schema7 are unchanged; all local crate versions are0.6.0.

### Usable increment

- apps/web/src/playback.rs is a pure bounded occurrence-index state machine, consumed by the root PlaybackUi bridge. Shuffle visits every occurrence once per cycle and records real previous/forward traversal. Repeat-all refills a full permutation while excluding the previous last item from the first draw only. Repeated copies of the same track stay separate occurrences.
- Manual Next bypasses repeat-one; real natural EOF under repeat-one restarts the current track. Repeat-off stops at exhaustion. Previous after3s seeks to the start while preserving paused/playing state; otherwise it returns through actual shuffle history or sequential order, respecting boundaries.
- Play next takes priority even during shuffle. Insert/swap/remove remap deck/history/current/priorities synchronously, and current removal is rejected. Actual queue editing preserves active media. Legacy queue restore is capped1000, queue/cursor persistence remains account-scoped, and transient modes reset across account changes. Consolidated the duplicate cursor-persistence effect; metadata/favorite edits retain traversal.
- Repeat-one now has a distinct original SVG and repeat controls expose pressed state. The shuffle queue explains that visible rows retain saved order. No precomputed upcoming preview, durable shuffle journal, exact position resume or new native/audio-effect capability is implied.

### Executed checks and actual failures corrected

- Final fmt, strict native workspace/all-target Clippy and strict wasm32 Clippy passed:target/checks/playback-final-{fmt,clippy,clippy-wasm}.log.
- `CARGO_BUILD_JOBS=2 cargo test --locked --workspace -- --test-threads=2`:107 passed,0 failed. Eight new pure-model tests cover2048 length/seed permutations, full repeated cycles,10000 repeat-all transitions/bounded history, manual-vs-ended repeat, previous/forward, empty/singleton/legacy bounds, next priorities and edits. They caught an intermediate repeat-all implementation that omitted the preceding last occurrence for an entire new cycle; it was fixed by excluding it only from the first draw. Log:playback-final-rust.log; targeted log:playback-unit-final.log.
- Native0.6 release build passed (playback-native.log); release WASM/static build passed using the pinned wasm-bindgen CLI (playback-web-final.log). JS syntax passed. Locked dependency policy passed (playback-deny.log); regenerated notices retain the6 known metadata-only gaps (playback-notices.log).
- `(cd tests/browser && VELA_TEST_BINARY=target/release/vela-server VELA_LIVE_IMPORT_TEST=1 npx playwright test)`:22 passed in34.1s. New scenarios20/21 use genuine30s PCM uploads and actual decoder EOF, not a synthetic ended event. They test fair traversal, Previous/Next history, paused seek, repeat-one/manual-next, repeat-all, explicit-next through reorder/remove, duplicate rows and reload cursor. Scenario19 additionally confirms account-mode reset. All prior browser regressions including licensed live import/private races passed. Log:playback-browser-recheck.log.
- Initial runs failed on test assumptions: the old-play test looked only at the first50 random-UUID tracks, and the expanded account-mode test tried Shuffle before initializing any queue in its fresh context. Tests now search for the real audio and explicitly start it. Assertions and production limits were preserved. Failure logs/traces are retained underplayback-browser{,-final}.log andplayback-{first,second}-browser-failure; no failing run is represented as green.
- Eight responsive/theme screenshots were regenerated and copied totarget/checks/playback-final-screens; phone dark and desktop light were inspected again, with layout consistent with the eight reviewed0.5 combinations. No Firefox/Safari, screen-reader, physical output or native client claim. The9 Python tests and20 clean restarts/both worker failures were executed earlier in this session for the0.5 slice; their code is unchanged and their exact logs remain in the section below. Docker daemon remains unavailable.

### Data/runtime checkpoint

The owned0.5 development process was gracefully stopped and returned exit0. A fresh private0600 backup target/upgrade-backups/before-0.6.0-20261016T155224Z.tar.gz was restored in an isolated disposable directory and verified. Existing schema7,1 user,20 tracks and0 playlists match before/after, quick_check=ok and foreign_key_check has0 errors. The0.6 server now serves127.0.0.1:3210 using the same data/ and dist/; health reports0.6.0. No source media, account or playlist was reset/deleted; production credentials were not inspected or created. Evidence:playback-upgrade-{backup,runtime}.json andplayback-dev-server.log. Reinspect counts on resume because the user may continue adding music.

No external publication or remote CI occurred. The0.5 and earlier archives remain intact.The0.6 runtime/source checkpoint29ee2023aa0a1601824c9bd740beb56e33b182ef was packaged under target/releases into native x86_64 Linux and exact-source archives. Extracted-package startup/version/assets/WASM/schema7/SIGTERM checks and both SHA256 entries passed (playback-{package,package-verify,checksums}.log). The0.1 checksum manifest was rechecked separately as well. Next autonomous slice: persist explicit playback context/exact position with stale-source/session guards, then implement reconnect/expired-session recovery and root-coordinated Now Playing. Privacy-controlled listening events are a separate prerequisite for truthful Home recents/radio/mixes. Native/offline/devices/rooms/multi-server/federation remain absent.

## Previous listener checkpoint0.5.0

## Listener experience0.5.0 (2026-10-16)

The actual starting repository was clean main atb7033e4, source/runtimeac01e40, version0.4.0. Its baseline was rerun:97 Rust tests,16 Chromium tests (including the opted-in licensed live import), fmt, strict native/WASM Clippy and release builds passed before changes. Seven existing crates were evolved; no new framework, crate, migration or persistent-data reset was introduced. One read-only parallel experience reviewer identified player/navigation risks but then hit the environment usage limit and delivered no code; the main agent implemented/integrated this increment. No external publishing occurred.

### What users can use

- New real Home with greeting, newest owned tracks, favorites, albums and authorized playlists; useful first-upload/import actions and honest loading/error/empty/refresh states. No imaginary recents, global catalog or recommendations.
- A separate Search page with300ms debounce, Enter, Ctrl/Cmd+K and tracks/albums/artists/playlists categories. New authenticated GET /api/search bounds each category8, filters owned catalog/current named playlist grants, rejects malformed/unknown/empty/control/overlong queries and never grants media access. Literal group/playlist matching and FTS tracks reuse existing services/projections.
- Shared listening-oriented sidebar with real library/playlist shortcuts, original SVG icons, dark/light responsive styles, and five phone tabs where More opens Settings and its More from Vela list reaches the remaining destinations. Jobs navigation is administrator-only; existing uploads/imports/folder administration and metadata/playlist flows remain functional.
- Root-owned audio survives Home/Search/Library/album/artist/playlist/history/theme/menu changes. Home/Search use typed cancellable requests, physical AbortController cancellation and independent session/query/generation/lifetime guards. Late play-promise errors cannot overwrite a newer play. Next/last queue actions preserve duplicate occurrences and active playback; cursor changes now persist alongside order. Logout releases the media source and clears pending play identity.

### Executed evidence

- `cargo fmt --all -- --check`:passed; `CARGO_BUILD_JOBS=2 cargo clippy --locked --workspace --all-targets -- -D warnings`:passed; `cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings`:passed. Final logs:target/checks/experience-final-{fmt,clippy,clippy-wasm}.log.
- `cargo test --locked --workspace -- --test-threads=2`:99 passed,0 failed (prior97 plus2 real TCP aggregate-search tests). New tests cover private same-file owners, named playlist grant/revocation without stream access, empty-group hiding, literal hostile text,8-result bounds and malformed query fields. Log:experience-final-rust.log.
- `cargo build --locked --release -p vela-server` and `VELA_WASM_BINDGEN=target/tools/wasm-bindgen-0.2.129-x86_64-unknown-linux-musl/wasm-bindgen scripts/build-web.sh`:passed. Logs:experience-final-{native,web}.log. JavaScript syntax passed for player.js and music.spec.mjs.
- `(cd tests/browser && VELA_TEST_BINARY=target/release/vela-server VELA_LIVE_IMPORT_TEST=1 npx playwright test)`:20/20 passed in38.6s against real isolated native/WASM builds. Existing16 workflows retained; four new cases exercise actual Home/all-kind Search, navigation audio-node/source/time continuity, next/last duplicate order and reload cursor,300ms query ordering/physical abort/invalid bytes, late failed old play after genuine newer playback, and private Search replies held across logout/account change. No catalog responses were fabricated. Real locally generated45s PCM audio supplements existing FLAC/MP3/artwork and licensed live Ogg import. Log:experience-browser-final.log.
- Eight Home screenshots were reviewed at320/768/1440/1920px in both themes, with reduced motion and no horizontal overflow. The More sheet passed keyboard focus containment/Escape/return-focus checks. Copies:target/checks/experience-final-screens/experience-home-*-*.png. Baseline images are retained inexperience-baseline-screens. In-app Browser reported no available browser; isolated Playwright Chromium was the fallback. No physical sound, screen-reader, Firefox/Safari or native device claim.
- `python3 -m unittest discover -s tests -p 'test_*.py'`:9 passed. `python3 scripts/verify_shutdown.py --runs20 --worker-failure` (CLI spelling `--runs 20`):20 real SIGTERM/restart exits0; separate import/scanner storage-failure fixtures correctly failed the process. Locked cargo-deny policy passed with only the established compile-time paste maintenance exception. Notices were regenerated;6 known metadata-only license gaps remain. Compose configuration passed; Docker daemon is absent, so no container build/runtime verification. Logs:experience-{python,shutdown,deny,notices,compose,docker}.log.

### Data safety and local runtime

An initial current-data snapshot was restored/checked before development (target/upgrade-backups/before-0.5.0-20261016T094642Z.tar.gz). Persistent development data changed during this session and was re-inspected rather than reset: the final pre-start snapshot contains1 account,20 actual tracks,0 playlists, schema7. Fresh backup target/upgrade-backups/before-0.5.0-final-20261016T153528Z.tar.gz was restored to a separate disposable fixture and matched schema/counts/quick_check. The previous listener was already absent when inspected; a0.5 server is now running on127.0.0.1:3210 with the existing data directory and built dist. Health/setup report0.5.0, setup stays claimed, SQLite quick_check isok and foreign_key_check has0 errors. No production credentials were read/created and no development library/media was deleted. Backup/runtime evidence is inexperience-upgrade-{backup,runtime}.json. Reinspect on resume: users may continue adding data.

### Scope and next work

The feature matrix, product flows, design system, ADR0009 and contracts document exact verified subsets. Home/Search/next-last are usable but the full consumer roadmap remains incomplete. Next highest-priority slice: a framework-free queue/playback transition model, fair shuffle previous/forward behavior, deterministic previous-button seek and recovery/resume tests; then root-coordinated Now Playing and privacy-controlled listening events for real radio/mixes. Exact position resume, native/offline, device handoff, rooms, multi-server and federation remain absent. No performance-superiority or new latency measurement is claimed.

The earlier progress below is historical evidence, not the current runtime/account state. Local0.5 runtime/source checkpoint3894e4e67736fa558c44a0541ddad44aee2c29dd was packaged into target/releases/vela-0.5.0-{x86_64-unknown-linux-gnu,source}.tar.gz. scripts/verify_package.py extracted the native artifact and verified default startup, version/first-run assets/WASM, schema7 and clean SIGTERM. Both0.5 checksums and all preserved0.1–0.4 manifests passed. readelf reports highest GLIBC symbol2.34; actual runtime testing covers this x86_64 Linux host only. Logs:experience-{package,package-verify,checksums}.log. No archive was published externally.

## Historical0.4 checkpoint

Checkpoint2026-10-16, version0.4.0. The initially empty repository is a seven-crate Rust/Dioxus modular monolith. Actual specialized agents contributed media/import/UI, the job read model and security/QA review; the principal reviewed and integrated shared contracts, migrations and transport. No external publishing occurred. This is a functional early product slice, not completion of the master roadmap.

## Implemented and verified

- Atomic first-administrator setup, Argon2id accounts, hashed random sessions, CSRF/origin/rate controls and administrator-created users. Private catalog/artwork/stream ownership is independently enforced.
- Real sequential batch uploads, bounded Lofty metadata/artwork processing, exact BLAKE3 deduplication, immutable publication, FTS search/favorites/stats and original HTTP range streaming. Multipart headers are bounded before parsing; canonical opening boundaries prevent a reviewed preamble bypass. Oversized headers/additional parts fail before publication.
- Durable bounded direct HTTP/S imports with pinned public egress, limits/progress/retry/cancel/restart recovery. Real HTTP download tests ingest actual audio; no fake source adapter responses.
- Private local collaborative playlists: viewer/editor grants, collaborator list/revoke/leave UI, transactional revisions, actor/request-bound idempotency, expiring/revocable metadata-only links. Public views omit private contributor entries; permission to read playlist metadata never grants streaming.
- Persisted private album/artist identities, migration/backfill and automatic transactional regrouping. Owner-isolated summaries/track pages, album disc/track ordering, bounded cursors and visible-group counts. Original audio and embedded-source strings remain intact. These are provisional catalog groups, not verified recording/edition identities.
- Administrator-configured read-only folder scanning: Linux descriptor-relative no-follow traversal, real FLAC/MP3 snapshots, bounded durable jobs/counters/cancellation/restart, private fingerprints and original-file preservation. Unchanged/exact-duplicate rescans retain catalog edits; source removal retains owned music. Admin Settings uses real source/job APIs; its browser workflow passed.
- Typed Library/Favorites genre/format/year/audio-minimum filters composed with search and private favorites; title/newest keyset orders, bounded private suggestions, draft/applied separation and session-safe resets. The actual filter/mobile/race/pagination browser scenarios passed.
- Unified owner-private Jobs history over actual durable imports/scans, bounded mixed keyset pagination, type/state filters, real counters/bytes and authorized cancel/import retry/new scan pass. Fixed explanations omit source URLs/raw diagnostics, owner-constrained joins protect foreign references, and activity/action hints share one read snapshot. Visible active-only polling, stale/error recovery and session/filter/lifetime guards are implemented; legacy import polling stops on Jobs. Eight mobile destinations fit320px.
- Owner-only catalog metadata editing with typed bounded fields, audit, revision CAS and explicit stale-draft reload. Real tests verify search/group refresh, unchanged audio bytes/hash, private same-file catalogs, dedup preservation and restart persistence.
- Browser playback controls, duplicate-safe queue ordering, account-scoped persistence and stale-response guards. OS MediaSession metadata is cleared on logout. New album/artist pages use real API data and bounded50-item requests; their browser validation passed.
- Exclusive data-dir lock, recognized temporary-file cleanup, supervised durable worker, graceful drain, standard-library SQLite/media backup with safe restore. A shutdown race was reproduced and fixed;20 isolated clean restarts and deliberate fixture worker failure were previously verified.

## Executed checks

- Latest `cargo test --locked --workspace -- --test-threads=2`:97 passed (6 core unit,7 core job integration,13 imports,10 media unit,6 catalog integration,8 filter integration,8 ingestion integration,4 metadata integration,2 scanner unit,8 scanner filesystem/SQLite integration,3 multipart guard,22 real TCP API). Includes legacy migration, private cursors/joins, coherent job snapshots, real cancel/retry/new scan history, FTS integrity, actual upload/stream/sharing/import recovery,16 held large streams with concurrent uploads and real folder ingestion, backup restore and public/static isolation.
- Latest strict native Clippy (`--workspace --all-targets -- -D warnings`):passed. Strict WASM Clippy and native/WASM release builds passed.
- Nine Python tooling regressions passed (five backup/restore and four license-discovery/provenance/byte-integrity checks). JavaScript syntax and Compose configuration passed. Locked dependency policy checks passed with the documented single unmaintained compile-time paste exception (SECURITY.md); no runtime vulnerability advisory is ignored.
- Sixteen isolated headless Chromium E2E passed against production native/WASM builds. They covered setup/upload FLAC/metadata/artwork/audio decoding/time advancement/favorites/playlist sharing/revocation, malicious URL/mobile layout, a real licensed Wikimedia URL import and stale search/account responses. Album/artist playback, imported Ogg decoding and stale album/artist selection were verified. Revisioned metadata save/two-tab conflict/reload/mobile dialog and cancellation across account changes passed. The folder workflow added a real MP3, retained edited FLAC tags during dedup/rescan, played the result and removed the source without deleting music. Filters covered composition/favorites/sort/reset/invalid drafts, delayed responses/private suggestions and52 real WAV uploads yielding50+2 pages despite drafts. New Jobs scenarios inspected the completed real URL import, ran52 genuine folder passes, invoked Scan again, checked50+2 history/stable older refresh/navigation, injected a503 to verify visibly stale recovery, and delayed real responses across filter/account changes. Eight navigation destinations were visible at320px with no horizontal overflow; the mobile screenshot was inspected. Physical sound/background/mobile hardware behavior is unverified.
- Synthetic SQLite10k/100k/1M and sparse100k search/filter measurements retain their exact SQL scope. A new actual native authenticated task-history HTTP benchmark measured1,800 requests across10k/100k owned+equal foreign terminal job histories and verified all50-row pages. Reports/raw samples/source hashes are in PERFORMANCE. This is serial warm loopback task metadata, not audio/concurrent playback capacity or superiority evidence.
- Docker CLI exists but its daemon is unavailable: no image build/runtime claim. ARM64, Safari/Firefox, desktop/mobile packaging, actual devices and remote CI are unverified.

## Resume checkpoint

Latest fmt, strict native/WASM Clippy, native/WASM release builds,97 Rust tests,16 browser tests (including opted-in live import),9 Python tooling tests and20 release SIGTERM/restart runs passed. Independent isolated import-worker and scanner-worker database failures each correctly stopped the process. Dependency policy passed with patched rustls0.23.46 and only the existing compile-time paste exception. Compose configuration passed; Docker daemon remains unavailable. Job HTTP measurements and current binary/script/runtime-source hashes were independently checked against the reports.

The0.1.0 metadata-edit checkpoint e61d963,0.2.0 scanner checkpoint a5f6a6d and0.3.0 filtering checkpoint953b41439aab68ec89a721feb822a0caa6aaf20a/archives are preserved. The0.4.0 runtime/source checkpoint is ac01e405d67df8f69a7b4455b72030adcba5e37c. Local native/server+WASM and exact-source archives were built under target/releases. verify_package.py extracted the native archive, verified default startup, health/version/first-run HTML/bootstrap/WASM, expected schema7 and clean SIGTERM. The version-specific0.4.0 checksums and all preserved0.1–0.3 manifests passed. Native ABI inspection requires GLIBC2.34; actual runtime validation covers this x86_64 Linux host only. Later documentation commits record evidence without altering runtime source. Logs live under ignored target/checks, which survives environment cleanup; do not depend on old /tmp files or a previous server process. No external release is authorized. Next product priorities are batch metadata edits and invitation-based sharing with separately scoped media permissions. Native profiling of year-range and combined catalog predicates remains measured performance work. ZIP/trusted-storage adapters, multi-server, federation, OpenSubsonic and native/offline remain in ROADMAP.

Architecture/security/operations/roadmap/limits/ADRs/research docs are maintained. Cargo.lock is included in the checkpoint source. Test data is isolated in temporary directories; no synthetic catalog is shipped.

## Local development instance

The0.4.0 release server was started on127.0.0.1:3210 with /home/rasul/Projects/vela/data and built dist assets after a fresh private pre-upgrade backup (`target/upgrade-backups/before-0.4.0-20261016T090618Z.tar.gz`). Health returned ok/version0.4.0, all seven migrations were applied and SQLite quick_check returned ok. Setup remained unclaimed; no administrator credentials or music were created outside isolated tests. This persistent development directory is ignored by Git and may now receive user data. On resume, check/reuse the listener and preserve data; never reset it. Stop gracefully before replacing the running server or applying upgrades.

## Scanner integration checkpoint

Version0.2.0 adds migration0004, the scanner crate and administrator Settings UI. Main agent integrated the domain/config/schema/API/supervision; specialized scanner, web and security agents contributed their assigned work. Security review fixed foreign-owner scan-map references with database triggers, ancestor symlink rejection, actual temporary-name cleanup, absolute default-root normalization and acquiring the HTTP listener before starting workers. The first scanner test run found a final completion timeout in a120-duplicate-artwork debug fixture after cancellation/shutdown had succeeded. Reducing that fixture to24 real files while preserving cancellation/recovery assertions and improving diagnostics passed the targeted recheck and full76-test suite. Production behavior was not changed to hide the fixture timeout.

The subsequent dependency check found four new rustls runtime advisories; Cargo.lock was upgraded to patched0.23.46 and dependency policy passed without new advisory exceptions. Native/WASM Clippy and release builds, the complete77-test Rust suite, ten browser workflows, five Python backup tests and20 restart checks plus two worker fault checks passed against that lockfile. Private pre-upgrade backups are saved under ignored target/upgrade-backups; development data was not reset. Native/source packaging and extracted-package startup verification passed. The packaging clean-tree guard initially caught an unstaged migration; it was included in the atomic source checkpoint before archives were created. No incomplete archive was delivered.

A permanent eighth scanner integration test now exercises foreign file-map insertion, track/source reassignment and owner rewrites against actual migrated SQLite after two real private ingestions. All writes were rejected and foreign-key/ownership checks remained intact. The final full suite is77 tests; this extends the earlier76-test checkpoint rather than substituting a mocked guard test.

## Typed library filter checkpoint (0.3.0)

Specialized media/web/review contributors implemented their assigned boundaries; main integrated the DTOs/schema/transport/API/browser tests and reviewed changes. Service tests cover real FLAC plus explicit synthetic metadata fixtures for ties, both FTS branches, bounds, literal hostile tags, private facets/cursors and truncation. Real TCP tests exercise complete composition and fixed JSON errors. Browser tests use the genuine backend, private generated PCM audio and account sessions; no catalog response is fabricated.

The first expanded E2E run hit the existing production login limiter. Tests now reuse only their own in-memory authenticated fixture sessions, leaving the limiter unchanged. A second run caught an overly strict desktop Settings selector; it was corrected. All13 final scenarios passed against the final release. The mobile filter screenshot was inspected and no horizontal overflow observed.

The review found/fixed a title tuple-collation expression that prevented full index seeking. A100k-owner SQL benchmark then measured expensive table reads for genre facets. Migration0006 appends raw genre to its existing index prefix; identical facet SQL/canonical values are now covering. Python SQLite fixture p50 dropped from156.088ms to22.614ms, with about1.30MiB added across200k total tracks. Before/after raw reports and exact limits are in PERFORMANCE; this is not native/API throughput. Wide year-range/combined plans remain documented for native profiling.

Core build.rs now watches additions to the migration directory, as stable SQLx macro embedding otherwise only tracks existing files. A permanent core integration checks embedded migrations5/6 and the covering index. Review packages now record and verify schema_version after startup. The final full Rust suite is88 tests; fmt/native+WASM strict Clippy/release builds,13 browser tests including live import,5 Python backup tests,20 real SIGTERM/restarts and independent failure of both workers passed. Dependency policy still passes with only the recorded compile-time paste exception. Compose configuration passed; Docker runtime is separately recorded. Private pre-upgrade backups remain under target/upgrade-backups; development data/original source folders were not reset.

## Private task center checkpoint (0.4.0)

Specialized core/web/review contributors implemented their assigned boundaries; main integrated the framework-free contracts, index migration0007, HTTP route, real TCP/browser regressions and documentation. There are still seven crates and two existing durable workers, with no duplicated queue or generic scheduler. Source URLs/raw worker diagnostics never enter the aggregate task read model. Indexed branches bound projections and a single read transaction preserves flags/activity consistency.

Seven real migrated SQLite service tests cover mixed UUID/time/kind ties, anchor deletion, status filters, ownership/role restrictions, raw URL/error redaction, foreign track references, bounds/index plans and concurrent WAL commits. Two new real TCP tests cancel/retry genuine queued imports without external requests and ingest real FLAC/MP3 through the scan worker, cancel another queued pass, create a new pass and verify preserved history/unchanged-file counters. The complete97-test Rust suite passed.

The first expanded browser run caught an exact Settings selector that did not match its desktop accessible name. The second reached the unchanged production login limiter; repeated test logins now reuse only this suite's own isolated sessions. No login/egress/security policy was weakened. All16 scenarios then passed; after increasing mobile labels to10px, the final production-native/WASM browser run passed again. It includes a real licensed URL import,52 actual scan passes with50+2 history pages, actual Scan again, stable older-page refresh, delayed real filter/account responses, stale503 recovery and no parallel legacy import polling on Jobs. Mobile task/nav screenshots were inspected.

The QA contributor ran1,800 native authenticated loopback job-history measurements with10k/100k owned+equal foreign explicit synthetic terminal rows. Every timed response had exactly50 matching owner rows, cursor and action flags. Binary/runtime source/script fingerprints match the retained reports. Scope, hardware, samples, SQLite versions and shared-host activity are recorded in PERFORMANCE; no audio/concurrency/capacity claim follows. Existing data was backed up before the schema7 development upgrade and was not reset. Local native/source archives from ac01e405d67df8f69a7b4455b72030adcba5e37c passed extraction/startup/schema7/SIGTERM verification and version-specific checksums; previous archives/checksums are preserved.

Release hardening reduced metadata-only dependency notice gaps from12 to6 using actual shipped nested licenses, reviewed r-efi AUTHORS grants, exact-commit wasite licenses and byte-proven winapi root texts. The generator verifies cached SHA256 provenance and rejects tampered/missing/oversized texts. Four new Python checks plus the five backup regressions passed. The six remaining packages have no full grant at their inspected packaged VCS commits; pinned archive evidence is retained rather than substituting generic license text. No external redistribution was performed.

The principal independently reran the retained winapi provenance helper against the pinned public tree: all2809 authored files matched, with no unmatched file. The packaged notice generator completed offline with all cached hashes verified and exactly the six documented remaining text gaps. Final package logs: target/checks/jobs-package.log and jobs-package-verify.log; final E2E: jobs-browser-release.log; Rust/Clippy/build/tooling/shutdown evidence uses the jobs-* prefix. The development listener returned health0.4.0 and setup remained unclaimed after schema7 upgrade.
## better-ui polish — CSS verified (2026-10-19)

Applied the better-ui findings: `:active:not(:disabled){transform:scale(.96)}` and
explicit `background-color/filter/transform` transitions for web/native buttons, icon
buttons and the native play toggle; `scale(.98)` for clickable playlist/listening
cards; exact `background-color` transitions on track rows; neutral 1px pure-black/white
`--image-outline` on artwork images; unused transform removed from card transitions.
Web rebuilt; Chromium41 passed/1 live-import skip (2.1m). Native CSS was not opened in
a real window in this pass.

## Native Settings + application scale — source+native QA (2026-10-19)

New `apps/desktop/src/presentation.rs` device store at
`$XDG_CONFIG_HOME/vela/desktop.presentation.json` (0700/0600, versioned
`{app_scale_percent,theme,version}`); 80–150 step5, malformed/oversized/future
documents recover to 100/Dark without deleting the unreadable text, atomic staged
write keeps the previous file on failure. The `.native-app` wrapper receives
`--app-zoom`/`zoom` and `data-theme` on the first render, before sign-in; live
changes never touch the AudioEngine/queue/selection/position. A real Settings
destination (sidebar + compact nav + signed-out entry) exposes −/+/slider/reset,
System/Dark/Light and truthful save/repair state. Desktop108 passed/3 ignored;
full native peer/session/offline QA passed (`target/checks/native-settings-qa2/`).
NOT_RUN: real visual/HiDPI/640px/short-height review, audio-preservation observation,
disk-full, Orca. Range deviation 80–150 vs plan 75–200 documented.

## Product expansion first slices — artwork + device scale (2026-10-19)

- `0f9f12b` native authorized artwork: authenticated HEAD+GET, 8MiB/MIME/magic
  bounds, private origin/account artwork cache (0600/0700, atomic, 200 files/64MiB),
  bounded presentation store (4 inflight/32 queued/512 entries/32MiB data URLs),
  data-URL covers + neutral fallback in Library/search/favorites/player/queue/
  album/artist/playlist rows, logout clears. Desktop96 passed/3 ignored incl. real
  server authorization/cache-reuse tests. ART09/12/14 and native visual runs NOT_RUN.
- `1618e63` web device appearance: versioned `vela.device.presentation.v1`
  (80–150% scale, System/Dark/Light), boot-time application before first frame,
  Settings → Appearance with stepper/slider/reset/status, one CSS zoom composing
  with browser zoom, malformed-store recovery, no credential/token storage.
  Web27 tests; targeted Chromium SCALE01 passed (4.2s) incl. reload persistence,
  same audio element/src/queue and storage-denied recovery. Full browsers/Firefox/
  screen reader/native scale NOT_RUN.

Both source slices postdate frozen runtime0.12.6; no schema/server migration and no
live data/runtime access. Plan deviation: scale range80–150 selected instead of the
proposal's75–200.

## Native 144 Hz diagnosis — cap attributed to WebKitGTK (2026-10-19)

User reports ~50 Hz. Measured with a real WebKitGTK window on the 3440×1440@144
DP-1: `requestAnimationFrame` is **72.3 fps** (focused, native Wayland, DMABUF
enabled), and a bare WebKitGTK page without Dioxus measures **71.8 fps**; a bare
GTK tick callback measures **60–67 fps**. Dioxus by default forces
`GDK_BACKEND=x11` and `WEBKIT_DISABLE_DMABUF_RENDERER=1` on Wayland; source now sets
`with_disable_dma_buf_on_wayland(false)` in both the app and QA driver, so the app
uses the native Wayland surface with DMABUF. This removes the app-imposed XWayland/
software path, but the remaining ~72 fps cap reproduces outside Dioxus and cannot be
raised by app CSS/code: it is in the GTK3/WebKitGTK compositor stack. Reaching true
144 Hz requires a GTK4/WebKitGTK 6 host (not supported by current Dioxus desktop) or
a Chromium-based shell. `WEBKIT_FORCE_COMPOSITING_MODE`, `WEBKIT_DISABLE_DMABUF_RENDERER=0`
and vblank env variants produced the same 72.3 fps. Full native peer/session QA
passes with the Wayland flag (`target/checks/wayland-dmabuf/`). No claim of 144 Hz.

## Native chrome cleanup — verified (2026-10-19)

Removed the GTK menu bar (`Config::with_menu(None)`) so no Window/Edit/Help strip
remains above the app, and deleted the visible `.native-player-status` row,
preparation progress/bytes, "Preparing original audio", offline-check copy and the
"Opening public track…" notice. The player now shows only track/controls/seek/volume;
errors remain visible. Source context is kept as a `hidden` element for the native QA
harness and screen-reader state, not painted. A→B→A cache hits no longer surface any
loading/log text; whole-file preparation still disables transport silently while it
runs. Rebuilt `target/release/vela-desktop`; native peer/session/offline QA passed
(`target/checks/ui-clean-player5/`). Wide native visual inspection remains NOT_RUN.

## Native wide-window layout + inline forms — CSS fixed (2026-10-19)

The 2288×937 native screenshot showed a large empty left band and stacked,
touching folder/playlist controls. Cause: `.native-content` combined
`max-width:1600px;margin:auto` with `padding-left:calc(sidebar+40px)`, so the
centered 1600 box plus the sidebar inset produced a double offset; and native
`inline-form` had no flex/gap rules at all (web-only), so inputs and buttons
stacked flush. Fix: native and web content are now left-anchored (`max-width:none;margin:0`) and
native `.inline-form` mirrors the web flex row with `gap:9px` and 12px bottom
spacing. The same double-offset existed in web `.main-content` (sidebar padding +
`margin:auto`) and was removed. `target/release/vela-desktop` rebuilt. A Playwright audit of the user account at 320×640, 375×812, 768×1024,
1280×800, 1920×1080, 2288×937 and 3440×1440 over Home/Search/Library/Albums/
Artists/Favorites/Playlists/Listening history/Jobs/Network/Settings found zero
horizontal overflow or page errors (77 combinations) against an isolated restore
of the user data with the rebuilt assets; screenshots in
`target/checks/ui-audit/`. Wide-window NATIVE visual verification is
NOT_RUN (the existing native QA window is 1280×840 and no credentials were used
to open the signed-in screen).

## Browser gate restored: wasm-bindgen CLI on PATH (2026-10-21)

The web build and the real-browser suite could not run because no
`wasm-bindgen` CLI was reachable: `scripts/build-web.sh` exits when
`$VELA_WASM_BINDGEN` / `wasm-bindgen` is missing, and the pinned copy the
docs referenced (`target/tools/wasm-bindgen-0.2.129-x86_64-unknown-linux-musl/`)
was absent — only the project's `target/tools/wasm-bindgen-0.2.129/bin/`
(built for the host) existed.

Installed `wasm-bindgen-cli 0.2.129` with `cargo install
wasm-bindgen-cli --version 0.2.129 --locked`, matching `Cargo.lock` exactly
(the version is load-bearing: a mismatched CLI refuses the generated wasm).
`/root/.cargo/bin/wasm-bindgen --version` reports `0.2.129`, identical to the
project-pinned binary.

Executed, not claimed:

- `VELA_WEB_PROFILE=dev scripts/build-web.sh` → `dist/` populated.
- `VELA_WEB_PROFILE=release scripts/build-web.sh` → release wasm
  3,632,887 B in `dist/pkg/`.
- `cd tests/browser && npx playwright test` → **60 passed, 1 skipped, 0 failed
  in 5.7m** on the release build. The single skip is the opt-in
  `VELA_LIVE_IMPORT_TEST` network case; the coarse-pointer skip is
  engine-dependent, not a failure.

The earlier debug-build run showed one real failure
(`music.spec.mjs` "first-run setup, real FLAC upload, …"): after
"Clear upcoming tracks" the drawer kept 2 rows instead of 1. It is a
flake, not a queue-contract regression: the browser fixture
`crates/media/tests/fixtures/signal.flac` is 1.0 s long, so the
occurrence can reach its end and auto-advance while the test is still
asserting, moving the cursor past the row the assertion assumes. The
same test passes on the release build (60/61) and on a debug build when
run on its own, which is the expected signature of a timing race rather
than a deterministic defect. The 1-second fixture stays a latent flake
source for queue assertions and is worth replacing with a longer fixture.

`dist/` is git-ignored, so this change adds no tracked artifacts.

## The stall guard no longer mistakes a clock jump for a dead stream (2026-10-21)

`music.spec.mjs` "sleep timer pauses the shared player and can be canceled"
failed consistently: after the sleep timer advanced the fake clock by four
minutes the player was paused while the test still expected it to play.

Cause: the browser stall guard measured the stall with the wall clock
(`Date::now() - progress_at >= 7900`). Playwright's `page.clock` replaces
both `setTimeout` and `Date.now()` **and** `performance.now()`, so advancing
the clock by four minutes fired the 8-second guard immediately and the
guard's own age test passed — the guard called `recover()` and paused a
stream that was perfectly alive.

A wall-clock jump is not evidence of a stall. The same is true for a device
that was suspended, a corrected system clock, or a resumed tab. The guard now
reads the media element instead: the element must have no error, must not be
at `HAVE_NOTHING`, and must have no buffered data past the current position.
A live element that still holds media is never "stalled", whatever the clock
says. This also matches the project rule that real playback progress is
corroborated by the media element, not by a self-reported timer.

`web-sys` gained the `TimeRanges` feature for `HTMLMediaElement.buffered`.

Executed, not claimed: `cargo fmt --check`; strict `cargo clippy -p vela-web
--all-targets -- -D warnings`; strict wasm32 clippy; `cargo test -p vela-web`
(45 passed); release web build; full Chromium Playwright suite.
