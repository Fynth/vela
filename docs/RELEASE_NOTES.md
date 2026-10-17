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
