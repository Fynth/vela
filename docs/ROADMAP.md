# Authoritative backlog

Status meanings: implemented=source exists; verified=checks recorded in PROGRESS; partial=exit criteria not all met; planned=no finished feature. No feature is complete because a UI mockup or schema exists.

| Milestone | Current state | Exit condition and outstanding scope |
| --- | --- | --- |
| 0 Repository | Verified foundations | Stable dependency verification, workspace/server/web, WAL migrations, CI, architecture/threat model; package/runtime evidence remains in hardening |
| 1 Music cloud | Verified upload/playback slice | Account setup/auth, real batch browser uploads, metadata/artwork, private paginated catalog, original range streaming, player; expanded storage wizard remains |
| 2 URL import | Verified direct-source slice | Preview, durable queue/progress/retry/cancel, streaming, exact dedup, SSRF; browser/public download and recovery tested; source collections/licenses/storage estimates need richer adapters |
| 3 Complete library | Partial | Search/favorites/stats and paginated normalized album/artist browsing and revisioned catalog editor and configured read-only snapshot scanning present; next genres/editions, batch metadata editing, organization, job center, advanced filters |
| 4 Sharing | Partial | Local accounts, explicit collaborator viewer/editor grants, revision-safe edits, expiring/revocable metadata links present; collaborator list/revocation and leave UX present; next invitations, permission-specific media access, revision feed, profiles, ownership transfer |
| 5 Multi-server | Planned | Server identity registry, secure credentials, native/OpenSubsonic adapters, bounded concurrent unified search, origin-labelled queues, two-server real integration |
| 6 Federation | Planned, disabled | ActivityPub interoperability profile, explicit opt-in visibility, WebFinger/discovery, origin verification/signatures/replay limits, follow/access requests, durable deliveries, remote metadata cache, blocked peers, privacy tests between two instances |
| 7 Playback quality | Partial browser controls | Queue/seek/volume/repeat/shuffle/keys/media-session integration; next optional controlled FFmpeg, quality selection, validated ReplayGain/gapless/crossfade, sleep timer, playback progress/resume |
| 8 Native/offline | Planned | Desktop packages, Android Media3 service, iOS AVFoundation background audio, download manifests/hash/resume/quota, device transfer/sync; actual device evidence required |
| 9 Interoperability | Planned | OpenSubsonic contracts/client tests, private listening history/stats, authorized lyrics/LRC, typed smart playlists, M3U/export, deterministic recommendations, optional distinct podcasts/radio |
| 10 Shared experiences | Planned | Authorized WebSocket rooms, clock/latency/reconnect semantics, shared queues, opt-in activity feeds, collection exploration, interoperability |
| 11 Hardening | Partial | Regression/security/benchmark/backup/deploy work underway; ongoing audits, real container/ARM64 builds, stress/seek/upload/restart tests, accessibility review, release notes |

## Next ordered slices

1. Preserve the0.2.0 folder-scanning checkpoint and its executed verification evidence. Keep immutable source copies, private roots and metadata-edit preservation when expanding storage modes. Crash-safe orphan reconciliation remains outstanding.
2. Complete library organization: typed genre/quality/year filters and a coherent background job center, then batch metadata editing. Direct-reference external storage needs its own lifetime/authorization/backup contract; current scans intentionally copy into managed storage.
3. Complete local sharing administration: invites with expiry/acceptance, narrow separate stream/download grants, near-real-time revision updates. Keep metadata links view-only by default.
4. Source adapters: ZIP with zip-slip/bomb limits, administrator-trusted WebDAV and S3 (separate egress policy), Archive.org permitted assets, explicitly licensed catalogs, RSS/episodes as podcast content. Playlist metadata imports require authorized providers, never commercial stream extraction.
5. Multi-server/native adapter and two-server E2E; then ActivityPub interoperability/security protocol work. Avoid new empty crates before useful implementations exist.

## Cross-cutting acceptance

Every slice includes validation, permission/error/loading/empty/cancellation/recovery states, bounded concurrency and data portability. Large catalogs use bounded pages now; evaluate virtualization when list views grow. Profile before optimization and publish only reproducible measurements. Preserve private defaults; no automatic external metadata requests, federation, public user lists or listening activity. No credential fabrication, DRM bypass, privileged automatic self-update or external publishing.
