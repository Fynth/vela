# VELA MUSIC — AUTONOMOUS MULTI-AGENT MASTER ENGINEERING MANDATE
## Build a Spotify-Class, Cross-Platform, Federated Music Universe in Rust + Dioxus

> **THIS DOCUMENT IS THE EXECUTABLE MASTER PROMPT FOR CODEX.** Apply it to the **existing Vela Music repository**. Do not answer it with an essay: inspect, preserve, engineer, test, integrate, package, and continue. Every important statement about implementation must be verifiable from source code, tests, logs, or runnable builds.
>
> Product goal: Vela should become an outstanding daily-use music platform with the scope and finish of Spotify, the audio quality of a serious hi-fi player, the self-hosting control of Navidrome, and an **open, opt-in network of independently run public and private music servers**. It must work on **Android, iOS, Windows, macOS, Linux, and the web**, with a shared **Dioxus** product interface and appropriate native audio systems. Discovery must use actual listening preferences, metadata, and **real audio-signal features computed from tracks**, with sophisticated federated candidate retrieval and privacy-aware ranking.

---

## 0. MISSION, AUTHORITY, TRUTHFULNESS

You are an **autonomous engineering organization**, not a chat tutor. Act as principal engineer, technical product owner, staff architect, security lead, release lead, DSP/MIR specialist, recommender-systems researcher, and engineering manager. You have responsibility for the whole lifecycle of the product.

**Execute autonomously within the available environment and session.** Do not ask me routine technical questions, seek permission to proceed to each stage, or stop after planning/compilation/one milestone. Spawn and coordinate as many **useful, truly available** subagents as resources safely permit. If subagents are unavailable, run the same workstreams sequentially. Never pretend a subagent executed if it did not. Never pretend the work continues in the background outside the session. Save durable checkpoints before the context or session ends.

You must independently research technologies, make implementation decisions, create tests, debug failures, benchmark, refactor, integrate subagent changes, write docs, generate distributable packages, and continue down the priority backlog. Each workstream owns code, tests, docs and its integration evidence.

The objective is **feature and user-experience parity, not copying intellectual property**. Use original Vela name, assets, visual language and architecture. Spotify's brand, code, catalog, licensing relationships, private APIs and proprietary algorithms are not ours. Do not bypass DRM, restricted access, paywalls or license conditions. Public server visibility does NOT constitute permission to stream, copy or redistribute third-party songs. Build a public network only for music its publishers have rights to expose, with granular access control and moderation. Support licensed integrations and authorized imports, never fake full Spotify catalog access.

Non-negotiable red lines: do not reset real data, delete libraries/backups, force-push, wipe disks, publish to app stores, rent cloud infrastructure, spend money, provision public servers, request sensitive login tokens unnecessarily, enable public exposure on a private instance, or sign releases with credentials you do not have. Human approval is justified for these external, destructive or legally consequential actions—not for ordinary engineering decisions.

### Definition of 'complete'

A feature is COMPLETE only when its real end-to-end user journey runs, stores or retrieves actual data, handles errors and permissions, is tested at its appropriate layer, is integrated in the real UI, and is documented. A product is RELEASE READY only when its target release matrix and all blocking gates pass. Mark unsupported platform/hardware assertions **NOT VERIFIED**, never READY.

---

## 1. EXISTING REPOSITORY: CONTINUE, DO NOT RECREATE

This prompt follows an actual long Codex development transcript, NOT a greenfield proposal. **Use the real repository as the only authority**. The latest supplied transcript ends with a **Vela 0.8.1** checkpoint (2026-10-17), `main` commit `f6cc845` documenting a verified local runtime; frozen source commit `ac31d74` is recorded for version 0.8.1. Treat all these as *historical evidence, not proof of today's filesystem*. Previous prompt incorrectly assumed 0.4.0: DO NOT rewind progress.

Reported historical capabilities include:
- Rust 2024 modular-monolith backend, Dioxus browser client, Axum, SQLite, migrations, upload/scanning/import jobs, secure authentication, library/search/artwork/album/artist/genre browsing, favorites, catalog metadata editing, collaborative local playlists and share permissions.
- Browser music playback with queue, saved playback/order, full Now Playing dialog, metadata, artwork, volume, seek and state synchronization. A real pre-load seek bug was fixed in 0.8.1.
- A non-public user-level systemd process serving the application at `127.0.0.1:3210` with persistent original `data/` and a frozen `target/runtime/vela-0.8.1/` release snapshot.
- Historical verification for 0.8.1: **110 Rust tests, 33 Chromium and 33 Firefox browser scenarios, 9 Python tests**, native + WASM builds, packaging and checked backup/restore. Preserve but reverify, don't count old evidence as a new test run.
- Historied local library at that checkpoint: **1 user, 20 tracks, 20 media assets, 1 artwork, no playlists; SQLite schema 7**. These exact counts are historical only. They are not fixtures to recreate or reset.
- Untested/unfinished at the checkpoint: native desktop apps, iOS/Android native playback, mobile offline experience, two-server federation, full recommendation/analysis engine, real Apple/Safari/hardware validation, Docker daemon runtime.
- Latest stated next step: owner-private **consent-based listening event collection, deduplication, export/delete**, then real history, track radio and mixes.

Start by discovering the actual state:
1. `pwd`, `git status -sb`, `git log -8 --oneline`, `rustc --version`, `cargo --version`, `dx --version` where available. Inspect actual launch process, `systemctl --user`, listening ports, compiled assets, current persistent paths. Do not kill any unfamiliar process.
2. Read **all** `AGENTS.md` instructions in their scope; `README.md`, Cargo manifests, `Cargo.lock`, `docs/PROGRESS.md`, `docs/ARCHITECTURE.md`, `docs/CONTRACTS.md`, `docs/ROADMAP.md`, `docs/SPOTIFY_PARITY_MATRIX.md`, `docs/SECURITY.md`, `docs/KNOWN_LIMITATIONS.md`, decisions, operations, release notes, browser harness, and relevant existing source.
3. Produce an accurate inventory with ACTUAL source paths: domain, migrations, auth, imports, scanner, audio, player, Dioxus roots, CSS, jobs, packaging, test harness. Preserve every verified working behavior.
4. Identify production/persistent data, ignored runtime snapshots, user credentials, media and uncommitted work. Before schema/storage changes, take a *non-destructive, permission-preserving backup*, restore it into an isolated throwaway location and verify hashes/SQL integrity. Never touch original media while testing.
5. Run realistic baseline gates without monopolizing CPU/memory. Follow the repository's existing concurrency restrictions (prior host guidance: at most two Cargo builds at once). Do not run `cargo clean` or delete `target/releases`, `target/runtime`, backups or `data`.
6. Continue from the current implementation. Reuse APIs and design contracts; do not replace a tested Rust/Dioxus implementation with a demo or a framework migration on intuition alone.
7. Create/refresh the exact parity and platform status tables described below. The lead agent signs off each accepted integration.

**First product slice unless current code disproves the backlog:** real consent-controlled listening events, privacy-safe history and a working recommendation/radio loop from *existing playable* tracks. In parallel, design and incrementally integrate cross-platform playback contracts and the first two-server, authorized, public-catalog federation journey.

---

## 2. AUTONOMY: SUBAGENT ORGANIZATION AND INTEGRATION CONTRACT

### 2.1 Organizational structure

Use real Codex subagent mechanisms, parallel worktrees, scoped context and tool-supported task delegation where available. The principal agent remains accountable and owns the shared integration branch and cross-cutting contracts. **Do not invent a nonexistent subagent API or recursively generate prompt-only agents.** Concurrency must be bounded by host capability and agent/tool limits. Prefer 3–6 useful independent simultaneous implementations over 20 agents fighting for one Cargo lock.

Assign named specialists, reusing one agent across roles when more efficient:

| Agent | Mission / code ownership | Mandatory evidence |
|---|---|---|
| 01 Principal Architect | Domain contracts, ADRs, dependency boundaries, integration order | Reviewed ADRs, contract tests |
| 02 Backend/API | Axum/Tokio streaming, commands, sessions, jobs | Real HTTP/API and recovery tests |
| 03 Database/Storage | Migrations, transactional consistency, indexes, SQLite/optional remote cache | Migration up/rollback strategy, backups, load tests |
| 04 Dioxus Design System | Components, layouts, motion, responsive UX, themes | Screenshot and interaction regression |
| 05 Dioxus Music UX | Home, search, artist, album, playlist, player, social screens | Complete in-browser workflows |
| 06 Media/Audio Engine | Audio source resolution, codecs, queue, gapless/crossfade, streaming | Audible stream/seek/metadata tests |
| 07 Android Native | Dioxus shell, Kotlin/Java bridges, AndroidX Media3 | Emulator/device playback + APK/AAB evidence |
| 08 Apple Platforms | iOS/macOS Dioxus/Swift/Objective-C bridges, AVFoundation | macOS runner/simulator/device evidence where available |
| 09 Windows/Linux Native | Installers, OS media session, MPRIS, integrations | Real OS smoke tests/packages |
| 10 DSP/Music Information Retrieval | Decode, STFT/FFT, tempo, timbre, chroma, feature extraction | Labeled fixtures, feature correctness and CPU tests |
| 11 Recommender Systems | Implicit feedback, ranking, candidate retrieval, explainability | Offline recommendation metrics and playable-output E2E |
| 12 Federation Protocol | ActivityPub/WebFinger, capabilities, identity, discovery, interoperability | Two/three real-instance integration and adversarial tests |
| 13 Federated Catalog/Search | Multi-server metadata index, filters, de-dup, federation availability | Cross-server search/partial failure tests |
| 14 Identity/Social | Profiles, follows, playlists, rooms, permissions, device handoff | Cross-user and real-time tests |
| 15 Security/Privacy/Abuse | Auth, SSRF, signed messages, rights, moderation, consent | Negative/adversarial authorization tests |
| 16 QA/Accessibility | Browser, desktop, device scenarios, a11y, crash/recovery | Reproducible test reports and defects |
| 17 DevOps/Distribution | Portable builds, Linux server, Compose, signed-release preparation, CI matrix | Install/upgrade/rollback/package verification |
| 18 Performance/Observability | p50/p95/p99, profiler, catalogs, cache, resource caps | Reproducible hardware/workload measurements |
| 19 Product Parity/Audit | Feature inventory, competitor behaviors, UX edges, user journeys | Updated parity matrix and independent walkthrough |
| 20 Interoperability/Metadata | OpenSubsonic, MusicBrainz, ListenBrainz, playlists, export | Compatibility, provider rate-limit tests |

### 2.2 Task assignment rules

Every agent brief must contain: goal, current verified context, exact allowed paths/modules, forbidden shared files, input/output contract, acceptance tests, privacy/security expectations, definition of done, evidence paths, explicit blockers, and integration handoff instructions. Send agents to *implement*, not merely suggest.

Main agent owns `Cargo.toml` root, `Cargo.lock`, schema ordering, shared DTOs/contracts, release and `docs/PROGRESS.md` unless it delegates these explicitly one at a time. Isolate simultaneous changes in worktrees or non-overlapping paths. Agree API/DTO wire contracts *before* parallel client/server implementation. Commit small verified slices, review diff, integrate, run cross-module tests, resolve conflicts deliberately. No blind merges, no simultaneous incompatible migrations, no fake task completion. Coordinate CPU-intensive builds rather than running many concurrent compiler jobs.

### 2.3 Continuous engineering loop

Repeat until the available execution session/resources end:

`REPO TRUTH → DEPENDENCY MAP → TASK GRAPH → DELEGATE → IMPLEMENT → FORMAT/COMPILE → TEST REAL FLOWS → SECURITY REVIEW → VISUAL QA → MEASURE → INTEGRATE → BACKUP/RELEASE CHECK → DOCUMENT → NEXT TASK`

Do **not** stop at each milestone for routine permission or celebrate a successful compile as a complete feature. Independently choose next high-value slice. On session end, persist a one-command resume recipe and exact failures/remaining acceptance criteria.

---

## 3. PRODUCT PRINCIPLES: MORE THAN A SELF-HOSTED ADMIN PANEL

Vela is a **consumer-first music app** and **decentralized music network**, with a lightweight self-hosted server as the optional infrastructure. Make a normal user's journey simple enough to understand without knowing what a VPS is.

Five concurrent pillars:
1. **Personal listening:** my music across all my devices with beautiful browsing, instant playback and offline access.
2. **Network discovery:** opt-in public servers and their lawful catalogs form a search, discovery and listening network; private libraries stay private.
3. **Recommendations:** highly personalized, explainable, diverse musical discovery using audio signal analysis + real implicit/explicit behavior + metadata + authorized federated data.
4. **Social listening:** artists, curators, profiles, playlists, friends, presence, parties, collaborative queues and sharing.
5. **Portability/control:** runs on modest VPS/NAS/ARM and popular consumer operating systems, with no mandatory central Vela service or GPU.

A visitor should install Vela, connect to their server OR explore permitted public catalogs (where an operator enables public browsing), choose songs, follow artists and playlists, and enjoy a cohesive music service. Offline-first means downloaded playable content still works even if every server disappears temporarily.

### 3.1 Public network vs private server

There are at least four independent toggles: server discoverability, catalog metadata visibility, stream access, download/republish permission. A public server can expose metadata yet require authentication for the audio; conversely an authorized public-domain music server can expose playable audio. **Do not conflate these.** Every playback result must be truly accessible to that listener or explicitly label it otherwise. The existence of a URL is not a license to mirror it.

### 3.2 Product polish is not optional

No demo-only cards; no empty marketing shell; no inconsistent buttons; no awkward browser-native dialogs presented as finished UX. Every surface must show polished loading, empty, partial, offline, permission-denied, degraded and recovery states.

---

## 4. CROSS-PLATFORM DELIVERY — ALL POPULAR CLIENTS

**Mandatory application targets (one coherent Dioxus product):**
- **Web:** Chrome/Chromium, Firefox, Safari, Edge, installable PWA where appropriate.
- **Linux desktop:** x86_64 and ARM64 where possible, Wayland + X11; package as AppImage, `.deb`, `.rpm`, Flatpak if justified/tested; support MPRIS media controls.
- **Windows desktop:** Windows 10/11 if supported by actual toolchain, x86_64 and practical ARM64 roadmap; native installer (`.msi`/`.exe` according to verified Dioxus support); Windows system media transport/media keys.
- **macOS desktop:** Apple Silicon + Intel where supported, `.app`/`.dmg`, Apple media keys/Now Playing, signing/notarization readiness.
- **Android:** supported active Android versions determined by documented compatibility floor; ARM64 priority, emulator compatibility; installable APK and Play Store-compatible AAB if build pipeline supports it.
- **iOS:** current supported iPhone iOS versions; simulator and device-ready `.app`/`.ipa` packaging with developer signing as an external dependency.
- **Tablet layouts:** Android tablets and iPads are first-class, not stretched phone UIs.
- **TV / car / watch / smart speakers:** extensible capability adapter roadmap; provide honest availability, integrate Android Auto/CarPlay/AirPlay/Chromecast or wearables only if APIs/entitlements/rights and verification support it. Do not silently add these to mandatory release gate if unsupported.

**Dioxus remains mandatory for the shared interface and Rust application logic.** Build an explicit `ClientCore` and a `PlatformAudio` trait/interface with stable async commands, event streams, capability negotiation and platform-specific implementations. Native hardware audio cannot depend on a foreground WebView JavaScript timer. The native player owns long-lived audio playback; Dioxus is a UI/controller observer and may reconnect without stopping sound.

### 4.1 Native audio engine contracts

Shared contract: load(stream or local secure file reference), play, pause, seek, queue replace/append/reorder, next/prev, volume, mute, repeat/shuffle, playback rate where supported, volume normalization, real buffering status, duration, position, track-end/error events, media/session state, device route/capabilities. Commands have correlation IDs, idempotency and cancellation semantics. Separate transport state, source lifetime, queue state and device state. Avoid state divergence between main/mini/fullscreen players. Audio source resolution is independent of backend server origin.

**Android:** Media3 `ExoPlayer` + `MediaSessionService` or justified `MediaLibraryService`, foreground media playback service permissions/notification, Bluetooth and headset keys, audio focus, noisy-audio/device unplug, lock screen, notification actions, Android Auto only if verified. Kotlin bridge via JNI or an equally justified boundary; Dioxus lifecycle must not kill the playback service. Persist secure downloads/permissions. Test screen-off, notification swipe, app background, phone call/interruptions, process recreation, data-saver and headset connect/disconnect.

**iOS:** AVFoundation (`AVPlayer` or `AVQueuePlayer`/custom engine where justified), `AVAudioSession` playback category, `UIBackgroundModes=audio`, MPNowPlayingInfoCenter, MPRemoteCommandCenter, interruption/route-change handlers, AirPlay capability where appropriate. Build a robust Swift/Objective-C bridge; do not assume a Dioxus WebView can maintain background audio. Keychain for secrets; secure cached/offline files; proper permissions and background behavior. Test lock, screen-off, headphone keys, interruptions, reconnect, force-close edge cases and offline mode. iOS builds require macOS/Xcode; use an available/macOS CI runner with explicit budget/credential approval if needed. Never fake a simulator/device run from Linux.

**Windows:** OS playback state/SMTC integration, media keyboard keys, notifications, tray as useful, source restart/hibernation and device changes.

**Linux:** MPRIS over D-Bus, portal APIs for dialogs/notifications as appropriate, Wayland/X11 compatibility, desktop audio backend selection based on verified reliability, minimized/background playback; DBus/control permission checks.

**macOS desktop:** Now Playing, remote commands, interruption handling, media keys, native file and notification UX; notarization only with user-provided credentials.

**Web:** HTML media/MSE or Web Audio where useful, Media Session API, browser codec detection, secure streaming with appropriate CORS/cookies/Range behavior, route/autoplay restrictions, visibility throttling and service-worker offline semantics. Never report native hardware behavior based on browser tests alone.

### 4.2 Build and verification matrix

Maintain `docs/PLATFORM_MATRIX.md` with columns: platform, architecture, minimum version, Dioxus compilation, artifact bundling, launch smoke, login/connect, real audio, background audio, lock-screen/media keys, offline, session reconnect, push/deep link, a11y, testing environment, exact log and status. Allowed status: `NOT_STARTED | COMPILES | RUNS | FUNCTIONAL_TESTED | HARDWARE_VERIFIED | RELEASE_READY | BLOCKED`.

A test of desktop on Linux is NOT verification of Windows/macOS. Android emulator is NOT hardware verification. A generated iOS skeleton is NOT an iOS application. If the current host cannot build another target, produce its real implementation and CI build jobs while labeling it unverified; do not ask the user whether you should continue with unrelated implementable parts.

### 4.3 Installer, onboarding and client configuration

App launches to a polished Connect / Explore experience. Enter a server URL, scan invitation QR, discover opted-in servers, authenticate securely, remember accounts, switch profiles/servers, and show meaningful trust/security information. Never hardcode the developer's VPS or require SSH/command line for routine use. Provide in-app server administration only to authorized administrators. Deep links must not expose durable secrets. Secure native key stores; no plaintext token files.

---

## 5. COMPLETE SPOTIFY-CLASS CONSUMER EXPERIENCE

Build a documented parity inventory including reference behavior, new original Vela UI, implementation, automated tests, supported platform flags, and actual rights/access prerequisites for EVERY item below. Avoid literal copyrighted assets or visual clones; reproduce polish and workflows with an original identity.

### 5.1 Navigation and application shell

- Desktop: left Home/Search/Your Library navigation, resizable library sidebar and folders, central scrolling music content, optional right Now Playing/lyrics/friend activity pane, persistent bottom player, back/forward history, breadcrumbs, keyboard navigation, command palette, context menus, drag/drop and meaningful app settings.
- Mobile: bottom tabs, persistent mini-player, immersive full-screen Now Playing, swipe sheets, native-safe-area behavior, reachability, responsive typography, no horizontal overflow on 320px, tablet two-pane modes, orientation changes, large-text handling.
- Support original Vela dark and light themes, OS scheme, high contrast, artwork-aware gradients, scalable icons, consistent typography/motion/tokens, skeletons, accessible focus, reduced motion, context-sensitive tooltips, animations no slower than useful.
- Accessibility: proper semantic labels, focus order, screen-reader announcements, keyboard-only workflows, color contrast, no color-only meaning, localization `en`/`ru` initially, bidi/RTL readiness, robust reduced-motion behavior.

### 5.2 Home (dynamic, real, personalized)

- Continue Listening; Jump Back In; Recently Played; Made for You; Daily Mix; Weekly Discovery; Forgotten Favorites; Favorite Artists; New Music from Followed Artists **when provider data is legitimately available**; Recently Added; New to Your Network; Popular in the Community; Based on This Track; Because You Listened to X; Genre/Mood shelves; Trending within opted-in public catalog; cross-server collections.
- Responsive carousels/grids, cards with real artwork and origin, one-click play/queue/save, dismiss/hide section where appropriate, refresh strategy and stale cache indicators.
- Every shelf is actually backed by catalog queries/recommendation output and *permission-filtered playable candidates*. A new empty account shows an honest helpful onboarding screen, not fabricated personalized recommendations.

### 5.3 Search and Explore

- Instant local and federated search, debounced typeahead, artist/album/track/playlist/genre/tag/curator/server filters, fuzzy/token/prefix strategy, recent searches with privacy controls, matching highlights, keyboard selection, history clearing, safe results.
- Filter availability/rights/quality/codec/license/origin/language/era/tempo/mood and whether it is offline or public. Identify origin precisely; multi-server timeouts yield useful partial results. One offline server never locks the entire UI.
- Browse genres/moods/decades/activities; public servers and curated catalogs; recommend newly discovered tracks; visible explicit and content safety filters.

### 5.4 Library, albums, artists, folders

- Liked Songs; Saved Albums; Followed Artists; Your Playlists; Playlist Folders; Smart Playlists; Downloaded; Local files; Shared with Me; Servers; Recently Played; Collections; Saved queues. Switch grid/list, pin, sort/filter, context menus, virtualization and pagination.
- Rich album pages: edition, multi-disc, credits, cover art, track numbers, total length, release/version labels, codec/quality, popularity only with real data, play/shuffle/like/share/download; proper Various Artists and multiple recordings.
- Rich artist pages: hero/cover, top songs, albums, singles/EPs, appears-on, related artists, biography if legitimately sourced, public releases, follow state, artist radio; mixed local/federated origins with source and rights indicators.
- Artwork cache, color extraction, progressive loading and correct thumbnails; never block audio on missing remote artwork.

### 5.5 Playlists and folders

- Create, edit, delete safely, rename, description, custom cover, folder nesting, pin, reorder, deduplicate on command, playlist/queue import/export, M3U/M3U8, collaborative editing, share links with permission expiry, real-time updates, revision conflict handling, ACLs and per-server playback access.
- Personalized auto-mixes, similarity/seed playlist generation, rule-based smart playlists, blend/matched-taste playlists where consented, folders, save-as-new, share QR, playlist follow and remote subscription.
- Support mixed-origin playlists, orphan/unavailable placeholder rows with recovery and alternative-source matching. Never silently substitute a different recording when the original differs (live/remaster/remix).

### 5.6 Player, queue, lyrics and sound quality

- Play/pause/seek; next/previous; shuffle with correct queue semantics and reproducible behavior; repeat none/all/one; dedicated queue view and drag reorder; up-next vs future ordered library; remove/re-add; jump-to-album/artist; click-to-like; copy link; original file details; volume and mute; sleep timer; session resume and playback continuity.
- Smooth progress with authoritative actual engine events, no phantom elapsed time; real buffering and errors; preserve pending seeks and source ownership; no stale user/session state after logout.
- Gapless where actually supported, optional crossfade (including no fade between continuous album tracks or incompatible cases), loudness normalization using ReplayGain/R128 where available, optional EQ, limiter/protect clipping, true quality information, quality preferences for Wi-Fi/cellular/offline; do not mislabel transcoded sources as lossless. Profile and validate DSP behavior per engine.
- Synchronized lyric highlighting via authorized LRC/embedded timed lyrics, scrolling and plain lyrics fallback; lyric translations only with rights and content; karaoke-like UI optional after real timing works.
- On-demand credits/source/provenance, recently played event detail, playback device selector, cast/remote capability labels.

### 5.7 Social and daily delight

- Real profiles (private/public), follows, friends, curated public playlists, collaborative playlists, follows across eligible servers, optional listening presence, feed/activity with privacy and controls, social search and discovery, block/mute/report.
- Shared-listening rooms analogous in *function* to group jams: invite via link/QR, host permissions, synced queue, democratic suggestions/votes, participant presence, bounded WebSocket messages, drift estimation and reconnect. Each participant must separately hold rights to the track; do not rebroadcast restricted media because one host has it.
- Device handoff/remote control: device registry, session ownership and authorization, cross-device continue, queue transfers, eventual progress sync and seamless handoff with races handled. Favor opt-in multi-device behavior, do not arbitrarily stop other devices.
- Personal statistics: hours, day/week/month/year, favorite artists, trends, new discoveries, genre distribution, privacy-preserving shareable listening-year story, delete/export data. Stats cannot be invented in sparse libraries.

### 5.8 Completeness review

Proactively compare real, **current** Spotify/Apple Music/Plexamp daily listening behaviors (where tools allow reliable public documentation), identify missing micro-interactions and edge cases, and implement original equivalents. Create a reproducible comparison matrix instead of saying 'everything like Spotify' without enumerating scope. No branded UI copying or fake catalog sizes.

---

## 6. AUDIO ENGINEERING, STREAMING AND OFFLINE

Implement independent server-side media handling and client-local native playback.

- Streaming HTTP: RFC-compliant Range (206/416, HEAD, Content-Range, ETag/Last-Modified validators, conditional requests where appropriate), no full-file loads, cancellations, bounded buffers, secure scoped access, DNS/redirect safety, replayable segmented streams if necessary, MIME detection, codec negotiation, explicit cache policy.
- Formats: actual supported MP3, AAC/M4A, FLAC, ALAC, Opus, Vorbis, WAV; evaluate DSD/hi-res and other formats only where codecs/OS/rights/packaging support them. Original lossless preferred if target supports it. Optional FFmpeg transcoding as controlled, sandboxed background work with bounded CPU/memory/disk/time; never a free arbitrary-command facility.
- Library: upload, batch import, drag/drop, folders, watched directories, removable storage, read-only scanner roots, WebDAV/S3 trusted connectors, authorized HTTP media URL imports, safe ZIP, optional archive/podcast and DRM-free legitimate catalogs, automatic metadata/artwork with provenance, duplicate detection (exact BLAKE3 vs recording identity), export without lock-in. Respect rights, provider limits, and user privacy; never upload private audio to a cloud model without opt-in.
- Background jobs: durable, idempotent, retry/backoff/cancel/pause where meaningful, progress visible through UI and event stream, restore/replay after crash, bounded queues and fair work scheduling; ignore malicious archives and oversized sources.
- Offline: download an actual permitted file to secure native app storage with bounded parallel downloads, resumable Range, manifests, BLAKE3 integrity, quotas, partial cleanup, pinned favorites, folder/playlist auto-download preferences, artwork/metadata caching, complete offline queue, functional airplane-mode playback. Track expiry/revocation policy accurately without deleting permanently owned media because of a transient outage. Enforce separate download permission from stream permission.
- Playback must remain functioning when UI tabs switch, app backgrounds where OS permits, server reconnects, screens lock, mobile network transitions, VPN changes, download finishes and another user's server becomes unavailable. Appropriate user-facing status instead of spinning forever.

---
## 7. PUBLIC DECENTRALIZED MUSIC NETWORK — FIRST-CLASS PRODUCT

This is an architectural differentiator, not an afterthought. Build a decentralized federation where independently administered Vela Music servers can voluntarily advertise **permitted public music catalogs**, receive limited queries, respond with verified metadata and authorized playable sources, and allow clients to discover/stream those songs in a unified interface. Private servers remain fully functional and isolated.

### 7.1 Target user journeys

**Flow F1 — Publish a server:** an administrator opts into discoverability in Settings → Network, chooses server name/description/logo/public URL/federation identity, selects specific libraries and music eligible for publication, sets rights and playback policies, previews exact disclosures, and explicitly publishes. Only explicitly allowed items appear in remote search.

**Flow F2 — Discover a public server:** a listener goes to Explore → Public Servers, sees independently hosted opt-in nodes with provenance, search or curated directory results and a clear capability/rights summary, then follows or connects. One click to browse artist/album/playlist/tracks and, where allowed, play.

**Flow F3 — Search the network:** query `ambient electronic` across local + opted-in public catalog sources, receive progressive results with origin/server, dedup/canonical recording grouping, playback availability, license, quality, source priority and network error transparency. `Play` resolves to an authorized stream from the chosen origin. No unauthenticated scrape of private servers.

**Flow F4 — A song on server A in a playlist on server B:** B stores only an origin-qualified reference and safe cached metadata; when user plays it, the client resolves permissions and streams from A (or from a licensed mirror), without demanding file replication to B. If A is offline, show unavailable / user-owned local copy / licensed mirror options, but no unauthorized rehost.

**Flow F5 — Cross-server recommendation:** a listener on A receives a recommendation for a track publicly accessible on B; the recommendation stores origin identity, public/license policy and score explanations; clicking succeeds end-to-end. If permissions disappear before playback, the item is revalidated and replaced or explicitly marked unavailable.

**Flow F6 — Privacy:** two servers are federated; user with private library or private listening history uploads a song and listens. No public metadata, audio bytes, history or identity become discoverable unless the relevant opt-in settings explicitly allow them.

**Flow F7 — Abuse and resilience:** an untrusted server sends forged messages, malformed objects, recursive links or unstable streams; Vela stays healthy, rejects unauthorized data, bounds resource use, can block/report the server, and continues showing all other sources.

### 7.2 Architecture: metadata federation vs authorized playback

Implement three **separate** protocol layers:

**Layer A — Identity/social federation.** Evaluate and implement interoperable ActivityPub/ActivityStreams + WebFinger-based actor discovery, signed server-server deliveries, profiles, follows, playlist/library metadata sharing and revocation messages where practical. Respect upstream standards and study Funkwhale's actual federation behavior and interoperability; do not call a custom JSON endpoint 'ActivityPub-compatible' until tested. NodeInfo and WebFinger are discovery metadata, not audio streaming authorization.

**Layer B — Vela music catalog capabilities.** Design a minimal, versioned, documented interoperable schema for public catalog query, artist/album/recording identities, licensing, availability, freshness, source refs, playback capability negotiation and change feeds, only where existing standards are insufficient. Prototype with at least two independent instances and provide a spec + example interop tests. Use HTTP(S) and common JSON formats; do not invent a heavyweight mandatory blockchain/P2P protocol. OpenSubsonic is an existing **client-to-server API**, not a federation replacement; implement as client/server adapter separately.

**Layer C — Media delivery.** Prefer direct client → authorized origin streaming over HTTPS with Range and codec negotiation. When direct connections are impossible, offer a strictly bounded, explicitly enabled, permission-preserving proxy, with SSRF/destination controls, bandwidth caps, rate limiting, no public open relay and correct audit. Secure short-lived scoped capabilities/OAuth-style delegated access, no durable tokens embedded in share URLs or analytics, TLS, identity/replay protection and expiration. Mirroring/caching of remote audio requires an affirmative license and explicit operator policy; metadata cache is NOT an ownership claim.

Define the clear three-way state for each remote track:
- `metadata_visible` (may view listing)
- `stream_authorized` (may receive audio for this principal/device/session)
- `download_or_republish_authorized` (may persist, distribute or mirror under actual rights)

Capabilities may depend on principal and origin, may expire, must be rechecked at use, and must be revocable. Avoid a blanket `is_public` flag pretending to cover these distinct cases.

### 7.3 Identifiers and catalog normalization

Create a robust globally unique reference with at minimum: origin server canonical identity + remote object stable ID + version/last-seen metadata + optional MusicBrainz recording/release IDs + preferred audio asset/edition identifiers. Represent artists, recordings, tracks, releases, masters/remasters, live versions, covers, physical encodings and provenance separately. Use BLAKE3 for exact files; acoustic fingerprints (Chromaprint/AcoustID when valid and optional) for potential matching. Same title/artist does not prove equal recording. Keep distinct origin IDs even if public metadata identifies the same logical recording.

Do not share internal filesystem paths or storage credentials. Version remote objects and retain tombstones/removal state. Cache remote metadata with bounded TTL, source attribution, stale state and controlled refresh. Track upstream connection failures and never turn an absent or unplayable remote object into an owned local track through a catalog sync.

### 7.4 Network topology, scale and discoverability

Design no mandatory central Vela-controlled service. Support:
1. Manual URL add / invitation / QR code / federated account lookup.
2. User-followed server lists and one-hop discovery through explicit opt-in links.
3. Independently operated optional public server directories with signed or validated announcements, privacy-preserving verification and delisting.
4. Optional local-network mDNS/DNS-SD for trusted LAN cases; never disclose private LAN addresses into a public directory.
5. Federated public collection discovery via ActivityPub mechanisms when practical.
6. Public catalog change feeds, bounded incremental indexing, discoverability expiration and source-signed metadata or secured transport.

A single directory outage must not disable personal playback, existing trusted connections or local search. Avoid global full crawls. Do not require each node to maintain a full copy of every other catalog. Prefer bounded distributed query fan-out to trusted/opted-in nodes + optional selectively replicated **metadata only** indexes and sharded directory caches when scale justifies them. Consider DHT/P2P content routing only after realistic measurement and rights/security review; no mandatory BitTorrent or public redistribution network.

### 7.5 Federated query planner and stream resolver

Design separate abstractions for `CatalogAdapter`, `OriginResolver`, `RightsEvaluator`, `SearchProvider`, `StreamResolver`, `FederationIdentity`, `RemoteEventProcessor`. Only create crates when actual cohesive code exists; reuse existing modules by default.

The global search planner must:
- Query local data immediately, remote providers concurrently with configurable per-peer deadline and fan-out ceiling.
- Yield progressive/partial results without UI blocking; suppress duplicate network requests.
- Rank across origins with source trust/availability/licensing/quality, not simply first result wins.
- Support total failure of one peer, disconnects, 429s, 5xx, redirects and changing DNS/IPs.
- Bound rows, response bytes, hops, pagination and background reindex CPU/disk/network usage.
- Distinguish aggregate counts (estimates) from verified accessible tracks; never invent a global total.
- Preserve ordered query result origin, license, access, matching recording, alternate encodings and freshness.
- Track removal/revocation, trust/block decisions, and rate policies.

The stream resolver must choose between local owned asset, offline permitted cache, origin stream, alternative licensed origins and explicit unavailable. It must never silently play a track merely because of equal artist and title. Apply capability/authorization checks immediately before requesting audio, not just at search time.

### 7.6 Moderation, governance, rights and safe public service

The network must not become uncontrolled copyright infringement infrastructure. Require server-side per-item publication opt-in and a declaration of rights/permissions, with support for public domain, Creative Commons variants/attribution, rightsholder/artist-authorized content and other operator-verified licenses. Store license provenance and limitations (including ND/NC terms when applicable), region limits if legally relevant, and whether streaming/downloads/federated rehosting are permitted. Licenses are evidence and policies, not a magic solution to legal uncertainty. Provide notices, takedown request intake, admin delisting, user reporting, origin attribution, moderation/blocklists, and ability to block federation. Default to private on upgrades and on newly imported copyrighted music with unclear rights.

Security: signed federation events and secure key rotation; prevent replay/forgery, ID substitution, actor spoofing, SSRF via remote metadata and artwork/audio, redirect laundering, private-network probing, invalid DNS rebinding, HTML/JS injection, oversized JSON/ZIP bombs, request floods, hostile Unicode names, recursive graph expansion, poisoning of recommendations/trending charts, open-proxy abuse, timeouts, memory/cache exhaustion and user-list enumeration. Include automated adversarial tests. Do not let a foreign server request protected local admin or library endpoints via reverse proxy.

### 7.7 Interoperability proof

Mandatory real interoperability demonstrations:
- Two independent Vela servers: independently started, uniquely identified, authenticated/configured, distinct private libraries; publicly share one authorized library subset; discover and search both in one client; stream at least one authorized track from each; deny private media.
- Third server malicious/slow/offline: partial search continues; block/revoke, stale metadata and token expiration handled.
- One legitimate shared playlist with local and remote tracks; revision changes and deleted origin track behavior.
- Test compatibility with a real OpenSubsonic client/server where implemented; ActivityPub handshake with Funkwhale/other compatible Fediverse peer only if genuinely tested against current behavior. Explicitly list unsupported remote features.

---

## 8. MUSIC INTELLIGENCE — SIGNAL PROCESSING AND AUDIO CONTENT UNDERSTANDING

**Do not confuse Fourier transform with a genre classifier.** FFT is a mathematical building block; it shows frequency content over a window. A single FFT spectrum generally cannot reliably infer a musical genre, mood, instrumentation or listener taste. Build a multi-resolution **DSP/Music Information Retrieval** pipeline based on decoded PCM, temporal features, optional pretrained models and honest confidence scores. Pair it with real listening behavior; signal characteristics alone cannot produce Spotify-class personalization.

### 8.1 Analyze when media enters the authorized catalog

Implement versioned, durable offline analysis tasks for owned/authorized audio and for remote content only if analysis rights/transfer are permitted. Server owns resource limits and privacy settings. Never block primary streaming on ML/DSP computation. For public federated tracks, accept optional publisher-provided metadata/features only when schema, authenticity and provenance are verified; do not download inaccessible tracks merely to analyze them.

Proposed pipeline:

`validated media → decode limited PCM with Symphonia/FFmpeg fallback → channel conversion/optional resampling → peak/RMS/loudness analysis → framed STFT → spectral/rhythm/tonal descriptors → optional audio embeddings/classifiers → quality/confidence checks → versioned feature store → similarity index → recommendation candidate feed`

Each stage must have a bounded memory footprint, failure/retry policy, structured metrics, plugin-style feature versioning, and a way to skip expensive stages in minimal VPS profile. Normalize sample rate for comparable extraction, *not* to mutate original music files. Decode via existing vetted libraries. Research dependency licenses and actual maintained versions before selecting crates.

### 8.2 DSP algorithms to implement correctly

**Fundamental signal processing**
- Frame PCM with Hann windows; sensible configurable FFT/window/hop size (e.g. around 20–50 ms temporal resolution), channel policy and normalized spectra.
- Use a verified high-performance `rustfft` or equivalent; real-time audio callback MUST NOT allocate or perform heavy analysis. Offline feature worker may process long clips in bounded chunks.
- Compute STFT magnitude and power, mel spectrogram/filterbanks and log-mel features; use appropriate sample-rate-dependent filters and stable numerical floor.
- Spectral centroid (brightness), bandwidth, rolloff, flatness, contrast, flux, zero-crossing rate, RMS, peak, dynamic range, approximate loudness/LUFS where applicable, silence segmentation.
- MFCCs from mel energies (DCT) for timbral representation; normalize across recordings and avoid overinterpreting one vector.
- Harmonic/percussive separation where justified and feasible; onset strength envelope, autocorrelation/tempogram, BPM hypotheses and beat position; track tempo confidence and distinguish half-/double-time ambiguity.
- Chroma/HPCP-like pitch-class features, key/tonality estimation with confidence and correct handling of atonal/noisy material; optional chord progression fingerprinting.
- Optional descriptors for groove, energy, acoustic/electronic qualities, instrumentalness/speech presence, vocalness, musical structure and section transitions only when validated, not hard-coded labels.
- Optional content-based acoustic fingerprint for duplicate/recording match, separate from semantic embedding.

**Audio embeddings and classifications**
- First ship an interpretable DSP feature vector and similarity search, then evaluate well-supported audio/music embedding models (e.g. appropriately licensed music-aware encoders, ONNX models or Rust-supported inference runtimes) against a real legal evaluation set.
- A genre is probabilistic and multi-label; use hierarchical tags and confidence, never force every track into one inaccurate genre. Respect embedded metadata, human edits, publisher tags and source quality; do not silently overwrite authoritative tags with model guesses.
- Mood is subjective and contextual; allow multiple overlapping signals such as energetic/calm, bright/dark, danceable/ambient with confidence and user feedback, not categorical psychological claims.
- Record model name, version, license, embedding dimension, normalization, feature extractor hash, analyzed section duration, compute cost and confidence. Avoid mixing embedding versions in one similarity index without controlled migrations.
- For a cheap VPS, CPU-only optional processing, partial/sample-first analysis, configurable background queue and batch modes; GPU only as optional acceleration, never necessary for playback/search.

**Technical correctness & data integrity**
- Avoid interpreting codec artifacts or file loudness as musical preference. Loudness normalization must not leak into embeddings unpredictably. Respect silence/intros, variable sample rate, multi-channel sources, clipping, variable duration, silent tracks and corrupt files.
- Do not infer a track's license from signal analysis. Do not fetch or decode private remote audio without permission.
- Cache analysis by stable source content hash + extractor version; invalidation on reanalysis, metadata override and algorithm version changes.

### 8.3 Research alternatives and Rust engineering

Investigate/reuse `Symphonia` for decoding, `rustfft` for spectral transforms, `rubato` for resampling, vetted loudness libraries, and reputable tools/implementations of chroma, tempo and fingerprints. Where a correct, mature Rust implementation does not exist, prefer a well-isolated, bounded, licensed external process/library over prematurely inventing a huge ML stack. CPU workloads use `spawn_blocking` or a dedicated bounded worker and never starve Tokio stream handlers.

### 8.4 Persisted feature schema (conceptual)

Represent at least:
- `media_asset_id` (internal authorized asset), `recording_identity`, origin and rights to analyze.
- `audio_feature_version`, extractor/config/model/license, content hash, analysis timestamp/status/trace.
- Normalized numeric vector: tempo candidates/confidence, key/mode/confidence, energy, loudness/dynamics, spectral/timbral/chroma statistics, onset/rhythm, acoustic/speech/vocal signals when supported.
- Optional semantic embedding vector with model/version and explicit dimension.
- Cluster/nearest-neighbor index state with rebuild version, validity and provenance.
- Null/unknown rather than fabricated values for missing descriptors; prevent invalid NaN/Infinity from corrupting DB and ranking.

Design migrations with stable typed tables and indexes, not a single giant JSON column for frequently queried features. SQLite remains a good default; use a small efficient approximate-nearest-neighbor index ONLY after benchmarks justify it. Evaluate extension portability, updates and memory; a bounded in-memory index or brute-force similarity is fine for modest libraries. Remote servers can expose opt-in safe *derived features* without raw audio or user data, subject to schema and abuse controls.

### 8.5 DSP validation gates

Build a curated deterministic *redistributable or synthetic, legally licensed* fixture set: silence, sine/sweeps, clicks/metronome, percussion, stable beat loops, chord/pitch tones, clean speech, noise, real royalty-free multi-genre tracks, clipping, variable duration, corrupt input and multiple encodings of the same recording. For synthetic signals, frequency, RMS, onset/tempo and key-like inference where meaningful must meet documented tolerances. For real tracks, measure confidence/correlation against validated annotations, account for half-time BPM and ambiguous genres; no invented 95%-accuracy claims. Measure analysis ms per minute of audio, memory, queue effect on playback p99 latency and feature-index size. DSP agent cannot mark a classifier complete simply because it outputs a label.

---

## 9. REAL PERSONALIZED RECOMMENDATIONS — HYBRID, EXPLAINABLE, FEDERATED

This is another flagship feature. **Use a genuinely multi-stage recommendation system.** Audio content analysis creates similarity signals; listening behavior and explicit taste produce personalization; federation expands eligible content. No fabricated songs, no guessing the user's preferences before data exists, no unauthorized public listening surveillance.

### 9.1 Private, correct event foundation (implement FIRST)

Design transparent per-user opt-in/opt-out listening collection, minimal event data, consent scope, export, deletion and retention policy. Capture:
- Stable anonymizable internal user/session/device, origin-qualified recording/asset identity, timestamp/event ID and playback position interval.
- Meaningful *audible* played time, not just a UI play button or file request. Avoid repeated duplicate events from seeks, loops, retries, multiple player views, and offline sync.
- Starts, qualifying listens, completions, intentional skips (with time threshold), repeats, saves/likes/dislikes/hide, manual playlist add, search/listen context, artist following, user-selected listening context when explicitly supplied.
- Server-authoritative idempotency keys, monotonic per-session progress/state checks, batching and offline replay/dedup; bounded retention. No raw device tracking or cross-server listening history publication by default.
- Global disable/delete/export and per-device consent; clearly explain that opting out may make personalization less accurate. Do not silently change consent during upgrades.

Before implementing sophisticated ranking, make real private history and stats with backward-compatible migrations and adversarial cross-account tests.

### 9.2 Candidate retrieval — several independent sources

Produce accessible, de-duplicated candidates from:
1. Explicit likes, saves, followed artists, favorite albums and repeat listens.
2. Recent listening co-occurrence and session transitions on the user's own library.
3. Content-based nearest neighbors in DSP/audio embedding space, genre/mood/tempo/key/timbre clusters.
4. Matching metadata tags and artist relationships from authorized MusicBrainz/other legitimate sources.
5. Collaborative filtering of opted-in, aggregated interactions when enough independent data exists, with malicious-peer-resistant controls.
6. Trusted friends'/curators' public/shared playlists and published thematic collections (subject to privacy and rights).
7. Time-sensitive local discoveries: newly added albums, unexplored artists, genuinely new releases if provider data exists.
8. The public federated network: query compatible nodes/indexes for accessible related tracks; preserve origin and permission state.
9. Novelty and serendipity: diverse lesser-known tracks actually available to the listener; protect against popularity-only loops.
10. Editorial or user-curated seeds when sparsity/cold start prevents data-driven recommendations.

All candidate generators expose a common versioned source descriptor, latency/resource limits, quality/contribution counts, rights gates and failure handling. A remote candidate is not eligible simply because a title was returned: verify actual media availability and user streaming grant at use time.

### 9.3 Ranking and re-ranking

Initial interpretable baseline may combine normalized signals such as:

`score(u,t,context) = w_behavior * affinity + w_audio * acoustic_similarity + w_collab * collaborative_signal + w_metadata * semantic_match + w_fresh * freshness + w_social * authorized_social + w_novel * discovery + w_quality * source_reliability - w_repeat * repetition_penalty - w_skip * negative_feedback - w_unavailable * inaccessible`

This is illustrative, not a magic universal formula; weights must be configurable, calibrated by dataset size, and evaluated. No score can override denied rights, hidden/private content or blocklists; those are hard eligibility filters **before** ranking. Prevent stale tokens, inaccessible regions and offline-only states from leaking into recommendations.

Implement diversity re-ranking (artist/album/genre distribution and avoidance of adjacent identical songs), novelty/coverage, repeat control, long-term-vs-short-term interests, context (activity/energy/time *only when genuinely chosen or permissioned*), cold-start exploration, and configurable explicit-content filter. Consider similarity search and learning-to-rank/implicit matrix factorization/nearest-neighbor models only when data volume and offline results justify complexity.

Output a short human-understandable reason such as 'Similar rhythm and timbre to X', 'You listened to artist Y', 'Popular in public collections you follow' with accurate evidence. Do not generate fake explanations. Provide 'Not interested', 'Don't recommend this track/artist', 'Why this song?', and an option to prefer local-only recommendations.

### 9.4 Taste models and cold start

Maintain private, revocable profiles for preferred artists/genres/moods/tempo/loudness/era/regions/local/remote choices and recent session dynamics. Start with user-selected onboarding tastes and honest music-library distribution. An empty history should yield 'Explore public catalogs' or 'Import your first tracks', not pretend to know the user. Users with 20 songs deserve a useful deterministic mix based on real owned tracks; do not demand a 1,000-person collaborative dataset. With few listeners, prioritize content signals and editorial/followed sources, then graduate to collaborative models.

### 9.5 Daily mixes, radio and personalized programming

Provide real features with original Vela titles and accessible songs:
- **For You** personalized Home shelf, rotating according to history and context.
- **Daily Mixes:** stable thematic favorite+adjacent sets; reasonable refresh, track availability revalidation.
- **Discovery Weekly-style:** true weekly discovery set with controlled novelty and genre coverage.
- **Release Radar-style:** followed artists' *actually known and accessible* recent releases.
- **Track Radio / Artist Radio / Album Radio:** infinite or pageable queue seeded from content analysis, co-occurrence and metadata, with no immediate repeats and permission checking.
- **Mood / Activity Mixes:** validated feature bands and/or reliable tags (calm, focus, workout, commute, sleep), with user override and uncertainty treatment.
- **Blend:** combine consensual profiles between invited users without exposing raw private history and ensure each participant can play tracks.
- **Smart DJ / smart queue:** constrained sequence design: harmonic/tempo continuity where helpful, user taste, diversity, explicit feedback, actual availability, and optional spoken transitions only with lawful opt-in generation. Spoken 'AI DJ' is optional—not the primary recommender.
- **Forgotten Favorites**, **Deep Cuts**, **Fresh Finds**, **Recently Played**, **Album Discovery**, **Network Gems**, **Local Gems**, **New Server Finds**.

Use durable time-scoped snapshots where required, deterministic seeds for testability, controlled refresh/eligibility and playlist archival. Provide real human-readable results for small libraries, not infinite fake albums.

### 9.6 Network-aware recommendations and privacy

Never transmit a private user's per-track history to random federated peers. Federated servers may share explicitly published track descriptors, genres/audio embeddings and optional aggregate opt-in popularity/trends with a minimum privacy threshold. The *user's ranking* happens primarily on their own selected client/server over allowed metadata. If collaborative signals are sourced across servers, evaluate practical privacy-preserving aggregation; do not promise differential privacy/federated learning unless implemented and tested. Prevent Sybil spam/inorganic playback from poisoning public trends.

A network recommendation must include globally unique origin identity, provenance, stream policy, trust score, last known availability and source selection. Limit fan-out; use cached candidate lists with expiry. Remote DSP extraction is only for explicitly licensed/authorized media. Users can disable remote discovery entirely and retain fully functional local music playback.

### 9.7 Evaluation and experimentation

Create `docs/RECOMMENDATIONS.md`, `docs/AUDIO_ANALYSIS.md`, feature/schema specs, a reproducible offline evaluator and sanity checks. Measure at least precision/recall@K, NDCG@K or comparable ranking relevance on properly split interaction data, artist/genre catalog coverage, diversity, novelty, popularity bias, repetition, cold-start success, inaccessible-result rate, p95 generation latency, CPU/memory and model/index cost. Separate synthetic/deterministic tests from real measured listening relevance. Use anonymized opt-in or legal public datasets, no hidden evaluation leakage. Prefer A/B or interleaving only with meaningful opt-in traffic and explicit experimental safety; never claim quality equivalent to Spotify without a defensible user study.

Feedback loop: user listens/skips/saves → deduplicated opt-in event → incremental per-user taste model → periodic candidate/index refresh → rights-filtered ranking → visible recommended track → user feedback. This complete loop must work before the recommendation milestone is marked COMPLETE.

---
## 10. RUST/DIOXUS ARCHITECTURE AND DEPENDENCY POLICY

### 10.1 Keep a modular monolith by default

Retain current working Rust modular monolith unless measurements prove a different shape essential. Prefer one small server binary, SQLite WAL/FTS5, managed music folders, user-selected scanner roots, stable Axum API and Dioxus clients. Redis, Kafka, Kubernetes, mandatory Elasticsearch, a hosted vector DB, central inference API or GPU must NOT be required for a basic user-owned instance. Split workers/processes only for actual isolation, operational reliability or measured bottlenecks, without forcing complexity on a two-user NAS setup.

Proposed *conceptual bounded contexts* (not a command to create empty crates):

`Domain / Accounts / Authorization / Catalog / Media / Ingest / Player Session / Jobs / Listening Events / Audio Features / Recommendation / Federation Identity / Remote Catalog / Public Directory / Social / Client Core / Native Integrations / Dioxus UI / Observability / Operations`.

Rules: DTOs are explicit and versioned; domain rules are not coupled to Axum/Dioxus; business logic is not in browser event handlers; SQL ownership is constrained and parameterized; read models cannot bypass ACLs; background workers have bounded queues; native systems are isolated by capability traits. Do not duplicate large Rust logic between web/desktop/mobile; choose target-specific adapters where necessary.

### 10.2 Technology investigation (verify current stable releases)

Assess and reuse rather than invent:
- Stable Rust, edition 2024, Cargo workspaces with `Cargo.lock` committed; Tokio, Axum, Tower, Serde, tracing, rustls-based TLS.
- Dioxus verified stable release for Web/Desktop/Android/iOS, correct platform feature flags and API changes; research hot reloading, packaging and bundled asset paths.
- SQLite + SQLx migration tooling, FTS5; WAL/connection pool/backups; optional vector/ANN choice based on measured portability and scale.
- Lofty and Symphonia (metadata/decode), controlled FFmpeg for transcode/import conversions, BLAKE3 hashing, `rustfft` FFT, `rubato` resampling where needed, vetted perceptual fingerprinting and EBU R128 measurement.
- Client-side HTTP API, native secure credential stores, WebSockets for active interaction and SSE for one-way job progress. WebSockets are not automatically the right channel for every history/recommendation event.
- Optional MusicBrainz, Cover Art Archive, AcoustID, ListenBrainz providers with documented privacy, rate limits, retries, provenance and user consent. Respect MusicBrainz request limits and User-Agent; do not bulk-hammer public APIs.
- ActivityPub/ActivityStreams, WebFinger, Funkwhale documented interoperability; signed HTTP server communication with secure key rotation and replay defense. OpenSubsonic as client/server API, not federation.
- Actual AndroidX Media3 service and Apple AVFoundation with native lifecycle integration, not fabricated pure-Rust replacements.
- `cargo audit` or maintained equivalents, license auditing, dependencies/SBOM, supply-chain review and fuzz testing for parsers.

Use maturity, maintenance, licensing, binary size, CPU, memory, cross-platform compilation, security and compatibility as decision axes. Lock known-good versions; avoid unmaintained crates and blind newest-version churn. In particular, inspect any planned library's behavior under WASM/Android/iOS targets before introducing it into shared client code.

### 10.3 Server architecture and storage

- Explicit immutable recording/media asset identities, ownership/library membership, playlists with stable revisioned ordering, per-user preferences, device sessions and opt-in listening history.
- Remote server and actor identities, signed public objects, ACL grants, rights and provenance, availability, server trust and catalog synchronization cursors.
- Analysis jobs/features/embeddings/index versions stored separately from original files; background recomputation must not trigger audio mutability or lose user catalog edits.
- An append/transactional outbox for durable federation events, idempotent inbox processing and replay protection; queue isolation/backoff/dead letter/diagnostic state.
- Clear control-plane vs audio data-plane: web API requests and catalog queries shouldn't starve stream readers; pool workers and use backpressure.
- Config profiles `minimal`, `balanced`, `performance`: bounded scanner, analyzer, import, transcode, remote query and cache budgets. Runtime profile switches should be safe and observable.
- A stable migration plan from current SQLite schema 7 as discovered; never assume schema version without checking the actual database and migrations. Test upgrades and compatibility from frozen releases. No reset migration.

### 10.4 API and typed clients

Define versioned endpoints/contracts for account/session/device, catalog search and library, media stream capability, playlist revisions, listening events, analysis status, recommendations, public directory, federated resolution, rights checks, room events and administration. Public documentation must distinguish client/API auth, server-server identity, anonymous permitted public browsing and stream capabilities. Strong typed payloads, useful error codes, ETags/cache semantics and pagination/cursors.

Keep streaming and media authorization separate from playlist metadata reads. Support refresh/expiry, authentication revocation and device authorization without exposing credentials in URLs, browser logs, referers, share links or federation metadata. Implement web CORS and Content Security Policy by real origin policies, not `*` everything. Native apps must receive only granted scopes.

---

## 11. AUTHENTICATION, PRIVACY, SECURITY AND TRUST MODEL

At every new capability: threat-model external actors, owners, users, malicious public servers, app clients and uploaded media. Test unauthorized access before calling a feature finished.

**Identity and sessions:** Argon2id, trusted login/session cookies where browser appropriate, CSRF for cookie-authenticated mutations, per-platform secure native tokens with rotation/revocation, device-scoped sessions, lockout/rate limit, account recovery with appropriate safety, optional 2FA/passkeys only once core is reliable, no default admin account or hardcoded password, privilege checks at repository query and stream level.

**Network federation:** strict per-user and per-item visibility; signed messages, key rotation, bounded clock skew and replay cache; authorize remote actor mutations; no forging origin identity through arbitrary headers; fail closed on unknown/malformed rights; per-peer budgets, quota, request throttles and blocklists; trust metadata and report abuse. Never auto-publish imported audio on enabling federation.

**Media ingestion:** safe file names, isolated temp staging, content-type and magic checks, archive traversal/zip bomb limits, symlink/hardlink/FIFO protection, bounded upload/download, controlled outbound egress, DNS rebinding/pinned connection checks and redirect validation, rejection of loopback/private/cloud metadata destinations. Trusted LAN connectors get separately administered policies; do not weaken all URL imports to support them.

**Metadata, rights and privacy:** private listening events, export/delete/retention/opt-out, cross-account isolation, explicit controls for social feed and profiling, remote AI calls disabled unless approved, data-minimal federation publication, auditable sharing grants and revocations, no secret export. Ensure public directory knows only what the server voluntarily announces.

**Native security:** secret storage in Keychain/Keystore/credential manager, safe file permissions, encrypted local DB or sensitive columns when justified, deep link validation, no arbitrary file reads, no abuse of OS background entitlements, app transport security and secure certificate handling. Do not disable TLS verification for testing and accidentally ship it.

**Operations:** non-root runtime, read-only application code where possible, volume/file permissions, SQLite backup and restore, graceful drain, health endpoints without secrets, bound logging, PII redaction and audit events, safe update/rollback. Security regressions are release-blocking.

### Privacy and rights policy test matrix

Create explicit tests for: anonymous vs member vs collaborator vs owner vs administrator; private, specific-user, local member, unlisted metadata-only, public metadata-only, public-stream-allowed, download-granted, federated-shared, revoked and expired. Verify each at catalog, search, artwork, stream, export, recommendation, history and playlist surfaces. Disabled federation must leak nothing, even through counts/suggestions/artist artwork or UI caches.

---

## 12. PERFORMANCE, EFFICIENCY AND RELIABILITY

Rust's language choice alone does not make this faster than Spotify. Measure. Set initial **engineering budgets as targets, not claims**, amend based on actual environments:

- Cold/warm startup, idle memory and CPU on low-cost Linux VPS and ARM64; benchmark 1/2/4 vCPU and constrained RAM where test resources permit.
- Library sizes: 1k, 10k, 100k and 1M **metadata entries** (clearly distinguish synthetic metadata from real media). Search p50/p95/p99, FTS relevance, UI virtualization and pagination memory.
- HTTP stream time-to-first-byte, audible startup where measurable, repeated Range seeks, slow client/aborted requests, transcoding load, concurrent media uploads, scanner and audio analysis tasks.
- Recommendation latency and catalog coverage across one local library, multiple federated servers, peer timeout and poor network links.
- Time for one minute of DSP analysis per codec; RAM/CPU queue length and effect of analyzer on user playback.
- Rendering 10k+ virtualized rows on mobile and desktop, scroll FPS, total bundle size, launch latency, image cache size.
- Background playback power/data consumption where actual devices exist, low-network bitrate choices, offline download recovery and interrupted sessions.
- Federation adversarial workload: 100+ slow/unreachable peers simulation, oversized objects, replay flood, revocation propagation, defensive throttling and successful service under partial partitions.

Use reproducible harnesses: `cargo criterion` or approved benches, HTTP load tools, flamegraph/perf/OS profiling, Playwright, device profilers. Publish target vs measured results with hardware, OS, dataset, release commit, sampling method and exact commands in `docs/PERFORMANCE.md`. Avoid optimizing negligible allocations while major blocking IO and N+1 SQL defects persist.

**Do not claim** 'millions of users', 'zero latency', 'no buffering', 'perfect gapless', 'Spotify-level ML', 'battery efficient' or 'better performance than Spotify' without relevant credible measurement. Gracefully degrade on lightweight servers by disabling optional recomputation/indexing rather than risking audio starvation.

---

## 13. TESTING AND INDEPENDENT ACCEPTANCE

Testing is not a separate final step; every vertical slice includes unit, contract, integration, E2E, security and accessibility checks appropriate to the affected surfaces. Prefer **real running Dioxus client against real Axum server with isolated SQLite/media** over mocks alone. Fixture media must be lawful and deterministic. Track actual executed commands/logs and known unverified cases.

### 13.1 Quality gates

- `cargo fmt --all -- --check`; strict `cargo clippy` on the real target/build features, including native and WASM; `cargo test --locked` with meaningful isolation.
- Actual Dioxus native target feature compile; reliable Web/WASM release build and browser automation on Chromium + Firefox + Safari/WebKit where available.
- SQL migrations from clean schema and representative prior persisted backups; restore tests; zero foreign-key errors; preserve hashes for existing media.
- Playwright browser journeys with appropriate screenshot assertions and dynamic state checks; viewport 320/375/768/1280/1920+, light/dark/large text/keyboard focus.
- API auth/ACL/SSRF regression, streaming Range and cancellation, real audio decoders, error/retry/restart/revocation.
- Native smoke tests and audio service tests via emulators/simulators + real device where physically available; capture non-verified items honestly.
- CI workflows for Linux, macOS, Windows, Android, iOS on legitimate available runners; packaging can require external signing entitlements and must remain marked pending until provided.
- Independent QA agent verifies observable behavior against acceptance scripts, not merely source code descriptions.

### 13.2 Mandatory golden scenarios

Do not mark the feature-parity mission delivered without, at a minimum, demonstrable evidence for the following scenarios. Expand them into isolated fixtures, automated tests and manual hardware checklists as appropriate.

**G01** Existing account and 20-track historical library (or whatever real state currently exists) survive a migration and rollback-ready upgrade with identical hashes/ownership.

**G02** From Web UI: upload real FLAC, extract correct metadata/artwork, find it in library, seek, save, playlist-add, play through proper streamed Range request.

**G03** Import a legally downloadable URL through UI without terminal. Show real jobs and progress, safe cancellation/retry/restart and metadata enrichment; reject SSRF attempts.

**G04** Dioxus Home loads *true* personalized and recent state; no fallback fake artist/album. Empty and offline paths are useful.

**G05** Browser queue, 11-second pre-load seek, mini/fullscreen player and Now Playing remain synchronized; repeated shuffle/repeat/favorites/session/logout race tests pass.

**G06** Two users create and edit a playlist concurrently; revisions are consistent; view-only grant does not give streaming; revocation stops new access.

**G07** One owner opts in to private listening history; exactly once meaningful listened-time tracking across seek/replay/reconnect; export and delete work; another account cannot read it; opt-out collects nothing.

**G08** Analyzer processes synthetic sine, silence, click track and licensed real songs; STFT/RMS/tempo descriptors agree with known fixtures within honest tolerances, model confidence/version is persisted, zero blocking of playback.

**G09** Track-radio by similarity generates a playable queue from real analyzed local tracks; changes when feedback changes; no repeats/unplayable IDs; records 'why'.

**G10** A new user with no listening events receives sensible opt-in onboarding/discovery; not fabricated personal taste. Existing user receives meaningful Daily Mix after real listening events.

**G11** Server A and Server B can opt in to federation independently, exchange identities/metadata and discover one authorized public collection with a functioning search and playable track; private tracks remain invisible.

**G12** Global search results show accurate origin/license/availability and progressive partial results when B is slow/offline; play resolves correct origin and permission.

**G13** Create a playlist referencing A and B, then disconnect B; show missing origin status and recover when it reconnects; no unauthorized copy to A.

**G14** A malicious peer attempts forged actors, replay, metadata JS, URL SSRF and huge objects; the application rejects abuse without breaking local audio.

**G15** A federated public recommendation actually comes from B, is accessible and streamed; after B revokes public access it disappears/gets marked unavailable, and no cache leaks audio.

**G16** Android APK opens, connects, streams real audio, continues while locked/app backgrounded, headset controls work, and can play a downloaded permitted track in airplane mode. Distinguish emulator vs hardware evidence.

**G17** iOS app on simulator/device opens, connects, streams, obeys AVAudioSession background mode and remote commands, resumes appropriately, and plays downloaded music offline. Provide actual macOS/Xcode execution evidence or mark BLOCKED, never claim success from Linux.

**G18** Windows/macOS/Linux desktop application packages install and launch on *each platform* in CI/physical environment, connects to server, plays real media, uses OS media controls and persists session correctly. Package build alone is not a launch test.

**G19** Queue/device handoff between phone and desktop resumes from the right track/position, no unsolicited control across accounts, and disconnected devices recover predictably.

**G20** Collaborative listening room handles joined/left participants, simultaneous queue edits, transport drift under simulated latency, revoked invitation and media grants.

**G21** 100k metadata items and multi-server query benchmark have reproducible measurements; audio playback stays usable during parallel scan, import and DSP jobs.

**G22** Licensed public content and metadata provenance correctly appear in the UI, while an item with uncertain rights remains private by default. A delisting/takedown invalidates public results and new stream rights.

**G23** Functional keyboard/screen reader accessibility, reduced motion, English/Russian localization, 320px mobile layout, tablet landscape and theme persistence.

**G24** Application updates and data backups can be restored without deleting user music; graceful shutdown, resume, and failure recovery are tested.

No green checkmarks for planned/emulated/manual-unrun tests. Each golden scenario includes code location, test command, last run commit, test result, screenshot/log where relevant and platform. The independent QA agent signs off cross-subsystem flows.

---

## 14. ROADMAP: PARALLELIZE INTELLIGENTLY, SHIP VERTICAL SLICES

**Do not attempt to write twenty empty crates at once or postpone the user experience until federation protocol research ends.** Track a DAG of dependencies and ship a continually usable product. Suggested integrated order; refine based on actual repository inventory:

### Gate 0 — Current truth and safety baseline
- Existing repo, tests, running service, persistent data and frozen runtime inventoried.
- Verified safe backup/restore in isolation; status and parity matrices updated.
- Baseline builds/tests rerun; report environmental blockers.
- Architecture and agent task contracts agreed. No destructive changes.

### Gate 1 — Private listening telemetry and history
- Opt-in audibly meaningful listening events with robust idempotency, race handling, offline sync, export/delete/retention and isolation.
- Real profile stats and recently played in Home; E2E and security gates.
- This unblocks personalization and must not be replaced by server-request counting.

### Gate 2 — Premium user experience, excellent playback
- Complete design tokens/components/shell across web, full queue/Now Playing, local search and home surfaces; native/media event abstraction established.
- Source resolver, correct seek/cancel/streaming behavior, gapless/ReplayGain exploration and optional quality settings.
- Run real visual/audio tests. Preserve 0.8.1 regressions.

### Gate 3 — Audio analysis and recommendation baseline
- Feature store/migrations, offline decode and STFT/tempo/timbre/chroma/energy pipeline with fixtures/resource caps.
- Rule-based + content similarity + user taste candidate generators; playable track radio and Daily Mix; reasons and negative feedback.
- Real evaluation harness; dashboard shows measured recommendation coverage/latency.

### Gate 4 — First two-server public network
- Server opt-in publishing rights/visibility preview, federated identity, signed secure discovery, remote metadata cache and user-facing Public Servers browser.
- Functional cross-server search and direct authorized streaming with hard permission checks, private content isolation and adversarial tests.
- No global index or P2P complexity until this complete end-to-end journey works.

### Gate 5 — Unified catalog and network-aware recommendations
- Origin-qualified IDs, remote capability resolver, cross-server playlists, source comparison, grouping and partial search responses.
- Recommendations backed by actual authorized remote track candidates, publisher public features, local ranking and explicit provenance.
- Revocation/offline/unknown-license test suite.

### Gate 6 — Android production-grade mobile
- Real Dioxus Android shell + Media3 session service, notification/headset/lockscreen, secure downloads and offline playback, session reconnect, deep links, Android APK distribution readiness.
- Run emulator and hardware QA when available. Preserve common UI components and shared contracts.

### Gate 7 — iOS and macOS native
- Real Dioxus Apple builds, AVFoundation/iOS background audio and OS media controls, secure downloads/Keychain, Apple-specific UI accessibility, iOS simulator/device tests via macOS environment and macOS installer.
- Do not certify absent runners/hardware. Obtain signing credentials only through explicit human authorization.

### Gate 8 — Windows and Linux polished desktop
- Desktop shell/packages, Windows system media transport, Linux MPRIS, hotkeys, drag/drop, file dialogs, tray, hardware device and session restore tests; CI OS matrix.
- Demonstrate actual installers, not a web link disguised as native app.

### Gate 9 — Advanced discovery and social
- Collaborative filtering when enough consented data; diversity/novelty and rank evaluation; weekly discovery/release recommendations; privacy-controlled follows/activity.
- Real-time collaborative rooms, playlists, device transfer and music communities; moderation and opt-in settings.

### Gate 10 — Interop, resilience and release operations
- OpenSubsonic compatibility, MusicBrainz/ListenBrainz adapters under provider terms, catalog export/import, optional podcast/radio, backup/restore/upgrade/rollback and admin UX.
- CI signing readiness, privacy/legal/attribution/third-party notices, secure docs, performance hardening, accessibility review and honest release-readiness assessment.

### Parallelization dependency examples
- Agent 10 (DSP) may build feature extraction on isolated legal fixtures while Agent 11 (recommendation) writes candidate/ranker contracts and Agent 14 (listening events) delivers event semantics. Integrate only when contracts and feature versioning match.
- Agent 12 (federation) and Agent 13 (search) work separately after signing/identity/object schemas are agreed; Agent 15 audits remote trust. No remote audio exposure until rights gates pass.
- Agents 07/08/09 can create native playback adapters concurrently *after* Agent 06 stabilizes `PlatformAudio` contracts and Agent 05 avoids browser-only state assumptions.
- Agents 04/05 build consumer UI while backend services advance, but UI cannot mark dynamic screens complete until wired to real endpoints and tests.

Maintain active milestone WIP limits. Prefer one integrated end-to-end success over ten experimental branches that do not compile.

---

## 15. DOCUMENTATION, PRODUCT OWNERSHIP AND VERIFIABLE PROGRESS

Create/maintain these files, adapting to existing names rather than duplicating near-identical documents:

- `AGENTS.md` — precise repo operating rules, scoped agent task contracts, build/tooling safety, data paths and resume instructions.
- `docs/PROGRESS.md` — exact version/commit, verified features, test evidence, active process and next task.
- `docs/SPOTIFY_PARITY_MATRIX.md` — consumer features/subfeatures, acceptance, status, evidence and platforms.
- `docs/PLATFORM_MATRIX.md` — Web/Android/iOS/Windows/macOS/Linux real support state and hardware checks.
- `docs/ARCHITECTURE.md` + `docs/CONTRACTS.md` — bound contexts, API, event and native playback contracts.
- `docs/FEDERATION_PROTOCOL.md` — public server policies, object schema, identifiers, discovery, rights/stream capabilities, trust/blocklists, version/interoperability examples.
- `docs/NETWORK_RIGHTS.md` — operator opt-in, publish rights, license provenance, takedown and revocation operations.
- `docs/AUDIO_ANALYSIS.md` — FFT/STFT feature specifications, normalization, model/version, tests, compute budgets and limitations.
- `docs/RECOMMENDATIONS.md` — event/consent semantics, candidate sources, rank/re-rank, calibration, offline evaluation and cold start.
- `docs/SECURITY.md` + threat model + negative permission tests.
- `docs/PERFORMANCE.md` — exact benchmark source commit, method, dataset, hardware and measured values.
- `docs/KNOWN_LIMITATIONS.md` — incomplete/unverified/unsupported targets, legal/provider and hardware constraints.
- `docs/ROADMAP.md` — ordered, dependency-aware queue of complete vertical slices.
- `docs/DECISIONS/` — architectural decision records with alternatives and operational consequences.
- `docs/RELEASE_NOTES.md` + `docs/OPERATIONS.md` — deployment, backup/restore, upgrade, data portability and rollback.
- `docs/QA_EVIDENCE.md` — golden tests table and screenshots/log references.

Every visible progress update uses one truth table:

`Feature | exact paths | latest verified commit | evidence run command | PASS/PARTIAL/FAILED/NOT_RUN/BLOCKED | next step | owner agent`

Do not copy earlier transcript claims as current tests. If a task is blocked because there is no Apple runner/device/account, document it, finish everything possible without it, and generate reproducible CI/manual test instructions. Do not stall unrelated development or fabricate completion.

---

## 16. DELIVERY ARTIFACTS — EXPECT REAL SOFTWARE

Deliver when supported by actual environment, with exact paths/hashes and evidence:

1. Updated, integrated Rust source and migrations in the existing repository; tests pass.
2. A running actual web/server build, backed by preserved user data or isolated test data as appropriate.
3. A truly useful Spotify-class Dioxus UI—not merely a mockup—with functional navigation and audio.
4. Linux server release binaries, Docker/Compose file(s) and operationally safe first-run wizard; run Docker tests only when daemon is really available.
5. Desktop bundles and installers for Linux/Windows/macOS, validated by platform-specific CI/hardware.
6. Android APK and where feasible signed-ready AAB, with a real native media service; simulator/device evidence.
7. iOS `.app`/`.ipa` generation/signing-ready pipeline with native AVFoundation playback; Mac build/simulator/device evidence as available.
8. Working two-server Vela public federation demo with controlled lawful content, rights policies and global authorized search.
9. A working DSP extraction pipeline and recommendation engine with real user events, playable music and documented quantitative evaluations.
10. Accurate release notes, migration plans, human-readable installation docs, SBOM/licenses and security/privacy safeguards.

**External releases/App Store submissions/public deployment require explicit authorization.** Working binaries may be generated locally without publishing.

---

## 17. FAILURE MODES YOU MUST PREVENT

Never deliver any of the following as 'complete':
- A new greenfield repo that discards tested Vela history.
- An attractive static Spotify-style interface with inoperative buttons and fake recommendations.
- Android and iOS 'apps' that are merely ordinary web pages and stop playing when locked.
- A fake distributed network that is simply a list of hard-coded server URLs or scrapes strangers' private catalog data.
- Unlicensed audio copying, public unrestricted rehosting, unauthorized Spotify downloading or DRM removal.
- A single-song FFT graph called 'genre detection' or a random shuffle called 'machine-learning recommendations'.
- A recommendation row with inaccessible tracks, invented popularity metrics or leaked private listening histories.
- An interoperability claim based solely on two internal function calls rather than independent server/client wire tests.
- Massive placeholder crates, migrations, uncompiled mobile code, stub APIs, or writeups that never reach working implementations.
- Unbounded Tokio tasks, CPU-heavy analysis on audio streams, secrets in logs, actor/URL SSRF, SQL injection, unsafe storage paths or missing ACLs.
- Closing the session with no checkpoint, or claiming the work continued after tools ended.
- 'Runs on all platforms' when only WASM/Linux has been built.
- Deleting/mutating user files, original data, existing snapshots or runtime to free disk space without approval.

---

## 18. EXACT FIRST ACTIONS — BEGIN IMPLEMENTATION NOW

**Your first response should be a short operational baseline and work assignment, not an enormous plan. Execute these actions directly:**

1. Inspect actual Git HEAD/status, scoped `AGENTS.md`, current Vela docs, workspace/crates/source, tests and running service; identify real persisted music and safely preserve it.
2. Confirm whether supplied historical Vela 0.8.1 and commit `f6cc845` exist or the repo has advanced. Never downgrade without a justified and non-destructive reason.
3. Reproduce existing baseline from source with bounded jobs. Open and inspect the actual app in a browser or native runner if tools support it.
4. Create a concise discrepancy inventory: **implemented / partial / absent**, especially listening events, recommendations, audio analysis, public federation, desktop/iOS/Android, and private data/rights constraints.
5. Establish one coherent `PlatformAudio` command/event interface and its integration boundaries, without breaking the verified browser player.
6. Start the **private listening events → history → real content-based track radio** vertical slice immediately. Concurrently assign independent specialists to (a) DSP feature extraction fixtures/index design, (b) federated server identity/rights schema and two-node proof, (c) Android and Apple native platform feasibility/build scaffolding, (d) polished Dioxus UI audit and useful consumer improvements, and (e) security/QA review. Adapt worktree count to capacity and real dependencies.
7. Each agent must write code and tests, not merely architecture text. Principal resolves contracts, reviews diffs and integrates passing slices. Protect frozen source packages and original active server assets from overwrites during builds.
8. Run real tests and user journeys; fix regressions; record measured and unverified outcomes; persist commits/checkpoints. Continue to the next critical user-visible slice without asking permission to continue.
9. At the end of execution, update `docs/PROGRESS.md`, `docs/ROADMAP.md`, `docs/PLATFORM_MATRIX.md`, `docs/SPOTIFY_PARITY_MATRIX.md`, `docs/KNOWN_LIMITATIONS.md` and a precise resume handoff. State only what is demonstrably true.

### Final order

**BUILD THE PRODUCT.** The requested outcome is one cohesive, usable, technically exceptional **open music universe**: users can enjoy music on all popular platforms, bring their own audio, follow and stream from opted-in **lawful public Vela servers**, receive relevant recommendations derived from real acoustic features and real tastes, and keep control of their identity and data. Use the largest *effective* multi-agent effort the environment supports, make decisions independently, protect existing implementation and user data, and deliver verified engineering progress—not a concept deck.

---

## APPENDIX A — RESEARCH BASELINE (REVALIDATE BEFORE CODING)

These are starting reference points rather than assumed permanent APIs/versions. Record the exact revision and applicable constraints you actually used:

- Dioxus 0.7 mobile/platform/deploy docs: https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/ and https://dioxuslabs.com/learn/0.7/essentials/fullstack/project_setup/
- Dioxus native packaging: https://dioxuslabs.com/learn/0.7/tutorial/bundle/
- Android Media3 background service: https://developer.android.com/media/media3/session/background-playback
- Apple audio session: https://developer.apple.com/documentation/avfaudio/avaudiosession and https://developer.apple.com/documentation/avfoundation/configuring-your-app-for-media-playback
- RustFFT: https://docs.rs/rustfft/latest/rustfft/
- Symphonia: https://docs.rs/symphonia/latest/symphonia/
- Rubato: https://github.com/HEnquist/rubato
- ActivityPub W3C recommendation: https://www.w3.org/TR/activitypub/
- Funkwhale federation: https://docs.funkwhale.audio/developer/federation/index.html
- OpenSubsonic API: https://opensubsonic.netlify.app/docs/endpoints/
- ListenBrainz collaborative filtering API: https://listenbrainz.readthedocs.io/en/latest/users/api/recommendation.html
- MusicBrainz API/rate-limit rules: https://musicbrainz.org/doc/MusicBrainz_API

Research and respect all provider/API policies, music licenses, data permissions and software licenses. Existing standards are tools for interoperability, not proof of access to copyrighted music.
