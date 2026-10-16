# Music experience parity

Audit2026-10-16: actual main HEADb7033e4, version0.4.0; clean initial tree. Baseline rerun:97 Rust tests,16 real Chromium scenarios including the opted-in licensed import, fmt/strict native+WASM Clippy and release builds passed. Existing development data contains one account and was backed up/restored separately. Status applies to the described scope, not the whole reference product. VERIFIED requires executed evidence; PARTIAL means usable subset or current implementation awaiting checks; NOT_STARTED means absent; BLOCKED means an actual external dependency prevents progress. All native desktop/mobile clients below are NOT_STARTED; a tested Linux server is not a desktop client.

| Feature / priority | Status | Existing/proposed location | Acceptance / tests | Platform / rights limits |
| --- | --- | --- | --- | --- |
| Setup/auth/private accounts P0 | VERIFIED | core/auth.rs; server/lib.rs; web/main.rs | G01, TCP auth/CSRF/ownership; browser01 | Chromium web; no public registration |
| Consumer Home P0 | VERIFIED | web/experience.rs (0.5) | EXP01/G02: actual additions/favorites/albums/playlists, cold start | Owned catalog; no invented recents/recommendations |
| Separate multi-kind Search P0 | VERIFIED | media/search.rs; core/playlists.rs; server; web/experience.rs | EXP02/G03: bounded tracks/albums/artists/granted playlists;2 TCP + browser16/17/19 | Local only; literal ASCII group names/FTS tracks |
| Shared shell/navigation P0 | PARTIAL | web/main.rs, shell.rs, icons.rs, style.css | EXP03/G02/G13: persistent audio, keyboard/phone/theme | Original Vela SVG/branding; five mobile tabs + More; broader shell still partial |
| Browser playback P0 | PARTIAL | web/main.rs; player.js | G01/G04: FLAC/MP3/Ogg decode/seek; EXP03 navigation continuity | HTMLMediaElement, actual browser codec support |
| Play next / add last P0 | VERIFIED | web/main.rs; experience.rs | EXP04: occurrence-aware insertion, no interruption | Bounded browser queue; no handoff |
| Fair occurrence order P0 | VERIFIED | web/playback.rs; root PlaybackUi | PLAY01/02:8 unit tests + playback E2E + mode isolation | Chromium audio; transient history, no native proof |
| Queue/repeat/shuffle P0 | PARTIAL | web/main.rs | Existing duplicate/reorder E2E; EXP04 | 0.6 bounded fair history/priority model; no drag/drop/save-queue/handoff |
| Playback resume/reconnect P0 | PARTIAL | web/main.rs | G07 future position/network recovery tests | Queue persists by account; exact position not implemented |
| Library/filters/favorites P0 | VERIFIED | media/filters.rs; web/filters.rs/main.rs | Typed filter/TCP; browser11–13 |50-item pages; no full-library virtualization |
| Albums/artists P0 | PARTIAL | media/catalog.rs; web/main.rs | G04; existing group/race E2E | Provisional groups; visible-page playback; no editions/biographies |
| Playlist create/share/collaboration P0 | PARTIAL | core/playlists.rs; web/main.rs | G05; revision/idempotency/revoke TCP/E2E | Named local grants; metadata never grants audio |
| Upload/URL/folder ingest P0 | VERIFIED | media/imports/scanner; web modules | G01/G10; real audio/SSRF/recovery | Direct authorized files only; no DRM extraction |
| Jobs/progress P1 | VERIFIED | core/jobs.rs; web/jobs.rs | Existing mixed-page/TCP/browser14/15/23 | Owned jobs; no SSE or global admin queue |
| Metadata/artwork P1 | PARTIAL | media/metadata.rs; web/metadata.rs | Existing unchanged-source/revision tests | No write-back/bulk/enrichment/recording matching |
| Context menus/info P1 | PARTIAL | web/experience.rs (0.5) | EXP04 actual next/last/favorite handlers and technical details; general context menus remain partial | Actual local fields; unavailable actions omitted |
| Full-screen/right Now Playing P1 | NOT_STARTED | web future playback view | G02 layout/state sync | Persistent bottom bar/queue exist |
| History/consent/export P1 | NOT_STARTED | future listening service/migration | G07/G14 event dedup, privacy/delete/export | No fabricated played minutes |
| Deterministic recommendations/radio P1 | NOT_STARTED | future media application service | G06 sparse library/reasons/authorization | Accessible files only; no paid AI/catalog |
| Weekly/daily/rotation mixes P1 | NOT_STARTED | future durable listening jobs | G06 real refresh/materialization/diversity | Requires real history/consent first |
| Playlist folders/pins/smart rules P1 | NOT_STARTED | core playlists future | Typed rules/revision/move tests | No unrestricted SQL |
| Playlist invites/media grants P1 | NOT_STARTED | core sharing future | G05 expiry/revoke and separate audio tests | Existing links are metadata-only |
| Profiles/follows/presence P1 | NOT_STARTED | future social core | Two-account privacy tests | Explicit visibility/consent required |
| Listening rooms P1 | NOT_STARTED | future authorized WebSockets | G12 two-client reconnect/rights | Synchronize commands; no unauthorized rebroadcast |
| Devices/handoff P1 | NOT_STARTED | future playback sessions | G08 two clients/accept-before-stop/idempotency | Independent sessions preserved |
| Offline downloads/sync P1 | NOT_STARTED | future client manifest/native adapters | G09 disconnected actual playback/integrity | Download permission and secure tokens required |
| Linux desktop P1 | NOT_STARTED | future Dioxus desktop target | Build/package/media-key runtime test | Current native artifact is server only |
| Windows/macOS desktop P1 | NOT_STARTED | future platform CI | Actual target builds/runtime matrix | No runner/hardware validation yet |
| Android/iOS P1 | NOT_STARTED | future Media3/AVFoundation bridges | G09 physical background/lock-screen tests | SDK/entitlements/device validation required |
| Multi-server/OpenSubsonic P1 | NOT_STARTED | future client adapters | G11 two real servers/origin IDs | OpenSubsonic is not federation |
| Optional federation P1 | NOT_STARTED | future ActivityPub profile | G11 remote authorization/replay/SSRF | Disabled; no automatic library publication |
| Lyrics/LRC P2 | NOT_STARTED | future metadata/player view | Timing/fallback/authorized-provider tests | Embedded/user-authorized text only |
| ReplayGain/transcode/quality P1 | NOT_STARTED | future bounded media adapter | Actual negotiated-codec/CPU/cancel tests | Originals stream directly today |
| Gapless/crossfade/EQ P2 | NOT_STARTED | future platform audio adapters | Measured audio correctness per platform | Never inferred from WASM compile |
| Listening statistics/yearly story P2 | NOT_STARTED | future listening aggregate service | Real listened-time/dedup/private-share tests | No global popularity/trending counters |
| ZIP/WebDAV/S3/podcasts P2 | NOT_STARTED | future imports adapters | Bounded trusted-storage/archive tests | Separate public egress policy; legitimate assets |
| Backup/restore/deployment P0 | PARTIAL | scripts/backup.py; deploy/; Dockerfile | G14;9 Python regressions; extracted packages | Local Linux verified; Docker daemon unavailable at baseline |
| Accessibility/cross-browser P0 | PARTIAL | web/style.css/player.js; browser tests | G13 keyboard,320px,light/dark/reduced motion | Screen-reader/Firefox/Safari/native unverified |

## Ten largest experience gaps, in order

1. P0 listening-oriented Home (bounded0.5 scope verified; personalization next) — web/main.rs → experience.rs; EXP01 real cards/cold start, G02.
2. P0 all-category Search (four local categories verified in0.5; full/remote search next) — media/catalog.rs/core/playlists.rs → search API/page; EXP02 owner/grant bounds and delayed queries, G03.
3. P0 old play-promise callback race (fixed/verified in0.5) — web/main.rs start_audio; EXP05 reject an older play after a successful newer selection/logout.
4. P0 navigation/mobile shell (Home/Search/five-tab increment verified in0.5) — web/main.rs/style.css; EXP03 five phone tabs plus keyboard More sheet, G13.
5. P0 queue: next/last verified in0.5; fair shuffle/previous/repeat order model added in0.6 (PLAY01/02); exact resume/recovery next — web/main.rs; EXP04 next/last/duplicates, then state-machine property tests, G07.
6. P0 resume and offline-network errors lack explicit recovery — web/main.rs/player.js; G07 server restart/seek/session tests before new engines.
7. P1 no private listened-event foundation or mixes — crates/core/src/listening.rs and crates/media/src/discovery.rs (proposed), web/experience.rs; G06 deterministic sparse-library radio, history opt-out and dedup first.
8. P1 no complete Now Playing/lyrics context — web/main.rs; EXP06 root-coordinated desktop/mobile view and real technical fields.
9. P1 no secure device/offline client layer — web/client.rs, proposed platform/android and platform/ios bridges; G08/G09 real devices, persisted manifests, scoped credentials.
10. P1 sharing is metadata-only and network clients absent — core/playlists.rs and future adapters; G05 separate grant tests, G11/G12 two instances/rooms.

G01/G04/G05/G10/G14 have verified existing subsets, not complete golden-demo acceptance. G02/G03/G13 are the current0.5 increment. G06/G08/G09/G11/G12 remain NOT_STARTED. Native Vela features are separated above from future authorized integrations; missing remote media must never be silently substituted. No experimental or unavailable feature receives a decorative active button.

Executed0.5 evidence:99 Rust tests,20 Chromium scenarios including licensed live import, strict native/WASM Clippy, release builds,9 Python tooling tests and20 clean restarts plus both worker-failure checks. Browser16 covers EXP01/03/04,17 covers EXP02 query ordering/physical abort,18 covers EXP05 old play rejection,19 covers private search after account change. Eight reviewed Home screenshots cover320/768/1440/1920px and both themes. These verify bounded increments, not complete G02/G03/G13 or cross-browser/native parity. No schema change (version7); fresh existing-data backup and fixture restore passed.

Playback0.6 acceptance IDs: PLAY01 tests fair occurrence permutations, repeat-one EOF/manual-next separation, repeat-all full cycles/boundaries, previous seek/history and bounded state. PLAY02 tests real queue priority/remapping under edits, duplicate rows, cursor restore and account-mode reset. Queue breadth remains PARTIAL until save/drag-drop/durable context/recovery and native validation exist. Current browser scenarios20/21 cover these real audio/queue contracts;19 covers account-mode isolation;22 retains the private job-response regression.

Executed0.6 evidence:107 Rust tests (the prior99 plus8 playback-model tests),22 Chromium scenarios including the licensed import, final native/WASM strict Clippy and release builds passed. PLAY01/02 are VERIFIED to this browser/pure-model scope. Initial expanded browser runs exposed fixture assumptions (first-page-only track lookup; toggling transport before any queue existed); tests were corrected to search/start real audio while preserving assertions, and the complete22-case rerun passed. No server schema/data semantics changed.

Playlist response follow-up0.6.1: PLNAV01 is browser20; PLAY01/02 are now21/22, and the private Jobs regression is23. The regression was first run against0.6.0 and reproduced an abandoned detail opening after route return. The patch binds playlist detail/refresh/share UI to session/object/view generations without changing permissions, media rights or schema. Current status/evidence is recorded in PROGRESS after the full rerun.

Final0.6.1 evidence:107 Rust tests and23 real Chromium scenarios including PLNAV01 passed; strict native/WASM Clippy and release builds passed. PLNAV01 is VERIFIED for detail navigation, committed-delete refresh, monotonic same-view revision and share-token presentation across selection changes. Broader social/media-grant/native features remain PARTIAL/NOT_STARTED. The existing account and20 tracks were backed up/restored and retained under schema7; active local assets/binary are now a frozen verified snapshot.
