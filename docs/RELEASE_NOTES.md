# 0.12.5 — spectral-4 key + native folders

Patch release: DSP spectral-4 adds a heuristic Krumhansl key with profile margin; web
Radio shows Estimated key. The portable native client adds playlist-folder create/
filter/move/rename/delete. No schema migration, HTTP contract change or original-data
rewrite.

# 0.12.4 — playlist folders (schema12)

Owner-scoped playlist folders: create/list, CAS rename/delete, CAS move of owned
playlists and unfiling on folder deletion. Migration0012 is additive; original playlists
remain visible and unfiled. Chromium/Firefox each pass42 scenarios (41+1 skip). The
native client consumes the additive playlist field and adds folder create/filter/move/
rename/delete in source; the preserved0.12.4 desktop archive predates that native UI.
Source DSP spectral-4 adds a heuristic Krumhansl major/minor key label with profile
margin; it also postdates the0.12.4 runtime. Exact-package
Chromium and Firefox each passed42 scenarios with1 live-import skip (1.9m/2.3m; Firefox
needed one clean rerun after a sleep-timer pause flake). The schema11 backup was
restore-verified, then the new runtime applied schema12 on an isolated temporary port
after the installer correctly refused implicit migration; counts were unchanged.

# 0.12.3 — privacy device revocation + native offline checkpoint

Patch release for the frozen local runtime: Settings can list consented listening
devices and revoke one with privacy-revision CAS, deleting only that device's saved
sessions. The portable native client now includes private offline copies with a local
Previous/Next queue and confirmed removal, cached public metadata with age/Retry live,
and preview/per-peer search cursor pagination. No schema migration or original-data
rewrite. Exact-package Chromium and Firefox each passed40 scenarios with1 live-import
skip (1.9m/2.2m); the0600 backup was restore-verified and the handover kept schema11
and all catalog/media/analysis counts.

# 0.12.2 — spectral-3 chroma + source pagination checkpoint

Patch release for the frozen local runtime: local DSP spectral-3 adds nearest-semitone
chroma and the Radio measurements now show the top pitch classes. The native desktop
source also includes saved public origins, browse-only cached metadata, and preview/
per-peer search cursor pagination. No schema migration, HTTP contract change or
original-data rewrite is part of this patch. Exact-package Chromium and Firefox each
passed40 scenarios with1 live-import skip (1.9m/2.2m); the0600 backup was
restore-verified and the handover kept schema11 and all catalog/media counts.

# 0.12.1 — bounded spectral-2 tempo patch release

Local DSP spectral-2 adds a bounded onset-interval tempo estimate with honest
confidence and Radio UI disclosure. The extractor version change keeps immutable
spectral-1 caches separate. No schema, HTTP contract or original-data change; this
is a server/web assets patch for the frozen local runtime; the portable native
client is also rebuilt as `vela-desktop 0.12.1` with cached-metadata fallback. Source
adds public preview and per-peer search cursor pagination (20/page), cached-peer age
and Retry live, and local DSP spectral-3 adds nearest-semitone chroma with a visible
pitch-class summary; the runtime archive includes spectral-3. The cache age/Retry live and explicit
offline-copy slices (queue Previous/Next, confirmed removal) postdate the preserved
0.12.2 desktop archive and will ship in the next native package.
Exact-package Chromium
and Firefox each passed40 scenarios with1 live-import skip; the preserved-data
handover kept schema11 and all original catalog/media counts unchanged.

# 0.12.0 — verified Linux native client + local package/runtime checkpoint

`vela-desktop` is a real Dioxus Linux application with an in-memory native HTTP
session, one supervised Rodio/CPAL output thread and an MPRIS2 observer. It signs in
to one Vela origin, pages/searches the private owned library, plays authorized
WAV/FLAC/MP3/MP4/Vorbis originals, and shares the framework-free occurrence queue
with the web app (play/pause/seek/volume/next/previous/shuffle/repeat, duplicates,
play-next, reorder, remove, sign-out). Prepared originals are unnamed private0600
files capped at250MiB with exact size+BLAKE3 ETag verification, created without a
directory entry from the first byte and released on replacement/failure/sign-out/
exit. Credentials are memory-only and no native listening events are collected.

Verified on this Linux x86_64 host against an isolated fixture server/account/media:
fmt, strict Clippy,27 desktop tests (25 run,2 opt-in), owned-null-sink audio and
bounded-channel private D-Bus MPRIS tests, and a real WebKitGTK window at1280×840
and640×700 including playback, seek, navigation continuity, queue edits, theme,
640px element bounds, sign-out and zero temporary directory entries during transfer/
after exit. Independent review findings on the named temp source, unbounded MPRIS
queues, dead-worker retry and QA sink assertions were fixed and re-verified. A first
compact run exposed queue rows overflowing the viewport; the assertion was
strengthened and the layout fixed.

The web player adds a root-owned sleep timer (5/15/30/60 minutes) with a visible
countdown and cancel; at zero it pauses the one shared audio element and reports it.
A real-browser clock scenario verifies pre-expiry playback, expiry pause, and
cancel/restart behavior in Chromium and Firefox.

The same source increment improves the web listener surface. Queue rows support
move-down; Clear upcoming keeps the current occurrence and played prefix; remove-
current explains its disabled state; first-page controls are disabled from the
incoming page cursor. Playlist owners can CAS-rename and permanently delete a
playlist with inline confirmation and move-down ordering; deletion cascades entries,
collaborator grants and share links but preserves tracks, assets and audio rights.
Real TCP ownership/CAS/idempotency/revocation tests pass, and Chromium40/40 plus
Firefox40/40 source runs include these scenarios and all prior regressions.

A local server/web package is archived and verified: extracted schema11/default setup,
19 matching frozen binary/static hashes, Chromium41/41 and Firefox41/41 against the
exact package, ten clean SIGTERM/restarts, and a verified pre-upgrade backup/isolated
restore. The persistent local unit now serves the frozen0.12.0 binary/assets with the
ORIGINAL data (1 user/20 tracks/20 analysis/schema11). This is still not a native
desktop installer or public release. There is no
secure credential persistence, offline/download support, background mobile audio,
favorites/playlists/albums native UI or Windows/macOS/Android/iOS implementation.
The active service remains the verified0.11.0 runtime with the original data; no
schema/API migration was introduced in these client slices. No artifact was published.

# 0.10.0 — private listening suggestions (local review candidate)

Home now offers exploration from owned favorites/real audio descriptors and,
while account collection remains enabled, qualified private history. Rotation
brings back actual retained listens. Explanations, play/queue/radio actions,
reversible Hide, confirmed restore and private exclusion export use real APIs.
Current consent and retention apply in one snapshot; retained disabled history
has no influence. Additive migration0010 preserves catalog, playlists, events and
analysis results and does not enable collection or analysis.

Source-driven bounded SQL fixes measured sparse-signal full-catalog scans without
changing rank outputs or permission policy. Final measurements and platform QA
are recorded in PROGRESS/PERFORMANCE. This is on-demand personal listening, not a
scheduled daily/weekly service, public catalog or native client release.

Backup and test restore before upgrading;0.9 source/binary does not know schema10.
Rollback uses a verified backup in a separate restored directory, retaining the
original data and immutable media. No external release/publication is performed.

# Local checkpoint0.8.1

Adding a track to an empty queue now gives both the persistent bar and Now Playing its known duration before media is loaded. Seek while paused, then press Play to start at that saved position. The browser's measured duration remains authoritative once loaded; queue/favorite edits preserve active audio and order. Fixes the0.8.0 bar showing0s/max1 while the full player showed the real duration. No schema/API/storage/upstream pin changes. PROGRESS records the reproduced failure and executed checks; packages remain local review artifacts.

# Local checkpoint0.8.0

Now Playing opens from the current track in the persistent player. Its cover, title, artist, album, original-file properties, seek/volume/transport, favorite and bounded saved-order queue preview use actual root state. Selecting a queue occurrence or using Media Session controls keeps the same dialog/audio element. Opening a paused bookmark does not fetch media. Escape/close restores focus; navigation, sign-out and expired sessions clear the view. Favorite feedback ignores superseded track/view/account replies and successful changes agree across catalog, library and queue.

No schema, HTTP API, storage semantics or upstream dependency pin changes. Native/server packages include an explicit now-playing.css asset; the static-file boundary still denies unrelated files. See PROGRESS for executed checks and KNOWN_LIMITATIONS for browser/platform limits. This is a local review artifact, not an externally published release or complete Spotify-class parity.

# Local checkpoint0.3.0

This is a local review checkpoint; PROGRESS records executed gates. No external release is published.

Library and Favorites can combine exact genre/format, release-year range and minimum bitrate/sample-rate/bit-depth filters with search. Title and recently-added sorting use bounded keyset pages with stable UUID tie-breakers. Private, bounded suggestions come from actual catalog tags. Unsaved fields stay drafts while paging; accepted conditions survive refresh/navigation and reset on account change. Validation retains the current result, and delayed requests cannot replace newer/private state.

Format/container labels and numeric measurements do not establish losslessness. Genre matching uses complete tags and ASCII NOCASE. Unknown properties are excluded by numeric criteria; concurrent catalog edits can move live page boundaries. Migrations0005/0006 add query indexes and the measured covering genre upgrade; back up before upgrading and restore matched old data/binaries for rollback. Existing audio and privacy grants are unchanged. Stable incremental builds now watch added migration files, and package verification checks the embedded schema version.

# Local checkpoint0.2.0

This is a local review checkpoint, not an externally published release. See PROGRESS for executed validation and KNOWN_LIMITATIONS for unsupported platforms/features.

## Read-only folder imports

Administrators can add named sources from installation-approved music folders in Settings, start scans, watch counters, cancel and rescan. The Linux scanner imports real audio metadata/artwork through the existing media pipeline, never modifies original folders and preserves catalog edits on exact-duplicate/unchanged rescans. Removing a source removes its scan tracking only; imported music stays playable.

Configuration defaults to the private data/sources folder. VELA_SCAN_ROOTS selects explicit absolute roots or [] disables scanning. The browser accepts root identifiers, never arbitrary host paths. Descriptor-relative no-follow opens, regular-file/hardlink checks, bounded traversal/copies/queues and free-space checks protect the filesystem boundary. One durable scanner worker runs alongside imports, with restart recovery and process supervision.

## Security and operations

The locked rustls version was updated from0.23.45 to0.23.46 after four runtime advisories appeared during review. No runtime advisory exception was introduced. Local archives now use version-specific checksums and preserve prior checkpoints. Migration0004 is forward-only: back up before upgrading, retain old binaries/backups, and restore the matched pair for rollback. Managed audio/backups work without original scan folders; back up original external collections separately.

Folder scans copy audio into managed storage and require additional space. Linux local filesystems are the supported scanner environment. File signatures provide best-effort change detection; blocked kernel/network I/O has no hard cancellation guarantee. Native clients, offline playback, federation and multi-server access remain planned.
