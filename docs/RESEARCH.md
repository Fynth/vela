# Technology and product research

Initial research performed2026-10-13; source links are upstream/project-owned unless marked as engineering inference. Version verification used the crates.io API and official Rust stable channel manifest; Cargo.lock preserves the selected reproducible graph.

## Verified versions and choices

| Component | Verified upstream stable | Selected | Rationale |
| --- | --- | --- | --- |
| Rust | 1.99.0, channel manifest2026-10-01 | 1.99.0, edition2024 | Current stable, pinned with wasm/rustfmt/clippy targets |
| Dioxus | 0.7.10 | 0.7.10 | Production stable0.7 series; shared Rust UI, web platform APIs isolated |
| Axum | 0.8.9 | 0.8.9 | Tokio HTTP, multipart and transport framework |
| SQLx | 0.9.0 | 0.8.6 | Established SQLite migration/transaction APIs, minimal enabled backends; newer generation deferred to measured migration |
| Tokio | 1.53.2 | 1.53.2 | Bounded async workers and I/O |
| tower-http | 0.7.1 | 0.7.1 | Static web serving; range transport implemented/tested separately |
| Lofty | 0.25.4 | 0.25.4 | Embedded music tags/properties/artwork with read limits |
| Symphonia | 0.6.1 | Deferred | Optional future decoding/analysis; browser currently decodes originals |
| reqwest | 0.13.5 | 0.12.28 | Verified destination-pinning/proxy/timeout APIs; newer generation not necessary for initial secure imports |
| image | 0.25.10 | 0.25.10 | Bounded image decoding/reencoding, narrow JPEG/PNG/WebP features |
| Argon2 crate | 0.6.0 | 0.5.3 | Established PHC/password verifier API and Argon2id defaults |
| BLAKE3 | 1.8.7 | 1.8.7 | Streaming exact asset hashing |
| wasm-bindgen CLI | 0.2.129 | 0.2.129 | Matches compiled wasm-bindgen crate exactly |
| Playwright | 1.64.0, npm registry | 1.64.0 | Isolated real browser E2E; test-only JavaScript dependency |

Primary references: [official Rust stable manifest](https://static.rust-lang.org/dist/channel-rust-stable.toml), [Dioxus releases](https://github.com/DioxusLabs/dioxus/releases), [Dioxus0.7 documentation](https://dioxuslabs.com/learn/0.7/), [Axum](https://docs.rs/axum/0.8.9/axum/), [SQLx0.8.6](https://docs.rs/sqlx/0.8.6/sqlx/), [Lofty](https://docs.rs/lofty/0.25.4/lofty/), [reqwest ClientBuilder](https://docs.rs/reqwest/0.12.28/reqwest/struct.ClientBuilder.html), [SQLite WAL](https://sqlite.org/wal.html), [FTS5](https://sqlite.org/fts5.html), [wasm-bindgen releases](https://github.com/wasm-bindgen/wasm-bindgen/releases).

The host initially used Rust1.98.1; it was upgraded to the verified current1.99.0. The wasm-bindgen upstream binary archive was SHA256-verified against its published checksum. No platform success follows merely from choosing a cross-platform framework.

## Product strengths and boundaries

These conclusions distinguish observed documented scope from Vela design judgments.

- [Navidrome overview](https://www.navidrome.org/docs/overview/) supplies a useful lightweight self-hosted music/server-interoperability baseline. Engineering inference: retain a simple single-process operational model while making browser-managed importing and explicit social/federated access separate vertical slices. A lightweight server should not acquire mandatory distributed infrastructure to enable optional discovery.
- [Jellyfin storage guidance](https://jellyfin.org/docs/general/administration/storage/) separates durable configuration/database/media concerns and recommends a local database. Engineering inference: Vela should preserve managed originals, isolate cache/staging and implement user-controlled administration without broad filesystem privileges.
- [Funkwhale federation specifications](https://docs.funkwhale.audio/stable/developer/federation/index.html) and [user following](https://docs.funkwhale.audio/stable/specs/user-follow/index.html) demonstrate music federation through ActivityPub, actor identities, Follow/Accept and authorized library audio access. Engineering inference: adopt standards and privacy boundaries, and avoid treating metadata discovery as redistribution authority. Interoperability requires a tested profile, not merely ActivityPub-shaped JSON.
- [Plexamp](https://www.plex.tv/plexamp/) emphasizes dedicated listening, discovery and quality controls. Engineering inference: develop a persistent player/queue and consistent navigation first; validate gapless/analysis/background support before promising parity. Optional analysis must not make GPU/cloud accounts mandatory.
- [Spotify playlist API](https://developer.spotify.com/documentation/web-api/reference/get-playlist) exposes authorized playlist metadata. [Apple MusicKit](https://developer.apple.com/documentation/musickit) offers platform-authorized catalog/playback integration. Engineering inference: borrow workflow clarity and library organization without copying visual design or assuming these APIs authorize downloading original commercial streams. Metadata matching/import is distinct from audio import.
- [ListenBrainz](https://listenbrainz.org/about/) is an open listening-data ecosystem. Engineering inference: start with private local listening history and deterministic recommendations, then add consent-based external submission/export; no cloud inference requirement.
- SoundCloud-style creator discovery is a product inspiration, not an implemented adapter. A source adapter needs documented legitimate downloadable assets and permission metadata; no universal SoundCloud/extraction capability is claimed.

## Security and protocol references

[ActivityPub W3C Recommendation](https://www.w3.org/TR/activitypub/) provides established federation transport/vocabulary and security considerations; it does not define Vela media rights. [IANA IPv4 special registry](https://www.iana.org/assignments/iana-ipv4-special-registry/) and [IPv6 registry](https://www.iana.org/assignments/iana-ipv6-special-registry/) inform conservative outbound policy. [Azure168.63.129.16](https://learn.microsoft.com/en-us/azure/virtual-network/what-is-ip-address-168-63-129-16) is specially rejected despite appearing globally routed. HTTP range/conditional behavior is checked against [RFC9110](https://www.rfc-editor.org/rfc/rfc9110).

## Reusable components rather than copied systems

Reuse Axum/Tokio/Tower transport, SQLx/SQLite migrations/FTS, Lofty tags, image codec implementations, rustls-backed reqwest, Argon2id, BLAKE3, Dioxus and native browser audio/File primitives. Retain direct originals whenever the browser supports them. FFmpeg stays an optional controlled later subprocess. Do not copy service architectures wholesale; no microservices, unrestricted plugins, commercial downloader or cloud dependency has been introduced.

## Folder scanner and TLS checkpoint (2026-10-15)

The configured-root implementation uses [cap-std4.0.3](https://docs.rs/cap-std/4.0.3/cap_std/) and [cap-fs-ext4.0.3](https://docs.rs/cap-fs-ext/4.0.3/cap_fs_ext/), with descriptor-relative no-follow opens and nonblocking file admission on Linux. ADR0006 records why imported snapshots reuse managed media rather than directly referencing mutable external paths. Rustls was upgraded to0.23.46 after the four new primary RustSec notices linked in SECURITY.md. No unrelated framework upgrade was bundled into this storage slice.
