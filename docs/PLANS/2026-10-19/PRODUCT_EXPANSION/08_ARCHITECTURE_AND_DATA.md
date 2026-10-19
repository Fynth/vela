# Architecture, persistence, capabilities and migration contracts

All tables/endpoints/modules described as additions are proposals. Inspect the
actual receiving checkout before allocating migrations or changing signatures.

## Preserve the modular monolith

Keep one Rust server, SQLite/WAL/FTS, managed media directories and bounded workers.
No Redis, queue broker, mandatory search cluster, cloud account or model service.
Retain the eight existing workspace members unless real coupling justifies another
cohesive boundary. Shared settings validation can begin as a framework-free module
with platform adapters; do not create20 empty settings crates.

Presentation code must not read the database. Preference services own validation,
scope, persistence and application receipts. Root playback coordinators consume
validated changes; settings pages do not send raw decoder commands or create audio.
Server transports independently authorize typed application commands.

## State ownership

| Plane | Owner/persistence | Examples | Explicit exclusions |
| --- | --- | --- | --- |
| Device presentation | Versioned bounded device store; root service | Scale/theme/text/density/shortcuts/panel sizes | Credentials, private catalog/history, account grants |
| Device audio/download policy | Native client store with account-specific refs where required | Output selection, network policy, quota limits | Arbitrary executable paths, automatic permission grants |
| Playback session | Root queue journal and engine adapter | Occurrence IDs, context/cursor, requested/actual state, position | Raw stream URL as stable identity; extra audio engine |
| Account preferences | Owner-scoped server service with revisions | Discovery choices, library organization, visibility preferences | Device scale/output names and user passwords |
| Privacy/rights | Existing dedicated consent, membership, publication services | History epoch, device consent, stream/download grants | Generic preference import or optimistic client-only grants |
| Server policy | Administrator service constrained by operator ceilings | Quotas/jobs/enablement/pending configuration | User-controlled TLS bypass, arbitrary roots or command templates |
| Operational bootstrap | Existing validated config/deployment | Bind/data roots/volumes/trust stores | An assumption that a web form can mount a filesystem |

Store sensitive credentials only through the reviewed platform/server secret adapter.
Reference a connection/integration by opaque ID; never place its token in preference
JSON, a URL, browser storage, support bundle or generated recommendation explanation.

## Preference record and application receipt

Define a closed registry of known keys and value types. Each read projects requested
value, effective value, source/default, scope, policy lock reason, capability reason,
apply mode, revision and pending-restart state. Omit secrets and private operator paths.

Each change carries key, typed value, expected revision where server-authoritative,
mutation ID and relevant device/account generation. A receipt distinguishes Applied,
Applied temporarily, Applies next track, Pending restart, Conflict and Rejected.
Persistence success alone is not engine acceptance. Do not use wall-clock “last write
wins” to override a concurrent consent or authorization change.

Client store: bounded schema/version/size, canonical numeric validation, atomic
replacement, private permissions, disk work off UI thread. Keep a last healthy
presentation snapshot for recovery. Browser storage denial gives a session-only
setting rather than disabling playback. Cross-tab presentation changes are optional;
do not accept a storage event as an authenticated account change.

Server store: use relational records/typed columns for preferences used in queries,
permissions, social relationships or ranking. A small bounded versioned document is
acceptable for rarely queried display preferences, not for catalog/playlists/history.
Prefer a small number of cohesive preference groups with revisions over per-key
transactions that cannot validate related fields atomically.

## Proposed service/API additions

Paths below are planning names, not current routes. Evolve existing transports first.

| Proposed operation | Contract |
| --- | --- |
| Read account preferences | Authenticated owner snapshot, bounded typed groups/revisions, no secrets |
| Update account preference group | Existing WriteAuth/CSRF/Origin checks, validated allowlisted values, CAS/mutation ID, fixed-size request bound |
| Read client capabilities | Current account/server/source policy projection, platform engine capabilities supplied separately by the client |
| Read administrative effective configuration | Administrator-only redacted active/pending/locked values; no environment dump |
| Validate administrative change | Pure bounded validation/dry-run, no side effects or arbitrary file access |
| Apply administrative change | Administrator reauthorization, expected revision, durable receipt/audit; runtime-safe versus restart-pending split |
| Read/save queue checkpoint | Owner+device scoped stable occurrence snapshot with cursor/revision; bounded length and no credentials |
| Device control WebSocket | Authenticated device binding, command sequence/ack/idempotency, bounded event sizes and reconnect cursor |
| Read incremental library/playlist changes | Authorized revision cursor, tombstones, bounded pages, cursor-expiry full-resync fallback |

Use existing privacy/network/playlist endpoints for those domains. A new preference
endpoint must not write their tables directly. Return clear403/409/422 semantics
through existing error conventions; never echo untrusted provider bodies/secrets.

## Precedence and policy evaluation

Enforced safety/authorization policy constrains every effective value. Within allowed
values, use explicit per-device/per-account choice, account default when appropriate,
then product default. Operator hard ceilings cannot be loosened from the UI.
Environment/operator locks must be visible as safe lock reasons, not silently win
over a Saved label. Runtime state distinguishes active and pending policy versions.

An administrative quota change affects future allocations/leases. It cannot delete
assets or cancel another user's active stream without a separately defined policy.
Timeout/retry/security limits are reviewed bounded presets, not arbitrary knobs.
Do not expose disable-auth/disable-TLS/allow-private-SSRF as normal personalization.

## Capability model

Compute a capability snapshot from server support/policy, authenticated grants,
catalog origin, active source/encoding, local engine/OS and actual output device.
Represent supported, unavailable, denied, pending and temporarily degraded with
safe reasons. Advertise implemented behavior only, ideally with protocol version.

Feature flags hide unfinished development. Capability evaluation explains a shipped
feature's availability. Neither substitutes for authorization. The server rechecks
every stream/download/share/edit; the UI's active button is only a hint.

Examples: public metadata does not enable stream; browser output selection may be
unsupported; offline may support repeat-one before shuffle-all; host OS permission
can block background transfer; exclusive mode may conflict with DSP. Keep requested
settings when temporarily unsupported but display actual effective behavior.

## Additive data-model evolution

| Need | Proposed entities/invariants |
| --- | --- |
| Rich catalog | Recording/release/edition/asset distinctions; existing track IDs remain valid aliases/references |
| Stable queue | Random occurrence ID, origin+object ref, context/item provenance, bounded order/revision; duplicate tracks allowed |
| Native restore | Versioned owner+origin+device snapshot; paused cold restore; validate before resolving media |
| Smart lists | Typed bounded AST/rule revision; materialized static snapshots separate from dynamic rules |
| Metadata | Per-field provenance/edit revision; original asset immutable; source write-back explicit |
| Recommendations | Materialized owner set with algorithm/version, privacy epoch, catalog/pref revisions and expiry |
| Devices | Device identity, capabilities, scoped control session, expiring presence and command receipts; separate from listening consent |
| Social/network | Origin-qualified actors/objects, grants/invitations/blocks, delivery IDs/replay window/tombstones |
| Transfers | Durable item manifest/progress/representation validator, quota reservation and final publication receipt |
| Notifications | Actual event ID, audience, read state and delivery preferences; no duplicated fictional jobs |

Do not assign migration number0013 in advance: inspect the current maximum at
implementation and let the integrator own numbering. Index owner+scope+revision and
actual paging/filter dimensions. Benchmark query plans before extending indexes.

## Async consistency and safety

Every client request captures account/session/origin/route/query/generation/lifetime.
Physically cancel superseded work and independently reject late results. Source
selection and audio element/output generations remain separate from page generation.
An appearance change never rotates the source generation or privacy epoch.

While a reversible feedback write is pending, defer ordinary view refresh that would
replace its target; session/privacy/lifetime changes still invalidate immediately.
On timeout, refetch/reconcile by mutation ID before retrying. Do not assert failed
receipt means server rollback. A settings write to serverA cannot complete into B.

## Progressive source integrity decision

The current native path verifies a complete authorized original before decode.
Starting before final BLAKE3 verification changes that contract. Do not quietly
call partial bytes verified because a strong ETag exists.

Evaluate a server-authenticated versioned chunk-integrity manifest bound to the
immutable representation digest, size and authorized asset. Verify each required
chunk before releasing it to the decoder, including range seeks. Bound manifest
size/chunk count, cache/staging bytes, lookahead and background tasks. Keep manifest
transport/authentication equivalent to the stream; no public manifest of private assets.

Alternative: document a different authenticated progressive integrity model and its
security tradeoff explicitly, keeping verified complete-file playback as the default
until reviewed. A performance feature cannot silently relax the established source
contract. Do not pretend a manifest is implemented merely because this plan describes it.

## Migration and rollback sequence

1. Inspect current dirty tree/runtime/schema and preserve original files/data.
2. Reuse verified backup tooling; restore to an isolated copy and verify foreign keys,
   counts, representative media hashes and actual prior playlists before schema work.
3. Add fields/tables with safe private defaults. Backfill in bounded transactions.
   Existing users do not become discoverable or opted into history by migration.
4. Support legacy client/snapshot read until a documented version boundary. Reject
   newer unsupported snapshot versions safely rather than reinterpret their fields.
5. Run same-artifact upgrade and fixture-restore tests. A binary rollback after a
   forward-only schema change may require the preserved backup, not SQL down-migration.
6. Package exact tested assets and record source/package/runtime identities separately.
7. Do not publish, alter original runtime or replace real data without the required
   specific authorization. Ordinary source implementation is a separate future task.
