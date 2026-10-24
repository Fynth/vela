# 0022: Native complete-file media cache

Date: 2026-10-19. Status: accepted for source `adc6ea0`.
Amended 2026-10-24 (TL-02): the eviction order follows the last *confirmed
hit*, not the insertion.

## Context

Native playback used to download the whole original before the first sound and
never reused bytes, so A→B→A transferred the full body again. The existing
whole-file size+BLAKE3 check is a real integrity guarantee and must not be
silently weakened.

## Decision

Add a reusable complete-file cache under
`$XDG_CACHE_HOME/vela/media/<origin-hash>/<account-id>/<track-id>.audio` with a
sidecar `{version:1, origin, account_id, track_id, size_bytes, etag, digest,
saved_at}`. Dirs 0700, files 0600. It is distinct from pinned offline copies
(`$XDG_DATA_HOME/vela/offline`), which are never evicted or touched.

`prepare` performs one bounded authenticated HEAD on the stream route first.
401/403/404 return the same error and cached bytes are never consulted. A hit
requires the sidecar origin/account/track/size/ETag to match the response and
the cached descriptor to re-hash to that ETag; the same descriptor is rewound and
handed to the decoder. A miss falls back to the unchanged full GET and verified
unnamed lease, then warms the cache best-effort with unique staging, fsync and
atomic publish (audio then sidecar, previous healthy pair restored on failure).

Bounds: 200 entries / 2GiB per account namespace, staging and orphans counted,
eviction only of this cache's own `<uuid>.audio`/`.json` pairs oldest by
`saved_at`, clean refusal when it cannot fit. Existing `preparing` admission
serializes HEAD+GET+store, so identical concurrent prepares do not duplicate
transfers.

`saved_at` is the recency of the entry: the time of its last **confirmed hit**,
not the time it was inserted. A confirmed hit is the origin re-authorizing the
representation and the cached original verifying against it. The mark is
best-effort and off the path to the first sound: it runs in the background, is
bound to the client lifetime (a sign-out stops it) and not to the per-selection
cancel token, and it is skipped while a publication of the same key runs. It
rewrites the sidecar alone — the cached original is not opened, read or written
— so the bytes, the size, the inode and the modification time of the audio file
are untouched, and it weakens no check of a hit: the origin's validator still
decides whether the entry may be used and the cached original is still
re-hashed from the descriptor the decoder receives. The clock has a
one-second resolution, so the new value is also strictly newer than the one it
replaces. A mark that finds no record, a record of another version or a record
of another representation writes nothing at all.

**Known boundary — a mark and a publication of the same key can race.** The
mark re-reads the record and the cached original immediately before its rename
and drops itself when either changed, but the window between that check and the
rename is not closed. A publication that lands in that window is overwritten by
the mark, which keeps the entry usable but can move its `saved_at` back to the
older value; the reverse order leaves the publication's fresh `saved_at` in
place. A mark that lands after the eviction pass computed its candidates can
still be evicted, which costs one re-download and nothing else. The mark does
not run against a live publication of the same key in the first place.

## Consequences

- A→B→A with a warm, still-authorized entry transfers no audio body.
- Integrity remains whole-file BLAKE3 before decode on both hit and miss.
- Cold playback may perform one extra local copy to warm the cache after the
  listening lease is already secured (follow-up: write staging directly).
- Eviction follows the last confirmed hit, not the insertion: a track that is
  played every day keeps its entry while a track that was downloaded once is
  evicted first. The refresh is best-effort — it is skipped while a
  publication of the same key runs, and a mark that loses the race with a
  publication can leave an older `saved_at` in place. It is not a strict LRU:
  the value is a one-second clock, and a mark is only as good as the last
  confirmed hit.
- The mark is a local cache fact, not a listening event: no media time is
  counted, nothing is reported to the origin, and nothing leaves the device.
- Cache persists across sign-out but is account/origin-scoped; an explicit
  erase/TTL policy is still open.
- Progressive playback (W03) is separate and still requires an ADR before it can
  relax whole-file verification.

## Verification

In-process real Vela server test checks sidecar ETag == BLAKE3(original), warm
lease inode == published cache file; A→B→A byte counters assert one GET per
track and only the two initial bodies; tampered same-size bytes force a new GET.
Desktop 71 passed/3 ignored; native peer/session/offline window QA passed.
Real-process body counters, ENOSPC and production-budget eviction are NOT_RUN.

Amended 2026-10-24 (TL-02): the desktop suite is **151 passed / 0 failed / 3
ignored** (143 before, all kept) and the native X11 window run is **314 checks /
24 PNG / exit 0** with `result.ok=true` and `temporary_originals_after_exit=0`
(`target/checks/tl02-recency-native/`). The new regressions assert that a
confirmed hit grows `saved_at` while the audio file keeps its bytes, size,
inode and modification time; that the eviction drops the entry that was never
played again and keeps the one that was; that a foreign validator, size,
identity, version or a missing record is refused with no file written; that a
canceled mark leaves no staging file; and that the prepared lease is returned
before the mark is written. The multiprocess mark↔publication race, a real
ENOSPC and the 200-entry/2GiB production budget are still NOT_RUN.
