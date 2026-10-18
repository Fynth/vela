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
  adding tracks from the library and folders remain web-only. Bounded entry counts/IDs are validated; a metadata-only entry is labelled.
- Albums and artists use the existing owner-scoped `GET /api/albums`,
  `/api/albums/{id}`, `/api/albums/{id}/tracks` and artist equivalents. The native
  views render real group summaries, open read-only detail, and queue/play the
  authorized group tracks; unknown/foreign groups, page cursors and IDs are bounded
  and validated. No edition/recording identity is inferred from a name.
- Queue controls include move-up/move-down and a Clear-upcoming action that keeps the
  current occurrence and earlier rows, removing only the suffix through the shared
  order model. The player bar exposes the current track's favorite state with the same
  owner-only mutation and queue synchronization as library rows.
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
not progressive streaming, gapless, offline, a download entitlement, background
mobile playback, secure credential persistence, device handoff, packaged install,
adding library tracks to playlists, folders, album/artist editing or a federated
source resolver. It sends no
native listening events. Only Linux x86_64 is built/tested on this host; Windows,
macOS, Android and iOS remain separate targets. Hardware output, screen readers and
physical media keys are not certified.

## Verification

- `cargo fmt --all -- --check`; strict Clippy `-p vela-desktop --all-targets
  --features native-qa`; `cargo test --locked -p vela-desktop`: 23 passed,
  2 ignored (opt-in audio/MPRIS) from25 listed tests (8 audio,6 client,3 MPRIS,
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
  catalog. Entry editing also passes: move entry2 up, restore by moving entry1
  down, remove entry2, list/detail/counts refresh. Evidence:
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
