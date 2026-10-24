# ADR0026 — Native warm-cache publication off the first-sound path

Date: 2026-10-23. Status: accepted for the TL-01 step-1 slice. No schema, API,
dependency or trust-model change.

## Context

`VelaClient::prepare` (`apps/desktop/src/client.rs`) resolves one owner-authorized
original, verifies exact size plus the response BLAKE3 ETag, and returns an
unnamed private `0600` lease. It then *synchronously awaits*
`CacheScope::store`, which performs a second full read+write+`fsync` of the same
bytes through `media_cache::copy_verified`.

Measured on this host with a real 37 635 761 B original
(`/var/lib/vela/media/6874dbb7....audio`), three trials of the identical
64 KiB read → write → `fsync` loop: **140.4 / 119.7 / 117.0 ms**. That cost sits
between verification and the first audible sample even though ADR0022 already
declares the cache an optimization, not a condition of playback.

TL-01 also names progressive decoding as the target outcome. That would hand
partially transferred bytes to the decoder, relaxing "whole-file size + BLAKE3
before decode", which `AGENTS.md` and
[NATIVE_CLIENT_CONTRACTS](../NATIVE_CLIENT_CONTRACTS.md) state as a current
contract. ADR0022 explicitly requires a separate ADR before that relaxation.

## Decision

1. **Publication leaves the critical path.** Once size and BLAKE3 have matched,
   `prepare` returns the verified lease immediately and warms the cache in a
   bounded background task that holds the `Arc<PreparedAudio>` lease and the
   client `lifetime` token. The caller's per-selection cancel token is not
   observed *by that background task*: the UI cancels it as soon as another
   track is selected, and aborting the warm-up of the track that just played
   would break decision 2. The one exception is the fallback below.
2. **No duplicate transfer regression.** A bounded in-flight publication
   registry (keyed by `media_cache::CacheKey`) makes a second `prepare` of the
   same track await the in-progress publication instead of starting another
   GET. A failed publication degrades to the ordinary verified GET.
3. **Trust is unchanged.** The decoder still only ever receives a lease whose
   exact size and whole-file BLAKE3 were verified against the origin ETag. Two
   kinds of lease exist and the difference is named, not flattened: a **cold**
   transfer is leased as an unnamed private `0600` file with `nlink() == 0` on
   Linux, and a **warm hit** leases the published cache file itself, which keeps
   its single name and reports `nlink() == 1`. Both are `0600` regular files that
   were re-hashed from the very descriptor the decoder receives; the cache file
   is re-verified on every hit.
4. **Progressive decoding remains unapproved.** This ADR does not authorize
   partial-byte decode, Range-based playback, or a weaker verification mode.
   W03 needs its own ADR naming the exact trust model, the bounded fallback and
   the verified path it keeps.

## Round 2 corrections (2026-10-24)

The first implementation of decision 2 was rejected in review with three P1
defects in the coordination itself. The corrections keep the decision and
change its mechanics:

1. **The wait never holds the transfer admission.** The `preparing` semaphore
   covers the transfer body itself, never the wait for a running publication and
   never the final cache read that may serve a hit. A `prepare` that waits for a
   publication of track A therefore cannot refuse the play of track B with
   `PREPARE_BUSY`. The admission is taken after the wait, and the state is
   re-checked with it held: the cache is read again, and a publication of the
   same key that runs now is waited for without the admission and taken again for
   the transfer. Only the first of the two post-admission reads runs under the
   admission; the second one runs after the admission was released for the wait,
   because a warm hit must never be answered with `PREPARE_BUSY`. The warm re-read
   that answers a publication that ended inside the first read is released from
   the admission for the same reason (round 4): it also re-hashes a whole
   published original, and the transfer takes the admission again if it misses.
2. **The single-flight slot is observed before the cache is read.** A
   publication that ends between the cache miss and the decision can no longer
   be mistaken for a cold cache: the slot was captured first, the wait returns
   immediately, and the cache is read again instead of a second body being
   transferred. The slot carries an explicit *ended* flag that flips only after
   the cache entry was published (or the copy failed), so a waiter that observes
   the end knows the state of the cache.
3. **A live publication is never replaced.** `register_publication` joins a
   running publication of the same key instead of overwriting its entry: a
   second publication would copy the very same bytes. An expired wait therefore
   creates no second publisher, and a new publication of that key is possible
   only after the previous one really ended. The registry bound counts *live*
   publications, not dictionary entries: an entry of a publication that ended
   describes nothing that runs.

## Round 3 correction (2026-10-24)

Decision 2 was implemented in `cached_lease` but not in the second, post-admission
read: `recheck_cache` read the cache **before** it observed the single-flight
slot, so a publication that ended inside that window was answered as a cold
cache and cost a second body (`GETs=2` / 131072 B instead of 1 / 65536). Both
reads now observe the slot before they read, and a miss is followed by the
bounded wait and a second read. A slot that was live at the read and has ended
since is answered with a second read of the cache even when the earlier wait had
already run out — that was the remaining hole, found in review of this round.
No other decision changed.

## The registry-full fallback (round 3)

The background task above is the ordinary path. When the registry of running
publications is already full (`MAX_IN_FLIGHT_PUBLICATIONS`, 32 live copies), no
background task starts and `prepare` awaits `scope.store(...)` **inside itself**,
on the path to the first sound. That fallback deliberately keeps the caller's
per-selection cancel token: it observes `cancel`, so a new selection aborts the
copy instead of leaving a stale synchronous copy behind. The consequence is
honest and narrow: in that corner a warm-up of track A canceled by selecting B
means A→B→A transfers the body of A again. It is a cache-only effect — the
decoder always receives its own verified lease — and it needs 32 simultaneous
live publications to be reachable.

## Consequences

- Cold playback no longer pays a second full copy before the first sample.
- The cache can lag behind playback by one publication; A→B→A inside that window
  waits for the publication rather than re-downloading.
- A publication aborted by logout/cancel leaves no lease, no staging file and no
  half-published entry; a later `prepare` re-verifies from the origin.
- A pending publication never blocks another track: the wait for it holds no
  transfer admission.
- A publication that ends between the cache miss and the single-flight
  decision costs no second body; the cache is read again.
- An expired wait costs one ordinary transfer **unless the publication ends
  inside the first post-admission read, which is answered with a second read of
  the cache instead**; it never starts a second publisher, and the running
  publication keeps the single-flight slot of its key.
- Eviction remains insertion-age FIFO; this ADR changes neither budgets nor
  integrity checks.
- Progressive playback stays a follow-up with its own ADR and its own evidence.

## Verification

Real in-process Vela server tests assert: `prepare` returns before the cache
publication completes; A→B→A performs one GET per track with unchanged body
counters; a concurrent second `prepare` during publication issues zero extra
GETs; a failed publication still plays and re-fetches exactly once; logout
during publication stops it and leaves no orphan staging permanently behind.
Round 2 adds: a pending publication does not block the play of another track
(the other track is leased, not refused as busy); a publication that ends
between the cache miss and the single-flight decision costs no second body
(1 GET, 65536 bytes, not 2 GETs and 131072 bytes); an expired wait starts no
second publisher and leaves exactly one live registry entry, and exactly one
new publication runs after the first one ended.
Round 3 adds: a publication that ends between the post-admission miss and the
single-flight decision costs no second body (the slot is observed before that
read); a publication that ends inside the first post-admission read *after* an
expired wait costs no second body either (the `ended` branch); a publication
that ignores the client lifetime publishes nothing and leaves no staging file
once its copy has really started.
Round 4 adds: the warm re-read that answers an ended publication runs without
the transfer admission — a held re-read does not refuse the play of another
track with `PREPARE_BUSY`, and the track is still transferred once. The test
gate also restores only the gate it installed, so two tests installing it
concurrently cannot steal each other's slot.
Existing desktop suite (129 passed / 0 failed / 3 ignored at baseline) must stay
green.
