# Local checkpoint0.2.0

This is a local review checkpoint, not an externally published release. See PROGRESS for executed validation and KNOWN_LIMITATIONS for unsupported platforms/features.

## Read-only folder imports

Administrators can add named sources from installation-approved music folders in Settings, start scans, watch counters, cancel and rescan. The Linux scanner imports real audio metadata/artwork through the existing media pipeline, never modifies original folders and preserves catalog edits on exact-duplicate/unchanged rescans. Removing a source removes its scan tracking only; imported music stays playable.

Configuration defaults to the private data/sources folder. VELA_SCAN_ROOTS selects explicit absolute roots or [] disables scanning. The browser accepts root identifiers, never arbitrary host paths. Descriptor-relative no-follow opens, regular-file/hardlink checks, bounded traversal/copies/queues and free-space checks protect the filesystem boundary. One durable scanner worker runs alongside imports, with restart recovery and process supervision.

## Security and operations

The locked rustls version was updated from0.23.45 to0.23.46 after four runtime advisories appeared during review. No runtime advisory exception was introduced. Local archives now use version-specific checksums and preserve prior checkpoints. Migration0004 is forward-only: back up before upgrading, retain old binaries/backups, and restore the matched pair for rollback. Managed audio/backups work without original scan folders; back up original external collections separately.

Folder scans copy audio into managed storage and require additional space. Linux local filesystems are the supported scanner environment. File signatures provide best-effort change detection; blocked kernel/network I/O has no hard cancellation guarantee. Native clients, offline playback, federation and multi-server access remain planned.
