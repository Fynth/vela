# Safe migration into the existing Dioxus clients

All paths below were inspected at 4f16a32. Recheck current source before editing.
This document does not create new components, DTOs, assets or migrations by itself.

## 1. Current implementation map

| Area | Existing location | Required transfer boundary |
| --- | --- | --- |
| Web root/actions/player/route selection | apps/web/src/main.rs | Keep the root mounted; change composition without replacing transport authority |
| Sidebar/mobile destinations | apps/web/src/shell.rs | Reference order/geometry and complete authorized route discoverability |
| Home/global Search/request scopes/skeletons | apps/web/src/experience.rs | Real section binding, shared query, matching geometry and current freshness guards |
| Web transport | apps/web/src/client.rs | Reuse typed requests, AbortController scopes and existing cookie/CSRF handling |
| Web audio lifecycle | apps/web/src/browser_player.rs, apps/web/player.js | Preserve source/session/element generation and cancellation |
| Pure queue domain/bookmarks | apps/web/src/playback.rs, session.rs | Reuse duplicate/order/shuffle/repeat/paused-restore semantics |
| Web Now Playing | apps/web/src/now_playing.rs, apps/web/now-playing.css | Restyle root dialog; preserve native top-layer/focus/action ownership |
| Web preferences/Settings | apps/web/src/preferences.rs, settings.rs | Keep actual 80–150% andDark/Light/System; make functional settings discoverable |
| Web filters/metadata/playlists | filters.rs, metadata.rs and main.rs | Keep real queries/drafts/revisioned writes; no client-only fake global sort |
| Radio/mixes/private history | radio.rs, personal_mixes.rs, listening.rs, audible.rs | Keep eligibility, consent, measurement and feedback receipts |
| Jobs/import/scan/network UI | jobs.rs, scanner.rs, network.rs and main.rs | Restyle without dropping existing flows or weakening ownership |
| Shared icons | apps/web/src/icons.rs | Extend original outline family only for actual missing symbols |
| Web styles/assets/bootstrap | apps/web/style.css, index.html, bootstrap.js, vela.svg | One deliberate role/cascade migration and first-frame preference application |
| Native root/presentation | apps/desktop/src/ui.rs | Retain root AudioEngine/client/queue/session; extract presentational pieces only where useful |
| Native audio/network/cache | apps/desktop/src/{audio, client, media_cache, offline, artwork, artwork_cache}.rs | No second downloader/player or credential exposure for visual convenience |
| Native preferences/window | apps/desktop/src/presentation.rs, window.rs | Preserve device store and shared frameless Linux builder/controls |
| Native styles | apps/desktop/style.css | CSS is embedded through document: :Style; web static paths are not automatically native assets |
| Transport DTOs | crates/domain/src/lib.rs | Consume real data; only add contracts when an accepted capability truly needs them |
| Static asset authorization | apps/server/src/lib.rs | Exact allowlist for shipped new assets; never allow an entire filesystem subtree |
| Build/package | scripts/build-web.sh, package-release.sh, verify_package.py and desktop package tools | Copy and verify the exact assets the tested artifact actually uses |
| Real browser/native regression | tests/browser/, apps/desktop/examples/native-ui-qa.*, scripts/verify_native_client.py | Extend existing fixtures and evidence; do not replace with screenshots of mock responses |

There is no apps/web/src/api.rs and no inspected apps/web/desktop Dioxus.toml. Do not
invent these as existing integration points or run dx new/cargo new. Read the actual
current build scripts instead of imposing the prototype's Next.js build system.

## 2. Collision-safe role migration

Use a Vela namespaced canonical layer for the target design. This is a proposed
mapping; implement it once per client and remove conflicting obsolete selectors after
the corresponding consumers migrate. No second runtime theme or UI framework.

| New canonical role | Approved target | Existing reader compatibility during migration |
| --- | --- | --- |
| --vela-bg | #131513 | Old--bg aliases this |
| --vela-text | #f3f3ed | Old--text aliases this |
| --vela-card | #1b1e1b | Old--surface aliases this |
| --vela-popover | #1c201d | Map overlay surfaces explicitly |
| --vela-secondary | #272c27 | Old raised surface where its semantic role fits |
| --vela-muted-surface | #232823 | Do not assign this to old--muted text |
| --vela-muted-text | #9aa296 | Old--muted aliases this while old consumers remain |
| --vela-quiet-text | #798372 | Old--quiet only on contrast-qualified surfaces |
| --vela-primary | #d3e9a5 | Old--accent aliases this while it means action |
| --vela-on-primary | #1b2511 | Old--accent-text aliases this |
| --vela-accent-surface | #293025 | Independent surface role, never old action--accent |
| --vela-border | #2a2f2a | Old--line aliases this |
| --vela-input-border | #383f37 | Input role, not all borders |
| --vela-danger | #ef9290 | Old--danger aliases this |
| --vela-sidebar-bg | #0e100f | New color role; never length--sidebar |
| --vela-sidebar-width | 216px baseline | Old length--sidebar aliases this until all layout readers migrate |
| --vela-player-height | 88px baseline | Old length--player aliases this until migrated |
| --vela-player-bg | #10130f | Independent player surface |

Add named roles for approved search/filter/hover/current/rail/mobile values from 02.
Keep actual Light overrides on the same canonical roles; do not create a second set
of component selectors for every route. Old aliases must not silently invert meaning
halfway through a slice. Inventory every `var(--sidebar)`,`var(--muted)` and
`var(--accent)` reader before replacing it. Unit/semantic lint can catch a color in
width or a background in text, but real renders remain necessary.

Use one clear cascade. Do not add a third large CSS appendix whose selectors fight
the previous redesigns. Consolidate only touched component rules with reviewable
before/after behavior; avoid an unrelated whole-file rewrite. New layout tokens are
owned by the shell, not redeclared independently by Home, Settings and player.

## 3. Shell extraction without audio remount

1. Identify existing root player element/output instance and record its identity,
   source generation, cursor, position and current context in an isolated test.
2. Introduce/reshape presentational shell boundaries around current children. The
   root audio owner remains outside route/filter/dialog keyed subtrees.
3. Use a main-column header and central scroll container. Apply sidebar offset once;
   zero it at mobile. Reserve player/navigation heights once. Remove old inherited
   page insets that would apply a second offset.
4. Share Topbar/Search query state and a bounded navigation stack. Back/Forward is
   view history, not queue history. Stable per-view scroll/focus anchors do not carry
   credentials or old-account DTOs.
5. Restyle the existing player first, then routes. Root commands update all row/card/
   drawer projections. Do not retain duplicate page-local player booleans “temporarily”.
6. Move native visual pieces into cohesive local components only when it makes a
   real boundary. Do not invent 30 empty crates or a full component framework.
7. Verify source/engine identity and actual playback before accepting each route.

The native client remains a client. It does not mount a local server or render the
web build in place of its own guarded native client. Its embedded stylesheet and
window origin need separate asset/geometry evidence.

## 4. DTO-to-component mapping and genuine gaps

| Needed presentation | Actual data / missing contract |
| --- | --- |
| Track row | Track: title, artist, album, track/disc number, duration_ms, codec/rates/size, artwork_id, favorite; no media path in DTO |
| Album card/detail | AlbumSummary: id, title, artist, artist_id, year, count, duration, artwork_id; fetch actual album context |
| Artist circle/detail | ArtistSummary: id, name, track/album counts, artwork_id; artwork_id is not proof of portrait provenance |
| Sidebar playlist | PlaylistSummary: id, name, owner_id, revision, item_count, can_edit, folder_id; no dedicated cover in this summary |
| Playlist item | PlaylistEntry: id, track, position, can_stream; metadata permissions do not authorize audio |
| Track sort | TrackSort: Library, Title, Newest; Artist is a genuine extension if shipped |
| Actual art | Authorized artwork endpoint/native store; reuse state/error/cache limits |
| Global favorite count | Need actual aggregate count/projection, not just loaded favorites page |
| Album save state | Not supplied by Track.favorite or current AlbumSummary; needs a real owner-scoped capability |
| Editorial target | Needs stable actual curated refs/configuration; no matching by demo title or mood guessing |
| Recently listened albums | Needs current consent-qualified album projection or an honestly different catalog shelf label |

For a missing capability choose the smallest truthful delivery. The full target may
add an owner-scoped saved-album service after stable album identity and migration
backup/restore are proved. It must include read/write/revocation/export/privacy and
tests. Until that package is accepted, the card cannot fake save state by favoriting
one song. The transfer stays PARTIAL for that target action if it is omitted.

Artist sort requires validated enum/HTTP input, indexed order with deterministic
tie-break, cursor encoding and regression across pagination. Never implement it as
`sort()` on the first page. A schema/index change follows the existing additive
migration and backup/restore discipline. Do not change media ownership/storage to
achieve a layout goal.

Playlist covers use an actual authorized bounded mosaic/detail projection or neutral
fallback. Avoid fetching every playlist's entire track list at startup just to build
three thumbnails. Group summaries and current player refs can share single-flight
artwork requests; display state is not a new downloader.

## 5. Asset/font delivery

Proposed app-owned paths are exact, not an existing route claim:

- `/assets/brand/vela-coast.png` and `/assets/brand/vela-wave.png` for approved decorative art.
- `/assets/fonts/manrope-latin.woff2` and `/assets/fonts/manrope-cyrillic.woff2` for inspected subsets.
- A bundled font notice/source record in the existing third-party notice mechanism.

The concept atlas remains a reference/isolated visual fixture, not the default image
for real albums. Do not add a wildcard public `/assets/**` bypass. Add each approved
asset to server protection/build copying/package verification; confirm correct MIME,
cache policy and denial of unlisted neighboring files.

Native currently embeds CSS with include_str. Relative `/assets/...` URLs cannot be
assumed to resolve to the configured music server or the desktop binary. Use one
reviewed app-owned Dioxus asset/bundle resolver or embedded-font adapter compatible
with the actual pinned release; test the packaged application offline. Keep font/
brand assets separate from account-authorized covers and credentials. Do not add a
runtime CDN as a quick way to make the preview look correct.

Use licensed font files/notices with required Unicode ranges. Preload only first-
screen subsets; reserve fallback metrics. Verify Cyrillic, Latin, variable weights,
long labels, network denial and cold/warm launch. No late font swap may move current
controls or turn a click into another action.

## 6. Requests, scope and performance

Reuse RequestScope and the independent freshness checks. Every new view query has a
physical cancellation path plus account/origin/query/generation/lifetime validation.
Menu/picker receipts own safe mutation lifetime even if the overlay closes. Ordinary
refresh must not erase pending feedback; privacy/account changes invalidate at once.

Keep cursor pagination, bounded rendering and lazy decoding. A six-cover Home shelf
does not need a 1m-row catalog request. Resize/scale/scroll geometry is measured around
real changes, not on every audio clock tick. Avoid per-frame Rust state for CSS hover/
shimmer. Stop decorative work hidden/offscreen/settled. Do not create audio transfer
traffic just to populate a hover state or source badge.

Measure current host cold/warm launch, source start, menu latency, scroll/frame intervals,
art requests/memory and idle CPU. The WebKitGTK frame-rate diagnosis is an existing
platform limit, not evidence that this new CSS reaches 144Hz. Preserve Wayland/DMABUF
and the Linux window builder; never change Hyprland rules to hide a layout bug.

## 7. Preferences and scaling

Keep the current web `vela.device.presentation.v1` and native presentation service.
Do not change storage schema just to rename colors. Never write credentials/audio
URLs into presentation preferences.100% follows platform scaling; app zoom is applied
once and composes with browser zoom/monitor DPI. Reference 216/88 dimensions are the
100% design, not a promise of physical pixels at 150% or DPR2.

Validate actual viewport geometry with current zoom. Fixed bars,100dvh, scroll insets
and safe areas can interact; do not blindly multiply every measurement byDPR or add
a second compensating transform. If the current adapter needs a layout correction,
make it in that single owner and preserve existing scale storage/recovery tests.

## 8. Integration discipline

Main integrator owns root wiring, Cargo/migrations, cross-module contracts, allowlists
and builds. This planning task delegates nobody. A later authorized implementation
can delegate isolated presentation/tests with nonoverlapping files and real review;
one coordinated Cargo process, at most 2 compiler jobs. Preserve current dirty work.

First slices need no schema migration. Any real later data-contract addition needs
explicit API tests and a verified isolated backup/restore before changing persistent
semantics. Never clear a user's library, delete target snapshots, replace original
media, reset history or upgrade the frozen service merely to simplify a screenshot.
