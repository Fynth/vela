# 0022: Native complete-file media cache

Date: 2026-10-19. Status: accepted for source `adc6ea0`.
Amended 2026-10-24 (TL-02): the eviction order follows the last *confirmed
hit*, not the insertion.
Amended 2026-10-24 (TL-02, round 2): the recency label of a confirmed hit is
the wall clock of the moment of the mark — never a value that runs ahead of it
— and the number of marks that run at the same time is bounded.

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
re-hashed from the descriptor the decoder receives. A mark that finds no
record, a record of another version or a record of another representation
writes nothing at all.

**The recency label is the wall clock of the moment of the mark** (whole
seconds since the epoch), and it is never a value from the future. The label
must stay comparable between tracks: a label that ran ahead of the clock (for
example `max(now, previous + 1)`) would make one track look older than a track
that was really played later, and the eviction would then drop exactly the
entry that was played most recently. Two confirmed hits inside the same second
therefore carry the same label, and the order between them is resolved by the
tie-break on `track_id` that the eviction already applies — the same order for
every run, and not a silent preference for one of the two tracks.

**The number of marks that run at the same time is bounded.** The mark is an
optimization: the recency of an entry is a cache fact, not a grant, and a
dropped mark costs one recency label. The bound is taken *before* the task
starts and without waiting; a mark that cannot take a slot is dropped instead
of queued, because a queued mark would occupy the blocking pool that the next
transfer needs, and the prepared lease must never wait for a mark. The slot
lives as long as the task, so it is released when the task ends, is aborted or
panics.

**Known boundary — a mark and a publication of the same key can race.** The
mark re-reads the record and the cached original immediately before its rename
and drops itself when either changed, but the window between that check and the
rename is not closed. A publication that lands in that window is overwritten by
the mark: the entry stays usable **as long as the republished representation is
byte-for-byte the same one** (the same ETag). If the origin changed the
representation in the meantime, the older sidecar now describes bytes that are
not there, every hit of that entry misses and the entry is fetched again once
— the integrity check still holds, the cache simply does not serve that entry
any more. The reverse order leaves the publication's fresh `saved_at` in place.
The label of a mark is the current clock, so the race cannot move it *back* to
an older value: the only way the label of an entry goes back is a system clock
that moves back. A mark that lands after the eviction pass computed its
candidates can still be evicted, which costs one re-download and nothing else.
The mark does not run against a live publication of the same key in the first
place.

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
  the value is a one-second wall clock, two hits inside the same second share
  it and the tie-break on `track_id` decides their order, and a mark is only
  as good as the last confirmed hit.
- The mark is a local cache fact, not a listening event: no media time is
  counted, nothing is reported to the origin, and nothing leaves the device.
- The marks are bounded: at most four run at the same time for one client, a
  mark that cannot take a slot is dropped, and the prepared lease never waits
  for a mark. A dropped mark costs one recency label, never a playback.
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

Amended 2026-10-24 (TL-02, round 2): the desktop suite is **155 passed / 0
failed / 3 ignored** (151 before, all kept) and the native X11 window run on
the final tree is **314 checks / 24 PNG / exit 0** with `result.ok=true` and
`temporary_originals_after_exit=0` (`target/checks/tl02-r2-native/`). The round
adds four regressions and corrects two claims of the first round:

- the label of a confirmed hit is the wall clock of the moment of the mark, so
  ten hits in a row never write a value from the future, and the label of one
  track never runs past the label of a track that was really played later (the
  eviction then drops the older entry, as it must);
- the marks are bounded: a mark that cannot take a slot is dropped, the
  prepared lease is returned anyway with the verified bytes, and the slot is
  released when the held marks end, so the package does not lose the recency
  of an entry forever.

The round-1 native artifact `target/checks/tl02-recency-native/` belongs to an
intermediate revision of the two Rust files and is **not** evidence for the
final tree; the round-2 artifact above is. An earlier round-2 run of the same
code is preserved under `target/checks/tl02-r2-native-071703/` (07:17:03): it
exercises byte-identical sources, but the two files were rewritten afterwards
by the mutation experiments, so its `mtime` is older than the final tree's.
The scope of both round-2 runs is the native window class — one run each, not
repeated runs, and not a browser render.
