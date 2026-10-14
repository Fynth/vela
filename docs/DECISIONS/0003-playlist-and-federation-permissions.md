# ADR0003 — Metadata permission is separate from media permission

Accepted2026-10-13. Local playlist grants have viewer/editor permissions. Owners mint expiring revocable random tokens hashed at rest; public links expose playlist titles only. Original streams remain owner-authorized. Concurrent changes use revision CAS in BEGIN IMMEDIATE transactions plus actor/request-bound mutation UUIDs; no CRDT for online-only editing.

ActivityPub/ActivityStreams plus WebFinger are the selected standards for later federation investigation. Existing Funkwhale specifications distinguish actors, library subscriptions and audio authorization. Standards provide transport/vocabulary, not implicit redistribution permission. Vela federation requires an explicit interoperable capability profile, signed/origin-verified requests, replay/resource controls and separate discover/profile/activity/playlist/library/stream/download settings. No federation implementation is claimed by this ADR and no central directory is required.

Tradeoffs: shared titles may show unavailable audio/artwork until separate access exists. Revocations are local authoritative checks. Remote disclosure later cannot guarantee removal from administrators/backups. Cross-server identity must include an origin, and metadata caching cannot establish local asset ownership.
