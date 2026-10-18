# Authoritative backlog

Status meanings: implemented=source exists; verified=checks recorded in PROGRESS; partial=exit criteria not all met; planned=no finished feature. No feature is complete because a UI mockup or schema exists.

| Milestone | Current state | Exit condition and outstanding scope |
| --- | --- | --- |
| 0 Repository | Verified foundations | Stable dependency verification, workspace/server/web, WAL migrations, CI, architecture/threat model; package/runtime evidence remains in hardening |
| 1 Music cloud | Verified upload/playback slice | Account setup/auth, real batch browser uploads, metadata/artwork, private paginated catalog, original range streaming, player; expanded storage wizard remains |
| 2 URL import | Verified direct-source slice | Preview, durable queue/progress/retry/cancel, streaming, exact dedup, SSRF; browser/public download and recovery tested; source collections/licenses/storage estimates need richer adapters |
| 3 Complete library | Partial | Search/favorites/stats, paginated album/artist browsing, revisioned editor, read-only snapshot scanning, typed filters/title/newest ordering and private import/scan job center present; next verified editions, batch metadata editing, organization and remaining filter dimensions |
| 4 Sharing | Partial | Local accounts, explicit collaborator viewer/editor grants, revision-safe edits, owner CAS rename/delete, expiring/revocable metadata links present; collaborator list/revocation/leave UX present; next invitations, permission-specific media access, revision feed, profiles, ownership transfer |
| 5 Multi-server | Partial: native public playback | Server identity registry, secure credentials, native/OpenSubsonic adapters, bounded concurrent unified search, origin-labelled queues, two-server real integration |
| 6 Federation | Partial opt-in public profile; social protocol planned | ActivityPub interoperability profile, explicit opt-in visibility, WebFinger/discovery, origin verification/signatures/replay limits, follow/access requests, durable deliveries, remote metadata cache, blocked peers, privacy tests between two instances |
| 7 Playback quality | Partial browser controls | Queue/seek/volume/repeat/fair shuffle/history/next priorities/keys/media-session integration; 0.7 adds browser context/progress restore and bounded reconnect; 0.8 adds shared-state Now Playing; 0.9 adds consented history and explicit local radio;0.12 adds a verified root sleep timer; next optional controlled FFmpeg, quality selection, validated ReplayGain/gapless/crossfade |
| 8 Native/offline | Partial: Linux desktop slice | Real0.12 Dioxus Linux client with in-memory native HTTP, Rodio/CPAL output thread, MPRIS2, bounded prepared originals and verified Library/Queue playback. **Verified:** portable x86_64 tar package with exact-manifest/`ldd`/real-window launch
smoke. **Outstanding:** deb/AppImage/signed installer, secure credential persistence,
native playlist-folder UI, offline manifests/hash/resume/quota, Android Media3 service, iOS AVFoundation background audio, Windows/macOS clients, device transfer/sync; actual device evidence required |
| 9 Interoperability | Planned | OpenSubsonic contracts/client tests, private listening history/stats, authorized lyrics/LRC, typed smart playlists, M3U/export, deterministic recommendations, optional distinct podcasts/radio |
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
verified (loopback debug transport); native playlist-folder UI and album/artist editing remain future work. A real-window
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
