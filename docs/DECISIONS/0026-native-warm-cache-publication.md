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
   client `lifetime` token. The caller's per-selection cancel token is
   deliberately not observed: the UI cancels it as soon as another track is
   selected, and aborting the warm-up of the track that just played would
   break decision 2.
2. **No duplicate transfer regression.** A bounded in-flight publication
   registry (keyed by `media_cache::CacheKey`) makes a second `prepare` of the
   same track await the in-progress publication instead of starting another
   GET. A failed publication degrades to the ordinary verified GET.
3. **Trust is unchanged.** The decoder still only ever receives a lease whose
   exact size and whole-file BLAKE3 were verified against the origin ETag; the
   lease remains an unnamed private `0600` file with `nlink() == 0` on Linux.
4. **Progressive decoding remains unapproved.** This ADR does not authorize
   partial-byte decode, Range-based playback, or a weaker verification mode.
   W03 needs its own ADR naming the exact trust model, the bounded fallback and
   the verified path it keeps.

## Consequences

- Cold playback no longer pays a second full copy before the first sample.
- The cache can lag behind playback by one publication; A→B→A inside that window
  waits for the publication rather than re-downloading.
- A publication aborted by logout/cancel leaves no lease, no staging file and no
  half-published entry; a later `prepare` re-verifies from the origin.
- Eviction remains insertion-age FIFO; this ADR changes neither budgets nor
  integrity checks.
- Progressive playback stays a follow-up with its own ADR and its own evidence.

## Verification

Real in-process Vela server tests assert: `prepare` returns before the cache
publication completes; A→B→A performs one GET per track with unchanged body
counters; a concurrent second `prepare` during publication issues zero extra
GETs; a failed publication still plays and re-fetches exactly once; logout
during publication stops it and leaves no orphan staging permanently behind.
Existing desktop suite (129 passed / 0 failed / 3 ignored at baseline) must stay
green.
