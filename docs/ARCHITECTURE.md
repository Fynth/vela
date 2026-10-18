# Authoritative architecture

## Private listening and local audio discovery0.9

Additive migrations0008/0009 preserve existing catalog, playlist and media semantics. Domain listening/audio_analysis modules own framework-free contracts. Core ListeningService owns explicit account/device consent, CAS privacy revisions, cumulative idempotent sessions, private keyset history, export/delete and retention. The server adds authenticated/no-store listening routes, CSRF writes and an hourly supervised physical retention task. Root BrowserPlayer owns a conservative monotonic/media-time collector beside its existing source generations. Source recovery is not a new listening occurrence; actual replay after EOF is. Account changes, consent changes and erasure invalidate pending work and export generations.

Media AnalysisService owns a separate durable versioned derived cache and one bounded blocking Symphonia/RustFFT worker. It does not depend on listening consent or perform automatic network requests. Explicit owner-authorized analysis batches are distinct from streaming. Radio reads at most500 owned candidate feature vectors and actual tags in a SQLite snapshot, diversifies deterministically and returns real track IDs/explanations. The Dioxus radio page receives existing root Actions and never creates audio. Queue/storage/auth/metadata-only playlist permissions remain unchanged. ADR0013/0014, AUDIO_ANALYSIS, RECOMMENDATIONS and CONTRACTS specify limits. Native engines, multi-server/federation and personal recurring mixes remain separate milestones.

## Listening experience increment (0.5.0)

Keep the seven-crate boundary and schema7. SearchQuery/SearchResults are domain DTOs; media owns bounded track/group matching, core owns collaborator-aware playlist search, and GET /api/search only authenticates/validates/composes. Each category is limited8 in SQL. Group/playlist names use parameterized literal ASCII-case-insensitive substrings; tracks retain FTS. There is no federated/global search or cross-service snapshot guarantee.

Web client.rs owns typed same-origin transport; new Home/Search pages own cancellable request groups and exact session/query/generation/lifetime checks. Presentation receives root Actions rather than accessing storage. Home uses real bounded newest/favorite/group/playlist APIs. Root App retains the only audio element, queue occurrence/cursor and playback handlers while shell.rs handles navigation. Closed hash destinations support browser history without putting credentials in routes. Audio play promises are guarded by element/source/request identity; navigation never replaces the audio source. ADR0009 and PRODUCT_SPEC define the incremental consumer UX. Existing modules and API contracts remain compatible.

Vela is a modular monolith: one Axum/Tokio process, one local SQLite database, immutable managed media and generated web assets. Edition2024; Rust1.99.0 pinned. No Redis, mandatory FFmpeg, external search engine or cloud service. Dioxus0.7.10 compiles to WebAssembly; HTMLMediaElement owns browser decoding. Domain contracts do not depend on HTTP, SQL or UI frameworks.

| Boundary | Current responsibilities |
| --- | --- |
| `vela-domain` | Framework-independent serialized API contracts and opaque IDs |
| `vela-core` | Configuration, WAL/FK migrations, Argon2id accounts, sessions, private playlist services and bounded background-task read model |
| `vela-media` | Bounded metadata/image validation, BLAKE3 asset identity, immutable publication, owner-authorized catalog/asset lookup, byte-range selection |
| `vela-scanner` | Read-only directory capabilities, bounded snapshots, persistent scan sources/jobs/fingerprints |
| `vela-imports` | Source interface, controlled egress, durable bounded direct downloads and job recovery |
| `vela-server` | HTTP routing, auth/CSRF extraction, browser origin checks, multipart streaming, response headers, semaphore lifetimes, worker supervision |
| `vela-web` | Dioxus interface, same-origin API calls, account-scoped queue state, native browser audio/File APIs |

```mermaid
flowchart LR
 Browser[Dioxus web app] -->|cookie + CSRF for writes| HTTP[Axum API]
 HTTP --> Core[Accounts and playlists]
 HTTP --> Media[Catalog and ingestion]
 HTTP --> Scanner[Durable folder scans]
 Scanner --> Roots[Operator-approved read-only folders]
 Scanner --> Media
 Scanner --> DB
 HTTP --> Imports[Durable import queue]
 Imports --> Egress[Pinned and validated outbound HTTP]
 Egress --> Internet[Public downloadable audio]
 Imports --> Media
 Core --> DB[(SQLite WAL + FTS5)]
 Media --> DB
 Imports --> DB
 Media --> Disk[Immutable audio + JPEG artwork]
 HTTP -->|authorized original byte ranges| Disk
```

## Playback order (0.6.0)

The existing root uses a PlaybackUi bridge for all queue starts/selections/edits/transport commands. Its framework-free playback.rs model emits explicit transitions over occurrence indices, with a bounded unplayed shuffle deck, history cursor and manual-next priority. Insert/swap/remove remap all references synchronously; catalog-only tag/favorite updates leave traversal intact. A full repeat-all cycle includes every occurrence and prevents immediate boundary replay when alternatives exist. Previous and natural EOF have distinct semantics. Queue/cursor storage remains per account, legacy restore is capped1000, and duplicate cursor-persistence effects are consolidated. Version0.7 extends this with an atomic validated account bookmark and restores that account’s volume/modes/context/position while rebuilding transient traversal. ADR0010 defines exact invariants and limitations. No new DB or native engine is introduced.

## Persistence and identity

Migration0001 defines accounts/sessions, physical media assets, private tracks, artwork, FTS5 triggers, favorites, durable import jobs, playlists/items/collaborators/mutation history, hashed expiring shares and audit events. Foreign keys and constraints are enabled on all pool connections; four connections,10s busy timeout,15s acquire timeout, WAL and FULL synchronous mode. Transactions preserve playlist revisions and import state.

Local track UUIDs identify private catalog entries. Physical asset UUIDs identify encoded files; BLAKE3 identifies exact bytes and permits shared disk storage without shared catalog access or disclosing duplicates across accounts. Migration0002 persists private artist/album UUIDs and maintains associations transactionally for existing and new tracks. Album keys include effective album artist and release year; original embedded strings remain available. This provisional grouping does not establish recording/master/edition identity (ADR0004). Future remote references must carry origin server+object identity; remote cache entries must remain distinct from locally owned assets.

## Upload/publication

Two concurrent uploads stream into random staging paths with per-file/body/time limits. A framing guard caps part headers at8KiB, rejects additional parts, splits parser input into16KiB frames and yields between frames to bound eager multipart read-ahead. The browser's batch upload sends one file per request. Metadata validation/copying/image processing use one shared semaphore and blocking workers. Client cancellation during transfer removes the temporary upload. After complete receipt, a bounded detached task owns the upload permit and staging path and finishes atomic catalog publication even if the client leaves. Shutdown drains these tasks before closing SQLite.

Ingestion makes an immutable copy, hashes while copying, validates content and tags, checks managed physical-audio quota, publishes with no replacement and file/directory fsync, then commits catalog rows. A crash between immutable publication and database commit can leave a retained unreferenced blob. Startup removes only recognized unpublished temporary files, never published content or arbitrary files. Reconciliation remains a tracked requirement.

## Streaming

Auth determines asset ownership before opening files. Original bytes stream in64KiB buffers; one permit remains owned by each audio/artwork response body until completion/cancellation, maximum16. GET supports inclusive/open/suffix ranges,416 with total-size metadata, ETag and If-Range, weak If-None-Match comparison. HEAD ignores Range. Unsupported/invalid range units or multiple ranges may receive the full representation. File size is checked against persisted metadata.

## Imports

The adapter/egress interfaces separate source resolution from durable jobs. One serial worker claims queued jobs, stores progress, checks transfer/file/queue/disk bounds, validates received audio through the same media service, and atomically transitions status. Restart requeues interrupted work and starts transfers again from zero; successful ingestion is exact-deduplicated. Processing is a noncancellable atomic phase. Transient network failures retry with bounded attempts; user retries are explicit. SQLite stores durable progress; UI polls active jobs every2s. SSE is planned.

The transport disables proxy inheritance, redirects, credentials and compression; every redirect gets fresh validated/pinned DNS resolution. Public import never gains trusted-local storage access. Infrastructure worker failure stops the serving process instead of silently losing the worker while health reports success.

## Collaboration and privacy

All content begins private. Named local collaborators can view playlist metadata or mutate when granted edit permission. Mutations run BEGIN IMMEDIATE transactions with a compare-and-increment revision, valid mutation UUID, actor/request fingerprint and ordered stable entries. Reusing an identical mutation returns current state without applying it twice; conflicting stale revisions receive409. Shared titles never authorize original audio. Streaming remains owner-only for this slice. Expiring random link tokens are hashed at rest, and links use URL fragments in the browser to avoid carrying the token in initial-page URLs.

## Planned platform boundaries

Native engines will implement shared playback contracts behind Android Media3, iOS AVFoundation and an evaluated desktop backend. Offline manifests, device/session state, incremental synchronization and multi-server adapters are separate milestones. Federation will use ActivityPub/ActivityStreams, WebFinger and peer authorization, with explicit visibility for profile/metadata/audio/downloads; no federation routes or public directory exist currently.

Client asynchronous actions are bound to the initiating CSRF session identity. An old task is dropped before it resumes after a session change, and user-specific draft controls reset. Library request generations independently prevent older search/page responses overwriting newer ones. Queue order tracks occurrence indices so repeated copies of the same song do not move the playing indicator to the wrong occurrence. Both behaviors have real browser regression coverage.

Native shutdown distinguishes expected worker completion after cancellation from infrastructure failure. A30s HTTP drain deadline prevents slow connections from indefinitely holding shutdown; atomic import/upload processing still drains before SQLite closure. The process lifecycle helper tests repeated real SIGTERM/restart and deliberately broken isolated import-job and scanner-job storage.

## Catalog metadata edits

Migration0003 adds an owner-private metadata revision. GET/PATCH metadata snapshots use a typed complete update and a16KiB body bound. One BEGIN IMMEDIATE transaction verifies ownership, validates fields, checks revision, changes only catalog tags, updates grouped identities/FTS via triggers and records a fixed audit event. Stale writes receive409. The native browser dialog retains the draft and offers explicit reload; its async work is scoped to session/editor generation and the mounted component. Originals, technical properties and asset hashes remain unchanged; exact re-ingestion cannot overwrite an edited owner catalog. ADR0005 records the boundary.

## Read-only folder snapshots

Migration0004 adds private scan sources, durable bounded jobs/counters and a per-source file map. Triggers enforce source/track ownership even on reassignment. Administrator source APIs expose configured root IDs/names only, never host paths. VELA_SCAN_ROOTS is operator configuration; absent configuration provides only data/sources. Explicit [] disables it. Configuration rejects unsafe/overlapping roots and aliases into managed data.

On Linux, cap-std/cap-fs-ext opens every root component and child directory without following symlinks. A bounded descriptor-relative depth-first walker admits regular, singly linked audio files and rejects symlinks/special files. Blocking traversal/copy runs on the blocking pool. A64KiB snapshot copy checks file size, free space, deadlines/cancellation and descriptor changes, then fsyncs and enters the shared bounded media pipeline. Originals are never modified. The scanner uses one worker alongside imports, and both are supervised after the HTTP listener is acquired.

Cached size/mtime/ctime/device/inode signatures and hashed relative paths skip unchanged files; this is best-effort change detection. New/changed content is exact-deduplicated into immutable managed storage; metadata edits survive repeated scans. Missing source files and source removal never delete owned music. Shutdown requeues interrupted scans; explicit cancellation retains already published tracks. The UI displays actual job counters with active-only polling. Linux is the implemented scanner platform; blocking kernel/network filesystem operations have no hard cancellation guarantee. See ADR0006.

## Typed catalog filtering

Migration0005 adds owner-prefixed indexes for trimmed genre/codec predicates, release years and title/newest order. Migration0006 keeps the genre prefix/order and appends raw genre to make grouped facets covering. A framework-free TrackFilters value and closed TrackSort enum compose with the existing owner/search/favorite predicates; query values are SQL parameters. The legacy list_tracks method delegates with defaults. The transport bounds/deserializes query forms and maps service validation to fixed JSON400 responses. GET /api/catalog/filters returns private, bounded genre/format suggestions from a coherent SQLite snapshot.

All pages return at most100 tracks plus one internal sentinel, with50 used by the UI. Default UUID order remains compatible. Title/newest cursors resolve a sort key only within the owner and compare the full key/UUID tuple, without OFFSET. Explicit NOCASE belongs on the title cursor's bound RHS so SQLite can seek the full tuple index. Foreign/missing sorted cursors share the same error. These are live pages; concurrent metadata changes can move boundaries between requests.

The Dioxus form separates draft values from accepted conditions. Search/filters are committed only when the latest authorized request succeeds; pagination and refresh read that accepted state. Invalid drafts preserve displayed results and invalidate older pending requests. Account changes clear all conditions and suggestions. Facet requests also carry session/revision/generation guards and fetch only while their form is visible. ADR0007 records units, collation and losslessness limits.

Core's build.rs watches the migrations directory, including newly added files, so stable incremental builds rerun SQLx embedding. An integration test checks the embedded filter/covering migrations, and native review packages record/verify their schema version against the created SQLite database. See [SQLx migration recompilation](https://docs.rs/sqlx/0.8.6/sqlx/macro.migrate.html) for the upstream stable-build requirement.

## Unified background tasks

Core JobService projects the existing durable import/scan tables, without owning their execution or duplicating state. Each owner-scoped indexed branch reads limit+1 rows; a bounded merge uses created_at/UUID/kind descending. The cursor includes every tie-breaker and remains valid after its anchor is removed. Migration0007 adds history tie-breakers and owner/status indexes. Rows, joined action hints and filter-independent owned activity share one WAL read snapshot. At most100 jobs are returned (50 in the UI); no OFFSET or whole-history materialization is used.

GET /api/jobs exposes no source URLs/paths/raw diagnostics. Ordinary accounts see only their imports; administrators additionally see their own scans. Import track titles/IDs and scan source names/IDs use owner-constrained joins. Existing services independently authorize cancel/retry/scan-again requests. A new scan is a new pass; the old history is retained until its source is removed.

Dioxus Jobs provides type/state filters, latest/older pages, real byte progress or indeterminate scan counters, errors/stale snapshots and actual mutation/navigation controls. Mounted visible pages poll while any authorized owned work is known active, even if filters hide it. Idle pages require refresh; leaving/logout invalidates timers and pending results. Exact session/generation/selection/lifetime guards prevent stale updates. Legacy import polling stops on Jobs. Catalog aggregates refresh on terminal/active polling so already published files survive cancel/failure visibility. Browser uploads remain in their transfer dialog, not the durable history. See ADR0008 and CONTRACTS.

## Playlist view lifecycle0.6.1

The root PlaylistView centralizes session/route/object/generation checks for detail opens and post-mutation refresh. Navigation invalidates the view even when the user later returns to the same route or object. Refresh cannot replace a newer selected revision. Share URL and collaborator UI changes retain their initiating view; a late response for A is discarded after B is selected. Existing server authorization, playlist CAS/idempotency and metadata-only links remain authoritative. This extends ADR0009's stale-request contract without another crate or migration.

## Browser resume and recovery0.7

session.rs validates versioned owner-bound bookmarks with a4MiB serialized cap,1000 occurrences, canonical UUIDs and finite bounded seek/preferences. Root App restores paused without setting a media src; PlaybackUi synchronizes occurrence edits and real Library/Home/Search/Album/Artist/Playlist provenance. Selecting a playlist row uses its stable entry ID in the complete stream-authorized subset, retaining duplicate occurrences and separate media permissions.

browser_player.rs is the single audio adapter, not another UI player or server service. It owns source generation/element/session identity, pending metadata seek, play-promise identity, bounded recovery timers, abortable10s HEAD probes and an8s actual-progress watchdog. Browser callbacks use wasm_bindgen_futures::spawn_local with explicit lifecycle cancellation and session/source guards. Success does not replace auth;401 releases audio/private state and returns to sign-in. Failed/obsolete work cannot reset a newer occurrence, including repeated URLs. Atomic snapshots flush on edits/seek/pause/page lifecycle and at most once per5s during time updates. Legacy structural keys remain for rollback; shuffle traversal is rebuilt and is not a listening-history journal. ADR0011 documents limits.

## Root Now Playing view (0.8.0)

ADR0012 adds presentation-only now_playing.rs beside the existing browser_player/session/order modules. Root owns its visibility, per-open feedback generation and actions; it neither owns another audio element nor performs HTTP. The bottom bar, native modal and Media Session bridge all control the same source. Native dialog focus/inertness is separate from legacy overlay trapping. Favorite commits update library/catalog/queue consistently while local error/busy feedback matches session, generation and occurrence. now-playing.css is an explicitly allowed static asset included in build/package checks. Schema7, HTTP contracts, storage and upstream pins remain unchanged.

Browser0.8.1 projects the current queued metadata duration during owner-bound sync only while the source is unloaded. After metadata arrives the browser remains authoritative. Both bar/modal consume that single root duration, including pre-load seeking; sync does not start audio, modify generations or reset traversal.

## Private on-demand recommendation read model0.10

MediaService owns candidate eligibility, favorite/qualified-history anchors,
validated descriptor affinity and artist/album re-ranking in recommendations.rs.
It reuses discovery.rs rather than adding a framework/service/ML dependency.
One read snapshot captures current core-owned privacy preferences, retained
qualified sessions, owned assets and explicit exclusion preferences. No derived
history cache requires erasure coordination. A default-off request executes no
history SQL. Server transport authenticates and bounds typed GET/PUT/DELETE;
web PersonalMixes inside existing Home owns only presentation and guarded
requests. Playback remains the single root HTMLMediaElement. ADR0015 and
CONTRACTS define bounds and ownership; daily/weekly refresh and remote/native
recommendations remain independent milestones.
