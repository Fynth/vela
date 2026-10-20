# Native Linux client0.12 — verified bounded increment

## Distribution pipeline2026-10-20

Distribution0.12.7 adds Windows portable exe/zip, Linux tar.gz/deb/AppImage and Dioxus-generated Android APK build paths without adding another HTTP/audio owner. Android API26+ manifest disables backups and cleartext. Stable signing and actual device/background validation are separate. See [DISTRIBUTION](DISTRIBUTION.md).

Status: verified Linux x86_64 source and portable-package slice (0.12.6). It includes
explicit offline device copies, opt-in Secret Service session resume and native
playlist folders. It is not a system installer/auto-updater, not mobile/background
audio and not completion of G18. The audit's keyring/offline lifecycle defects
AUD-01/02/04/11 are fixed in source `3094754` (real two-process Secret Service QA);
AUD-03/07/08/12 fixed in source `e225b63` (basic native offline QA passed); AUD-06 fixed in `a913574` (origin-scoped offline namespace); AUD-05 (cold offline launch) fixed in `3aeee09` (private offline profile + read/play-only offline library, real X11 cold-launch QA); see [CURRENT_STATE.md](CURRENT_STATE.md).
Evidence is listed at the end; see ADR0017 for the decision record.

TR10 (2026-10-20, source `4ceafdc`; typography/QA `47fb518`+`d314a2c`): the packaged
stylesheet now self-hosts the approved Manrope typography and the real X11 native QA
verifies fonts, frameless window controls, app scale80–150 and one `AudioEngine`
output authority. The portable package was rebuilt and launch-verified from source
`4ceafdc` (SHA256 `58da18db…`); the earlier0.12.6 archive from `7fe1ca0` is preserved
under `target/desktop-releases/previous/`.

## Ownership

Source extension2026-10-19: `Ui.wants_play` is root requested intent, independent of
delayed decoder snapshots. Current-track/current-occurrence activation pauses or
resumes without replacing source/order; a different duplicate starts a new selection.
Group/playlist header Play starts context through a separate callback. Native public
identity is origin+object ID. Existing session/account/gate/selection/lifetime guards
still govern every deferred result; completion reads current intent before loading.

Owned online queues retain shuffle/all/one preferences. Public queues expose neither
shuffle nor repeat. Offline queues are sequential with repeat-one; EOF replays the
verified descriptor without leaving the offline domain. MPRIS ignores unsupported
mode commands and publishes effective source modes without discarding owned
preferences. Public MPRIS metadata/transport remains unsupported.

`window::builder` is shared by production/QA: Linux decorations=false,1280×840
initial logical size,640×480 minimum; accessible window actions/drag region live in
the existing toolbar. Other platforms retain their frame. QA defaults to floated
X11; `--backend wayland --window-only` runs a distinct unresized tiled Hyprland
fixture. Exact owned PID/xwayland=false and PNG dimensions=DOM×DPR are verified.
The QA backend variable is independent of Dioxus's later GDK_BACKEND mutation.
Unsupported CLI combinations fail before resource allocation. See
[interaction review](UX_INTERACTION_REVIEW_2026-10-19.md) for tests and limits.

Main owns workspace/`Cargo.lock`, this contract, `apps/desktop/src/lib.rs`,
`main.rs`, tests/builds/docs. `apps/desktop/src/audio.rs` is the audio adapter;
`client.rs` is the native HTTP/account boundary; `ui.rs` and `style.css` are
presentation; `mpris.rs` is the Linux media observer. No contributor runs Cargo,
changes original runtime/data or commits; max2 compiler jobs, one Cargo build at a
time. The existing server, schema, HTTP API and frozen0.12.6 server runtime are unchanged.

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
  Sign-out clears catalog/queue/player and stops native audio. The top of
  `style.css` embeds the approved Manrope Latin/Cyrillic subsets as data-URI
  `@font-face` rules (TR10, below); no font is fetched, installed or served from
  an asset protocol.

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

## Self-hosted Manrope typography (TR10)

`apps/desktop/style.css` begins with two `@font-face` rules that embed the approved
`apps/web/assets/fonts/manrope-latin.woff2` (23,056 bytes) and
`manrope-cyrillic.woff2` (11,820 bytes) subsets (SIL OFL 1.1; no new font source,
license at `apps/web/assets/fonts/OFL.txt`) as `data:font/woff2;base64` URIs. Each
face declares `font-weight:200 800`, `font-display:swap` and its own `unicode-range`,
so the variable weight axis and Cyrillic coverage are preserved with no font CDN,
network request, asset protocol, extra package file or user-installed font. `:root`
now starts with `Manrope` followed by the unchanged
`Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
fallbacks. The stylesheet grows from51,263 to98,729 bytes; the 34,876 raw font bytes
become the embedded base64 `include_str!` compiles into the binary.

The real X11 window QA (`scripts/verify_native_client.py --test-peer`, evidence
`target/checks/tr10-native-head/`) passed260 checks and15 screenshots, including
`native-typography-dark.png` at1280×840 = DOM×DPR. TR10 typography checks read the
real renderer, not a stylesheet claim: computed `:root` and `.native-app`
`font-family` start with `Manrope` and keep the fallbacks; exactly two `Manrope`
`FontFace` entries report `status:loaded` with `weight:200 800`; exactly two
`@font-face` rules each carry one `data:font/woff2;base64` source and a distinct
Latin/Cyrillic `unicode-range`; `document.fonts.load`/`check` succeed for both
subsets; and a24-character Cyrillic sample (`Привет, мир! Ёё № «Vela»`) renders in
the inherited Manrope stack with Manrope metrics (199.83px vs207.72px for
`sans-serif`), so the loaded face is actually used instead of silently falling back.
`performance` records zero network font requests. Font loading is real: nothing in
the QA installs a font, injects a face or rewrites the family.

The same run verifies the frameless window chrome (decorations=false, exactly one
accessible minimize/maximize/close group with44px hit boxes and a687px drag region),
the validated app scale on the real stepper (100%→105%→100%→80%→150%→100%: both
ends disable and clamp, one `zoom`/`--app-zoom` value with no stacked transform, no
viewport overflow, same native source and paused position), and exactly one
`AudioEngine` output authority (one `.native-player` band, one
`[data-audio-generation]` surface, zero `audio`/`video`/`iframe` elements, and
exactly one process-matched native output stream on the QA-owned
`vela_native_test_null` Pulse/PipeWire sink). Wayland remains the prior window-only
evidence; no full Wayland run was made for TR10.

## Portable package

`scripts/package-desktop-linux.sh` builds a portable x86_64 tar archive containing the
release binary, `vela-desktop.desktop`, scalable icon, README, LICENSE and BUILD.txt.
`scripts/verify_desktop_package.py --launch` validates the exact manifest/modes,
`--version`, missing-library check, a real GUI launch/screenshot and normal window
close with zero temporary originals. This is not a `.deb`/AppImage/Flatpak, signature,
app-store or physical-audio certification.

TR10 package (2026-10-20, source `4ceafdc`): after preserving the previous0.12.6
artifact, `scripts/package-desktop-linux.sh` (release build,2 cargo jobs) created
`target/desktop-releases/vela-desktop-0.12.6-linux-x86_64-unknown-linux-gnu.tar.gz`
(SHA256 `58da18db0f0af4986ab1069840682f2e673f3024f5137fe25241820c00a367ac`,
version0.12.6, BUILD commit `4ceafdc482db1789159e6bae53f7015769b9b700`,
5,323,519 bytes). Its packaged `bin/vela-desktop` (13,790,928 bytes) prints
`vela-desktop 0.12.6`, contains both embedded `data:font/woff2;base64` Manrope faces
and the `Manrope,Inter,ui-sans-serif` root stack, and `ldd` reports no missing
libraries. `scripts/verify_desktop_package.py … --launch` exited0 with the exact
six-file manifest/modes, the version match, a real `Vela Music` window at a real X11
launch, a screenshot, a normal window close and zero temporary originals. Evidence:
`target/checks/tr10-package-verify.json`, `tr10-package-evidence.txt`,
`desktop-package-screens/vela-desktop-0.12.6-tr10-package-launch.png` (plus the earlier
`tr10-package-guard-refused.log` from the uncommitted-tree attempt).

The previously documented0.12.6 package built from `7fe1ca0` (SHA256
`7abe3982073470d36e4680b56845aa6858a0c21b3c86f2007862ba9ba16d6063`) is preserved
byte-for-byte under `target/desktop-releases/previous/` with its original `.sha256`
file (whose recorded path still names the canonical location); nothing was deleted.
Prior0.12.0–0.12.5 archives are preserved, and the historical0.12.6 feature evidence
(`target/checks/desktop-0.12.6-verify.log`,
`desktop-package-screens/vela-desktop-0.12.6-package-launch.png`) describes the
`7fe1ca0` build. The new archive keeps version0.12.6 but is built from source
`4ceafdc`, so it also carries source changes made after `7fe1ca0`; only its BUILD
commit identifies it.

## Reusable complete-file media cache

Native `prepare` first re-authorizes with one bounded authenticated HEAD on the
stream route, then may reuse a complete verified original from
`$XDG_CACHE_HOME/vela/media/<origin-hash>/<account-id>/<track-id>.audio` plus a
JSON sidecar (origin/account/track/size/ETag/BLAKE3/saved_at, 0700/0600). A hit
requires the sidecar to match the response and the cached descriptor to re-hash
to that ETag; the same fd is rewound for decode. A denial never consults cached
bytes. Misses keep the unchanged whole-file size+BLAKE3 unnamed lease, then warm
the cache best-effort with unique staging, fsync and atomic publish. Bounds are
200 entries/2GiB per account, staging/orphans counted, cache-only FIFO eviction;
pinned offline copies live elsewhere and are never evicted. ADR
[DECISIONS/0022](DECISIONS/0022-native-media-cache.md). Real-process body
counters, ENOSPC and production-budget eviction remain NOT_RUN.

## Opt-in saved session

Sign-in offers an explicit **Remember this session on this device** checkbox (default
off). When checked, only the opaque 64-hex `vela_session` token plus its canonical
origin is written through stdin to the freedesktop Secret Service via `secret-tool`;
the password, username, CSRF token, media paths and history are never stored. The
sign-in screen shows a **Continue saved session** action only when an item exists; it
resumes by calling `/api/me` with the stored cookie and reconstructs the CSRF token
from that authenticated response. No automatic passwordless login occurs.

Saving clears the fixed Service attribute first so repeated opt-ins do not accumulate
stale tokens. Explicit sign-out waits for deletion and reports a visible error if the
item remains; malformed/expired saved sessions are also cleared with a visible sign-in
error. AUD-05 scope: when the private offline profile (below) names the same canonical
origin and real saved copies exist, a cold start attempts that saved session once by
itself and falls through to the offline library when the server does not answer. Nothing
is stored beyond the already opt-in token, no password is ever used, and no attempt is
retried. The fixed release service name is
`org.vela.music.desktop.session`; QA may override it only in debug builds. This is Linux
freedesktop Secret Service only — no Windows Credential Manager/macOS Keychain, Android
Keystore or biometric unlock is claimed. The token is a bearer credential protected by
the user's unlocked keyring collection.

## Offline device copies

An explicit per-track **Save offline** action downloads through the same validated
authorized original path (exact size, BLAKE3 ETag,250MiB cap) and then copies the
completed unnamed original into an account- and origin-scoped private cache. Layout:
`$XDG_DATA_HOME/vela/offline/<blake3(canonical origin)>/<account-id>/<track-id>.audio`
plus a versioned JSON manifest that also records the canonical origin. Directories are
0700 and files0600, written via private temp+rename. AUD-06: two origins that hand out
the same account/track UUIDs cannot share a namespace, and a manifest whose recorded
origin/account does not match the requested canonical namespace is ignored. Legacy
account-only copies from before origin scoping are preserved byte-for-byte but are not
derived, read or played as a copy of whichever origin asks next. Limits
are100 tracks/2GiB; malformed, mismatched-size or orphaned entries are ignored. New
manifests are v2 and carry the BLAKE3 digest of the copied device bytes; Play offline
re-hashes the file from the same opened descriptor and refuses a same-size tampered copy
with a visible integrity error; a failed manifest write removes the copied audio.
Legacy v1 manifests (length-only) remain readable and will verify on the next save. No
credentials, session tokens, audio URLs or listening events are stored.

### Cold offline launch (AUD-05)

A cold start has no account and no session, so the private namespace (canonical origin +
account UUID, AUD-06) cannot be derived from the device. `offline_profile.rs` therefore
keeps one private, non-secret profile document at
`$XDG_DATA_HOME/vela/offline/profile.json`: `version`, canonical `origin` and
`account_id` only — never a session token, password, CSRF token, audio URL, media path,
username, queue or listening event. It is mode 0600 inside the 0700 offline tree, written
atomically (private `create_new` temp file, `fsync`, `rename`), claimed by a sign-in
generation so a pending write cannot recreate it after an explicit sign-out, written after
a real sign-in/resume and removed on explicit sign-out. The saved copies and manifests are
never deleted, rewritten or derived from a profile lookup; a corrupt, oversized,
foreign-version or non-canonical document is ignored (`None`) and left in place. Two
origins that mint the same UUIDs also get two different profile states, because the profile
is only ever used together with its own canonical origin.

Startup order: (1) read the profile and the real saved copies of exactly its namespace,
locally; (2) read the opt-in saved session exactly as before; (3) if a saved session names
that origin, resume it once through the existing guarded resume; (4) otherwise, and after
an unreachable, timed-out, refused, unusable or expired resume, open the offline library.
A real sign-in always wins, nothing autoplays, nothing is retried and no account is
fabricated. If there is no usable profile, no real copy, or the profile/copy lookup fails,
the ordinary sign-in screen stays (with an honest reason for a failed lookup).

The offline library is **read/play-only**: real saved copies only, Play through the existing
`offline::prepare` path (manifest check plus BLAKE3 re-hash on the same descriptor) and the
single native `AudioEngine`, honest empty/unreadable/corrupt states, and an explicit
**Sign in to Vela** action. Saving, removing, favorites, search, playlists, the server
library, the server queue and sign-out remain unavailable without a real sign-in; the
screen renders no account, no account navigation and publishes no listening event. Every
deferred local result is guarded by the same generation/epoch/identity rule as the
signed-in list (AUD-03/AUD-08), with the profile as the live identity. A normal sign-in
replaces the offline screen and drops its transient queue, decoder lease and projected list.

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
audio/manifest exist at0600 and the v2 manifest carries a64-hex BLAKE3 digest.

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

- TR10 source checkpoint (`4ceafdc`; TR10 code `47fb518`+`d314a2c`):
  `cargo fmt --all -- --check` passed; strict Clippy `-p vela-desktop --all-targets
  --features native-qa -- -D warnings` passed; `cargo test --locked -p vela-desktop`
  ran116 tests with113 passed,3 ignored (opt-in audio/MPRIS) and0 failed; the
  `native-ui-qa` example builds. The real X11 native QA (`--test-peer`, owned null
  sink) passed260 checks and15 screenshots with0 temporary originals:
  `target/checks/tr10-native-head/` (`report.json`, `native-typography-dark.png`);
  logs `target/checks/tr10-head-{fmt,clippy,build,tests}.log` and
  `tr10-native-head.log`. That run verifies both embedded Manrope subsets and the
  Cyrillic sample metrics, the frameless window controls/drag region, app scale80–150
  on the real stepper, and exactly one `AudioEngine` output stream. Wayland remains
  the earlier window-only evidence; hardware output, screen readers, physical media
  keys, Internet/two-host and every non-Linux target remain NOT_RUN.
- AUD-05 cold offline launch (source `3aeee09`, test `d20faec`): `cargo fmt --all --
  --check` passed; strict Clippy `-p vela-desktop --all-targets --features native-qa --
  -D warnings` passed; `cargo test --locked -p vela-desktop` ran127 tests with124
  passed,3 ignored (opt-in audio/MPRIS) and0 failed (new `offline_profile` tests:
  roundtrip/permissions, refused identities, corrupt+oversized+foreign-shaped documents
  ignored and kept, foreign-origin profile naming no other namespace while the tree and
  a synthetic copy pair stay byte-identical, and sign-out superseding a pending
  sign-in write; plus a new `OfflineOwner` profile-identity guard test); `cargo build
  --locked -p vela-desktop` and the `native-ui-qa` example build. The combined real X11
  run — `python3 scripts/verify_native_client.py --backend x11 --server
  target/debug/vela-server --test-peer --test-session --test-offline-launch` — exited0
  with `report.json` ok=true (260 checks,15 screenshots), `offline-report.json`
  ok=true (26 checks, `native-offline-dark.png`) and `session-resume-report.json`
  ok=true (6 checks), plus0 temporary originals after every process exit and3 TLS peer
  connections. The cold launch started a second real window while the fixture server
  was stopped: the private profile at `…/vela/offline/profile.json` is0600 and carries
  exactly `account_id`/`origin`/`version` (unchanged by the offline launch), the offline
  library rendered instead of the sign-in form for the saved copy's canonical origin,
  it listed the real saved copy and no server catalog row, exposed no search, removal,
  favorite, server queue, account navigation or sign-out, rendered one `.native-player`
  with one `[data-audio-generation]` and zero HTML media/iframe elements, played the
  saved copy through the offline path to900 ms with exactly one process-matched output
  stream on the owned `vela_native_test_null` sink, offered repeat off/one but never
  shuffle or repeat-all, kept one source and position across pause/resume, closed
  normally and left0 temporary originals. The saved session then resumed automatically
  from the profile at startup (one attempt, no retry), the tampered device copy still
  disabled Play offline, and the explicit sign-out withdrew the keyring item and the
  offline profile. Evidence: `target/checks/aud05-native/`. NOT_RUN: Wayland/full-backend
  cold launch, hardware audio output, screen readers, physical media keys and every
  non-Linux target. An earlier attempt of the same combined command failed the
  pre-existing `TR03 narrow band renders the real paused and muted states` check because
  it ran against a stale `native-ui-qa` driver binary (the JS driver is `include_str!`d)
  while a Cargo build competed for the host; that report and a stale-but-green run are
  preserved as `target/checks/aud05-native-failed-run1/` and
  `target/checks/aud05-native-stale-green/`, and the final run above is the only
  evidence claimed.
- TR10 portable package (source `4ceafdc`, version0.12.6): the release build and
  `scripts/package-desktop-linux.sh` produced the archive above, and
  `scripts/verify_desktop_package.py --launch` passed (real `Vela Music` window,
  screenshot, normal close, exit0,0 temporary originals). Evidence:
  `target/checks/tr10-package-verify.json`, `tr10-package-evidence.txt`. This is not a
  `.deb`/AppImage/Flatpak, signature, app-store or hardware-audio certification;
  Windows/macOS/mobile packaging remains NOT_RUN.
