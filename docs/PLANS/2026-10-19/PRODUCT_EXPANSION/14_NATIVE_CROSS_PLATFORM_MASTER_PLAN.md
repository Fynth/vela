# Native Vela: cross-platform delivery, artwork, caching and performance mandate

English implementation-agent specification. Planning only,2026-10-19.
Read this with the package README, architecture, recipes,320-feature catalog and
QA plan. This file answers the additional request for a complete native desktop/
phone product, not just a web UI in a window. No native implementation is changed here.

## 1. Mission and immutable boundaries

Evolve the actual Vela repository into a polished native music client for Linux,
Windows, macOS, Android and iOS. Share Rust business logic and Dioxus presentation
where appropriate, while using real platform audio/background/system integrations.
Do not rewrite working Linux code or introduce React/Tauri/Flutter as a shortcut.

Start with actual usability: artwork, coherent layout/settings/scale, reliable queue,
authorized cache reuse, measured next preparation and offline cold start. Then native
platform breadth, device handoff and social discovery. Many features belong in this
program; no feature count justifies breaking basic playback or leaking private media.

Keep one audible output authority per device, explicit requested/actual transport,
stable queue occurrences, authorization per source, safe cancellation and privacy.
A hidden WebView audio tag is not an Android/iOS native service. A compile success
is not physical background playback, and a null sink is not a physical audio audit.

## 2. Current-source findings requiring action

Current source anchor6fa2346 plus preserved dirty launch/QA changes listed in
[repository audit](01_CURRENT_STATE_AND_GAPS.md). Inspect again before editing.
Those exact pre-existing source bytes were independently committed as7113fd5 during
the planning pass. NAT12 describes the initial observation; use the newer commit's
own evidence for its renderer result, not an invented rerun by this documentation task.

| Native issue | Evidence and current classification | Repair/acceptance |
| --- | --- | --- |
| NAT01 Cover artwork is not rendered in core native surfaces | SOURCE_OBSERVED: ui.rs Library and player use neutral-cover SVG placeholders; Track already has artwork_id | Implement ART01–ART14 below; this is not proof tags/art ingestion failed |
| NAT02 No full native Settings route/scale | SOURCE_OBSERVED: toolbar theme/sign-out only; no app scale service | EP01–EP03 and SCALE01–14 |
| NAT03 Offline namespace lacks canonical origin in its current root API | SOURCE_OBSERVED: offline.rs account-only data_root/load/prepare signatures; older AUD-06 remains a prerequisite | Non-destructive versioned origin-aware migration before multi-account/multi-origin offline |
| NAT04 Cold offline launch needs a complete local account path | Known open audit/progress boundary, not reproduced in this documentation pass | Restart with server unreachable and play a permitted saved track; no successful /api/me prerequisite |
| NAT05 Uncached native source prepares entire original before decode | SOURCE_OBSERVED client.rs prepare/audio lease; warm complete cache already exists | Measure stages; add validated progressive path only after integrity gate, keep fallback |
| NAT06 Reusable cache is not actual-next preparation | SOURCE_OBSERVED: no authoritative next-plan prefetch service | Queue peek/single-flight/bounded speculative worker with cancellation |
| NAT07 Native durable queue and stable duplicate provenance incomplete | Latest review names remaining scope | Stable occurrence/entry IDs, paused restore, concurrent edit tests |
| NAT08 Album/artist/playlist visuals remain limited | SOURCE_OBSERVED sparse native composition and neutral art | Real covers/details/actions and responsive screenshots, no fabricated metadata |
| NAT09 Native everyday upload/import still depends on web workflow | Current native empty-state directs users to web | Shared authorized native Add Music client flows |
| NAT10 Public remote MPRIS projection remains unsupported | Current contract intentionally reports unsupported remote control state | Capability-aware metadata/commands when actually implemented |
| NAT11 Large UI root increases regression risk | ui.rs combines many route/state/transport operations | Extract presentation/services incrementally behind tested contracts; no wholesale rewrite |
| NAT12 Renderer/refresh behavior is under active uncommitted work | main/QA files change DMABUF launch and add an FPS branch | Preserve work; benchmark actual backend/hardware, no guaranteed144Hz from a config flag |
| NAT13 Native accessibility/scale/physical output breadth unverified | Historical X11/Wayland subset and null-sink evidence only | Actual keyboard/AT/DPI/output route/device testing |
| NAT14 Cross-platform clients are not all implemented | PLATFORM_MATRIX distinguishes Linux from absent/blocked targets | Platform-specific build/run/background/offline/system-control evidence |
| NAT15 Dependency/license release gates have known dated failures | CURRENT_STATE/SECURITY records maintenance notices/notice gaps | Re-run/fix release gate; do not silently ignore failures |

Older AUD-01/02/03/04/07/08/11/12 fixes have subsequent source evidence. Reproduce
before reopening them; do not use an old audit as proof they are still unfixed.
Every suspected bug needs a current reproduction or a clearly labeled source-risk
finding, a minimal regression test, repair evidence and a documented residual scope.

## 3. Artwork: diagnose the actual pipeline first

Confirmed source path: Track/AlbumSummary/ArtistSummary can carry artwork_id; web Art
uses the authenticated `/api/artwork/{id}` route; server artwork handler authorizes
through MediaService using current user ID. Native Library/player currently render
neutral placeholders regardless of that available ID. Therefore implement the
native authorized loading/presentation path before rewriting tag extraction.

Diagnosis sequence:

1. Use an isolated account and lawful FLAC/MP3 with embedded cover. Verify current
   API DTO has a nonempty valid artwork_id and the authorized artwork route returns
   the expected image. Verify a foreign account gets denial/not-found.
2. For an affected real track, inspect only authorized metadata when consent exists;
   do not export a user's cover/music to an external image service.
3. If artwork_id is absent, classify missing embedded art versus extraction/grouping
   defect. Preserve the original file and test the ingest/group selection path.
4. If the route fails, isolate authentication/ownership/file publication/MIME/limits.
5. If bytes succeed but UI remains blank, inspect asset bridge/CSP/image decode/
   cache key/generation/layout; do not hide errors behind a permanent music icon.

No public art grant is invented. Current remote public-catalog artwork may be absent
by design. An artist without a permitted portrait should have a tasteful typographic
fallback, not borrow an unrelated album cover as a factual artist photograph.

## 4. Artwork service and renderer bridge

Add a cohesive native artwork service near current client/media helpers, with:

- Closed request type: canonical origin/account/session epoch, valid artwork ID,
  requested rendition and presentation lifetime. No arbitrary URL/path supplied by UI.
- Native HTTP fetch using in-memory authorized headers; bounded connect/read/total
  deadlines, response MIME/size verification and cancellation.
- Auth check before private cached artwork is disclosed in an online authorization
  context; offline art belongs to an explicitly saved local manifest/account path.
- Single-flight requests per authorized namespace/artifact/rendition. A recycled row
  or canceled page cannot publish another item's late image.
- Bounded compressed-byte cache and independently bounded decoded-image memory.
  Cache key includes origin/account and content/art revision plus rendition; never
  only a song title or bare UUID from multiple servers.
- CPU/image inspection off UI thread; cap dimensions/pixel count/decode allocation
  and animated frame handling. Reject active SVG/HTML from untrusted origins initially.
- Versioned derived thumbnails, e.g. small row, medium tile, large player; final
  pixel sizes selected from logical layout×DPR with a capped resolution ladder.

Preferred desktop bridge spike: a registered application asset handler exposes only
approved artwork handles to the renderer and fetches through the Rust service.
Dioxus DesktopService asset registration exists in the pinned documentation, but
validate the actual asset request/response API, CSP and asynchronous lifetime before
choosing it. A virtual handle is scoped to the current WebView/session epoch and
known artwork ID; it must not become a general authenticated HTTP/file proxy.

Alternative first slice: bounded sanitized thumbnail bytes delivered through the
existing trusted bridge into revocable object URLs. This may be simpler but carries
copy/base64/memory costs; measure before selecting it. Revoke URLs and release decoded
resources on replacement/logout/unmount. Never inject session cookies/tokens into
image URLs, localStorage, remote HTML or browser fetch headers.

On mobile use the same logical artwork contract with platform cache/storage adapters.
System media artwork is a bounded separately retained rendition with the platform's
required lifetime, not an unrestricted file URL or a huge decoded album original.

### Initial artwork API and resource contract

The current `/api/artwork/{id}` is an owner-authorized original-image GET, not a
documented rendition/validator API. Its handler also takes a permit from the shared
stream semaphore. Do not assume conditional validators/rendition query parameters
already exist or that loading hundreds of images cannot contend with audio. Inspect
Axum's implicit HEAD handling separately from a dedicated optimized validator path.

N01 minimum complete slice may fetch bounded originals through the existing route,
render actual covers and deduplicate active requests. Subsequent rendition/disk-cache
work must define a compatible private application asset-cache contract before
persisting responses. Preserve current endpoint cache semantics until intentionally
reviewed; do not bypass no-store just to add a disk-cache checkbox.

Proposed server extension, after independent review: authenticated bounded rendition
selection from a closed size ladder, immutable representation revision/digest, MIME,
length and authorization revalidation. Original versus derived-art identity is explicit.
Every rendition/conditional request reauthorizes; a strong validator alone is not a
grant. A changed artwork reference produces a new key, and unknown/foreign IDs reveal
no owner's files. Reuse existing metadata/artwork service instead of another server.

Adopt separate bounded artwork/thumbnail work admission so artwork storms cannot
exhaust all audio-stream capacity. Set limits from a recorded baseline. Suggested
starting test profiles, not measured capacity claims: one image transform worker,
at most4 network art requests,8MiB response cap, explicit decoded-pixel/memory budget,
128/256/512/1024 rendition ladder and no speculative full-original artwork download.
Validate high-resolution legitimate covers against server downsampling limits rather
than accidentally rejecting all large covers. Mobile low-memory profiles use smaller
decoded pools and release offscreen textures first. Publish exact selected defaults
and failure reasons only after ART resource/stress tests.

## 5. Artwork presentation and acceptance

Implement one shared presentation primitive with Loading/Ready/Missing/Denied/Failed
states and fixed aspect ratio. Use placeholders only for missing/unavailable images;
loading should not indefinitely impersonate Missing. Provide bounded manual Retry
for a real failure, deduplicate repeated errors and avoid toast spam per list row.

Integrate every relevant surface: Library/Favorites rows, queue entries, bottom
player, fullscreen player, album grid/detail, artist discography, playlist mosaics,
search, recommendations, offline library and system media metadata. Use authorized
album/playlist/artist art according to type, not one universal square icon.

| Test | Required result |
| --- | --- |
| ART01 | Real uploaded embedded cover appears in native Library and bottom player |
| ART02 | Same asset appears correctly in album/detail, queue, search and favorite views |
| ART03 | Missing cover produces stable original Vela fallback with no broken-image glyph |
| ART04 | Wrong account/private art denied; stale response after logout/origin switch never renders |
| ART05 | Invalid UUID/arbitrary path/redirect/excessive image rejected with bounded resources |
| ART06 | Oversized dimensions/decompression/animated payload cannot exhaust UI memory |
| ART07 | Duplicate visible references share one fetch/rendition instead of a request storm |
| ART08 | Scrolling/recycling rows never swaps covers between different item IDs |
| ART09 | Offline saved cover loads after cold startup with server stopped |
| ART10 | Revoked/changed art follows documented revalidation/version policy; no cross-origin UUID collision |
| ART11 | Crop/aspect ratio/light/dark/high-DPI/high-scale layouts are visually reviewed |
| ART12 | System media control cover/title refer to the same current occurrence and privacy setting |
| ART13 | Cover failure never blocks Play or creates another audio owner |
| ART14 | Decoded memory, fetch counts and cache budgets remain bounded during long scrolling |

## 6. Shared native architecture

Use a framework-free coordinator for queue intent/session transitions, typed catalog
adapters, device preference service, source/transfer service, local cache/download
manifests and platform playback interfaces. Dioxus renders projections and dispatches
commands. Keep the existing playback.rs occurrence logic until a tested migration
improves reuse. A new shared client crate is justified only when real mobile/desktop
consumers remove duplicated service logic; do not add empty platform scaffolds.

Separate requested item, authorized resolved source, active engine generation,
queue cursor, media clock and UI route. Every platform callback includes engine/
selection/session epochs. Ordered bounded command/event bridges reject duplicates,
late completions and foreign account commands. Polling a high-frequency clock must
not force every library row to rerender.

Native integration map:

| Platform | Playback/system boundary | Build/runtime evidence required |
| --- | --- | --- |
| Linux | Existing Rodio/CPAL output authority; MPRIS; Wayland/X11 Dioxus window; Secret Service | Real local output, route changes/media controls, normal tiling/HiDPI, package install/run |
| Windows | Evaluate current Rodio backend against actual codecs/seek/output; Windows media-session bridge and secure credentials | Windows runner compile plus actual install/play/control/output-switch validation |
| macOS | Evaluate native engine/OS media-session behavior; Keychain; package/privacy/security integration | Apple host compile/run, actual output/remote control and packaging/signing scope |
| Android | Media3 player/media session in service, foreground/background lifecycle, native download/data source adapter | SDK/NDK build, emulator and physical background/headset/offline tests separately |
| iOS | AVPlayer/AVQueuePlayer where appropriate, AVAudioSession, remote commands/Now Playing, protected offline files | Xcode/simulator/device, background/interruption/lock-screen/offline tests; entitlements explicit |

Do not force a single decoder/backend across OSes if it breaks native lifecycle.
Shared semantics do not require identical low-level engines. Do not replace Dioxus
presentation just because a bridge needs a small amount of Kotlin/Swift/native code.

## 7. Audio source, byte cache and pinned downloads

Current complete-file cache already reauthorizes via bounded HEAD, compares strong
validator/size and hashes from the opened descriptor used for playback. It has
account+origin isolation and atomic publication; preserve that behavior. Do not
reimplement it as an unauthenticated URL cache to make benchmarks look fast.

Audit offline namespace independently: migrate account-only paths/manifests to
canonical origin/server+account+asset identity before multi-server offline. Detect
ambiguous legacy entries, retain them privately, and require explicit origin binding
or server proof; never guess an origin and expose another server's bytes.

Distinguish:

1. Current-source verified lease/buffer, held while decoder reads.
2. Evictable complete/sparse playback cache with byte/entry budgets.
3. Speculative next-source staging governed by queue/network/power policy.
4. Explicit pinned downloads with durable metadata/art/lyrics and integrity state.

Only evict inactive unpinned cache entries. Global/device budgets account staging,
partials/orphans, thumbnail derivatives and concurrent reservations. User-visible
Clear cache excludes pins and active descriptors. Lowering a budget previews reclaim
or blocks new work; it does not silently remove a chosen offline album.

Implement shared transfer leases keyed by authorized representation. Play outranks
speculation; canceling a UI consumer does not kill another legitimate consumer's
source. Use bounded buffers and streaming file I/O. Atomic publication validates
exact bytes/hash before declaring a complete download. Resume uses strong validators,
Range/If-Range and safe restart on representation changes.

For progressive playback preserve the explicit integrity gate in architecture. Early
decode before final full-file hash changes the security contract. A chunk-verified
representation model or another explicitly reviewed model must exist before release.
Keep full-file fallback for unsupported codecs/seek layouts. Show actual stage and
quality; local server does not bypass authorization/hash/decode requirements.

## 8. Queue preloading and continuity

Add a root actual-next planner after stable queue/peek semantics. Candidate snapshot
contains occurrence/origin/asset ref, queue/source/policy generations and mode.
Use one speculative job initially, a bounded byte budget and conservative unknown/
metered-network behavior. Advance does not create an extra output/player.

Recompute on shuffle, repeat, queue move/remove/insert, source failure, selected
quality, network policy and account change. Explicit Play next outranks context.
Repeat-one normally needs no separate next transfer. Empty/exhausted queues do no
work. A stale prefetched track may stay as permitted evictable cache, but it must
never autoplay as if still next.

On Android evaluate Media3 preload/data-source APIs against Vela's actual-next model,
not visible index proximity alone. Media3 cache/download implementations must honor
Vela namespaces/validators/authorization and the pinned-versus-evictable distinction.
Avoid two competing Rust and Java caches independently downloading the same media.

On iOS evaluate prebuffering/scheduled items with one authoritative player. Use
documented resource-loading or a reviewed authorized source path; do not depend on
undocumented HTTP-header injection as the sole authentication strategy. Bridge
callbacks have bounded deadlines and generation guards; cold offline uses actual
permitted local media without server contact. Hardware buffer boundaries need real
measurements before gapless/crossfade claims.

## 9. Visual completion of the native client

Retain the original Vela palette/icons and improve actual screen composition. Linux
frameless controls stay integrated into existing toolbar; preserve other OS window
conventions. Add App size, text/density, safe areas, keyboard navigation and readable
track rows. Avoid oversized blank banners and uniform boxes around every control.

Native desktop must include real Home/Library/Search, album/artist/playlist detail,
queue, fullscreen/compact Now Playing, Downloads/Add Music, device picker and Settings
as those backends become available. Mobile uses bottom navigation, mini/fullscreen
player and sheets; desktop admin forms cannot simply be squeezed into a phone.

Use artwork intentionally: borderless album shelves, small row images, large current
art, contrast-safe accent washes and useful metadata. Artist portrait, album cover,
playlist mosaic and user avatar are different types. Dense data uses lists/tables;
settings use rows. Do not fetch decorative art from external services without rights.

Every active control has an actual command, capability reason or recoverable state.
Complete right-click/long-press menus, focus, Escape/Back, keyboard reorder, selection,
Undo receipts, loading/error/empty states and source-origin labels. Navigation,
scale/theme, opening lyrics and adding favorites must preserve current audio.

Visual acceptance includes exact dark/light screenshots at small phone/tablet/
640/1280/1440/1728/ultrawide effective widths; app75–200%, text200%, long metadata,
missing/corrupt art, offline/slow/error states. Actual platform minimum widths are
recorded, not fabricated by requesting an unsupported window size.

## 10. Optimization program, driven by evidence

No “super optimization” claim without a baseline and a reproducible improvement.
Create a traceable stage timeline: UI click, auth resolution, HEAD/GET, cache lookup,
hash/verify, metadata/decode, output load, engine clock advance and physical audio
where measurable. Separate cold first play, warm validated hit, current-row resume,
prefetched Next, random seek and offline launch.

Optimize in this order:

1. Remove blocking file/hash/image/keyring/network work from UI thread.
2. Bound async tasks/channels/transfer buffers/decoded artwork; current work outranks speculation.
3. Eliminate duplicate requests and repeated data conversion/copies with single-flight leases.
4. Keep one output thread/player, reuse connections/decoders where correctness permits.
5. Make row rendering granular: current progress updates only transport/time, not the entire catalog.
6. Virtualize large lists with stable IDs and bounded prefetch overscan; preserve keyboard focus.
7. Load correct-sized artwork near viewport; release decoded resources and object URLs promptly.
8. Tune indexed SQL and incremental sync; do not download entire catalogs into UI state.
9. Profile renderer/layout/paint/compositor and audio CPU separately; validate GPU/DMABUF behavior per backend.
10. Tune based on measurements, then retain regression budgets and raw evidence.

Suggested initial targets for discussion, NOT measured results: zero source reload
for current-row pause/resume; zero body transfer on a valid complete-cache hit;
at most one speculative source worker initially; no idle60/144Hz app-level render
loop; preference input feedback within100ms on recorded hardware. Set cold-start/
seek/Next/handoff budgets from actual decoder/network/platform measurements.

Measure p50/p95/p99 start/seek/API latency, RSS/heap/decoded textures, UI frame intervals/
jank, audio underruns, CPU wakeups, storage/write amplification, transfer bytes and
mobile power/thermal behavior. Validate10k/100k catalogs and large queue edits under
concurrent scan/import/DSP. Synthetic1M rows are metadata stress only, not a real
listening catalog or proof of supported concurrent listeners.

Do not use unsafe code, custom allocators/schedulers or a new distributed service
to avoid a straightforward bounded/cache/index fix. Keep resources configurable
through the real policy-aware Settings system rather than hardcoded magic promises.

## 11. Bug elimination workflow

Maintain a native defect ledger with current source hash, symptom, minimal steps,
platform/backend/device, expected/actual result, evidence, severity, suspected owner,
regression test, fixed commit and remaining limits. Distinguish confirmed defect,
missing feature, unsupported platform, stale installed build and performance hypothesis.

Audit every lifecycle: first launch/login/logout/expiry; rapid Play/Next/Pause; seek;
EOF/repeat/shuffle; queue/context edits; cover loading; navigation/theme/scale;
offline/save/remove/repair; server restart/transfer cancellation; route/device change;
background/foreground/locked screen; process crash and update/restore.

Test late responses after route/account/origin/engine replacement, same UUID on two
servers, delete during active playback, image decode failure, disk full, denied
filesystem, keyring timeout, revoked grant, stale control socket and interrupted
handoff. Preserve originals and captured failures. Do not mark “all bugs fixed” from
one green suite or replace failures with mocked success responses.

Release with zero unresolved known P0 defects in the declared supported scope; P1
exceptions need named impact/workaround/owner. Unsupported targets remain explicit.
This is an honest quality gate, not a mathematical guarantee of no future defects.

## 12. Native delivery order and future delegation

| Native tranche | Deliverable | Gate |
| --- | --- | --- |
| N00 | Actual-state audit, dirty work preservation, runtime/art fixture baseline | Current defect ledger and exact source/runtime separation |
| N01 | Authorized cover pipeline in all core native surfaces | ART01–ART14, no credentials in renderer |
| N02 | Native Settings, scale/theme/text/density and recovery | SET/SCALE suites, real Wayland/X11 visual continuity |
| N03 | Stable queue/native restore/offline origin migration and cold launch | Actual restarted server-offline playback and identity collision tests |
| N04 | Storage controls/single-flight/actual-next preparation | Measured body reuse, cancellation/quotas and actual shuffle-next |
| N05 | Reviewed progressive sources and audio quality improvements | Integrity decision plus codec/output/seek/latency evidence |
| N06 | Linux daily feature completion and exact package | Artwork/Home/Search/Library/Add Music/queue/settings and real output/media controls |
| N07 | Android Media3 service/downloads/system controls | Physical background/locked/Bluetooth/airplane-mode evidence |
| N08 | iOS AVFoundation/background/offline/system controls | Apple toolchain and actual device evidence |
| N09 | Windows/macOS native runtime/media controls/packages | Each actual OS install/run/seek/art/cache/restore proof |
| N10 | Devices/handoff/rooms/multi-server integration | Two real clients, authorized participants and real independent origins |

Artwork and scale can be separate bounded changes once shared root contracts are
agreed. Do not make P0 artwork wait for a complete federation implementation.
Use the existing320-feature catalog to fill each mature native surface; unsupported
features remain hidden or truthfully explained until their actual services ship.

Future task ownership: integrator owns Cargo/migrations/root wiring and build
schedule; artwork engineer owns a new isolated art adapter/components; source/cache
engineer owns agreed cache/transfer files; platform engineer owns native bridge;
QA/security owns real fixtures/read-only review/tests. Never assign simultaneous
edits of ui.rs/main.rs or competing full builds. Delegate only within a future
authorized implementation task and the actual available concurrency.

## 13. Native references and verification limits

- [Dioxus0.7.10 DesktopService](https://docs.rs/dioxus-desktop/0.7.10/dioxus_desktop/struct.DesktopService.html): pinned zoom/asset integration surface; exact source still governs adapter design.
- [Media3 background session](https://developer.android.com/media/media3/session/background-playback): platform audio lifecycle; no Vela Android build was executed here.
- [Media3 offline downloads](https://developer.android.com/media/media3/exoplayer/downloading-media): persistent manager/service and requirements; integrate, do not duplicate download authorities.
- [Media3 preload manager](https://developer.android.com/media/media3/exoplayer/preloading-media/preloadmanager): native preparation mechanism; adapt ranking to actual Vela queue. Initial generic preloading URL was inaccessible; this documented page was found by search.
- [Apple AVAudioSession](https://developer.apple.com/documentation/avfaudio/avaudiosession): native playback lifecycle; hardware verification remains mandatory.
- [Apple AVQueuePlayer](https://developer.apple.com/documentation/avfoundation/avqueueplayer): candidate Apple queue engine; fetched page had minimal JS-only content, so exact implementation API usage remains an implementation-time verification.
- [Apple Now Playing](https://developer.apple.com/documentation/mediaplayer/mpnowplayinginfocenter): system media projection with privacy considerations.
- [Microsoft system media controls](https://learn.microsoft.com/en-us/uwp/api/windows.media.systemmediatransportcontrols): OS transport integration; select exact desktop/Win32 binding with a Windows spike rather than assuming UWP sample code applies unchanged.

## 14. Copyable native implementation prompt

Use this section only when the user authorizes implementation, not while creating
this documentation. The full section is a task instruction to a future agent:

> You are the principal native client engineer for the EXISTING Vela Music Rust/
> Dioxus repository. Read AGENTS.md, current PROGRESS/CONTRACTS/PLATFORM_MATRIX and
> docs/PLANS/2026-10-19/PRODUCT_EXPANSION/README.md plus this native master plan.
> Re-inspect the actual checkout, dirty work, live runtime and data before edits.
> Preserve existing user changes, libraries, frozen runtime/target backups and all
> working source/auth/cancellation/privacy behavior. Do not recreate the app.
>
> Deliver N00 then the complete N01 authorized-artwork slice first: current native
> Library/player use neutral placeholders despite existing artwork_id and the
> private artwork route. Implement a bounded authorized native art service/renderer
> bridge, wire actual images into core surfaces and execute ART01–ART14 with real
> fixture media. Do not leak cookies/tokens to the renderer or fetch arbitrary URLs.
> Missing public/artist art stays an honest fallback. Record source versus runtime
> evidence; do not claim tags were broken solely from a placeholder screenshot.
>
> Continue N02–N06 with real Settings/scale, stable native queue restore, origin-safe
> offline cold launch, existing-cache improvements, single-flight actual-next
> prefetch, measured optimization and reviewed progressive integrity. Keep one
> output authority and the current verified-source fallback. Never claim prefetch
> from an ordinary complete cache or claim144Hz from a renderer option.
>
> Build proper native platform adapters: Android Media3 service/download/session;
> iOS AVFoundation/AVAudioSession/system controls; Linux existing Rodio/MPRIS;
> Windows/macOS verified engines/media controls/packages. Shared Dioxus UI and Rust
> services remain coherent. Use actual OS toolchains/devices when available and
> label missing evidence precisely. Unavailable Apple hardware is not permission
> to pretend an iOS build passed or to stop all independent Linux/Android work.
>
> Implement320-catalog features in dependency-ordered complete increments, prioritizing
> daily listening over decorative feature count. Audit all lifecycle/permission/
> race/visual failures, add meaningful regressions, fix introduced defects and
> continue while the active execution session permits. Use real subagents only
> when authorized/available with nonoverlapping ownership; main owns integration,
> Cargo/migrations/build schedule. At most2 compiler jobs on this host.
>
> Run fmt/strict native+WASM/workspace Clippy, relevant real server/unit/property/
> browser/native tests, actual artwork/cache/prefetch/offline metrics and responsive
> dark/light/HiDPI/scale screenshots. Follow Better Design guidance/review on actual
> rendered surfaces. Record exact tests, source/artifact hashes, platform scope,
> failures/blockers and a resumable checkpoint. No fake rows, mocked completion,
> unauthorized media/DRM bypass, destructive user-data replacement, external
> publication, paid services or privileged deployment without required authority.
