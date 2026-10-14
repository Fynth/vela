# ADR0004: persisted private catalog groups

Status: accepted,2026-10-14.

Tracks initially stored embedded album/artist strings. Users need stable album and artist pages, musical album ordering and bounded pagination; grouping every request by display names cannot provide stable identifiers or safe future metadata edits.

Migration0002 introduces owner-scoped artists and albums, backfills existing catalogs and maintains track associations transactionally with insertion/metadata-update triggers. Source strings and original files remain intact. Artist names are trimmed and compared using SQLite ASCII NOCASE; bundled credits are not heuristically split. Album identity combines trimmed title, effective album artist (embedded album artist, then track artist) and release year; zero means unknown. Different known years remain separate. Empty album tags stay ungrouped. IDs are random persisted UUIDs. This is a provisional catalog grouping rule; recording/master/edition identity needs verified identifiers and explicit review later. Unicode case folding and uncertain-year reconciliation are not claimed.

Album-artist relationships use composite owner constraints. Track association rewrites reject foreign-owned group IDs. Queries separately enforce track and group ownership, hide empty stored groups and select artwork only from authorized tracks. An artist page includes direct performer credits and album-artist tracks, counting a track once. No metadata enrichment requests or public disclosure are added.

Summary lists use UUID keyset pages up to100. Album tracks order by disc, track number and ID, with unknown track numbers last; their cursor is an authorized member track UUID whose sort tuple is resolved within one read snapshot. Artist tracks use UUID order. The client requests50 entries, replaces each page, and labels playback as the visible tracks. Session/generation guards discard stale private responses. Arbitrary or foreign track cursors do not disclose track metadata.

Triggers keep ingestion, upgrades and future catalog edits consistent, avoiding duplicated grouping algorithms in transports. FTS updates are restricted to indexed metadata columns so association-only changes cannot corrupt external-content FTS during insertion. Stored orphan groups are retained and hidden rather than deleting potentially referenced identities. Full edition models, album/artist editing and reconciliation remain separate backlog items.
