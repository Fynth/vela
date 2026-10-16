# Authoritative backlog

Status meanings: implemented=source exists; verified=checks recorded in PROGRESS; partial=exit criteria not all met; planned=no finished feature. No feature is complete because a UI mockup or schema exists.

| Milestone | Current state | Exit condition and outstanding scope |
| --- | --- | --- |
| 0 Repository | Verified foundations | Stable dependency verification, workspace/server/web, WAL migrations, CI, architecture/threat model; package/runtime evidence remains in hardening |
| 1 Music cloud | Verified upload/playback slice | Account setup/auth, real batch browser uploads, metadata/artwork, private paginated catalog, original range streaming, player; expanded storage wizard remains |
| 2 URL import | Verified direct-source slice | Preview, durable queue/progress/retry/cancel, streaming, exact dedup, SSRF; browser/public download and recovery tested; source collections/licenses/storage estimates need richer adapters |
| 3 Complete library | Partial | Search/favorites/stats, paginated album/artist browsing, revisioned editor, read-only snapshot scanning, typed filters/title/newest ordering and private import/scan job center present; next verified editions, batch metadata editing, organization and remaining filter dimensions |
| 4 Sharing | Partial | Local accounts, explicit collaborator viewer/editor grants, revision-safe edits, expiring/revocable metadata links present; collaborator list/revocation and leave UX present; next invitations, permission-specific media access, revision feed, profiles, ownership transfer |
| 5 Multi-server | Planned | Server identity registry, secure credentials, native/OpenSubsonic adapters, bounded concurrent unified search, origin-labelled queues, two-server real integration |
| 6 Federation | Planned, disabled | ActivityPub interoperability profile, explicit opt-in visibility, WebFinger/discovery, origin verification/signatures/replay limits, follow/access requests, durable deliveries, remote metadata cache, blocked peers, privacy tests between two instances |
| 7 Playback quality | Partial browser controls | Queue/seek/volume/repeat/fair shuffle/history/next priorities/keys/media-session integration; 0.7 adds browser context/progress restore and bounded reconnect; 0.8 adds shared-state Now Playing; next private listening events, then optional controlled FFmpeg, quality selection, validated ReplayGain/gapless/crossfade and sleep timer |
| 8 Native/offline | Planned | Desktop packages, Android Media3 service, iOS AVFoundation background audio, download manifests/hash/resume/quota, device transfer/sync; actual device evidence required |
| 9 Interoperability | Planned | OpenSubsonic contracts/client tests, private listening history/stats, authorized lyrics/LRC, typed smart playlists, M3U/export, deterministic recommendations, optional distinct podcasts/radio |
| 10 Shared experiences | Planned | Authorized WebSocket rooms, clock/latency/reconnect semantics, shared queues, opt-in activity feeds, collection exploration, interoperability |
| 11 Hardening | Partial | Regression/security/benchmark/backup/deploy work underway; ongoing audits, real container/ARM64 builds, stress/seek/upload/restart tests, accessibility review, release notes |

## Next ordered slices

The2026-10-16 consumer-experience brief supersedes the earlier batch-metadata-first ordering. See SPOTIFY_PARITY_MATRIX for the broader phased backlog and concrete ten-gap audit.

1. Preserve0.5 verified Home/local Search/shared shell/play-next increment and existing ingest/share/security workflows. One root audio node, bounded real content, private access and responsive keyboard navigation are required at every subsequent merge.
2. Playback correctness:0.6 adds the pure bounded queue/repeat/shuffle transition model with occurrence remapping, fair traversal/history and previous-button seek. Version0.7 adds atomic context/position/preferences bookmarks, paused restore, generation-safe source handling, bounded retry/stall recovery and expired-session handling. Version0.8 adds a root-coordinated Now Playing modal with real transport/context/file details/favorites/queue preview and native focus. Next establish privacy-controlled listening events before discovery/audio effects.
3. Preserve verified Now Playing/track context and implement the listening-event foundation with privacy/consent, deduplication and real listened-time thresholds; then deterministic sparse-library radio/mixes with playable candidates and explanations.
4. Sharing: expiring invitations and explicitly separate media grants, revision updates and accessible collaboration. Then device presence/handoff with two actual clients and Linux desktop validation; mobile/offline require native services and real platform evidence.
5. Native multi-server adapters/two-instance integration, then optional standards-based federation and authorized social/rooms. Extend catalog editions, batch edits, smart rules and ZIP/trusted WebDAV/S3/authorized source adapters as independent tested slices. Preserve immutable originals and egress boundaries.

## Cross-cutting acceptance

Every slice includes validation, permission/error/loading/empty/cancellation/recovery states, bounded concurrency and data portability. Large catalogs use bounded pages now; evaluate virtualization when list views grow. Profile before optimization and publish only reproducible measurements. Preserve private defaults; no automatic external metadata requests, federation, public user lists or listening activity. No credential fabrication, DRM bypass, privileged automatic self-update or external publishing.
