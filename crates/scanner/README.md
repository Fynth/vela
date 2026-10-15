# Read-only folder scanner

This crate turns explicitly configured operator storage roots into private,
immutable managed media snapshots. Administrators choose a root identifier in
the application; the application never accepts a filesystem path from a user.
HTTP authorization, CSRF, configuration validation and process supervision live
in the server/core crates.

The Linux implementation opens absolute roots component by component from a
trusted directory descriptor, refusing symlinks throughout. Every subsequent
directory/file open is descriptor-relative and refuses symlinks. A nonblocking
open precedes the regular-file and single-hardlink check, so a raced FIFO cannot
block the worker. Unsupported files, special files and invalid UTF-8/control
names are skipped. Directory iterators form a depth-first bounded stack; depth
includes the root iterator.

Each admitted file is streamed through a 64 KiB buffer into a private `scan-`
staging file, checked against file size, free-space reserve, a 15-minute copy
deadline and cancellation between reads. Metadata size/mtime/ctime/device/inode
is compared before/after copying. The snapshot is fsynced before the real media
service validates and publishes it. Source entries are never written, renamed
or deleted. Ordinary filesystem calls can remain blocked in the operating
system on faulty/network storage; the deadline is checked between calls.

Jobs and cancellation are durable in SQLite. One worker processes bounded
queues serially. Startup and shutdown safely requeue interrupted passes, while
atomic media publication and the fingerprint update finish before cancellation
is honored. Recognized abandoned regular staging files are cleaned at startup;
other names, directories and symlinks are preserved. Infrastructure failures
stop the worker instead of masquerading as malformed audio.

Per-source hashed relative path keys and small descriptor fingerprints let
unchanged files with a still-owned track skip ingestion. This is a best-effort
change detector, not a cryptographic content verification pass. Changed files
are ingested/deduplicated through the normal media pipeline; exact duplicates
preserve catalog edits, while different bytes append a new snapshot. Missing
source files and removed source definitions never delete owned music.

Other operating systems report configured roots unavailable until their safe
descriptor behavior is implemented and validated. No cross-platform scanner
claim is made. Real filesystem/SQLite/Lofty tests reside in `tests/scanning.rs`;
the workspace's main agent executes and records their results.
