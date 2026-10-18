# ADR0017: native Linux playback behind a bounded preparation boundary

Accepted 2026-10-18 for 0.12, verified on this Linux x86_64 host only.

## Context

The web client owns one `HTMLMediaElement` and the shared occurrence queue, but a
browser document cannot provide MPRIS, real OS audio output ownership or a native
client that remains a first-class target. The master mandate requires a genuine
native client, not a WebView wrapping the web app. Android/Apple toolchains are
absent on this host; Linux GTK/WebKitGTK/ALSA and a session D-Bus are available.

The existing server exposes owner-authorized Range streams and the owner-scoped
catalog API. No server-side playback session, download grant or remote-capability
service exists. Credentials must not be written to plaintext native storage, and
private listening events may only be collected under the existing consent model.

## Decision

Add an eighth workspace application, `vela-desktop`, for Linux x86_64:

1. **Shared contracts, separate transport.** It consumes the existing framework-free
   domain DTOs and reuses `apps/web/src/playback.rs` (occurrence order, shuffle deck,
   repeat semantics) by path, so queue transitions do not fork. HTTP is a dedicated
   native client with in-memory cookie/CSRF only, TLS verification on, redirects and
   proxy inheritance disabled, and canonical HTTPS (or numeric loopback HTTP for
   development) origins.
2. **Bounded source preparation before decode.** The first adapter downloads one
   authorized owned original into a private `0600` temporary file, verifies the
   exact expected size and the response BLAKE3 ETag, and caps the file at 250 MiB.
   On Linux the file is unlinked immediately and each decoder reopens it through
   `/proc/self/fd/N`, so an interrupted GUI cannot leave a completed original named
   on disk. A prepared source lives only while its queue occurrence is current and
   is released on replacement, sign-out, failure or exit. This is deliberately not
   progressive streaming, offline storage or a download entitlement.
3. **One audio authority.** `AudioEngine` owns a long-lived supervised output thread,
   one bounded command channel and generation/source guards. Rodio/CPAL decode WAV,
   FLAC, MP3, MP4/AAC-family and Vorbis through the bounded local source; seek,
   pause, resume, volume, EOF and replacement are explicit commands. No second audio
   element, iframe or remote page is used.
4. **OS integration is an observer.** MPRIS2 runs on its own thread with a typed
   command channel; it receives immutable public metadata/state snapshots and never
   owns credentials, HTTP, account state or audio. A failed session bus degrades to
   in-app controls. No local file paths or private fields are published.
5. **Consent stays explicit.** This slice sends no native listening events, does not
   infer playback history from preparation/decode, and keeps the account in memory
   only. Theme is currently in-session; persistence and favorites UI are later work.

## Alternatives considered

- WebView `<audio>` or an embedded web page: rejected; it would not provide native
  output ownership, OS media controls or a real desktop application, and would
  duplicate the web player.
- Progressive decode directly from the authenticated HTTP Range response: deferred
  until a resumable/seekable reader and cancellation model are implemented and
  measured; preparation is simpler to bound and verify first.
- Persistent native credentials (keyring/Keychain): deferred; the first slice must
  not pretend secure storage exists, so the account is memory-only and requires
  sign-in on launch.
- Server playback sessions/device handoff: out of scope; no server contract exists
  yet and inventing one would create a duplicate authority.

## Consequences

- Playback has an explicit "preparing" phase with bounded byte progress, cancel and
  retry, and cannot start before the full authorized original is local. Startup
  latency and disk usage are therefore real costs; neither is claimed as gapless.
- Offline playback, mobile background audio, packaged installers, secure persistence,
  gapless/crossfade/ReplayGain, remote/federated sources and device handoff remain
  unimplemented. Only Linux x86_64 window/audio behavior is verified on this host.
- The reusable pieces (`PlaybackOrder`, DTOs, playback state vocabulary) can be
  shared with later Android/iOS adapters without moving policy into the UI.

## Evidence

`target/checks/native-0.12-window4/` (real WebKitGTK window, isolated fixture server,
owned Pulse null sink, screenshots at 1280×840 and 640×700, zero leftover temporary
originals), `native-0.12-{fmt,clippy,tests}.log`, and the opt-in ignored unit tests
under `apps/desktop/src/audio.rs` and `mpris.rs`. Exact results and unverified items
are recorded in PROGRESS/PLATFORM_MATRIX/QA_EVIDENCE; design-only claims are not
acceptance.
