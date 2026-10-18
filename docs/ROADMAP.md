# Authoritative backlog

Status meanings: implemented=source exists; verified=checks recorded in PROGRESS; partial=exit criteria not all met; planned=no finished feature. No feature is complete because a UI mockup or schema exists.

| Milestone | Current state | Exit condition and outstanding scope |
| --- | --- | --- |
| 0 Repository | Verified foundations | Stable dependency verification, workspace/server/web, WAL migrations, CI, architecture/threat model; package/runtime evidence remains in hardening |
| 1 Music cloud | Verified upload/playback slice | Account setup/auth, real batch browser uploads, metadata/artwork, private paginated catalog, original range streaming, player; expanded storage wizard remains |
| 2 URL import | Verified direct-source slice | Preview, durable queue/progress/retry/cancel, streaming, exact dedup, SSRF; browser/public download and recovery tested; source collections/licenses/storage estimates need richer adapters |
| 3 Complete library | Partial | Search/favorites/stats, paginated album/artist browsing, revisioned editor, read-only snapshot scanning, typed filters/title/newest ordering and private import/scan job center present; next verified editions, batch metadata editing, organization and remaining filter dimensions |
| 4 Sharing | Partial | Local accounts, explicit collaborator viewer/editor grants, revision-safe edits, expiring/revocable metadata links present; collaborator list/revocation and leave UX present; next invitations, permission-specific media access, revision feed, profiles, ownership transfer |
| 5 Multi-server | Partial: public preview only | Server identity registry, secure credentials, native/OpenSubsonic adapters, bounded concurrent unified search, origin-labelled queues, two-server real integration |
| 6 Federation | Partial opt-in public profile; social protocol planned | ActivityPub interoperability profile, explicit opt-in visibility, WebFinger/discovery, origin verification/signatures/replay limits, follow/access requests, durable deliveries, remote metadata cache, blocked peers, privacy tests between two instances |
| 7 Playback quality | Partial browser controls | Queue/seek/volume/repeat/fair shuffle/history/next priorities/keys/media-session integration; 0.7 adds browser context/progress restore and bounded reconnect; 0.8 adds shared-state Now Playing; 0.9 adds consented history and explicit local radio; next optional controlled FFmpeg, quality selection, validated ReplayGain/gapless/crossfade and sleep timer |
| 8 Native/offline | Planned | Desktop packages, Android Media3 service, iOS AVFoundation background audio, download manifests/hash/resume/quota, device transfer/sync; actual device evidence required |
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

## Next ordered slices

The full VELA_ULTIMATE_AUTONOMOUS_FEDERATED_MUSIC_MASTER_PROMPT.md supplied2026-10-17 supersedes prior priority ordering. It was read in full; the existing0.8.1 repository/runtime were inspected and backed up. New golden IDs G01–G24 are distinguished in QA_EVIDENCE from older UX IDs.

1. Preserve verified0.9 private meaningful listening events, global/device consent, idempotency, retention, export/delete and real History/Home recents, plus all existing player/ingest/share/security regressions.157 Rust tests,35 Chromium and35 Firefox scenarios pass; exact scope/evidence is in PROGRESS.
2. Preserve verified explicit serial DSP worker and measured/tag-fallback track radio; add actual consented history/favorites-based Home ranking, exclusions/feedback, sparse-library daily/weekly mixes and durable refresh. The20-run PCM release benchmark is in PERFORMANCE; measure real-music relevance/scale before increasing concurrency or adding ML.
3. Implement an opt-in lawful network slice: stable origin identity/capabilities and per-object publication/rights provenance first; two independent instances and private negative controls, then signed/replay-safe standards-based social discovery and separately authorized streams. No public audio by default. FEDERATION_PROTOCOL/NETWORK_RIGHTS now define the limited read-only public profile; signed social federation remains unfinished.
4. Implement actual Linux desktop and Android native audio behind a shared command/event/capability contract, then secure offline manifests/sync. Linux GTK/WebKit/ALSA are present; Android SDK/Gradle are absent and no devices are verified. Apple builds remain blocked on an Apple toolchain/runner, with implementable shared work continuing. PLATFORM_MATRIX separates code, compilation and hardware evidence.
5. Continue independent device handoff, shared rooms, sharing invitations and scoped media grants; multi-server/OpenSubsonic adapters, recording/edition models, richer organization, accessibility, remaining lawful source adapters and production hardening. Each needs its own complete acceptance rather than an empty crate.

## Cross-cutting acceptance

Every slice includes validation, permission/error/loading/empty/cancellation/recovery states, bounded concurrency and data portability. Large catalogs use bounded pages now; evaluate virtualization when list views grow. Profile before optimization and publish only reproducible measurements. Preserve private defaults; no automatic external metadata requests, federation, public user lists or listening activity. No credential fabrication, DRM bypass, privileged automatic self-update or external publishing.
