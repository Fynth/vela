# Native Linux client0.12 — verified bounded increment

Status: verified source slice on Linux x86_64 only. It is not a packaged installer,
not mobile/background audio, not offline playback and not completion of G18.
Evidence is listed at the end; see ADR0017 for the decision record.

## Ownership

Main owns workspace/`Cargo.lock`, this contract, `apps/desktop/src/lib.rs`,
`main.rs`, tests/builds/docs. `apps/desktop/src/audio.rs` is the audio adapter;
`client.rs` is the native HTTP/account boundary; `ui.rs` and `style.css` are
presentation; `mpris.rs` is the Linux media observer. No contributor runs Cargo,
changes original runtime/data or commits; max2 compiler jobs, one Cargo build at a
time. The existing server, schema, HTTP API and frozen0.11 runtime are unchanged.

## User journey

Launch `vela-desktop`, sign in to one Vela origin, page/search the private owned
library, play an original through the device's native audio output, pause/resume,
seek, change volume, navigate Library/Favorites/Playlists/Queue without interrupting
audio, favorite/unfavorite owned tracks, search favorites, browse real albums,
artists and playlists, play album/artist/playlist stream-authorized subsets, create/
rename/delete own playlists, reorder/remove playlist entries, use next/previous/
shuffle/repeat, queue duplicates/play-next/remove/reorder, switch theme, sign out. MPRIS exposes actual state and forwards play/pause/next/previous/
seek/volume where the bus is available.

## Implemented boundaries

- `client.rs`: in-memory session cookie and CSRF only; no cookie jar, config file,
  keychain or local storage. HTTPS DNS origins or numeric loopback HTTP; no
  userinfo/query/fragment/control/non-canonical path; redirects and ambient proxies
  disabled; TLS verification on; bounded DNS/connect/read/JSON/media operations;
  exact expected size plus response BLAKE3 ETag before preparation; closed UUID
  stream paths; errors never echo password/cookie/URL/query/upstream body. Requests
  carry independent cancellation and session/query/selection/generation/lifetime
  checks.
- `lib.rs` + `audio.rs`: one long-lived output thread on a supervised device, one
  bounded command channel, generation/source guards, real decoder duration/seek,
  pause/resume/volume/EOF/replacement/stop. Prepared originals are unnamed private
  `0600` temporary files capped at250MiB, created without a directory entry from the
  first byte on Linux (`O_TMPFILE`/create-and-unlink); each decoder reopens
  `/proc/self/fd/N`. A lease lives only while its current occurrence is playing and
  is released on replacement/failure/sign-out/exit. A dead worker is reported as
  not playable and can be replaced only by an explicit same-account reconnect.
- Favorites use the existing owner-only `/api/tracks?favorites=true` read and
  `/api/tracks/{id}/favorite` write; the native view reuses the same catalog/audio
  rights, updates matching queue rows, and re-reads the filtered page after an
  unfavorite. A feedback gate plus busy track id prevents duplicate/rapid mutations;
  failures remain visible and no optimistic success is claimed.
- Playlists use the existing `GET /api/playlists` and `GET /api/playlists/{id}`
  reads. The native view lists real owner/granted playlists, shows entry metadata
  and whether each entry is streamable, and queues only the `can_stream` subset under
  a playlist context. Owners can create, CAS-rename and confirm-delete playlists via
  the same authenticated/CSRF/Origin-safe client; entry reorder/removal use the
  revisioned order/remove APIs with selected-id guards and update list counts/detail;
  native folders use the owner-scoped folder APIs for create, per-folder filtering, CAS
rename/delete and CAS playlist move; folder deletion unfiles playlists. Bounded entry
counts/IDs are validated; a metadata-only entry is labelled.
- Albums and artists use the existing owner-scoped `GET /api/albums`,
  `/api/albums/{id}`, `/api/albums/{id}/tracks` and artist equivalents. The native
  views render real group summaries, open read-only detail, and queue/play the
  authorized group tracks; unknown/foreign groups, page cursors and IDs are bounded
  and validated. No edition/recording identity is inferred from a name.
- Queue controls include move-up/move-down and a Clear-upcoming action that keeps the
  current occurrence and earlier rows, removing only the suffix through the shared
  order model. The player bar exposes the current track's favorite state with the same
  owner-only mutation and queue synchronization as library rows.
- `remote.rs` reads one canonical peer public catalog and its stream endpoint with
  a credential-free, no-redirect/no-proxy client; it validates advertised
  origin/server identity, IDs, text/duration bounds and grants, requires bounded
  size + BLAKE3 ETag, prepares an unnamed private0600 original and loads it into the
  single native engine. Remote mode keeps a separate in-memory origin-labelled queue (never persisted,
  never mixed with private history); previous/next move within it, including across
  origins. It does not mutate the private queue, and MPRIS reports stopped/no
  metadata while it plays. A debug-only canonical→loopback
  transport override exists solely for the isolated two-instance QA; release builds
  ignore it. See ADR0018.
- `peers.rs` stores at most32 canonical public origins in a versioned JSON file under
  the user config directory, private0600/atomic, rejecting credentials, malformed,
  oversized or unsupported files. It stores no server metadata, tokens or history;
  the Public view can open/remove saved origins, save the current preview, and search
saved peers (`q`, at most eight peers×20 first-page results) concurrently with
independent five-second timeouts and per-origin error reporting. Both the single
preview and each saved-peer search result page follow that peer's validated
`next_cursor` through Load more, append only new IDs, keep cursors per origin and
clear the control when exhausted. There is no merged global cursor or automatic
cache refresh. A successful catalog response
is cached as public metadata under the user cache directory (atomic private0600,
versioned, at most64 files/128KiB each); when a saved peer is unreachable, search
returns its cached rows as **browse-only** (`Cached metadata · <age> · stream
unverified`) with no play action and an explicit Retry live action. A failed retry
stays cached; a successful retry replaces that origin with live metadata. Cached
metadata is never treated as a current grant, and there is no background refresh.
- `playback.rs` (reused by path from the web crate): bounded occurrence indices,
  fair shuffle deck/history, explicit next priority, repeat semantics. Structural
  queue edits synchronize the order model; metadata does not reset traversal.
- `mpris.rs`: separate thread/current-thread Tokio runtime, bounded64-update and
  32-command channels, bounded text, no credentials/HTTP/audio ownership.
  Missing/failed session bus degrades to in-app controls. No local paths or private
  fields are published.
- `ui.rs`/`style.css`: real loading/empty/error/preparing/cancel/retry states,
  duplicate occurrence identity, disabled-state explanations, responsive
  1280×840 and640×700 layouts, light/dark, keyboard focus and reduced motion.
  Sign-out clears catalog/queue/player and stops native audio.

## Explicit non-goals / limitations

This first adapter downloads the whole authorized original before decoding. It is
not progressive streaming, gapless, background mobile playback, Windows/macOS/mobile
secure credential stores, device handoff, signed/packaged install, album/artist editing, Internet
TLS/two-host validation or a general federated source resolver. Offline copies are an
explicit private cache with a local queue, not a server entitlement or background sync.
Peer catalogs support saved canonical origins, concurrent search, browse-only cached
fallback and 20-record cursor pagination; cached rows never authorize playback, and no
cross-server download or automatic cache refresh is claimed. It sends no
native listening events. Only Linux x86_64 is built/tested on this host; Windows,
macOS, Android and iOS remain separate targets. Hardware output, screen readers and
physical media keys are not certified.

## Portable package

`scripts/package-desktop-linux.sh` builds a portable x86_64 tar archive containing the
release binary, `vela-desktop.desktop`, scalable icon, README, LICENSE and BUILD.txt.
`scripts/verify_desktop_package.py --launch` validates the exact manifest/modes,
`--version`, missing-library check, a real GUI launch/screenshot and normal window
close with zero temporary originals. This is not a `.deb`/AppImage/Flatpak, signature,
app-store or physical-audio certification. Evidence:
`target/desktop-releases/vela-desktop-0.12.5-linux-x86_64-unknown-linux-gnu.tar.gz`
(SHA256 `c6bd29810be2da28faecfa02bc90a0b56b10b52a89add77905385aaa5349d3e7`,
BUILD commit `3eedf1b`); prior0.12.0–0.12.4 archives are preserved. It includes saved
peers, cached browse-only metadata and preview/per-peer search pagination. Evidence:
`target/checks/desktop-0.12.5-verify.log`,
`desktop-package-screens/vela-desktop-0.12.5-package-launch.png`.

## Opt-in saved session

Sign-in offers an explicit **Remember this session on this device** checkbox (default
off). When checked, only the opaque 64-hex `vela_session` token plus its canonical
origin is written through stdin to the freedesktop Secret Service via `secret-tool`;
the password, username, CSRF token, media paths and history are never stored. The
sign-in screen shows a **Continue saved session** action only when an item exists; it
resumes by calling `/api/me` with the stored cookie and reconstructs the CSRF token
from that authenticated response. No automatic passwordless login occurs.

Explicit sign-out clears the Secret Service item; an expired/invalid saved session is
cleared with a visible sign-in error. The fixed release service name is
`org.vela.music.desktop.session`; QA may override it only in debug builds. This is Linux
freedesktop Secret Service only — no Windows Credential Manager/macOS Keychain, Android
Keystore or biometric unlock is claimed. The token is a bearer credential protected by
the user's unlocked keyring collection.

## Offline device copies

An explicit per-track **Save offline** action downloads through the same validated
authorized original path (exact size, BLAKE3 ETag,250MiB cap) and then copies the
completed unnamed original into an account-scoped private cache. Layout:
`$XDG_DATA_HOME/vela/offline/<account-id>/<track-id>.audio` plus a versioned JSON
manifest; directories are0700 and files0600, written via private temp+rename. Limits
are100 tracks/2GiB; malformed, mismatched-size or orphaned entries are ignored. No
credentials, session tokens, audio URLs or listening events are stored.

The Library shows saved copies and **Play offline** decodes the local file through the
single native engine with context `Offline · <title>`. The offline queue contains all
saved copies in newest-saved order; Previous/Next move only within that queue and the
controls disable at its edges. A saved copy can be removed with inline confirmation;
removing the currently playing copy stops offline playback. Removal deletes only the
private device copy and manifest, never the server original. There is no background
sync, automatic download/refresh, bulk storage manager, Android/iOS offline or
server-side entitlement refresh. Sign-out clears in-memory state but retains the account-scoped files; another
account cannot see them. The real-window QA stops the fixture server, verifies the HTTP
health probe fails, plays the saved original to advancing media time, and checks the
audio/manifest exist at0600.

## Verification

- `cargo fmt --all -- --check`; strict Clippy `-p vela-desktop --all-targets
  --features native-qa`; `cargo test --locked -p vela-desktop`: 28 passed,
  2 ignored (opt-in audio/MPRIS) from30 listed tests (8 audio,6 client,3 MPRIS,
  8 shared playback). Logs: `target/checks/native-0.12b-{fmt,clippy,tests}.log`.
- Ignored `audio::tests::native_owned_null_sink_play_pause_seek_eof_replace_stop_and_cleanup`
  executed under an owned Pulse null sink: passed.
- Ignored `mpris::tests::real_session_bus_exposes_properties_and_forwards_controls`
  executed under a private `dbus-run-session`: passed after switching to bounded
  update/command channels.
- An independent read-only review found that the first adapter created a named
  temporary file during transfer, so closing the window while preparing could leave
  a partial original. Main changed preparation to an unnamed file from the first
  byte, asserted zero directory entries during a held transfer, and re-ran the
  complete suite. Other review findings fixed: unbounded MPRIS queues, a dead audio
  worker still advertised as playable (explicit reconnect now), QA sink assertions
  and a UTF-8 slice panic; the reported test-count discrepancy was resolved by
  enumerating25 actual tests.
- Two-instance public playback passes: the harness starts a second isolated Vela
  server, enables its canonical public origin and publishes the fixture with an
  explicit stream grant; two isolated peers are published on canonical origins; the native client saves both,
  searches them together, receives results from both, plays the second origin's public
  track, then Previous/Next move across the two origin results at EOF-safe points. Evidence:
  `target/checks/native-public-peer5/` (`native-public-dark.png`). Internet TLS and a
  real remote host remain NOT_RUN; the QA uses a debug-only loopback transport.
- Queue/favorite checks pass: move-down follows the current occurrence, move-up
  restores it, Clear-upcoming leaves only the current row, and the player heart
  acknowledges the API and updates the shared queue state. Evidence includes
  `target/checks/native-queue-favorite3/`.
- Album/artist journey passes in the same real window: real album groups, album
  detail (`Real Fixtures · Vela Test`), `Play album` native audio, artist groups,
  artist detail, `Play artist` native audio, and Library return. Evidence:
  `target/checks/native-browse-window3/` (including `native-albums-dark.png`).
- Playlist journey passes in the same real window against the current0.12 debug
  server: real list card, native create → open → rename → cancel-delete → confirm
  delete, fixture playlist survives, two-entry detail, `Play playlist` starts native
  audio, the shared queue receives both entries, and Library navigation restores the
  catalog. Entry editing also passes: add an owned library track through the
  revisioned item API, play three entries, move entry2 up, restore, remove one,
  list/detail/counts refresh. Evidence:
  `target/checks/native-playlist-editing/` (including `native-playlists-dark.png`)
  and earlier `native-playlist-edit-window3/`.
- Favorites journey passes in the same real window: the owner favorite API
  acknowledges the row, Favorites shows exactly that track, unfavorite re-reads the
  filtered page to empty, and Library navigation restores all three owned tracks.
  Evidence: `target/checks/native-favorites-window9/` (including `native-favorites-dark.png`),
  `native-favorites-{clippy2,workspace-tests}.log`, and233 workspace tests.
- `scripts/verify_native_client.py` against an isolated fixture server/account/media
  in a real WebKitGTK window: 15 named checks including real WAV/FLAC/MP3 playback,
  seek, queue duplicates/reorder, 640×700 element bounds, no overflow, sign-out and
  zero leftover temporary originals. Also passed the close-while-playing variant.
  Evidence: `target/checks/native-0.12b-window/`, `native-0.12b-mpris-session.log`;
  earlier `native-0.12-window4/` and
  `target/checks/native-0.12-close-playing/` plus screenshots.
- A first compact run exposed queue grid tracks overflowing640px; the assertion was
  made element-level, `.native-queue-list` was constrained to `minmax(0,1fr)`, and
  the complete run passed. Failed evidence remains under `native-0.12-window2/`.
