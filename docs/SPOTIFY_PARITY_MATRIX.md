# Music experience parity

## Public catalog0.11 — sourcefe0484d, exact package verified2026-10-18

| Feature / priority | Status | Exact implementation | Acceptance / tests | Platform and rights scope |
| --- | --- | --- | --- | --- |
| Explicit origin and per-track publication P0 | VERIFIED for documented source scope | domain/network.rs; core/network.rs; media/publications.rs; migration0011; server/network.rs | Packaged Chromium39/Firefox39, PN01; API ownership/CSRF/CAS/grants, legacy10 preservation, no auto publication | Native server + web; supported declared CC0/BY/BY-SA only, not legal verification |
| Public catalog preview / authorized media P0 | VERIFIED for read-only source profile | server/network.rs + network/tests.rs; imports/egress.rs; web/network.rs | 200 Rust; actual independent A/B HTTP, malicious/slow peer, DNS/redirect/size limits, Range/HEAD/download/revoke; packaged Chromium39/Firefox39, PN02 | Read-only profile1; no AP/signatures/follows/cache/unified remote queue; latest G11/G14/G15/G22 subsets |
| Durable local supervision P1 | VERIFIED for same-build local scope | scripts/install-user-service.py; tests/test_user_service.py; docs/USER_SERVICE.md |22 Python tests incl13 installer; actual frozen0.10 install, original10→11 upgrade and frozen0.11 validation-only recheck | Linux user systemd enabled, private loopback; no root/linger/reboot or native audio claim |

Next actual experience gaps: standards-based signed/blocked peers and saved origin
adapters; native Linux/Android playback and offline manifests; Apple toolchain and
native bridges; device handoff; room authorization; scheduled/calibrated mixes;
public provenance moderation; richer catalog editions/organization; accessibility;
large public-catalog/concurrent streaming measurement. Do not mark the entire
network or platform milestone verified from this first profile.

## Personal discovery checkpoint0.10 — source902dd47, verified2026-10-18

| Feature / priority | Status | Exact implementation | Acceptance / evidence | Platform / caveat |
| --- | --- | --- | --- | --- |
| Private Home exploration / rotation P0 | VERIFIED for documented web/server scope | crates/media/src/recommendations.rs; apps/web/src/personal_mixes.rs; domain/src/recommendations.rs; server/src/recommendations.rs |176 Rust tests and strict native/WASM Clippy; exact package Chromium37/Firefox37; PC01/PC02 and G07 | Native server + web only; on-demand owned catalog, max500 candidates/12anchors, no scheduled/network model |
| Reversible recommendation exclusions P0 | VERIFIED for documented web/server scope | migrations/0010_recommendation_exclusions.sql; same media/server/web modules | Owner FK, serialized1000 limit, desired-state hide/undo/clear, UUID-only export; real TCP and concurrency tests | Separate from history erase, no change to media rights/favorites/playlists |
| Query scaling P0 | VERIFIED within recorded synthetic scope | crates/media/examples/benchmark_recommendations.rs | Preserved baseline + actual release10k/100k/1m; output projection equivalence | Synthetic metadata, unique timestamps and small history; no human relevance/stream capacity claim |

The earlier ten-gap inventory is historical. Personal on-demand shelves and feedback
are implemented in this slice; durable daily/weekly refresh, calibrated quality,
large-history/tied-time profiling, lawful two-server publication/rights and native
clients remain ordered work. PROGRESS records exact verified source/package/runtime.

## Current mandate audit2026-10-17

Baseline was re-established at main f6cc845/application ac31d74:110 Rust tests,33 Chromium scenarios; actual loopback0.8.1 supervised runtime and20-track original library, fresh private backup/restore. New0.9 working tree evolves the same seven crates. New G01–G24 IDs are the latest mandate; older G numbers below remain historical. No native/federated completion is implied.

| Feature / priority | Status | Exact implementation | Acceptance | Platform / caveat |
| --- | --- | --- | --- | --- |
| Private listening foundation P0 | VERIFIED for web/native server scope | core/src/listening.rs; domain/src/listening.rs; migration0008; server/src/listening.rs; web/src/audible.rs,listening.rs,browser_player.rs | G07-private-history: consent/global+device, actual played time, repeat/EOF, owner/CAS/dedup, export/delete/stale suppression | Browser self-report; no public popularity or physical sound attestation |
| DSP and durable analysis P0 | VERIFIED for web/native server scope | media/src/analysis.rs,analysis_service.rs; domain/src/audio_analysis.rs; migration0009; server/src/analysis.rs; web/src/radio.rs | G08: known real PCM/FLAC/MP3, finite spectrum, bounded workers/recovery, original preservation/owner ACL | Explicit local analysis,120s prefix; no genre/mood/tempo/LUFS claims |
| Explained local track radio P0 | VERIFIED for web/native server scope | media/src/discovery.rs; web/src/radio.rs; root Actions | G09: real seed/candidates, measured distance/tag fallback, diversity, actual queue playback |500 newest owned candidates; personal recurring mixes absent |
| Lawful public network P0 | PARTIAL:0.11 source under verification | server/network.rs; media/publications.rs; web/network.rs | G11/G14/G15/G22 subsets; actual two-node/hostile/rights/revoke and PN01/PN02 | Read-only opt-in public profile; full social federation remains unfinished |
| Native audio clients P0 | NOT_STARTED / Apple validation BLOCKED | docs/PLATFORM_MATRIX.md only | G16/G17/G18 real Media3/AVFoundation/desktop engines/device evidence | Browser WASM/native server are not native clients |

Historical0.9 ten-gap inventory (current0.10 implementation/evidence above supersedes the personal baseline entry):

1. P0 personal Home/mixes: core listening aggregates + media discovery ranking and web/experience.rs; G04/G10 consented real taste/favorites, sparse/cold-start, exclusions and durable refresh.
2. P0 DSP completeness/quality: media/analysis.rs,analysis_service.rs,discovery.rs; G08/G09 evaluate real music relevance, optional validated tempo/chroma, broader lawful codecs, large-catalog candidate index and contention.
3. P1 history/analysis management: web/listening.rs,radio.rs and core/jobs.rs; G07/G08 device-grant removal, analysis cancel/task-center projection, accurate failure/reconnect UX and browser privacy-race expansion.
4. P0 lawful origin/publication model: core policy/domain/migrations + server discovery; G11/G22 explicit rights, provenance, private-by-default and revocation.
5. P0 native Linux/Android engines: future platform bridges and shared playback contracts; G16/G18 actual background/media keys, codec/audio-focus validation.
6. P0 iOS/macOS toolchain and native implementation: platform/ios/AVFoundation plus Dioxus target features; G17/G18 actual Apple build/device evidence, currently blocked for validation.
7. P1 multi-server/native catalog adapters: client transport/domain origin IDs; G12/G13 bounded fanout and authorized cross-origin queue, slow/offline peers.
8. P1 offline/device handoff: durable manifests/journal/device sessions; G13/G19 real offline playback/hash/rights, two-client accepted transfer and no duplicate commands.
9. P1 social/media grants/rooms: core/playlists and future authenticated events; G06/G20 actual invitations, separate metadata/stream/download grants and authorized synchronized clients.
10. P1 complete UX/operational hardening: web/shell, context actions, large lists, a11y/en/ru; deploy/scripts/CI; G21/G23/G24 measured contention, full keyboard/screen-reader, restore/container/native matrices.


## Historical UX matrix

Audit2026-10-16: actual main HEADb7033e4, version0.4.0; clean initial tree. Baseline rerun:97 Rust tests,16 real Chromium scenarios including the opted-in licensed import, fmt/strict native+WASM Clippy and release builds passed. Existing development data contains one account and was backed up/restored separately. Status applies to the described scope, not the whole reference product. VERIFIED requires executed evidence; PARTIAL means usable subset or current implementation awaiting checks; NOT_STARTED means absent; BLOCKED means an actual external dependency prevents progress. All native desktop/mobile clients below are NOT_STARTED; a tested Linux server is not a desktop client.

Current0.8 increment (2026-10-17): root Now Playing verified with110 Rust tests,33 Chromium scenarios, native/WASM strict checks and responsive images; detailed evidence and remaining platform limits are in PROGRESS. The audit paragraph above is the preserved starting baseline, not the current version.

| Feature / priority | Status | Existing/proposed location | Acceptance / tests | Platform / rights limits |
| --- | --- | --- | --- | --- |
| Setup/auth/private accounts P0 | VERIFIED | core/auth.rs; server/lib.rs; web/main.rs | G01, TCP auth/CSRF/ownership; browser01 | Chromium web; no public registration |
| Consumer Home P0 | VERIFIED | web/experience.rs (0.5) | EXP01/G02: actual additions/favorites/albums/playlists, cold start | Owned catalog; no invented recents/recommendations |
| Separate multi-kind Search P0 | VERIFIED | media/search.rs; core/playlists.rs; server; web/experience.rs | EXP02/G03: bounded tracks/albums/artists/granted playlists;2 TCP + browser16/17/19 | Local only; literal ASCII group names/FTS tracks |
| Shared shell/navigation P0 | PARTIAL | web/main.rs, shell.rs, icons.rs, style.css | EXP03/G02/G13: persistent audio, keyboard/phone/theme | Original Vela SVG/branding; five mobile tabs + More; broader shell still partial |
| Browser playback P0 | PARTIAL | web/main.rs; player.js | G01/G04: FLAC/MP3/Ogg decode/seek; EXP03 navigation continuity | HTMLMediaElement, actual browser codec support |
| Play next / add last P0 | VERIFIED | web/main.rs; experience.rs | EXP04: occurrence-aware insertion, no interruption | Bounded browser queue; no handoff |
| Fair occurrence order P0 | VERIFIED | web/playback.rs; root PlaybackUi | PLAY01/02:8 unit tests + playback E2E + mode isolation | Chromium/Firefox audio; transient history, no native proof |
| Queue/repeat/shuffle P0 | PARTIAL | web/main.rs | Existing duplicate/reorder E2E; EXP04;0.12 move-down/clear-upcoming/first-page browser assertions | 0.6 bounded fair history/priority model; no drag/drop/save-queue/handoff |
| Browser resume/reconnect P0 | VERIFIED | web/session.rs, browser_player.rs, main.rs, player.js | RES01–04/NET01–02/AUTH01,3 unit policy tests +7 real browser cases | Paused per-account bookmark; bounded streaming retry; no offline/native/device sync |
| Library/filters/favorites P0 | VERIFIED | media/filters.rs; web/filters.rs/main.rs | Typed filter/TCP; browser11–13 |50-item pages; no full-library virtualization |
| Albums/artists P0 | PARTIAL | media/catalog.rs; web/main.rs | G04; existing group/race E2E | Provisional groups; visible-page playback; no editions/biographies |
| Playlist create/rename/delete/share P0 | PARTIAL | core/playlists.rs; web/main.rs | G05; revision/idempotency/revoke TCP/E2E;0.12 owner CAS rename/delete + confirmation + move-down; Chromium/Firefox40 | Named local grants; metadata never grants audio; no folders/smart rules/owner transfer |
| Upload/URL/folder ingest P0 | VERIFIED | media/imports/scanner; web modules | G01/G10; real audio/SSRF/recovery | Direct authorized files only; no DRM extraction |
| Jobs/progress P1 | VERIFIED | core/jobs.rs; web/jobs.rs | Existing mixed-page/TCP/browser14/15/30 | Owned jobs; no SSE or global admin queue |
| Metadata/artwork P1 | PARTIAL | media/metadata.rs; web/metadata.rs | Existing unchanged-source/revision tests | No write-back/bulk/enrichment/recording matching |
| Context menus/info P1 | PARTIAL | web/experience.rs (0.5) | EXP04 actual next/last/favorite handlers and technical details; general context menus remain partial | Actual local fields; unavailable actions omitted |
| Full Now Playing P0 | VERIFIED | web/now_playing.rs, now-playing.css, root main.rs, player.js | NP01–03/G02/G13:0.8.1 queued duration/11s pre-load seek/resume, real audio continuity, modal focus, transport/Media Session, metadata/favorites/preview, stale replies; NET02/AUTH01 expiry/recovery | Chromium + Playwright Firefox157/Linux; phone full-height/desktop modal; reported source properties, no native certification |
| Sleep timer P2 | VERIFIED | web/main.rs; tests/browser/music.spec.mjs |0.12 root 5/15/30/60 countdown/cancel; real audio still playing pre-expiry, paused+status at zero, restart/cancel stays playing; Chromium/Firefox41 | Browser clock only; no OS sleep/handoff |
| Persistent right Now Playing/context panel P1 | NOT_STARTED | future shared shell view | G02 stable panel sizing/context navigation | Full modal and persistent bar exist |
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
| Accessibility/cross-browser P0 | PARTIAL | web/style.css/player.js; browser tests | G13 keyboard,320px,light/dark/reduced motion | Screen-reader/Safari/native unverified;0.8 adds Firefox157/Linux headless |

## Ten current experience gaps, in order

The baseline Home/local Search/shell, stale-play guard, occurrence order browser resume/recovery and root Now Playing increments are now implemented to the verified scopes above. Keep their EXP/PLAY/RES/NET/AUTH regressions at every merge. The remaining prioritized gaps are:

1. P0 broader playback/accessibility verification — apps/web/src/browser_player.rs, player.js and tests/browser; G07/G13: WebKit/Safari, stock OS/browser variants, keyboard/screen-reader audit, pending seek/expiry/reconnect and large-queue rendering. Current headless Chromium/Firefox evidence does not certify these targets.
2. P1 private listened-time foundation — proposed crates/core/src/listening.rs, domain DTOs/migration and root player; G07/G14: consent/opt-out, deduplicated meaningful duration, export/delete, owner isolation and recovery.
3. P1 personalized Home/radio/mixes — proposed crates/media/src/discovery.rs and apps/web/src/experience.rs; G06: sparse-library deterministic ranking/diversity/reasons from actual accessible IDs, no fake history.
4. P1 richer catalog/navigation — crates/media/src/catalog.rs, crates/domain/src/lib.rs and web/main.rs; G04: edition/recording/contributor identity, full-context pagination, technical variants and responsive large-list behavior.
5. P1 playlist organization/actions and media sharing — crates/core/src/playlists.rs, web/main.rs/experience.rs; G05: folders/pins/cover/context menus, queue save/drag order, revisioned collaboration, invitations and separately revoked media grants.
6. P1 device registry/handoff and native desktop — proposed playback-session service/WS and desktop adapter; G08: two real clients, exact authorization, sequence/idempotency, accepted transfer before stopping origin and Linux packaging/media keys.
7. P1 mobile/offline — proposed platform/android and platform/ios bridges plus typed local manifests; G09: actual native background service, hashes/quotas/resume, airplane-mode audio and secure credentials. Dioxus WASM is not native validation.
8. P1 multi-server/federation/rooms — proposed catalog adapters and authorized transports; G11/G12: two instances/clients, explicit origin IDs/rights, bounded search, remote removal and private-by-default signed discovery/control.
9. P1 complete track context and navigation — apps/web/src/now_playing.rs, experience.rs and main.rs; G02/G04: accessible general context menus, artist/album navigation, credits when known and optional right panel with shared state. Current full player deliberately omits unsupported lyrics/devices/effects.
10. P1 interoperability and release hardening — proposed OpenSubsonic transport, lyrics/statistics/export services, deploy/CI/docs; real client contracts, authorized lyrics, accurate private summaries, backup restore, container/ARM/native matrix and remaining license-notice review.

G01/G04/G05/G10/G14 have verified existing subsets, not complete golden-demo acceptance. G02/G03/G13 are the current0.5 increment. G06/G08/G09/G11/G12 remain NOT_STARTED. Native Vela features are separated above from future authorized integrations; missing remote media must never be silently substituted. No experimental or unavailable feature receives a decorative active button.

Executed0.5 evidence:99 Rust tests,20 Chromium scenarios including licensed live import, strict native/WASM Clippy, release builds,9 Python tooling tests and20 clean restarts plus both worker-failure checks. Browser16 covers EXP01/03/04,17 covers EXP02 query ordering/physical abort,18 covers EXP05 old play rejection,19 covers private search after account change. Eight reviewed Home screenshots cover320/768/1440/1920px and both themes. These verify bounded increments, not complete G02/G03/G13 or cross-browser/native parity. No schema change (version7); fresh existing-data backup and fixture restore passed.

Playback0.6 acceptance IDs: PLAY01 tests fair occurrence permutations, repeat-one EOF/manual-next separation, repeat-all full cycles/boundaries, previous seek/history and bounded state. PLAY02 tests real queue priority/remapping under edits, duplicate rows, cursor restore and account-mode reset. Queue breadth remains PARTIAL until save/drag-drop/durable context/recovery and native validation exist. Current browser scenarios20/21 cover these real audio/queue contracts;19 covers account-mode isolation;22 retains the private job-response regression.

Executed0.6 evidence:107 Rust tests (the prior99 plus8 playback-model tests),22 Chromium scenarios including the licensed import, final native/WASM strict Clippy and release builds passed. PLAY01/02 are VERIFIED to this browser/pure-model scope. Initial expanded browser runs exposed fixture assumptions (first-page-only track lookup; toggling transport before any queue existed); tests were corrected to search/start real audio while preserving assertions, and the complete22-case rerun passed. No server schema/data semantics changed.

Playlist response follow-up0.6.1: PLNAV01 is browser20; PLAY01/02 are now21/22, and the private Jobs regression is23. The regression was first run against0.6.0 and reproduced an abandoned detail opening after route return. The patch binds playlist detail/refresh/share UI to session/object/view generations without changing permissions, media rights or schema. Current status/evidence is recorded in PROGRESS after the full rerun.

Final0.6.1 evidence:107 Rust tests and23 real Chromium scenarios including PLNAV01 passed; strict native/WASM Clippy and release builds passed. PLNAV01 is VERIFIED for detail navigation, committed-delete refresh, monotonic same-view revision and share-token presentation across selection changes. Broader social/media-grant/native features remain PARTIAL/NOT_STARTED. The existing account and20 tracks were backed up/restored and retained under schema7; active local assets/binary are now a frozen verified snapshot.

Browser0.7 acceptance: RES01 covers duplicate cursor, queue remapping, real Search provenance, position/preferences and no autoplay/no pre-Play stream; RES02 corrupt/foreign/oversized storage; RES03 actual stalled metadata, watchdog/HEAD recovery and late transfer; RES04 full authorized playlist context at the stable duplicate entry. NET01 aborts a genuine transfer and resumes at the saved seek; NET02 holds a genuine HEAD across pause/new selection, checks the fixed three-attempt budget and manual retry at320px in both themes. AUTH01 revokes a real dedicated session and verifies source/private UI removal. Existing PLAY/PLNAV/EXP and import/share/security tests remain. These are G07 browser subsets, not offline/device/native certification.

Final0.7 evidence:110 Rust tests,30 isolated real Chromium scenarios including the opted-in licensed live import, fmt, strict native/WASM Clippy and native/WASM release builds. Three policy tests cover snapshot bounds/identity/encoding/seek and retries. PROGRESS records exact logs, the corrected initial recovery panic, responsive review and runtime/package checkpoint. History/radio/Now Playing/native/offline/federation are not implied.

Follow-up0.7.1 extends RES01 to sequential Previous on a paused restored duplicate: it must seek the current bookmark to0 without loading/playing another row, then accept a pending seek before Play. Audio-clock selection now uses the active clock only when source/session/element metadata are valid, otherwise the bookmark. The0.7.0 archive is retained as a local review checkpoint; PROGRESS records the regression and final patch evidence.
