# ADR0015: Private on-demand exploration and rotation

Status: implemented; final product verification recorded in PROGRESS,2026-10-18. Supersedes no earlier schema.

Extend the existing modular monolith/radio rather than add a recommender service,
cloud API or inference dependency. Favorites are explicit preferences. Qualified
retained listening contributes only while CURRENT account collection consent is
on. A single owner-scoped SQLite read snapshot covers preferences, actual media,
hidden choices and signals. Opt-out retains exportable history but immediately
removes its influence from the next recommendation read. Erase needs no derived
cache invalidation because there is no materialized taste/history cache.

Exploration examines at most500 newest nonhidden playable metadata candidates;
combined favorite/recent anchors cap at12. Already-qualified tracks and anchors
are suppressed, with honest empty results. Rotation ranks qualified counts and
recency across retained owned tracks, including older media outside the candidate
window. The existing validated descriptor distance provides measured affinity;
missing/corrupt/silent/version-incompatible vectors fall back to actual tags.
Every returned track remains subject to the normal independent stream ACL.

Feedback is desired state, not a toggle. Additive migration0010 stores at most1000
owner/track exclusions behind a composite FK. BEGIN IMMEDIATE protects quotas,
ownership and idempotent concurrent hides. Undo, explicit restore-all and bounded
UUID-only export keep choices reversible and portable. Hidden choices are separate
preferences, not erased listening events. They never remove files or favorites.

These are on-demand mixes, not daily/weekly scheduled snapshots or collaborative
learning. Durable themed snapshots, calibrated quality evaluation, network rights
and native clients remain independent milestones. No dependency upgrade is needed.

Research revalidated2026-10-17: SQLite documents a read transaction's consistent
historic snapshot and BEGIN IMMEDIATE's write lock:
https://www.sqlite.org/lang_transaction.html . Built-in aggregate semantics:
https://www.sqlite.org/lang_aggfunc.html . ListenBrainz recommendation API
https://listenbrainz.readthedocs.io/en/latest/users/api/recommendation.html is an
optional external source, not used or required by this implementation. Private
signals are never sent there. No Spotify catalog or model access is assumed.

Executed acceptance and exact rank weights belong in RECOMMENDATIONS/PROGRESS
once implementation, real SQLite/TCP/browser gates and measurements finish.

Measured follow-up: baseline100k history-enabled p50 was1200ms and1m was10995ms.
EXPLAIN found scans of the owner catalog despite sparse favorites/history. Select
source-driven eligible bounded IDs before full projection; hydrate in the same
snapshot and restore ID order; reuse rotation evidence instead of aggregating
again. Exact four selector/projection outputs match the baseline. No schema,
rank weights or candidate ordering changed. Final measured scope and unresolved
equal-timestamp/history-density cases are recorded in PERFORMANCE/limits.

SQLite's documented CROSS JOIN loop-order control was rechecked2026-10-18:
https://www.sqlite.org/optoverview.html#manual_control_of_query_plans_using_cross_join .
Use it only at the measured sparse evidence-to-track boundary; no forced global
index hints or rank/candidate-policy change is introduced.
