# Media service

The service creates an immutable temporary copy, computes BLAKE3 while copying,
validates content with strict Lofty parsing, publishes content-addressed files
without replacing existing paths, flushes the file and Unix directory, then
commits a private track plus its physical asset in a SQLite transaction. The
shared ingestion semaphore serializes parsing, quota checks and publication.
The caller retains responsibility for its upload/import staging file.

Metadata reads are limited to 32 MiB with an 8 MiB limit on individual Lofty tag
allocations. Files requiring more parsing input are rejected. Artwork is limited
to 8 MiB, 4096 pixels per dimension and 16 million pixels; only PNG, JPEG and
WebP are decoded. Artwork is reencoded as a maximum 640-pixel JPEG. Invalid
artwork is discarded while valid audio remains importable.

Limitations: validation checks format structure and audio properties; it does
not decode every sample and cannot guarantee that every later frame is intact.
The codec field for WAV, AIFF, MP4 and MPEG containers deliberately avoids
asserting an encoding not established by the generic metadata reader. Playback
compatibility remains dependent on the client's decoder. Album statistics
group album title and album artist, falling back to track artist; separate editions and recordings are not
modeled yet. Managed quota accounting covers physical audio assets referenced
by SQLite, while thumbnails and staging have separate bounded per-file limits.
A process crash between durable file publication and SQLite commit can leave
an unreferenced file; offline integrity reconciliation is not yet implemented.
Startup removes only regular unpublished temporary files with the service-owned
`.vela-pending-` prefix in managed media/artwork directories. It retains
published content hashes, other files, directories and symbolic links.
Single-process operation with cloned service instances is the supported quota
concurrency model. FTS prefix search uses at most 20 literal terms; query length
is limited to 256 bytes. A bounded4097-row FTS probe preserves efficient sparse
searches; all filter values use SQL parameters. Typed filters combine whole-tag
genre, stored format label, release-year bounds and known minimum audio
properties. Minimum bitrate is in kbps; sample rate is in Hz. Unknown values
do not satisfy numeric criteria. A format/container label or a high sample rate
does not establish lossless encoding.

Library pages use UUID order by default; title order uses ASCII NOCASE plus UUID
and recently-added order uses descending addition time plus descending UUID.
Sorted cursor lookups are owner-scoped and share the page's SQLite read snapshot.
Pages contain at most100 tracks and one sentinel, without OFFSET. Concurrent
imports or metadata edits can move boundaries between requests; pagination is
not a catalog snapshot across requests. The original `list_tracks` method is a
wrapper with default filters. Genre and format suggestions are read in one
snapshot, collapsed by ASCII NOCASE and bounded100/32 plus truncation flags.

`tests/fixtures/signal.flac` and `beacon.mp3` are real one-second synthetic sine
tones generated locally with FFmpeg, not recordings from external artists.
The FLAC includes the locally generated solid-color PNG cover and embedded
title, artist, album, year, genre, track and disc metadata. These fixtures are
redistributable with this project's license. The tests ingest the real encoded
bytes into real SQLite and filesystem storage.
