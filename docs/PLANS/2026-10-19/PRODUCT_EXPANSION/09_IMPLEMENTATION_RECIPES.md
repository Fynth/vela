# Concrete subsystem recipes

These recipes apply to every matching EX entry. Combine each feature's row-level
acceptance with its recipe, the settings registry, and the delivery package. No
recipe creates permission to discard existing data or bypass platform verification.

## R01 — Preferences and Settings shell

Start in existing client roots and shell, not a parallel frontend. Introduce a pure
validator/registry and device persistence adapter first. Define root services once;
pages only render values and dispatch commands. Add native Settings navigation,
web category routing, local label/alias search, stable deep links and scope labels.

First ship only functional presentation controls: theme, scale, text/density where
implemented, motion, reset and About. Preserve web's valid legacy theme. Introduce
account preference transport later with owner/revision snapshots. Test storage denial,
corrupt documents, source continuity and session replacement. Keep admin forms out
of listener Settings. Required settings cases: SET01–SET16 in QA.

## R02 — Application scale and responsive layouts

Follow the entire scale specification. Spike pinned WebView zoom in an owned native
window and semantic token scaling in web. Select one adapter per platform after
measured focus/hit-test/reflow results. Add percent/reset before advanced settings
so recovery never depends on a server. Apply once, persist once, keep audio untouched.

Do not let future artwork/grid components hardcode a minimum content width that
breaks200% text. Add screenshots for real Settings and Library/player at scale
extremes, not only a slider unit test. Complete SCALE01–14 before calling it done.

## R03 — Queue identity, journal and commands

Add stable occurrence IDs independent of track/object IDs. Preserve origin, logical
track reference and optional playlist-entry provenance. Queue edits produce typed
commands with a revision/mutation ID. PlaybackOrder stays framework-free and bounded;
structural edits synchronously remap it. Metadata/favorite updates do not reset it.

Persist account+origin+device context/cursor/position/modes without stream URLs or
credentials. Restore paused and validate legacy snapshots. Introduce side-effect-free
actual-next preview before prefetch; it must not consume shuffle entropy/history.
Undo uses bounded snapshots/receipts, not inverse title-based guesses. Property-test
duplicate inserts/removes/reorders/repeats plus real-source continuity and EOF.

## R04 — Cache, pinned downloads and transfer coordinator

Retain current media_cache/offline separation and same-descriptor verification.
Expose actual usage and validated budgets. Maintain one quota allocator across
staging/complete cache jobs; account+origin namespaces remain isolated. Quota reduction
is a reclaim plan for evictable inactive entries, never forced deletion of pins.

Introduce a single-flight transfer key including origin, account, asset representation
and validator. Consumers own independent leases/cancellation; one canceled prefetch
must not abort an explicit Play/Save still using the transfer. Staging is unique,
private, bounded and atomically published. Capture HTTP request counts in a real
fixture to prove transfer reuse and authorization revalidation.

For resumable downloads, persist byte ranges/validator and reauthorize before resume.
If validator/length changes, discard incompatible staging safely and start a new
representation. Finalize only after integrity checks. Offline manifests contain
stable metadata/art/lyrics refs and current status; no refresh token or arbitrary
remote path. Test restart/disk-full/cancel/two-process behavior, not only happy path.

## R05 — Actual-next preparation and progressive playback

After R03/R04, a root-owned planner requests an immutable actual-next candidate plus
queue/source/policy generations. One bounded speculative worker follows it, governed
by network/storage/power policy. Reordering, user Next, repeat mode, unavailable source
or account change invalidates the plan and releases unnecessary leases.

Do not infer “next” as visible index+1 in shuffle. Do not decode into another output.
Keep current interactive preparation ahead of speculation. For progressive sources,
complete the explicit integrity decision in architecture before implementation;
retain complete-file fallback for formats requiring end metadata or unsupported seeks.
Measure cold/warm/body-byte/start/seek latency separately from decoding and audio output.

## R06 — Audio effects, quality and native outputs

Negotiate from actual decoder/output/server capabilities. Direct original is preferred
when compatible. Optional FFmpeg runs through structured preset arguments, bounded
concurrency/time/output size, cancellation and no shell URL interpolation. Cache
transcodes by original digest+preset/version; expose actual lossy/lossless result.

For normalization parse trusted tags or compute standards-appropriate loudness in a
separate versioned job. Existing RMS/FFT features are not LUFS. Define gain/peak,
headroom/limiter, album versus track mode, clipping and unknown-tag fallback. Test
PCM fixtures numerically. EQ/crossfeed need validated stable filters and bypass.
Gapless/crossfade require a single explicit mixer/engine authority; scope claims to
actual tested codec/OS outputs. Exclude exclusive/bit-perfect modes when DSP/resampling
breaks those properties, and show the effective path rather than marketing labels.

## R07 — Catalog, editions, credits and search

Evolve current album/artist/track/asset identities additively. Separate recording,
release/edition and physical encoding without reassigning playlist references.
Represent many-to-many contributors with roles and preserve unknown values. Multi-
library membership becomes part of every query and media authorization decision.

Build normalized searchable fields alongside original display strings. Choose tested
Unicode normalization/tokenization and exact-first ranking; do not rely on SQLite
ASCII NOCASE for Cyrillic/diacritics. Optional fuzzy matching uses bounded candidates
and short-query thresholds. Add indexes/query-plan evidence for real sorts/filters;
never scan a million rows to render a client-side alphabet jump. Virtualized rows
retain stable item IDs and keyboard focus. Search scopes execute through typed adapters.

## R08 — Metadata, artwork and lyrics

Preserve originals and explicit user edits. Embedded tags first, remote suggestions
with provenance/confidence second. A metadata review mutation is revisioned, field-
scoped and cannot silently erase recent manual edits. Bulk edits preview real changes.
Source write-back is a separate privileged job with diff/backup and read-only handling.

Artwork loading reauthorizes access, caps compressed bytes/dimensions/decode memory,
uses a blocking pool and bounded derived-size cache. Native uses an authorized asset
adapter, not a leaked cookie-bearing URL in public WebView HTML. Extract accent colors
from permitted images and validate contrast, rather than setting text to cover colors.

Lyrics: bounded embedded/plain/LRC parser; sanitize text, clamp timestamps, support
unsynced fallback and personal offsets. Provider access is explicit/licensed. Lyrics
never become unauthorized scraped content. Store copyright/provenance and offline
authorization separately from the existence of a metadata field.

## R09 — Playlists, rules, collections and portable formats

Use stable entry IDs, CAS revisions and idempotent mutations already established.
Add description/cover/folder features without copying another playlist's grants or
share tokens. For concurrent reorder, rebase semantic moves by stable entry neighbors
or return a conflict/diff; never replace the entire stale ordered list silently.

Smart playlists store a bounded typed AST: allowed fields/operators, max depth/node
count, sort and count/duration limits. Compile through parameterized SQL; user strings
are values, not SQL fragments. Use cost/row/time bounds and documented unknown-value
semantics. Dynamic results and static copies are different entities.

M3U/M3U8 import is a metadata resolver, not authority to open filesystem paths or
fetch arbitrary URLs. Preview matches/unknowns, preserve duplicate order and resolve
only approved local/connected sources. Export documented UTF-8 formats without tokens.
External playlist integrations import authorized metadata only; content matches must
come from legitimate accessible media.

## R10 — Private recommendations, DSP and scheduled mixes

Keep candidate retrieval, authorization filtering, ranking, diversity, explanation
and materialization distinct. Existing bounded content/favorite/private-history
baseline remains functional without models. Never query retained-but-disabled history,
even just to suppress plays. Hidden choices are separate reversible preferences.

Add durable scheduled snapshots with algorithm/catalog/policy/privacy versions,
deterministic seeds and explicit refresh timestamps. A weekly set may be small/empty
for a five-track library; never recycle known tracks while calling them unseen.
Filter inaccessible and hidden candidates before expensive limits/ranking. Record
candidate coverage, repeat fraction and latency locally with consent-minimal telemetry.

DSP work remains separate from streaming, using immutable asset+extractor cache and
bounded supervised decoders. Algorithm changes version outputs. Tempo/key/mood claims
require validation on annotated material, not synthetic FFT correctness alone.
Similarity journeys use actual descriptor distances and deterministic bounded graph
search; embeddings/speech are optional experiments, not requirements for discovery.

## R11 — History, statistics and reflection

Extend the existing cumulative interval/consent foundation. Native collection needs
current account/device consent, privacy revision and corroborated advancing clock.
Pauses/retries preserve one logical session; explicit new selections/replay after EOF
start another. Skips/seeks/downloads do not count as listened time.

Create aggregates from retained qualified records using timezone-aware windows and
idempotent incremental updates. Delete/retention invalidates/rebuilds dependent sets
transactionally. Distinguish private personal totals from public popularity. Share
cards are previewed explicit exports with separate visibility filtering. Scrobbling
uses an opt-in credential-backed outbox; external retention/deletion limits are clear.

## R12 — Devices, incremental synchronization and handoff

Introduce a device registry distinct from history-consent devices and an authoritative
per-device playback session. Save occurrence queue/bookmarks separately from live
control. Device commands carry session epoch, sequence, idempotency and narrow scope.
Use heartbeat expiry/reconnect snapshot and bounded WebSocket channels.

Handoff transaction: request destination capability/authorization; prepare destination;
capture source position/clock; destination accepts track/queue and reports ready;
coordinator commits one transfer epoch; source pauses only on accepted transfer.
Timeout/failure retains source playback. Late destination autoplay after canceled
handoff is rejected by transfer epoch. Define how seeking/queue edits during prepare
invalidate or update the transfer; do not quietly race two versions.

Measure with two real clients and separate authenticated sessions. Include A offline,
B unavailable, expired grants, repeated command delivery and source progress during
prepare. Independent listening stays independent unless transfer is explicit.

## R13 — Social, rooms and federated catalogs

Local profiles/follows/invitations/blocking precede cross-server delivery. Keep view,
stream, download, edit and re-share separate. Social activity and presence are opt-in,
not a side effect of collecting private history or enabling network discovery.

Rooms synchronize commands/timing and queue state; they do not grant media or relay
restricted third-party streams. Reauthorize participant media, bound suggestions/
votes/events and use authoritative snapshots/sequences/clock estimates. Reconnect,
host transfer/expiry and blocked participant behavior are explicit.

For federation evaluate ActivityPub/WebFinger/signed delivery and write a versioned
music profile before coding interoperability. Public URL fetches retain DNS/IP/
redirect/rebinding limits. Trusted LAN storage/casting has a separate operator policy.
Native Vela/OpenSubsonic are catalog adapters, not substitutes for federation.
Optional OIDC identity binds issuer+subject after proof; it is not music ownership.

## R14 — Native platform delivery

Linux first: preserve current Wayland/X11 work and shared builder, validate real
output/controls/packaging separately. Android: Media3 foreground/media service owns
player/session, Rust commands bridge by versioned typed messages; activity destruction
must not own playback lifetime. iOS: AVPlayer/AVAudioSession, background mode, remote
commands/interruption/route lifecycle and secure local files require Apple builds.

Windows/macOS need actual runners/native runtime checks and packaging, not a shared
WASM success. Native settings use common validation but platform adapters for secure
storage, outputs, notifications, file picker, background transfer and scale. Record
compile/run/hardware classes separately; do not promise entitlements not obtained.

## R15 — Ingest and operational settings

Reuse existing import/scanner/dedup/quotas instead of a parallel upload API. New
adapters declare recognize/preview/asset/license/cancel/resume capabilities and route
all egress through controlled infrastructure. Archives have strict path/file/depth/
uncompressed limits. WebDAV/S3 credentials are scoped and secret-backed; trusted
internal sources cannot widen ordinary-user URL access.

Admin settings expose safe effective configuration and validated bounded changes.
Never allow arbitrary FFmpeg commands, filesystem paths, secrets exports or disabled
authentication. Existing durable jobs remain the source of truth. Add analysis/
download projections only after their real lifecycle is integrated. Backup/restore/
upgrades require actual isolated restore evidence and deliberate original-data
handover. Complete license/advisory gates before external release claims.
