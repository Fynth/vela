# Verified progress

Checkpoint2026-10-14. The initially empty repository is a six-crate Rust/Dioxus modular monolith. Actual specialized agents contributed media/import/UI and catalog/security review; the principal reviewed and integrated the shared contracts. No external publishing occurred. This is a functional early product slice, not completion of the master roadmap.

## Implemented and verified

- Atomic first-administrator setup, Argon2id accounts, hashed random sessions, CSRF/origin/rate controls and administrator-created users. Private catalog/artwork/stream ownership is independently enforced.
- Real sequential batch uploads, bounded Lofty metadata/artwork processing, exact BLAKE3 deduplication, immutable publication, FTS search/favorites/stats and original HTTP range streaming. Multipart headers are bounded before parsing; canonical opening boundaries prevent a reviewed preamble bypass. Oversized headers/additional parts fail before publication.
- Durable bounded direct HTTP/S imports with pinned public egress, limits/progress/retry/cancel/restart recovery. Real HTTP download tests ingest actual audio; no fake source adapter responses.
- Private local collaborative playlists: viewer/editor grants, collaborator list/revoke/leave UI, transactional revisions, actor/request-bound idempotency, expiring/revocable metadata-only links. Public views omit private contributor entries; permission to read playlist metadata never grants streaming.
- Persisted private album/artist identities, migration/backfill and automatic transactional regrouping. Owner-isolated summaries/track pages, album disc/track ordering, bounded cursors and visible-group counts. Original audio and embedded-source strings remain intact. These are provisional catalog groups, not verified recording/edition identities.
- Owner-only catalog metadata editing with typed bounded fields, audit, revision CAS and explicit stale-draft reload. Real tests verify search/group refresh, unchanged audio bytes/hash, private same-file catalogs, dedup preservation and restart persistence.
- Browser playback controls, duplicate-safe queue ordering, account-scoped persistence and stale-response guards. OS MediaSession metadata is cleared on logout. New album/artist pages use real API data and bounded50-item requests; their browser validation passed.
- Exclusive data-dir lock, recognized temporary-file cleanup, supervised durable worker, graceful drain, standard-library SQLite/media backup with safe restore. A shutdown race was reproduced and fixed;20 isolated clean restarts and deliberate fixture worker failure were previously verified.

## Executed checks

- Latest `cargo test --locked --workspace -- --test-threads=2`:64 passed (3 core,13 imports,10 media unit,6 catalog integration,8 ingestion integration,4 metadata integration,3 multipart guard,17 real TCP API). Includes legacy migration, compilation-integrated grouping, private cursors, FTS integrity, actual upload/stream/sharing/import recovery, bounded concurrent streams and concurrent uploads, backup restore and public/static isolation.
- Latest strict native Clippy (`--workspace --all-targets -- -D warnings`):passed. Strict WASM Clippy and native/WASM release builds passed.
- Five Python backup regressions passed. JavaScript syntax and Compose configuration passed. Locked dependency policy checks passed with the documented single unmaintained compile-time paste exception (SECURITY.md); no runtime vulnerability advisory is ignored.
- Nine updated isolated headless Chromium E2E passed against production native/WASM builds. They covered setup/upload FLAC/metadata/artwork/audio decoding/time advancement/favorites/playlist sharing/revocation, malicious URL/mobile layout, a real licensed Wikimedia URL import and stale search/account responses. Album/artist playback, latest imported Ogg decoding and stale album/artist selection were also verified. Desktop/mobile screenshots were inspected; no horizontal mobile overflow was observed. Revisioned metadata save/two-tab conflict/reload/mobile dialog and cancellation across account changes also passed. An initially hidden native dialog was caught by E2E and fixed by opening after the DOM batch. Physical sound/background/mobile hardware behavior is unverified.
- Synthetic SQLite10k/100k/1M and adaptive sparse100k search measurements are documented in PERFORMANCE.md with exact scope. They are not HTTP/concurrent playback performance or product-superiority claims.
- Docker CLI exists but its daemon is unavailable: no image build/runtime claim. ARM64, Safari/Firefox, desktop/mobile packaging, actual devices and remote CI are unverified.

## Resume checkpoint

Latest fmt, strict native/WASM Clippy, native/WASM release builds,64 Rust tests,9 browser tests,5 Python backup tests and20 release SIGTERM/restart runs passed; deliberate fixture worker failure correctly stopped the process. Git checkpoint738bb25 preserves the preceding browsing slice. Git checkpoint e61d963 contains the verified metadata editing code. Local native/server+WASM and exact-source archives were built from e61d963 under target/releases; SHA256SUMS passed. verify_package.py successfully extracted the native package, ran its bundled web assets with default storage, verified empty first-run state/HTML/WASM and clean SIGTERM exit. Later checkpoint changes only persist packaging evidence. No external release is authorized. Next priority is authorized read-only scanner roots/durable progress and preserving catalog edits during rescans. Further source adapters, invitation/media grants, multi-server, federation, OpenSubsonic and native/offline remain in ROADMAP.

Architecture/security/operations/roadmap/limits/ADRs/research docs are maintained. Cargo.lock is included in the checkpoint source. Test data is isolated in temporary directories; no synthetic catalog is shipped.

## Local development instance

The verified release server was started on127.0.0.1:3210 with /home/rasul/Projects/vela/data and built dist assets. Health returned ok/version0.1.0 and setup remained unclaimed; no administrator credentials or music were created outside isolated tests. This persistent development directory is ignored by Git and may now receive user data. On resume, check/reuse the listener and preserve data; never reset it. Stop gracefully before replacing the running server or applying upgrades.
