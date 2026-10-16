# Vela Music

A Rust-first, self-hosted music cloud with a Dioxus web interface. This is an early implementation under active development, not a finished alternative to established servers.

Upload real audio files, browse albums/artists with embedded metadata and artwork, search, favorite, and stream original files. Album tracks follow disc/track order; large collections use bounded pages. Filter tracks by complete genre tag, format, release years and reported audio properties, combine with search/favorites, and sort by title or recently added. Applied conditions remain stable while paging; unsaved form changes stay drafts. Edit your catalog tags in the app with revision protection while keeping original audio files unchanged. Import supported public HTTP/HTTPS audio download links through the app with durable progress/retry/cancel controls. Create private playlists, grant local accounts viewer/editor access, and share expiring, revocable metadata-only links. Owners can list/revoke collaborators; members can leave. The browser player has seeking, volume, shuffle/repeat, queue controls and account-scoped queue persistence.

Administrators can scan configured read-only music folders in **Settings → Music folders**, watch progress, cancel, and rescan without changing original files or existing catalog edits. Removing a scan source keeps imported music.

**Jobs** brings your URL imports and administrator folder scans into one private, paginated history. Filter by type/state, inspect actual progress, cancel eligible work, retry imports or start a new scan pass. Active tasks refresh while the page is visible; idle history has a Refresh control. Uploads remain in their upload dialog.

Normal music uploads, direct URL imports and configured folder scans use the web interface. No SSH is needed for those workflows after installation. No catalog content is preloaded.

## Run locally

Install the Rust toolchain declared in `rust-toolchain.toml` and the matching WebAssembly binding generator:

```bash
cargo install wasm-bindgen-cli --version 0.2.129 --locked
scripts/build-web.sh
cargo run --locked -p vela-server
```

Open **http://127.0.0.1:3210**, create the administrator account, and choose **+ Add music**. FLAC, MP3, Ogg Vorbis/Opus, WAV and other files supported by the metadata reader may be uploaded; actual playback depends on your browser. No transcoding is implemented yet.

For a faster development web build, set `VELA_WEB_PROFILE=dev`. The server serves the generated `dist/` assets and uses `data/vela.db` plus managed media/artwork/staging directories.

## Docker

```bash
docker compose up -d
```

Compose builds locally, runs as a non-root user, persists a named volume and binds to loopback. First builds download Rust dependencies. The image recipe and Compose configuration are included; consult [verified checks](docs/PROGRESS.md) for whether this host's container runtime was available. AMD64 and ARM64 are architectural targets, not validated platform claims.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `VELA_BIND` | `127.0.0.1:3210` | HTTP listener |
| `VELA_DATA_DIR` | `data` | Database and managed files; exclusive to one server process |
| `VELA_WEB_DIR` | `dist` | Built web assets |
| `VELA_COOKIE_SECURE` | `false` | Set `true` behind HTTPS |
| `VELA_MAX_FILE_BYTES` | `262144000` | Per-file ceiling, 250 MiB |
| `VELA_MAX_STORAGE_BYTES` | `10737418240` | Managed physical audio budget, 10 GiB |
| `VELA_SCAN_ROOTS` | `data/sources` as Music folder | Operator-approved folder JSON; `[]` disables scanning; see Operations |
| `RUST_LOG` | application info | Structured diagnostics, without request URLs/cookies |

Complete initial setup through loopback before enabling public access. Use HTTPS for remote access and enable secure cookies. [Operations](docs/OPERATIONS.md) describes Caddy, systemd, backup/restore and upgrades. Public registration and federation are unavailable; accounts are created by the administrator in Settings.

## Development and evidence

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace -- --test-threads=2
cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings
scripts/build-web.sh
npm ci --prefix tests/browser
(cd tests/browser && npx playwright install chromium && npx playwright test)
```

See [progress](docs/PROGRESS.md), [architecture](docs/ARCHITECTURE.md), [roadmap](docs/ROADMAP.md), [security](docs/SECURITY.md), [performance](docs/PERFORMANCE.md), [known limitations](docs/KNOWN_LIMITATIONS.md), and [research](docs/RESEARCH.md). CI configuration does not mean CI has run. Native mobile/desktop, offline downloads, multi-server access, OpenSubsonic and federation remain roadmap work.

AGPL-3.0-or-later. Real test audio is locally generated sine-wave material. Optional live-import tests use Wikimedia's explicitly licensed example audio, with attribution recorded in `tests/browser/README.md`.
