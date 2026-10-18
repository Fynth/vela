# 0022: Native complete-file media cache

Date: 2026-10-19. Status: accepted for source `adc6ea0`.

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

## Consequences

- A→B→A with a warm, still-authorized entry transfers no audio body.
- Integrity remains whole-file BLAKE3 before decode on both hit and miss.
- Cold playback may perform one extra local copy to warm the cache after the
  listening lease is already secured (follow-up: write staging directly).
- Eviction is insertion-age FIFO, not strict LRU; hits do not refresh
  `saved_at`.
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
