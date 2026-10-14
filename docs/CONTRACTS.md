# Initial integration contracts

The main agent owns domain DTOs, core, database migrations, server HTTP wiring and root manifests. Contributors own only their crate. Shared DTO definitions are in crates/domain/src/lib.rs. Migrations below are authoritative.

## Media crate contract
`MediaService::new(pool: SqlitePool, data_dir: PathBuf, max_file_bytes: u64) -> MediaService` (synchronous constructor). Cloneable service.
`async fn ingest(&self, owner_id: &str, staged_path: &Path, original_filename: &str) -> anyhow::Result<UploadResult>` validates actual media, metadata/artwork, exact BLAKE3 duplicates per owner, atomic durable publication. Caller controls streaming into staging and cleans its staged file; service may rename it. Original filename never controls path. Staging is data_dir/staging; managed audio is data_dir/media; covers data_dir/artwork. Service ensures directories via async `initialize()`.
`async fn list_tracks(&self, owner_id: &str, query: Option<&str>, cursor: Option<&str>, limit: u32, favorites_only: bool) -> anyhow::Result<TrackPage>` stable UUID keyset pagination (limit max 100), parameterized FTS5 search with safe token quoting. `async fn get_track(&self, owner_id: &str, id: &str) -> anyhow::Result<Option<Track>>`. Favorite join per owner.
`async fn asset(&self, owner_id: &str, id: &str) -> anyhow::Result<Option<MediaAsset>>` where MediaAsset contains `path: PathBuf, content_type: String, size_bytes: u64, hash: String` (server-only DTO). `async fn artwork(&self, owner_id: &str, id: &str) -> anyhow::Result<Option<PathBuf>>` authorize via track ownership. `async fn stats(&self, owner_id: &str) -> anyhow::Result<LibraryStats>`.
`pub fn select_range(header: &str, size: u64) -> Result<(u64,u64), RangeError>` single byte ranges; end inclusive; suffix/open ranges; reject invalid/unsatisfiable; multiple ranges may be ignored by transport as permitted by HTTP (serve full 200). Unit tests including u64 overflow.
Media Cargo manifest owned by media agent. Dependencies via workspace or declared locally.

## Imports crate contract
`ImportService::new(pool: SqlitePool, media: MediaService, data_dir: PathBuf, max_file_bytes: u64) -> ImportService`, cloneable.
`async fn preview(&self, url: &str) -> anyhow::Result<ImportPreview>` SSRF policy pinned DNS for each request and redirect, HTTP/S ports 80/443 only, no userinfo, credentials or private addresses, bounded requests; no body-buffering.
`async fn enqueue(&self, owner_id: &str, url: &str) -> anyhow::Result<ImportJob>` bounds active jobs, validates URL before persisting. `async fn list(&self, owner_id: &str) -> anyhow::Result<Vec<ImportJob>>` max 100 newest jobs. `async fn cancel(&self, owner_id: &str, job_id: &str) -> anyhow::Result<bool>`. `async fn retry(&self, owner_id: &str, job_id: &str) -> anyhow::Result<bool>`.
`async fn run(&self, shutdown: tokio_util::sync::CancellationToken) -> anyhow::Result<()>`: bounded serial worker, startup recovery running -> queued, persist progress, resumability is restart-from-zero initially, clean temporary files, cancellation, limits, direct downloads only. Provide extensible source interface without unnecessary abstraction. URL never logged verbatim. Failed job stores safe message no credentials. Worker must not drop per-file limits or owner isolation.
Imports Cargo manifest owned by imports agent.

## Web HTTP contract
Same-origin /api endpoints; JSON errors `{error: string}`. Cookies HttpOnly SameSite=Strict; set `X-CSRF-Token` from AuthResponse on POST/PATCH/DELETE. GET /api/setup returns SetupStatus. POST /api/setup and /api/login Credentials -> AuthResponse. GET /api/me AuthResponse. POST /api/logout. Admin POST /api/users Credentials -> User. GET /api/tracks?q=&cursor=&limit=&favorites=true -> TrackPage. GET /api/stats LibraryStats. POST /api/upload multipart with audio `file` field (one per request; UI batch sequentially), UploadResult. GET /api/tracks/{id}/stream HTMLMediaElement cookie-authorized Range stream. GET /api/artwork/{id}. POST /api/tracks/{id}/favorite `{favorite: bool}`.
POST /api/imports/preview ImportRequest -> ImportPreview; POST /api/imports -> ImportJob; GET /api/imports -> Vec<ImportJob>; POST /api/imports/{id}/cancel, /retry. Poll every 2s only when authenticated and jobs active.
GET /api/playlists Vec<PlaylistSummary>; POST /api/playlists CreatePlaylist -> PlaylistSummary; GET /api/playlists/{id} PlaylistDetail; POST /api/playlists/{id}/items PlaylistMutation (track_id); PUT /api/playlists/{id}/order PlaylistMutation (item_ids); DELETE /api/playlists/{id}/items/{item_id}?revision=&mutation_id=; POST /api/playlists/{id}/collaborators CollaboratorRequest; POST /api/playlists/{id}/shares ShareRequest -> ShareResponse; DELETE /api/playlists/{id}/shares (all revoke). Shared public metadata route GET /api/shares/{token} PlaylistDetail, streaming remains independently authorized. Stale edits HTTP 409.

Web crate built with cargo build -p vela-web --target wasm32-unknown-unknown, then wasm-bindgen --target web to dist/pkg; index.html imports generated bootstrap. Build script will be supplied by main agent. Dioxus single-page renderer; no dx dependency required. Web author owns app manifest, index.html, source, CSS and optional JS playback/upload bridge; prefer Rust for core UI state. Native CSS artwork placeholders allowed for missing user artwork, no fake catalog data. Browser file upload must stream browser File via FormData, never unbounded Rust byte buffers. Use web-sys FormData and fetch. Actual HTML audio provides playback and keyboard/media-session bridge.

## Integrated extensions

`MediaService::with_storage_budget(u64)` configures shared physical-audio accounting. Imports accept `with_limits(ImportLimits)` with the same budget. `AppState` holds an exclusive data-dir OS lock and a shutdown token; `drain_uploads()` retains all upload permits before SQLite closure.

GET /api/playlists/{id}/collaborators returns Vec<PlaylistCollaborator> (owner only). DELETE /api/playlists/{id}/collaborators/{user} lets an owner revoke a grant or an invited member remove their own grant; owners cannot leave their own playlist. Grants are bounded100 per playlist. UI lists/revokes collaborators and offers Leave playlist. These remain metadata/edit permissions; no stream grant is implied.

The real Cargo output is target/wasm32-unknown-unknown/{debug,release}/vela-web.wasm, transformed by scripts/build-web.sh to dist/pkg/vela_web.js. An external bootstrap.js starts the renderer under the CSP.

## Catalog grouping slice

Migration0002 persists owner-scoped artists/albums, backfills existing tracks and maintains associations on ingestion and metadata changes using database triggers. SQLite trim/ASCII NOCASE names are provisional catalog grouping, not recording identity; album key includes effective album artist and known year (zero means unknown). Empty album tags remain ungrouped. Track artist credits are kept as one string, never guessed/split. Group UUIDs remain stable independently of track IDs; orphan groups remain stored but are hidden from browsing. Composite ownership is enforced for album artists, and track association writes reject foreign owners.

New domain DTOs: AlbumSummary{id,title,artist,artist_id,year,track_count,duration_ms,artwork_id}, ArtistSummary{id,name,track_count,album_count,artwork_id}; AlbumPage/ArtistPage contain items and next_cursor. Track DTO remains unchanged.

Media implements list_albums(owner, cursor:Option<&str>, limit:u32)->Result<AlbumPage>; album(owner,id)->Result<Option<AlbumSummary>>; album_tracks(owner,id,cursor,limit)->Result<Option<TrackPage>>. Album tracks order by coalesced disc (1), track number (2147483647), then ID; cursor is the last track UUID looked up inside the authorized album, preserving bounded pages. Missing/foreign album returns None. Invalid cursor returns an error.

Media implements list_artists(owner,cursor,limit)->Result<ArtistPage>; artist(owner,id)->Result<Option<ArtistSummary>>; artist_tracks(owner,id,cursor,limit)->Result<Option<TrackPage>>. Artist-associated tracks include direct track credits OR the album artist; count once. Artist tracks use UUID order. Summary counts/artwork consider only the authorized owner's tracks. All page limits clamp1..100 and use limit+1; opaque UUID cursors validated. stats uses persisted visible groups.

Transport GET /api/albums, /api/artists accept cursor/limit; GET /api/albums/{id}, /api/artists/{id} return summary or404. GET /api/albums/{id}/tracks, /api/artists/{id}/tracks return TrackPage or404. No query search in this slice. UI fetches50 per page, never whole catalog/album; play controls say Play these tracks. Every UI async request must retain existing session guard plus generation/selected-object guards. Signing out clears group state; no fake content/previews.
