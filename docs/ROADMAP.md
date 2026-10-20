# Authoritative backlog

## Distribution pipeline2026-10-20

Distribution0.12.7 adds the required exe/zip, Linux tar.gz/deb/AppImage and Android APK pipeline plus real makepkg validation. AUR publication needs the owner account/key; stable APK update signing needs a keystore. Hardware/device/background, macOS/iOS packaging and store submissions remain separate. See [DISTRIBUTION](DISTRIBUTION.md).

## Approved prototype transfer 2026-10-19 — documentation only

The [prototype transfer package](PLANS/2026-10-19/PROTOTYPE_TRANSFER/README.md)
turns the user's selected design into a portable English implementation brief for
DeepSeek or another harness. TR00–TR12 cover actual-source reconnaissance, collision-
safe roles/fonts/assets, shell/player, real Home/catalog/playlists, overlays/Settings,
mobile/native adaptation, regression review and local packaging. PT01–PT48 are future
acceptance scenarios, all NOT_RUN by this planning task.

Use the approved charcoal/pistachio/Manrope composition instead of reopening visual
direction. Preserve the prior Interaction Playbook's useful behavior and existing
artwork/scale/skeleton source work. Do not port the prototype's shared demo recording,
in-memory playlists, track-ID-deduplicated queue or play-event history. The next
authorized implementation starts TR00→TR01/TR02→TR03, with real playback continuity
and rendered evidence before claiming a completed transfer.

## Interactive listening refinement 2026-10-19 — proposed, documentation only

The [Interaction Playbook](PLANS/2026-10-19/PRODUCT_EXPANSION/INTERACTION_PLAYBOOK/README.md)
adds96 granular IX briefs and 11 IN delivery packages to the existing product plan.
It refines overlapping EX outcomes; it is not a second independent feature-count
backlog or evidence that the proposed interactions work.

First complete slice: IN00 real-source/rendered baseline → IN01 geometry-matched
catalog/art skeletons with honest pending/refresh/error states + IN02 single-track
right-click/More/keyboard commands → IN03 actual playlist picker and safe receipts/
Undo. Apply IN10 motion/accessibility/platform review at every integration. Then
selection/drag/queue, listening shelf/time planner, catalog/mix peeks and supported
offline/privacy/share/device interactions under their accepted prerequisites.

Reconciliation: native artwork (`0f9f12b`), web appearance (`1618e63`) and native
appearance (`eb36048`) are already implemented source foundations. Actual scale
bounds are80–150%; scoped native art QA was updated in `9c1ba4f`/`8c339e3`.
Remaining surface/HiDPI/native visual evidence must follow current QA_EVIDENCE.
Older "next artwork/scale" statements below are dated, not instructions to duplicate
those services. All new IX/IN work remains PROPOSED until real acceptance executes.

## English product/settings/native expansion2026-10-19 — proposed

The documentation-only request is addressed by the
[Product Expansion plan](PLANS/2026-10-19/PRODUCT_EXPANSION/README.md):320 feature briefs,
120 setting contracts,27 delivery packages and explicit native/QA gates. It consolidates
earlier outcomes without adding their feature counts together or claiming implementation.

Urgent next authorized implementation: native covers from the private artwork API,
searchable native/web Settings and device App size with HiDPI/reset/persistence.
Follow the [native master plan/prompt](PLANS/2026-10-19/PRODUCT_EXPANSION/14_NATIVE_CROSS_PLATFORM_MASTER_PLAN.md)
and ART/SCALE/SET acceptance; then native restore/origin-safe offline, cache controls,
single-flight/actual-next prefetch, measured optimization and actual Android/iOS/desktop
engines. No new source/runtime capability or application verification follows from this
entry. Existing implemented slices below retain their evidence.

## Implemented interaction and Hyprland repair2026-10-19

Current-row Play/Pause preserves source/position/order in web/Linux native; explicit
context Play stays separate. Requested intent wins over late native preparation;
queue occurrences toggle, remote transport survives navigation, and native album/
artist/playlist rows have per-entry playback. Offline repeat-one stays local;
unsupported modes are disabled/explained. Linux production/QA share a frameless
builder with integrated window controls. See [review](UX_INTERACTION_REVIEW_2026-10-19.md),
PROGRESS/QA_EVIDENCE for actual Chromium/Firefox/X11/Wayland checks and limits.

Next priorities, retaining auth/source/integrity boundaries:

1. Authorized native artwork with bounded decode/cache and ownership regression.
2. Stable duplicate playlist-entry provenance across concurrent edits; durable
   account-scoped native queue/context/paused-position restoration.
3. Native Add Music entry point and accessible contextual actions for existing APIs.
4. Progressive authorized sources and actual-next prefetch with range/hash/quota/
   cancellation/eviction/stale-account acceptance before rollout.
5. Explicit device queue/position sync and acknowledged handoff, then authorized
   rooms and actual Android/iOS native audio milestones.

These remain independent slices. No runtime/package upgrade or external publication
was performed for this source/debug repair.

## Implemented contrast/listening-layout slice2026-10-19

The newer Better Design request authorizes implementation after the earlier MD-only
planning task. Source UI now has brighter original mint contrast, readable labels,
one native shell grid, explicit track-action columns,44px main targets, compact
native create disclosures and a scrolling web sidebar. Existing player/auth/media
boundaries remain. [Review and exact evidence](DESIGN_REVIEW_2026-10-19.md).

This advances the everyday listening/design slice, not the entire Player Reboot
plan or Spotify parity. Next: authorized native artwork; broader accessibility/state
coverage; progressive-source/prefetch design with integrity and cancellation tests;
device queue/position synchronization and acknowledged handoff. Keep those distinct
from the complete-file cache already implemented before this UI pass. No runtime
upgrade, package replacement or public deployment is implied by source verification.

## User-prioritized player reboot2026-10-19 — planning only

The user's Hyprland/playback screenshots and explicit MD-only request are addressed
in [Player Reboot](PLANS/2026-10-19/PLAYER_REBOOT/README.md). It supplements the dated
audit with source-grounded explanations and 16 Markdown files: normal tiled-window
geometry; reusable private cache/progressive playback; actual-next preparation;
musical UI/screens; device queue/position sync and acknowledged handoff; independent
optional Vela ID; real native clients; authorized rooms/network; 240 feature proposals;
W00–W13 implementation order, acceptance and migration safeguards.

Priority is existing daily listening correctness before expansion. This entry does
not change implementation status: no app code/assets/schema/runtime were changed,
no agents delegated and no new application tests were run. Newer session/offline
source repairs must be preserved and distinguished from the installed desktop build.
The exact order and future file ownership are in
[execution dependencies](PLANS/2026-10-19/PLAYER_REBOOT/11_EXECUTION_DEPENDENCIES.md).

## Planning audit2026-10-19

The current-source audit and detailed next implementation plan are indexed in
[Listening Atlas: audit, repair and product roadmap](PLANS/2026-10-19/README.md).
It records20 findings/risks, repair ordering,42 future work packages, UX states,
feature implementations, an original visual direction and release acceptance.
This is planning at the user's request: no feature is marked implemented from the
plan or its design screenshots. Preserve the existing verified slices below;
historical contradictions identified by the audit were reconciled in place on
2026-10-19 (docs-only) with the source/deployed/design split in
[CURRENT_STATE.md](CURRENT_STATE.md).

Status meanings: implemented=source exists; verified=checks recorded in PROGRESS; partial=exit criteria not all met; planned=no finished feature. No feature is complete because a UI mockup or schema exists.

| Milestone | Current state | Exit condition and outstanding scope |
| --- | --- | --- |
| 0 Repository | Verified foundations; dependency advisories gate OPEN | Stable dependency verification, workspace/server/web, WAL migrations, CI, architecture/threat model; package/runtime evidence remains in hardening. 2026-10-19: cargo-deny FAILs3 INFO Unmaintained notices (AUD-20), no ignore added — see CURRENT_STATE/SECURITY |
| 1 Music cloud | Verified upload/playback slice | Account setup/auth, real batch browser uploads, metadata/artwork, private paginated catalog, original range streaming, player; expanded storage wizard remains |
| 2 URL import | Verified direct-source slice | Preview, durable queue/progress/retry/cancel, streaming, exact dedup, SSRF; browser/public download and recovery tested; source collections/licenses/storage estimates need richer adapters |
| 3 Complete library | Partial | Search/favorites/stats, paginated album/artist browsing, revisioned editor, read-only snapshot scanning, typed filters/title/newest ordering and private import/scan job center present; next verified editions, batch metadata editing, organization and remaining filter dimensions |
| 4 Sharing | Partial | Local accounts, explicit collaborator viewer/editor grants, revision-safe edits, owner CAS rename/delete, expiring/revocable metadata links present; collaborator list/revocation/leave UX present; next invitations, permission-specific media access, revision feed, profiles, ownership transfer |
| 5 Multi-server | Partial: native public playback | Server identity registry, secure credentials, native/OpenSubsonic adapters, bounded concurrent unified search, origin-labelled queues, two-server real integration |
| 6 Federation | Partial opt-in public profile; social protocol planned | ActivityPub interoperability profile, explicit opt-in visibility, WebFinger/discovery, origin verification/signatures/replay limits, follow/access requests, durable deliveries, remote metadata cache, blocked peers, privacy tests between two instances |
| 7 Playback quality | Partial browser controls | Queue/seek/volume/repeat/fair shuffle/history/next priorities/keys/media-session integration; 0.7 adds browser context/progress restore and bounded reconnect; 0.8 adds shared-state Now Playing; 0.9 adds consented history and explicit local radio;0.12 adds a verified root sleep timer; next optional controlled FFmpeg, quality selection, validated ReplayGain/gapless/crossfade |
| 8 Native/offline | Partial: Linux desktop slice | Real0.12 Dioxus Linux client with in-memory native HTTP, Rodio/CPAL output thread, MPRIS2, bounded prepared originals and verified Library/Queue playback. **Verified:** portable x86_64 tar package with exact-manifest/`ldd`/real-window launch
smoke; opt-in Secret Service token resume, v2 BLAKE3 offline manifests with startup sweep and a server-stopped offline queue, native playlist folders (0.12.6). **Outstanding:** audited keyring/offline lifecycle fixes, deb/AppImage/signed installer, background/automatic offline sync, Android Media3 service, iOS AVFoundation background audio, Windows/macOS clients, device transfer/sync; actual device evidence required |
| 9 Interoperability | Planned | OpenSubsonic contracts/client tests, aggregate listening statistics/dashboards (history/export/delete already exist since0.9), authorized lyrics/LRC, typed smart playlists, M3U/export, deterministic recommendations, optional distinct podcasts/radio |
| 10 Shared experiences | Planned | Authorized WebSocket rooms, clock/latency/reconnect semantics, shared queues, opt-in activity feeds, collection exploration, interoperability |
| 11 Hardening | Partial | Regression/security/benchmark/backup/deploy work underway; ongoing audits, real container/ARM64 builds, stress/seek/upload/restart tests, accessibility review, release notes |

## Current0.11 increment

Explicit private-by-default server identity, owner declaration/provenance,
metadata/stream/download grants, retained CAS revocation, bounded peer preview and
actual two-node negative HTTP tests are implemented. Network UI/PN01/PN02 and
persistent same-build Linux user-service installation are integrated; source200Rust and exact packaged Chromium39/Firefox39 plus preserved original-data
upgrade passed; exact scope/evidence is in PROGRESS/QA_EVIDENCE. None of this enables
public sharing or history collection on original data. Read-only profile1 is not
full social federation or a unified remote player. Next: standards-based identity/
signatures, blocked peers and real saved catalog adapters; actual Linux/Android
native playback/packaging and offline engine. Apple builds need an Apple toolchain.

## Current0.12 increment

The eighth workspace application `vela-desktop` is a real Linux native client, not a
WebView audio substitute. It signs in with memory-only credentials, pages/searches
the private owner catalog, plays authorized WAV/FLAC/MP3/MP4/Vorbis originals through
one supervised Rodio/CPAL output thread, and shares the framework-free occurrence
queue with the web app. Prepared originals are unnamed private0600 files capped at
250MiB, created without a directory entry from the first byte, verified by
size/BLAKE3 ETag and released on replacement/sign-out/failure/exit; a dead output
worker is disabled and can be reconnected explicitly. MPRIS2 state/commands run on a
separate thread with bounded channels and fail open to in-app controls. The native
client also has an owner-only Favorites view with per-row favorite/unfavorite,
queue-state synchronization and a verified empty/unfavorite/Library-return journey.
Native Playlists list/detail, owner create/CAS-rename/confirm-delete and playback,
album/artist group browsing, playlist entry add/reorder/removal, queue move-down/
Clear-upcoming, player favorite and direct second-server public-catalog playback are
verified (loopback debug transport); album/artist editing remains future work. A real-window
automated journey verified playback, seek, navigation continuity, queue duplicates/
reorder, light theme,640×700 element bounds and zero temporary directory entries;
MPRIS and null-sink audio tests also ran.

The same0.12 source increment fixed browser queue truthfulness: move-down, a
Clear-upcoming that keeps the current occurrence and played prefix, an explained
disabled remove-current, and first-page controls disabled from the incoming cursor.
Owners can now CAS-rename and permanently delete a playlist with inline confirmation
and move-down ordering; deletion cascades entries/collaborator grants/share links but
never tracks, assets or audio rights. Real TCP owner/editor/outsider/CSRF/stale/
replay/share-revocation/track-preservation tests pass, and Chromium40/40 plus
Firefox40/40 source runs include the new lifecycle queue scenarios and all prior
publication/history/DSP/radio/player regressions.

A local0.12.0 server/web package passed extracted-package and exact Chromium/Firefox
gates and was installed over the original data after a verified backup/restore
(schema11 unchanged, all rows/media preserved). This is still FUNCTIONAL_TESTED for
the Linux library/queue and packaged web slices only. It
is not a package, installer, secure credential store, offline client, background
mobile service, favorites/playlists/albums native UI or Apple/Android/Windows
implementation. Next native work: favorites/playlist/album surfaces and session
persistence with a reviewed native secret store, then a real Linux package and
external MPRIS/session smoke; the Android Media3 and Apple AVFoundation targets
remain blocked until their toolchains exist. No mobile hardware claim is implied.

## Next ordered slices

The full VELA_MUSIC_OMEGA_AUTONOMOUS_ENGINEERING_SPEC.md supplied2026-10-18 supersedes prior priority ordering. It was read in full; the existing dirty post-0.11 repository/runtime were inspected and preserved. Golden IDs G01–G24 are distinguished in QA_EVIDENCE from older UX IDs.

1. Preserve verified0.9 private meaningful listening events, global/device consent, idempotency, retention, export/delete and real History/Home recents, plus all existing player/ingest/share/security regressions.157 Rust tests,35 Chromium and35 Firefox scenarios pass; exact scope/evidence is in PROGRESS.
2. Preserve verified explicit serial DSP worker (spectral-3 tempo/chroma estimates) and measured/tag-fallback track radio; add actual consented history/favorites-based Home ranking, exclusions/feedback, sparse-library daily/weekly mixes and durable refresh. The20-run PCM release benchmark is in PERFORMANCE; measure real-music relevance/scale before increasing concurrency or adding ML.
3. Implement an opt-in lawful network slice: stable origin identity/capabilities and per-object publication/rights provenance first; two independent instances and private negative controls, then signed/replay-safe standards-based social discovery and separately authorized streams. No public audio by default. FEDERATION_PROTOCOL/NETWORK_RIGHTS now define the limited read-only public profile; signed social federation remains unfinished.
4. Continue actual Linux desktop packaging/session persistence and, only with their real toolchains, Android Media3 and Apple native audio behind the shared command/event/capability contract. Linux GTK/WebKit/ALSA are present and the0.12 native playback slice is verified; Android SDK/Gradle and Apple/Xcode remain absent, and no device is verified. PLATFORM_MATRIX separates code, compilation and hardware evidence.
5. Continue independent device handoff, shared rooms, sharing invitations and scoped media grants; multi-server/OpenSubsonic adapters, recording/edition models, richer organization, accessibility, remaining lawful source adapters and production hardening. Each needs its own complete acceptance rather than an empty crate.

## Cross-cutting acceptance

Every slice includes validation, permission/error/loading/empty/cancellation/recovery states, bounded concurrency and data portability. Large catalogs use bounded pages now; evaluate virtualization when list views grow. Profile before optimization and publish only reproducible measurements. Preserve private defaults; no automatic external metadata requests, federation, public user lists or listening activity. No credential fabrication, DRM bypass, privileged automatic self-update or external publishing.

- Масштабирование, мульти-origin федерация и легальный контент: см. [RESILIENT_SCALE_PLAN.md](RESILIENT_SCALE_PLAN.md).
