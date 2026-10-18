# ADR0018: native playback from an explicitly public peer catalog

Accepted 2026-10-18 for 0.12. Linux x86_64 native client only.

## Context

0.11 implemented an opt-in public catalog profile: an administrator enables a
canonical HTTPS origin, each owner explicitly publishes a track with license,
attribution, evidence and independent metadata/stream/download grants, and the
server exposes anonymous bounded catalog and media routes. The Dioxus web Network
view only previewed metadata and opened granted origin links externally. There was
no path from a second independently run Vela server into the native player, so
"playable authorized cross-server catalog" remained a product gap.

## Decision

Add a native `remote` module that talks directly to a peer's public profile:

1. Accept a canonical origin, fetch only `/api/public/catalog?limit=20`, bound the
   response to128KiB, reject redirects/proxies/cookies, and validate that the
   catalog's advertised origin and server UUID match the requested canonical origin.
2. Validate every public record's opaque ID, duration, text bounds/control
   characters and grant flags. Only records with `can_stream` are playable;
   metadata-only rows are labelled and cannot reach audio.
3. Prepare a public original by requesting only
   `/api/public/media/{id}/stream`, requiring a bounded content length and a quoted
   64-hex ETag, streaming into an unnamed private0600 temporary file, and checking
   size plus BLAKE3 before handing it to the one native `AudioEngine`. The session
   cookie is never attached to a peer request.
4. Remote playback is a distinct player mode: it replaces the native source, shows
   the public origin/title/artist and does not advance or mutate the private queue.
   Starting local playback clears the remote mode. MPRIS reports stopped/no metadata
   while remote audio plays rather than attributing it to a private track.
5. For the isolated two-instance QA only, a debug build may map one exact canonical
   origin to a loopback HTTP transport (`VELA_NATIVE_PEER_CANONICAL` +
   `VELA_NATIVE_PEER_TRANSPORT`). Release builds ignore the override, and the
   catalog's advertised identity is still validated. This is test transport, not a
   claim that Internet TLS/certificate validation passed.

## Consequences

- A listener can discover one opted-in public Vela catalog and play a track whose
  owner granted streaming, without copying the file into their own library, proxying
  bytes through the home server, or sharing private history.
- The peer must be reachable from the client; there is no saved peer list, unified
  remote search, cross-server queue, pagination beyond the first20 records, peer
  directory, caching, retry UI, or download/offline entitlement.
- Revocation is checked at each new catalog/stream request; bytes already received
  cannot be recalled. No DRM or license verification is implied.
- Internet HTTPS certificate/redirect behavior, Safari/mobile clients, real two-host
  networking and high-latency peers remain unverified. The QA uses two isolated
  Vela processes on loopback with a debug-only transport override.

## Evidence

`target/checks/native-public-peer5/`: the harness starts a second isolated Vela
server, enables its public catalog with a canonical advertised origin, publishes the
fixture with an explicit stream grant, then the native client fetches and plays that
track through the native engine and identifies the public origin context. Screenshot:
`native-public-dark.png`. Exact executed commands and remaining limits are in
PROGRESS/QA_EVIDENCE.
