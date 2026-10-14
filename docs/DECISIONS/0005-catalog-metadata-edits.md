# ADR0005: owner-only catalog edits with revision protection

Status: accepted,2026-10-14.

Imported tags can be incomplete or incorrect. Normal editing belongs in the application and must preserve owned originals. Playlist editors do not own every referenced track; their permission must not grant source metadata changes.

The metadata endpoint exposes a private revisioned snapshot to the track owner. A complete typed update can change title, artist, album, album artist, genre, year, track number and disc number only. Numeric unknowns are explicit null; unknown JSON fields are rejected. Text limits/control-character validation and a16KiB transport limit bound work. Audio asset identity, technical properties, artwork and original embedded tags are immutable through this endpoint.

One SQLite BEGIN IMMEDIATE transaction verifies ownership, validates input, compares the expected revision, updates catalog columns, advances the revision and records a fixed audit action with owner/object IDs. Triggers maintain FTS and persisted group associations. A stale revision receives409; no automatic last-writer-wins overwrite. The browser keeps the draft and offers an explicit reload that discards it. Asynchronous editor loads/saves are scoped to the initiating session and editor generation.

Edited catalog text is distinct from immutable embedded-source tags. Re-importing an identical file must retain the owner's edited catalog; another owner's catalog remains independent even when physical bytes are shared. Unknown release years and names still follow the provisional grouping policy in ADR0004. Explicit original-file write-back, enrichment review and batch edits are separate future features requiring their own authorization and conflict rules.
