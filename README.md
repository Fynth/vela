# Vela Music

A Rust-first, self-hosted music cloud with a Dioxus web interface. This is an early implementation under active development, not a finished alternative to established servers.

Upload real audio files, browse albums/artists with embedded metadata and artwork, search, favorite, and stream original files. Album tracks follow disc/track order; large collections use bounded pages. Filter tracks by complete genre tag, format, release years and reported audio properties, combine with search/favorites, and sort by title or recently added. Applied conditions remain stable while paging; unsaved form changes stay drafts. Edit your catalog tags in the app with revision protection while keeping original audio files unchanged. Import supported public HTTP/HTTPS audio download links through the app with durable progress/retry/cancel controls. Create private playlists, grant local accounts viewer/editor access, and share expiring, revocable metadata-only links. Owners can list/revoke collaborators; members can leave. The browser player has seeking, volume, repeat, shuffle without repeated occurrences within a cycle, actual previous/forward history, priority Play next, queue controls and account-scoped queue persistence. Transient shuffle history starts fresh after reload. Your queue, selected occurrence, position, playback context, volume and modes restore separately for each account, paused until Play. The browser retries interrupted streams within a fixed budget and offers manual retry; expired sessions return to sign-in. This does not download music for offline playback.

**Home** brings your actual new music, favorites, albums and playlists together. **Search** finds private tracks, albums, artists and explicitly shared playlist names, with category filters and Ctrl/Cmd+K. Play music, add it next or last in the queue, and move between pages without interrupting playback. Phones use Home/Search/Library/Playlists tabs plus a More menu; both themes adapt from320px to wide desktop screens. Select the current track in the player to open Now Playing with a large cover, real file details, shared transport, favorites and a saved-order queue preview. It uses the same active audio and closes with Escape. Known duration and paused seeking work before a queued track loads; explicit Play starts from the selected position.

**Network** lets an administrator explicitly enable a public catalog and lets each owner publish individual tracks with a declared open license, attribution and private source evidence. Metadata, streaming and downloads have separate grants; all audio rights start off. Revoke a publication without changing your private original. Preview another public server through bounded HTTPS-only discovery. No music becomes public just because a server is enabled. This is an initial read-only catalog profile, not full ActivityPub federation or a unified remote playback queue. See [network rights](docs/NETWORK_RIGHTS.md).

Administrators can scan configured read-only music folders in **Settings → Music folders**, watch progress, cancel, and rescan without changing original files or existing catalog edits. Removing a scan source keeps imported music.

**Private listening history** is off until you enable both your account and this browser in Settings. Vela records advancing, unmuted playback rather than downloads or seek distances. Recently played appears on Home after meaningful listens; Listening history provides a private paginated view. Export your synced journal or delete it and disable collection on every device. The browser's bounded pending journal is cleared on sign-out or opt-out; browser throttling can undercount listening.

Open **Track radio** beside a Library track or inside Home/Search audio details. Play a real queue from your own catalog, with a reason for each recommendation. Optional **Analyze seed** or **Analyze up to 50 library tracks** runs bounded local PCM/FFT analysis in a durable background worker, without changing originals or contacting a cloud service. Radio compares measured spectra and signal levels where available and otherwise uses catalog metadata. Coverage is limited to the500 newest other owned tracks. This is not a genre/mood classifier or a weekly/daily personalized mix service.

**Your next listening chapter** on Home explores real owned music using your favorites, available audio measurements and, only while account collection is enabled, qualified private listens. Your rotation brings back tracks you actually played. Each suggestion explains its basis; Play mix, Play next and queue actions use the persistent player. Hide a track from personal mixes, undo the choice, restore all hidden choices or export their IDs. These on-demand sets examine a bounded candidate window; they are not scheduled daily/weekly mixes or a global catalog. Turning history off also turns off its recommendation influence, without silently deleting your retained journal.

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
cargo build --locked -p vela-desktop
cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings
scripts/build-web.sh
npm ci --prefix tests/browser
(cd tests/browser && npx playwright install chromium firefox)
(cd tests/browser && npx playwright test --browser=chromium)
(cd tests/browser && npx playwright test --browser=firefox)
# Opt-in real native window + owned Pulse null sink; uses isolated fixture data:
cargo build --locked -p vela-server
cargo build --locked -p vela-desktop --example native-ui-qa --features native-qa
python3 scripts/verify_native_client.py --evidence target/checks/native-window --test-audio-engine
# Portable local desktop package (no external publication):
scripts/package-desktop-linux.sh
python3 scripts/verify_desktop_package.py target/desktop-releases/vela-desktop-*.tar.gz --launch
```

See [experience parity](docs/SPOTIFY_PARITY_MATRIX.md), [product flows](docs/PRODUCT_SPEC.md), [design system](docs/DESIGN_SYSTEM.md), [progress](docs/PROGRESS.md), [architecture](docs/ARCHITECTURE.md), [roadmap](docs/ROADMAP.md), [security](docs/SECURITY.md), [performance](docs/PERFORMANCE.md), [known limitations](docs/KNOWN_LIMITATIONS.md), [native client contract](docs/NATIVE_CLIENT_CONTRACTS.md), and [research](docs/RESEARCH.md). CI configuration does not mean CI has run. The0.12 Linux x86_64 desktop client is a real native Dioxus/Rodio/MPRIS application with a verified private library/queue/playback slice; packaged installers, secure credential persistence, mobile background audio, Windows/macOS builds, offline downloads, unified multi-server playback, OpenSubsonic and standards-based social federation remain roadmap work. Persistent same-build Linux user supervision is available through the validated [user-service installer](docs/USER_SERVICE.md).

AGPL-3.0-or-later. Real test audio is locally generated sine-wave material. Optional live-import tests use Wikimedia's explicitly licensed example audio, with attribution recorded in `tests/browser/README.md`.
