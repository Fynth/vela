# Delivery roadmap and acceptance ledger

Every TR package and PT test below starts PROPOSED/NOT_RUN. The planning task has
not run these application checks. Preserve existing named EXP/PLAY/RES/NET/AUTH/NP,
playlist/history/DSP, ART/SCALE/SET and later UX/menu/loading tests; this is an additive
acceptance map, not replacement evidence or renumbering of the mandate's G01–G24.

## 1. Delivery packages

| ID | Dependencies | Complete outcome and exit boundary |
| --- | --- | --- |
| TR00 | None | Reconcile HEAD/dirty/runtime/data, read contracts, map real actions/capabilities, capture equivalent before baselines; no user-data mutation |
| TR01 | TR00 | Collision-safe semantic roles, Manrope subsets/notices, approved app-owned brand assets and exact allowlist/package/native resolver; no runtime CDN |
| TR02 | TR01 | Desktop shell/sidebar/topbar/query/back-forward/central scroll with real destinations; sidebar offset/player reserve each applied once |
| TR03 | TR02 | Reference-style persistent player in web/native roots, real state/actions/quality, source continuity through navigation/theme/scale |
| TR04 | TR02/TR03 | Home header/filter views, real editorial targets, actual My Wave, album shelf/additions/footer, truthful sparse/empty/loading states |
| TR05 | TR02/TR03 | Library/Search/album/artist/detail rows and cards, real facets, pagination/order; accepted Artist-sort backend extension tested if exposed |
| TR06 | TR05 | Playlist list/detail/create/add/folders/sharing and consistent favorites; real cover projection, revision writes and explicit album-save capability gap resolved or recorded PARTIAL |
| TR07 | TR03/TR05/TR06 | Right Queue drawer, expanded player, context menus/pickers/receipts with native-dialog layering/focus/occurrence identity |
| TR08 | TR02/TR03 | Functional Settings and remaining imports/scanner/jobs/history/network/admin views; More→Settings retains every authorized destination |
| TR09 | TR04/TR05/TR06/TR07/TR08 | True mobile composition, tablet boundaries, mini-player+navigation reservation, touch/keyboard/safe-area/short-height recovery |
| TR10 | TR01/TR03/TR07/TR09 | Actual Linux native/Wayland package behavior, art/font resolution, window chrome/controls, scale and one AudioEngine; other platforms accurately classified |
| TR11 | All accepted preceding scope | Full relevant Rust/browser/native integration, negative permissions/races, visual review and measured performance/resource regression checks |
| TR12 | TR11 | Local package/asset verification, source-versus-artifact ledger, updated docs, coherent commit and resumable remaining-platform/action handoff |

TR11 gates run after each coherent vertical slice; do not defer all tests until the
last week. TR02/TR03 are a usable first increment. Then complete real Home and one
Library→album→playlist journey before restyling every secondary screen. No hundreds
of empty components or unfinished active buttons.

Each package records named PT IDs, actual diff, source/artifact hashes, check results,
screenshots and explicit unsupported behavior. A decorative version of a target
action does not pass. Backend additions require their own API/privacy/migration
tests and verified isolated restore before persistent semantics change.

## 2. Fixture contract

Use the existing generated lawful FLAC/MP3/PCM and cover fixtures with an isolated
SQLite/media directory. Create usersA/B, private libraries, owned/collaborative/read-only
playlists, metadata-only references, duplicate occurrences, denied art and expired grants.
Seed sparse 1/5-item and ordinary 6+album/50+track views through real supported APIs.
Real generated fixtures may have descriptive fixture metadata; never put the prototype's
commercial demo collection into real user accounts.

Hold/release actual responses through the current test infrastructure. Distinguish
an intentionally injected 503 from a real backend result. Do not fabricate successful
catalogs, permission projections, playlist writes, radio candidates or completed downloads.
Keep production login limiter/SSRF/auth policy intact. Never reuse production tokens.

Before/after images use the same fixture, route, state, viewport, theme, app/browser zoom,
DPR and browser/native version. Reference demo text/covers may differ from lawful
Vela fixture data; compare composition and element geometry, not a misleading global
pixel-diff score across different songs. An actual prototype screenshot is a separate
artifact and was NOT captured by this planning task.

## 3. Visual and responsive acceptance

| ID | Executed procedure required | Pass evidence |
| --- | --- | --- |
| PT01 | At 1284×856/100% open signed-in Home in dark | Actual sidebar 216, topbar 75, player 88 logical geometry within documented rendering tolerance; no double inset |
| PT02 | Render all shared components with fonts ready | Actual Manrope Latin/Cyrillic/weights; no silent fallback or late control shift |
| PT03 | Inspect sidebar/topbar active/hover/focus/disabled | Correct role colors, dot, state semantics and no shifted hit target |
| PT04 | Open Home with real editorial/mix targets | Correct two-card ratio, height, crop/layering, working controls; no demo rows |
| PT05 | Change all three Home filters | Exact eligible sections, real actions, stable audio and no stale view overwrite |
| PT06 | Inspect shelves/track rows at baseline | Borderless square art, six-or-fewer real previews, aligned columns, current state distinct from hover |
| PT07 | Repeat at 1499/1500/1501 and wide 3440 client width | Sidebar/insets/features adapt, max-width centered within main column, no blank double-offset band |
| PT08 | Repeat at 1100/1101 and 900/901 | Correct compact columns, secondary content removal, accessible full-catalog route |
| PT09 | Repeat at 760/761 and mobile 390×844 | Sidebar switches cleanly to mobile composition; no competing desktop bottom bar |
| PT10 | Repeat at 360/361 and 320 width with long Cyrillic | No essential clipped target, horizontal body overflow or lost playlist hero action |
| PT11 | Scroll to final row/footer on phone and desktop | Last item/focus scrolls above actual player/navigation; central scrolling independent of fixed topbar |
| PT12 | Exercise short-height/landscape/onscreen keyboard | Close, seek, submit and Settings recovery reachable; no rigid 400px clipping |
| PT13 | Open Queue/Now Playing/upload/create/Settings at each core width | Correct right drawer versus center dialog, internal scroll, focus/Close reachable, cover bounded |
| PT14 | Dark/Light/System and reduced-motion matrix | Functional existing preferences, readable roles, static useful states; light not falsely called prototype-identical |
| PT15 | App 80/100/125/150 plus browser/text 200/400%, nativeDPR | One zoom application, reachable controls, actual pointer rectangles correct, scale reset survives |
| PT16 | Review opaque and image-overlaid text/controls | Actual contrast, target spacing, focus, current marker and hover/touch parity; fix critical/serious findings |

Use exact reference dimensions at 100% only. Native screenshot PNG dimensions may be
logical viewport×DPR; record both. Coarse-pointer variants can increase actual reserved
player height when needed for 44px targets rather than overlapping invisible hit areas.
All newly reserved dimensions must be reflected in the shell's single layout authority.
For PT01, use a proposed±2px logical geometry tolerance after font readiness in a
matched reference environment; document any different platform rounding separately.
This is a future acceptance tolerance, not a measured current match or a font-pixel claim.

## 4. Listening and product behavior acceptance

| ID | Executed procedure required | Pass evidence |
| --- | --- | --- |
| PT17 | Play lawful fixture, seek 7s, toggle current row/player/card | Same active source/occurrence, time retained on Pause/Resume; no restart bug |
| PT18 | While playing, navigate Home→Search→album→playlist→Settings | Exact root media/output identity and source/context retained |
| PT19 | Rapidly selectA→B whileA prepare/play reply is held | StaleA cannot autoplay or replaceB; requested Pause wins during preparation |
| PT20 | Add same track twice, Play next, append, reorder/remove others | Distinct occurrence IDs, explicit-next priority, current source retained |
| PT21 | Enable shuffle, inspect drawer, use Previous/Next/EOF/repeat | Actual-next display agrees without consuming order; existing fair deck/history and repeat semantics pass |
| PT22 | Scrub preview/cancel/commit and keyboard seek | No seek storm, clock fighting or source recreation; unknown/unseekable state truthful |
| PT23 | Set volume, mute/unmute in both player surfaces | Actual level synchronized, prior value restored; no hard-coded 65% reset |
| PT24 | Topbar query→Search, fast type, clear, Enter, IME | One query, debounce/cancel/freshness, correct results; no composition submit or duplicate request owner |
| PT25 | Navigate Back/Forward with filters/scroll/selection | Correct view anchors restored, not queue history; buttons reflect real availability |
| PT26 | Open actual album/artist andPlay/Shuffle | Correct grouped/disc order, real context, appropriate unavailable-state behavior |
| PT27 | Apply actual facet/sort and cross multiple pages | Server-wide stable order/cursor; Artist option only if implemented, not loaded-page sorting |
| PT28 | Create playlist, add/remove/reorder with two clients | Real acknowledged entries/counts, CAS/idempotency, conflict preserves current edits and audio |
| PT29 | Favorite/unfavorite across row/player/list; save album if supported | Shared real state; album save independent of arbitrary representative track |
| PT30 | Open PKM/More/keyboard menu and playlist picker | Correct immutable target, real commands, no right-click autoplay; native top-layer click/focus works |
| PT31 | MobileMore→Settings→other destinations, desktop less-common routes | All existing authorized import/history/network/scan/jobs routes reachable; no raw admin screen forced on consumers |
| PT32 | Upload FLAC/MP3, preview authorized URL import, observe jobs | Real server storage/metadata/art/playback and task receipts; no object-URL-only success substitute |

## 5. Privacy, permission and race acceptance

| ID | Executed procedure required | Pass evidence |
| --- | --- | --- |
| PT33 | Metadata-only collaborator opens menu/playlist/current item | No unauthorized stream/art/download despite visible title; forced handler reauthorizes |
| PT34 | Hold catalog/art/picker reply, logoutA/loginB, release | No old private result, cover, toast, share link or focus target reappears |
| PT35 | Revoke stream/share while source/detail is cached | Cached title/art state does not grant new media access; actual denied recovery |
| PT36 | History account/device consent off/on, qualified listen, erase/export | No upgrade opt-in, no play-event fake listens, no retained-disabled reads, late outbox/export invalidated |
| PT37 | Show private and shared playlists/network-origin content | Accurate visibility/origin labels; no unconditionalOnly for you claim or redirected credential leak |
| PT38 | Fail favorite/playlist save, then make result uncertain | Safe rollback on known failure, reconcile uncertain retry, same mutation ID, correct Undo receipt |
| PT39 | Open/close nested dialogs/menu viaEscape/Tab/outside/native close | Correct active-layer focus/inertness, return target and cleanup; no invisible blocker |
| PT40 | Change route/query/account during refresh/retry/pagination | Physical abort plus independent guards; known authorized same-context data retained only when valid |

Do not infer general privacy correctness from a screenshot. Negative API tests must
hit actual authorization boundaries. Stored secrets, raw media URLs and private paths
never appear in copied track links, diagnostic exports or presentation preferences.

## 6. Assets, native runtime and resource acceptance

| ID | Executed procedure required | Pass evidence |
| --- | --- | --- |
| PT41 | Fetch exact shipped web font/brand/CSS paths and unlisted neighbor | Correct bytes/MIME/cache and packaging; unknown assets denied, no wildcard data exposure |
| PT42 | Start native packaged client with no network for bundled UI assets | Manrope/brand assets resolve locally; actual user covers/offline states remain authorization-aware |
| PT43 | Real owned Wayland/Hyprland tiled window + X11 fixture | Shared chrome/window controls, no reintroduced GTK strip, no compositor config changes; client geometry recorded |
| PT44 | Native art pending/ready/missing/denied/logout, long-scroll | Existing cache/single-flight/generation limits respected; no blank images or unbounded data-URL growth |
| PT45 | Real browser/native audio under navigation/theme/scale/overlay | One output authority, positive time/seek, no overlap; null-sink versus physical output clearly distinguished |
| PT46 | Compare cold/warm launch, menu, scroll, artwork and idle traces | Same host/build/workload baseline; bounded listeners/layers/requests, no claimed 144Hz from CSS |
| PT47 | Rust/WASM/native/browser integration and package checks | Exact commands/results/versions/failures/skips preserved; no weakened auth/race tests |
| PT48 | Final source-to-artifact and checkpoint audit | Only intended files committed, original data/frozen runtime preserved, remaining platforms/actions honestly marked |

## 7. Actual future command sequence

These commands are instructions, not a log from this planning task. Check current
repo support before execution. Run serially with CARGO_BUILD_JOBS=2. Do not run
cargo clean: target contains private backups and frozen runtimes.

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace -- --test-threads=2
cargo clippy --locked -p vela-desktop --all-targets --features native-qa -- -D warnings
cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings
cargo build --locked -p vela-server
cargo build --locked -p vela-desktop --example native-ui-qa --features native-qa
VELA_WEB_PROFILE=dev scripts/build-web.sh
npm ci --prefix tests/browser
```

Web build requires the lock-compatible wasm-bindgen CLI; the inspected script expects
0.2.129. Do not install a random newer CLI and blame the design for a binary mismatch.
The project's browser suite owns isolated servers/data. Run engines in separate
invocations and preserve artifacts before the next run:

```bash
cd tests/browser
npx playwright test --browser=chromium
npx playwright test --browser=firefox
```

Read the active Browser skill if present before controlling an interactive browser.
The existing automated suite is not a user's signed-in browser. Stock Safari/mobile
device validation remains separate from Chromium/Firefox/WASM compilation.

For native use the real current fixture wrapper, not production credentials. It
creates an owned null sink and disposable servers. First read its current help; then
use a fresh evidence directory for each run, for example:

```bash
python3 scripts/verify_native_client.py --help
python3 scripts/verify_native_client.py --backend x11 --test-audio-engine --test-peer --test-session --evidence target/checks/prototype-transfer-native-x11
python3 scripts/verify_native_client.py --backend wayland --window-only --evidence target/checks/prototype-transfer-native-wayland
```

The inspected wrapper requires window-only for Wayland; do not invent a full Wayland
resize/peer result. Secret Service, audio/display tools and hardware availability
can limit individual runs. Record exact unavailable dependencies and continue
independent checks. Release builds/local package verification followTR12 with current
package-script help; no public deployment or frozen-runtime update is implied.

## 8. Review and completion

Load Better Design principles before work and review rules after final changes.
Check actual visible copy in scoped states. Inspect-spacing must receive actual
rendered DOM states and return completed; loading a capture script measures nothing.
Minimum review widths 390/1440/1728 plus reference 1284, tablet,320 and actual native
cases. A tool-free harness can collect equivalent real DOM rectangles/screenshots
and document manual review; it cannot manufacture a Better Design VALID receipt.

Classify findings, fix every critical/serious regression and retain moderate issues
with exact scope. Follow the chosen design rather than generic tool suggestions to
install another system. Do not publish private pages or deploy a public preview as
an automatic review workaround. This music client is not an invented admin dashboard.

Full delivery requires complete accepted user journeys, not every optional future
feature. Report each PT asPASS/FAIL/NOT_RUN with artifact paths and dates. Distinguish
web source, real native window, audio engine, packaged launch, deployed runtime and actual
Android/iOS hardware. Responsive mobile web does not certify native background audio.
