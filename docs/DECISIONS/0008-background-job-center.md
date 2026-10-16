# ADR0008: One private view over existing durable workers

Date:2026-10-16. Status:accepted.

## Decision

Expose import and folder-scan history through a core JobService read model and authenticated GET /api/jobs. Retain the existing worker tables and independently authorized mutation APIs. Do not introduce a generic scheduler, extra crate, broker or duplicate job state. Browser uploads remain transfer-dialog operations rather than invented durable server tasks.

An owner-scoped branch reads at most limit+1 imports and, for administrators, limit+1 scans. Merge these bounded results by created_at descending, canonical UUID descending and kind descending (scan before import for exact collisions). The composite cursor carries all three keys and does not depend on the anchor row continuing to exist. Migration0007 adds the UUID tie-breaker to both history indexes and owner/status indexes for filtered history. Use a single deferred SQLite read transaction for rows, action flags and an EXISTS activity probe. A coherent snapshot matters when workers commit between queries. SQLite's [row-value comparisons](https://www.sqlite.org/rowvalue.html) and [SELECT ordering/limits](https://www.sqlite.org/lang_select.html) support the indexed branch implementation.

has_active includes all authorized owned activity, irrespective of the selected type/state/cursor. This allows an empty Completed view to learn about an active job finishing. It never includes another owner's jobs or administrator-only scans for ordinary accounts. UI polls every2s only while such activity is known and the mounted page is visible; idle pages offer explicit refresh. Existing import polling stops on Jobs. Response/action application requires the exact initiating session, request generation, selection and mounted lifetime. Errors retain visibly stale confirmed rows and disable their actions until refreshed.

No source URL, filesystem path or raw worker diagnostic enters this DTO. Import titles resolve only completed owned tracks; an unexpected foreign track reference returns the generic title without its ID. Scan titles resolve only owned configured sources. Fixed failed-state explanations avoid copying internal errors into a general activity view. Specific source/import controls retain their existing authorized APIs.

Cancel and import retry reuse the existing services. Scan again creates a new pass with a new ID, preserving earlier history; no retry lineage or generic state machine is necessary. Action flags are snapshot hints: the mutation rechecks ownership, state, quotas and source availability. A terminal scan cannot offer Scan again while its source already has a queued/running pass.

## Consequences and limits

At most202 projected rows are merged for a maximum100-item response. Result bounds do not establish constant SQL cost or server capacity. Histories have live cursor semantics; retries can change a row's state between filtered pages and source removal deletes that source's scan history under the existing foreign key. Source files and imported music are retained. No SSE, all-user administrator queue, scheduled scans, scan retry-in-place, task pause or durable browser-upload history is claimed.

Unit/service fixtures exercise actual migrated SQLite, collision/deletion cursors, private joins, all statuses, flags, plans and concurrent WAL updates. Real TCP tests independently execute queued import cancel/retry and actual FLAC/MP3 scan ingestion/cancellation/new-pass history. Browser coverage and executed release evidence belong in PROGRESS, not this design decision. Future durable operations should add a bounded projection only when their actual worker exists.

Downgrading the database in place is unsupported. Save a private pre-upgrade backup and restore it with its matching prior binary if rollback is necessary. Migration0007 changes indexes only; no source audio or sharing grants are rewritten.
