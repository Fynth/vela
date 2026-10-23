# Verified0.12.0 package and preserved runtime checkpoint

## DR-08 the verified Linux presentation parity — 2026-10-23

Only the Linux native presentation changed: `apps/desktop/style.css`,
`apps/desktop/examples/native-ui-qa.js`, `apps/desktop/examples/native-ui-qa.rs`,
`scripts/verify_native_client.py` and the new
`scripts/native_band_contrast.py`. The audio adapter, the HTTP/account
boundary, the MPRIS observer, the session store, the shared playback model, the
server, the domain API and the web client are untouched.

### The commands and their real output

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --locked -p vela-desktop --all-targets --features native-qa -- -D warnings` | exit 0, no warnings |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| `cargo test --locked -p vela-desktop` | `129 passed; 0 failed; 3 ignored` (the 3 are the opt-in audio/MPRIS tests) |
| `cargo build --locked -p vela-desktop --example native-ui-qa --features native-qa` | exit 0 |
| `cargo build --locked -p vela-server` | exit 0 |
| `DISPLAY=:99 python3 scripts/verify_native_client.py --backend x11 --server target/debug/vela-server --test-peer --evidence target/checks/dr08-native` | **exit 0** — `report.json` `ok=true`, **300 checks**, **21 screenshots**, 0 temporary originals; the separate reduced-motion window `reduced-report.json` `ok=true`, 8 checks, `native-reduced-glass-dark.png` |
| `python3 scripts/native_band_contrast.py --evidence target/checks/dr08-native --report report.json` | **exit 0** — verdict `PASS`, 0 failures, 26 glass frames measured |

The pre-change baseline is preserved and was not overwritten:
`target/checks/dr08-baseline-native/` (263 checks, 15 PNG). The new evidence is
`target/checks/dr08-native/` with `report.json`, `reduced-report.json`,
`native-player-band-glass-dark.png`, `native-player-band-glass-light.png`,
`native-topbar-glass-dark.png`, `native-player-band-glass-scrolled-dark.png`,
`native-compact-fallback-light.png` and `native-reduced-glass-dark.png`.

### What the window actually measured

- **Glass.** The player band computes `backdrop-filter: blur(16px)` over
  `rgba(19, 21, 19, 0.85)` and the topbar `blur(12px)` over the same role.
  Exactly two elements of the shell carry a backdrop filter
  (`header.native-topbar`, `footer.native-player`); the sidebar, the rows and
  the scroller carry none.
- **The underlap is real.** At 1280×840 the scroller reaches `839.98px` while
  the band starts at `727.98px`. In the same positive condition at 1280×500 the
  real catalog (804px of content in a 500px scrollport) was observed passing
  under the band, and the last row still cleared it (`row bottom 262px`,
  `player top 388px`). The last focus target stayed visible above the band.
- **The band is 112px and both ranges are 44px.** The band height is read from
  `--vela-player-height`; the seek and the volume inputs both measure 44px.
- **The contrast is reported by two separately named methods.**
  *Computed composite over live intersecting backgrounds* (the driver, live
  computed values): worst band text **16.476:1** (dark) / **14.547:1** (light),
  worst glyph **6.976:1** / **4.536:1**, focus ring **13.954:1**.
  *Rendered pixel sample* (`scripts/native_band_contrast.py`, the captured PNG):
  the worst sampled pixel contrast over 26 glass frames is **12.775:1** (dark
  player) and **13.047:1** (light player).
- **The compact fallback is honest.** At 640×700: no backdrop layer,
  `backdrop-filter: none` on both bands, the opaque player and canvas roles
  (`rgb(255, 255, 255)` / `rgb(244, 247, 243)`), `scroll bottom 537px = player
  top 537px`, the content keeps its own 24px bottom inset, and every control
  stays reachable.
- **The reduced-motion branch is a separate real window process** with GTK
  animations disabled for that process only: `matchMedia('(prefers-reduced-motion:
  reduce)')` is true, the eight duration roles read `0ms`, both bands report
  `transition-duration 0s` and `animation-name none`, and the same two glass
  bands are present.
- **The unchanged contracts were re-measured in the same window:** one
  `.native-player`, one `[data-audio-generation]`, zero `audio`/`video`/`iframe`
  elements, exactly one output stream on the owned null sink, and the source,
  the position and the queue preserved across the theme, the app-size sweep and
  the navigation.

### One pre-existing driver race was repaired

`apps/desktop/examples/native-ui-qa.js` read the settled library grid columns
while the library request was already pending. A settled row that had just
been replaced by the skeleton list made the driver call
`getComputedStyle(null)`, which aborted the run under load. The value is now
read while the library is settled, before the request is submitted. The check
compares the same two numbers as before — only the moment of the read moved.
The failure was reproduced once in this session and is preserved as
`target/checks/dr08-native-final/report.json` (`failure: "Argument 1
('element') to Window.getComputedStyle must be an instance of Element"`, 33
checks).

### What was not run

Screen reader and Orca, physical audio output, a Wayland session, a background
session, a package/installer build, Docker and every non-Linux target are
`NOT_RUN`. The web regression gate was executed as a regression gate only — no
web file changed in this increment: `cargo build --locked -p vela-web --target
wasm32-unknown-unknown --release` exit 0, `scripts/build-web.sh` exit 0
("Web assets built in dist/"), and `npx playwright test` in `tests/browser`
**90 passed / 1 skipped (6.9m), exit 0** in Chromium. Firefox was not run in
this increment.

## DR-06 the music in the first phone screen — 2026-10-22

The package changes the **density** of the mobile composition. Nothing in the
audio, the queue, the session, the owner-scoped data paths or the desktop
composition changed: no asset, no allowlist entry, no manifest, no migration
and no second media element.

The measured baseline on this host (Chromium, Library, **2 real tracks**,
default filters, no notices) was taken with `tests/browser/dr06-probe.mjs`
against the pre-change stylesheets and is written to
`target/checks/dr06-probe/dr06-baseline.json`; the "after" column below is the
same probe pointed at the changed stylesheets and is written to
`target/checks/dr06-probe/dr06-probe.json`. Both are real files, produced by
`node dr06-probe.mjs` with `VELA_TEST_WEB_DIR` selecting the stylesheet set:

| Surface | Before | After |
| --- | --- | --- |
| First `.track-row` top, 320×700 | **478.09px** | **378.91px** (99.18px gained; the plan required 88px) |
| First `.track-row` bottom, 320×700 | 593.09px | **491.91px** (the whole row, actions included, is above the 510px mini-player) |
| First `.track-row` top, 390×844 | 430.19px | **339px** |
| First `.track-row` bottom, 390×844 | 545.19px | **452px** (two full rows end at 565px, the player starts at 654px) |
| `.page-heading` height, 320×700 | 91.91px | 83.91px |
| `.section-toolbar` height | 111px | 100px |
| `.library-filters` height | 72.69px | 44px (the default order leaves the flow) |
| `.track-header` height | 50.5px | **36px** (one row; the empty actions heading no longer opens a second grid row) |
| `.track-row` height | 115px | **113px** (44 + 8 + 44 + 8 + 8 + the 1px rule) |
| `--vela-content-top` | 25px | **16px** |
| Document overflow, 320/360/390/760/761/768 | 0px | **0px** |

| Check | Paths | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- |
| The first track and its own actions are inside the first screen | apps/web/style.css, tests/browser/dr06-mobile.spec.mjs | Measured in the live document at 320×700 and 390×844 with a real account and two real uploaded tracks: the first `.track-row` starts at **378.92px** (limit 390px) and ends at **491.92px** (limit 510px) at 320×700, and at **339px** / **452px** at 390×844. The action row is a real 44px row of five 44×44 buttons whose right edge is the row's own right edge, so nothing is clipped and nothing touches the title or the time. The evidence is `target/checks/dr06-mobile/dr06-first-screen.json` and the screenshots `dr06-library-320x700.png`, `dr06-library-390x844.png` | PASS measured | A real mobile browser with a dynamic toolbar was not used |
| The table header is one row | apps/web/style.css | The `.track-header` measures **36px** (35..37) at 320 and 390, and its grid has a single row: the empty actions heading is `display: none`, so its three visible labels (`#`, `TITLE`, `TIME`) no longer open a second, empty row | PASS measured | — |
| The idle sentence of the mini-player reads in full | apps/web/style.css, apps/web/src/main.rs | The idle title is the short DR-05 wording "Nothing playing" at **12px/600** with the 10px/400 subtitle "Choose a track". Measured in both engines at 320×700: the text is never cut horizontally (`scrollWidth` ≤ `clientWidth` + 1) and it takes at most two lines (≤37px) inside the 120px band, whose 60px of content still clears the 44px seek rail. The two engines measure the same Manrope text differently — Chromium 87px, Firefox 89.87px for an 88px title path — so the sentence **wraps** in Firefox instead of losing its last glyph | PASS measured | — |
| The default library order is available to a screen reader, not shown | apps/web/src/filters.rs, apps/web/style.css | The summary carries `data-default-order="true"` **only** while the applied filters equal `TrackFilters::default()`. Measured at 320 and 390: the element is in the DOM, its `role` is `status`, its text is "Library order", and it is visually hidden with the existing `.sr-only` technique — `position: absolute`, a **1×1px** box and `clip-path: inset(50%)` — so the text stays in the accessibility tree while the filter block is **44px** tall instead of 72.69px. With an active sort ("Recently added") the marker is **absent**, the box is **288px** wide at 320 and **350px** at 390, the clip is gone and the text is not truncated | PASS measured | Screen reader NOT_RUN: the announcement is a DOM fact, not an observed announcement |
| The transport and the navigation are unchanged | apps/web/style.css | Measured at 320, 360, 390 and 760: the player band is **120px**, the navigation band is **70px**, the reserve is **190px**, the scroller ends at **654px** of 844px, the document overflow is **0px**, Play/Next/Queue are **44×44** and the seek rail is **44px** tall. The navigation is exactly **5** equal destinations, **60×53** at 320, **68×53** at 360, **74×53** at 390 and **149.6×61** at 760 — every one of them at least 44×44. At 761 and 768 the composition is the tablet one: the 88px band, no navigation and **0px** overflow | PASS measured | — |
| The safe-area inset is counted once | apps/web/style.css, tests/browser/dr06-mobile.spec.mjs | **SIMULATED**: the host has no cutout, so the inset is emulated through the same `--vela-safe-*` variables the shell consumes. Baseline: the navigation is **70px** and the reserve **190px**. With `--vela-safe-bottom: 34px`: the navigation row is **104px** (70 + 34, once), the player band still ends directly above it at **740px**, the two bands together fill the viewport exactly (740 + 104 = **844px**), the reserve is **224px** (120 + 70 + 34) and the scroller ends at **620px** = 844 − 224. The document overflow stays **0px**. The stylesheet still carries one reserve authority with one `--vela-safe-bottom` term | PASS measured (simulated) | Real hardware safe-area insets NOT_RUN: no device and no X server on this host |
| The empty editorial card keeps its own height | apps/web/home-editorial.css, tests/browser/dr06-mobile.spec.mjs | Measured at 390×844: the empty feature card is **191.9px** tall (limit 240px), its "Open playlists" action is **44px** tall and lies completely inside the card. At a 150% app size the card measures **326.45** visual px (**217.63** CSS px): the copy is not clipped (`scrollHeight` ≤ `clientHeight`), the action is still 44px in CSS pixels and inside the card. The empty wave — reached for real by failing the recommendations request — keeps its **160px** minimum, its copy is not clipped and its real action is a 44px target inside the card | PASS measured | A real 150% system font scale was not used; the application's own app size was |
| The long current title is shortened, not pushed out | apps/web/src/main.rs, apps/web/style.css | A real upload with an 80-character title is played at 320×700: the mini-player's title is **67px** wide with a `scrollWidth` of **423px**, so it is shortened with the standard ellipsis, the full name is in the element's `title` attribute and in the DOM text, the title stays on one line (**18px** tall) and the document overflow is **0px** | PASS measured | — |
| The composition survives a viewport and a page change | apps/web/style.css, tests/browser/dr06-mobile.spec.mjs | After the viewport moved 390 → 320 → 390 → 360 and the page moved Library → Home → Library: **1** media element, the same `src`, the same `data-source-generation`, the same playing state, a position that is not reset, and the same queue — the same rows, the same count and the same **one** current occurrence read through the queue drawer | PASS measured | — |
| The last focusable row clears the bands | apps/web/style.css, tests/browser/dr06-mobile.spec.mjs | Scrolled to the end of the one central scroller at 320, 360, 390, 760, 761 and 768: the last row's action row — the last focusable control of the list — ends at least **8px** above the scroller's bottom, so it is never behind the player band or the navigation | PASS measured | — |
| The reference widths render in both themes | apps/web/style.css, tests/browser/dr06-mobile.spec.mjs | Rendered and measured at 320×700, 360×800, 390×844, 1284×856, 1440×900 and 1728×1000 in the dark and the light theme: the document overflow is **0px** everywhere, the player band is **120px** on the phone band and **88px** on the desktop band, the navigation is **70px** with **5** destinations on the phone and absent on the desktop. The screenshots are `dr06-library-<width>x<height>-<theme>.png` in `target/checks/dr06-mobile/` | PASS measured | — |
| The composition holds at 80% app size and under reduced motion | apps/web/style.css, tests/browser/dr06-mobile.spec.mjs | At 390×844 the app size reaches **0.8** through four presses of the documented Settings control (not an injected variable), the document overflow stays **0px**, and the five row actions measure **35.19px** rendered = **43.99 CSS px** (`rect / zoom`), so they are still 44px targets. Under `prefers-reduced-motion: reduce` (`matchMedia` true) the row, header, library toolbar, Play control and player bar all report `animation-name: none`, a **0s** animation duration and a **0s** transition duration, and the composition is still the measured one — header **36px**, row **113px**, overflow **0px** | PASS measured | A real 150% system font scale and a real browser zoom were not used; `forced-colors` was not emulated |
| Rust quality and web build | workspace | `cargo fmt --all -- --check` exit 0; `cargo clippy --locked --workspace --all-targets -- -D warnings` exit 0 (a real recompilation of `vela-web`); `cargo test --locked --workspace -- --test-threads=2` (380 passed, 0 failed, 3 ignored); `cargo build --locked -p vela-web --target wasm32-unknown-unknown --release` exit 0; `scripts/build-web.sh` exit 0 (wasm-bindgen 0.2.129) into `dist/` | PASS | The native client's WebKitGTK rendering, its physical audio and the Docker runtime NOT_RUN: no X server on this host |
| Browser suites | tests/browser | The Chromium Playwright suite PASS (**78 passed, 1 skipped — the opt-in live import, 5.1m**) and the Firefox Playwright suite PASS (**78 passed, 1 skipped, 6.4m**), both including the new permanent `tests/browser/dr06-mobile.spec.mjs` (10 journeys) and the updated TR02 pre-list expectation. Firefox's audio assertions run through the host's `vela_ci` PulseAudio null sink | PASS | screen reader, forced-colors, browser zoom 200% and native WebKitGTK NOT_RUN |

The permanent coverage of this package is `tests/browser/dr06-mobile.spec.mjs`:
the first-screen budget, the width matrix, the filter summary's real
visibility, the empty editorial card, the empty wave, the safe-area
arithmetic, the long title, the app-size/reduced-motion scene and the
audio/queue invariant — 10 journeys. It owns its server,
its data and its sessions and it measures the live document rather than
trusting declared CSS values; the one exception is the single
`--vela-bottom-reserve` formula, which it reads from the served `/style.css`
to prove the shell kept one reserve authority. The evidence is in
`target/checks/dr06-mobile/`.

The one documented deviation from the plan is recorded in
[KNOWN_LIMITATIONS](KNOWN_LIMITATIONS.md): the plan's global
`.section-toolbar { margin-bottom: 0 }` is applied to the **library's**
section toolbar, the pre-list the plan measures, and the other pages keep
their section rhythm.

## Second DR-04/DR-05/DR-07 audit round: three point fixes — 2026-10-22

| Check | Paths | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- |
| The recovery status is a permanent live region (D1) | apps/web/src/main.rs, apps/web/style.css | The `role="status"` node of the recovery row is in the DOM before the first failure and only its text changes. Measured in the live document: before any failure the row is present with an empty text, the `is-empty` class, `position: absolute`, a height of **0** and **0** controls, and the feedback stack is **0px** tall; the real failure ("Connection interrupted. Reconnecting…", the transport's own bounded retry budget) arrives in the same node — the node carries a marker that survives the arrival — the row becomes `position: static` and **74px** tall with its one **44px** Retry control, and the stack grows to **74px**. The row is still withheld while the Now Playing sheet or the queue drawer is open: the node stays in the DOM, empty and collapsed, so the message is stated once | PASS measured | Screen reader NOT_RUN |
| The Undo receipt is a permanent live region (D1) | apps/web/src/main.rs, apps/web/style.css | The `role="status"` node of the Undo row is in the DOM before the first Add. Measured: the empty row is present with an empty text, `position: absolute`, a height of **0** and **0** controls; the real receipt "Added “Signal” to Region signals." arrives in the same node (the marker survives), the row becomes `position: static` and **70px** tall with its **2** controls (Undo and Dismiss) and the stack grows to **70px**; the dismissal empties the text, collapses the row again and the stack returns to **0px** — the height it had before the message. The controls sit outside the live region, so their names are never read with the message | PASS measured | Screen reader NOT_RUN |
| The Undo failure is a permanent alert region (D1) | apps/web/src/main.rs, apps/web/style.css | The `role="alert"` node of the Undo failure is in the DOM before the failure: measured, it is present with an empty text, the `is-empty` class, `position: absolute` and a height of **0**. The real failure — the DELETE of the Undo never reaches the server — arrives as a change of its text ("Connection interrupted. Please try again."); the node becomes `position: static` and **19.5px** tall and it carries a marker that survives the arrival, so it is the same node. The dismissal takes the failure with the receipt: the text is empty again and the node is collapsed at **0px**, exactly as it was before the receipt existed. The alert is no longer nested inside the `role="status"` region: the status is the text element of the row and the alert is its sibling | PASS measured | Screen reader NOT_RUN |
| One lookup of the queue drawer's dialog (D2) | apps/web/src/main.rs, apps/web/src/queue_drawer.rs | The drawer's own `drawer()` copy of the `get_element_by_id("queue-drawer-dialog")` lookup is gone. The root's `queue_dialog()` is `pub(crate)` and the drawer module calls it — `crate::queue_dialog()`, the same way the drawer already calls the root's `seconds()` helper. The drawer still opens its native modal dialog in the same deferred effect, so the focus trap, the top layer and the backdrop are unchanged | PASS source | — |
| The Now Playing trigger limitation says what the code does (D3) | docs/KNOWN_LIMITATIONS.md, apps/web/src/main.rs | The limitation now states that the frozen trigger is the control that holds the focus when the sheet opens — the code refuses only the document body and never checks that it is the sheet's own opener — that the previously focused control is frozen instead on a platform where a click does not focus a button, and that the browser's own focus restore is the fallback only for the document-body case. The sheet's own focus return is the native dialog's restore; the application freezes the element only to donate it to the drawer | PASS source | The other platforms were not measured |
| The shell's `container-type` side effect is recorded (U1) | docs/KNOWN_LIMITATIONS.md, apps/web/style.css | `container-type: inline-size` applies **style** and **inline-size** containment; it does **not** apply layout or paint containment, so the shell is **not** the containing block of its `position: fixed` descendants and does **not** establish a stacking context. Measured in Chromium: a `position: fixed` child of a `container-type: inline-size` element is anchored to the **viewport** (right edge = viewport's right edge, not the container's), a negative `z-index` descendant escapes the container and paints below the container's own background, and `getComputedStyle(shell).contain` reads `none`; the same probe with `contain: layout style inline-size` anchors the fixed child to the container and keeps the negative descendant inside it. The feedback stack, the mobile track menu and the native dialogs are therefore fixed layers of the viewport, not of the shell, and the shell's `overflow: hidden` does not clip them. The shell's own box is `100dvh / zoom` tall, so at 100% and 150% the container box and the viewport coincide — the measured, neutral case. The limitation records that `dvh` is the *dynamic* viewport size (it tracks the viewport as a mobile browser's toolbar shows and hides, and it is **not** the largest size in the `vh` family - `lvh` is); the dynamic viewport already accounts for the toolbar, so the visible area is not automatically shorter than the shell, and nothing is claimed about that case | PASS measured | A mobile browser with a dynamic toolbar NOT_RUN |
| Rust quality and web build | workspace | `cargo fmt --all -- --check` exit 0; `cargo clippy --locked --workspace --all-targets -- -D warnings` exit 0; `cargo test --locked --workspace -- --test-threads=2` (380 passed, 0 failed, 3 ignored); `cargo build --locked -p vela-web --target wasm32-unknown-unknown --release` exit 0; `scripts/build-web.sh` exit 0 (wasm-bindgen 0.2.129) into `dist/` | PASS | The native client's WebKitGTK rendering, its physical audio and the Docker runtime NOT_RUN: no X server on this host |
| Browser suites | tests/browser | Chromium Playwright suite PASS (68 passed, 1 skipped — the opt-in live import, 5.1m) and Firefox Playwright suite PASS (68 passed, 1 skipped, 6.3m), both including the new permanent-live-region journey in `tests/browser/dr05-states.spec.mjs`, the reworked `tests/browser/dr07-a11y.spec.mjs` and the two `prototype-transfer` journeys that assert the Undo receipt | PASS | screen reader, forced-colors, browser zoom 200% and native WebKitGTK NOT_RUN |
| The stack in the six viewports, both engines | target/checks/dr05-permanent-regions/ | Measured in the live document with the recovery status and the Undo receipt both present: 1284×856, 1440×900 and 1728×1000 give a **152px** stack (the recovery row 74px, the 8px gap, the Undo row 70px = 152px; the collapsed ordinary receipt is out of the flow at 0px and adds no second gap, because a flex `gap` only separates the items that are in the flow); 390×844 and 360×800 give **227px**; 320×700 gives **202.5px**. The intersection area with the player bar is **0px²** and with the mobile navigation **0px²** at every viewport, the document overflow is **0px** and the count of visible interactive targets with a side below 44px is **0**. The screenshots are `feedback-stack-<engine>-<width>x<height>.png` and the numbers are `regions-<engine>.json` | PASS measured | — |

The permanent coverage of this round is one new journey in
`tests/browser/dr05-states.spec.mjs`: "DR05 the recovery status and the Undo
receipt are permanent live regions". It owns its server, data and sessions,
signs in a real account, adds a real track to a real playlist, breaks a real
stream and breaks a real Undo, and it measures the live document: the
presence of the node, the text of its live region, the `is-empty` class, the
computed `position`, the height of the row, the number of its controls and
the height of the whole stack before the message, after the message and after
the dismissal.

Host note: Firefox needs an audio output to decode a media stream. The first
Firefox run of this round failed every playback test with
`audio.currentTime` stuck at 0 while the same build passed in Chromium. An
isolated probe outside the application reproduced it exactly — Firefox's
media element reported a decode error (code 3) on a plain data-URI WAV while
Chromium played the same file — and the cause was the host's PulseAudio: the
root server with the `vela_ci` null sink had died and left a stale socket.
The null sink was started again exactly as the CI workflow does
(`pulseaudio --start`, `module-null-sink`, `pactl set-default-sink
vela_ci`), the probe then advanced the clock (0 → 1.613s in three seconds)
and the Firefox suite passed in full. The failure was the host's audio
device, not the application.

## DR-04/DR-05/DR-07 audit round: one time format, one closer, one permanent receipt — 2026-10-22

| Check | Paths | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- |
| One time format in the crate (D1) | apps/web/src/queue_drawer.rs | The drawer's own `format_duration()` is gone; the queue rows use `seconds(track.duration_ms as f64 / 1000.0)` and the sleep countdown `seconds(sleep_remaining as f64)`, the root's helper. The output did not change: the removed helper divided the milliseconds by 1000 and printed `m:ss`, and the root's helper truncates the same number of seconds. Measured in the live drawer in Chromium and in Firefox: the two queued rows show the real `m:ss` of the server's real durations (`0:01` for the one-second fixtures) and the countdown keeps `Pauses in m:ss` | PASS measured | — |
| One closing path (D2) | apps/web/src/queue_drawer.rs, apps/web/src/main.rs | The drawer's `closer()` is gone. Escape, the close button and "Open library" call the root's handler, which closes the native dialog before the drawer leaves the DOM, clears the announcement line and restores the focus. Measured in Chromium and in Firefox: after Escape the dialog is gone and the focus is on the connected `Show playback queue` trigger; after the drawer's own close button the same; after "Open library" the focus is on the same trigger and not on the document body | PASS measured | — |
| Every rhythm role is the authority it claims (D3) | apps/web/style.css, docs/DESIGN_SYSTEM.md | All eleven roles are consumed through `var(...)` in the file that declares them, each with the value the literal had: `--vela-space-1` 4 uses, `--vela-space-2` 2, `--vela-space-3` 2, `--vela-space-4` 1, `--vela-space-6` 2, `--vela-space-8` 1, `--vela-radius-control` 4, `--vela-radius-cover` 3, `--vela-radius-panel` 1, `--vela-target-size` 3, `--vela-heading-section` 1. The rendered values did not move: the transport is still five 44px controls and four 4px gaps = 236px, the track grid gap is still 12px, the section heading is still 20px/600, the navigation step is still 44px and the group gap 24px (the permanent `tests/browser/dr04-density.spec.mjs` measures all of them) | PASS measured | — |
| The size map and the volume paragraph agree (D4) | docs/DESIGN_SYSTEM.md | The table and the paragraph now make one statement, and it is the code: 65×44 above 1100px of viewport width, 44×44 from 1100px down to 760px, and the volume is not in the player bar below 760px or below 600px of layout width. Measured in Chromium and in Firefox on a settled layout: 65×44 at 1284×856, 44×44 at 1000×856, and at 900×856 with the app's own 150% size (a 600px layout width) the volume is `display: none` while the queue and the mute are still on screen. The measured values are in `target/checks/dr04-density/dr04-density-size-map.json` | PASS measured | Firefox re-evaluates the shell's container query one layout late when a zoomed root is resized; a reload settles it, and the difference is recorded in KNOWN_LIMITATIONS |
| The covers are what the code says (D3) | docs/DESIGN_SYSTEM.md, apps/web/style.css | The "covers stay at 6px" line is replaced by the measured radii: the library track cover and the player bar's cover are 6px, the Now Playing sheet's cover is 16px (12px below 680px), the album card's cover is 7px and the mobile player bar's cover is 5px. The base cover radius is the `--vela-radius-cover` role | PASS measured | — |
| The root receipt is a permanent live region (D5) | apps/web/src/main.rs, apps/web/style.css | The live region is in the DOM before the first message and only its text changes. Measured in Chromium and in Firefox: the empty receipt is present with an empty text, the `is-empty` class, `position: absolute`, a height of 0 and no dismiss control; the message "Added to your playback queue." arrives in the same node (the node carries a marker that survives the arrival and the dismissal), the receipt becomes `position: static` with a height of 70px and a dismiss control; the dismissal empties the text, collapses the receipt again and the feedback stack is exactly as tall as it was before the message. The dismiss control sits outside the live region | PASS measured | Screen reader NOT_RUN: the announcement itself was not observed, only the DOM facts behind it |
| The drawer's own live region is still permanent | apps/web/src/queue_drawer.rs, tests/browser/dr07-a11y.spec.mjs | The drawer's `role="status"` line is in the DOM with an empty text before the first transition, holds exactly one text after the start, the cancel and the expiry, and the empty line measures 0px of height and 0px of margin. The `m:ss` of the rows and of the countdown is the root's format | PASS measured | Screen reader NOT_RUN |
| The focus returns to the connected trigger after every exit | apps/web/src/main.rs, tests/browser/dr07-a11y.spec.mjs | Measured in Chromium and in Firefox in five scenes: Escape, the drawer's own close button, "Open library", "Now Playing" → "View full queue" → Escape (the focus goes to the `Open now playing` trigger) and a drawer that re-renders under the keyboard. The trigger is connected in every scene and the focus is never on the document body. A control inside another open dialog is not frozen as a trigger, the Now Playing sheet hands its own trigger to the drawer, and the guarded restore does not take the focus away from a control that already owns it | PASS measured | — |
| The player's own now-playing controls close the drawer through the common path | apps/web/src/main.rs | "Open now playing" and "Expand player" call the root's `close_queue()`, so the native dialog never leaves the DOM open and the frozen trigger is never left in its slot. The behaviour of the two buttons is unchanged in the measured scenes | PASS measured | — |
| Rust quality and web build | workspace | `cargo fmt --all -- --check` exit 0; `cargo clippy --locked --workspace --all-targets -- -D warnings` exit 0; `cargo test --locked --workspace -- --test-threads=2` (380 passed, 0 failed, 3 ignored); `cargo build --locked -p vela-web --target wasm32-unknown-unknown --release` exit 0; `scripts/build-web.sh` exit 0 (wasm-bindgen 0.2.129) into `dist/`; `cargo build --locked -p vela-server` exit 0 | PASS | The native client's WebKitGTK rendering, its physical audio and the Docker runtime NOT_RUN: no X server on this host |
| Browser suites | tests/browser | Chromium Playwright suite PASS (67 passed, 1 skipped — the opt-in live import, 5.3m) and Firefox Playwright suite PASS (67 passed, 1 skipped, 6.9m), both including the extended `tests/browser/dr07-a11y.spec.mjs` (the close-button scene, the receipt scenes and the `m:ss` assertions) and `tests/browser/dr04-density.spec.mjs` with its new size-map journey. The evidence of the two engines is written to `target/checks/dr07-a11y-<engine>/` and `target/checks/dr04-density/` | PASS | screen reader, forced-colors and native WebKitGTK NOT_RUN |

The permanent coverage of this round lives in two journeys: the DR-07
journey owns the drawer's exits, the permanent announcement surfaces and the
time format, and the new DR-04 size-map journey owns the cover radius and
the three states of the volume. Both run in Chromium and in Firefox against
the real server with real uploaded tracks.

## DR-04 dense desktop control with real 44px targets — 2026-10-22

| Check | Paths | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- |
| Every visible control is a real 44px target | apps/web/style.css | Measured with `getBoundingClientRect()` on the live document, not from the stylesheet. The Library at 1284×856 with a real current track: **20** visible interactive targets had a side below 44px before the change (12 navigation items 181×41, the player's favorite 32×36, four transport buttons 28×28, three utility buttons 32×32) and **0** after it. The same measurement is 0 at 1440×900, 1728×1000, 768×1024, 390×844 and 320×700. The idle library (nothing playing) and the library with a current track are measured separately; the disabled transport is listed as disabled and the enabled controls are re-measured after the track loads | PASS measured | Screen reader NOT_RUN; the target sizes are visual geometry, not an announcement |
| The Play button and the seek range never overlap (DV-15) | apps/web/style.css | The real rectangles at 1284×856: before, the Play button was `y779..823` and the seek range `y811..855` — a 12px vertical overlap. After, the transport row is `y768..812` and the seek row `y812..856`, the range is `y812..856`, and the pairwise intersection area between the Play button, the four transport buttons, the three utility buttons, the player's favorite and the seek range is **0px²**. The same 0px² holds at 1440×900, 1728×1000 and 768×1024, and at the 80%, 100% and 150% app sizes | PASS measured | — |
| The band keeps its 88px and the hairline costs nothing | apps/web/style.css | The physical `border-top` is gone (`border-top-width: 0px`) and the hairline is drawn by `.player-bar::before` (`height: 1px`, `background: rgb(52, 59, 46)`, `pointer-events: none`). The band still measures 88px at 1284×856, 1440×900, 1728×1000 and 768×1024 (`y768..856`, `y812..900`, `y912..1000`, `y936..1024`) | PASS measured | — |
| The transport is dense, not inflated | apps/web/style.css | Five 44px controls and four 4px gaps = 236px of transport. The transport glyph is 15px and the utility glyph 17px; the Play face is 32px inside its 44px hit box; the volume is 65×44 at ≥1101px and 44×44 at ≤1100px | PASS measured | — |
| The navigation step is the item itself | apps/web/style.css | The measured navigation item is 181×44 with a 44px step, so twelve entries occupy 528px exactly. The measured gaps between the sidebar's own groups are 24px (the brand keeps its own 24px bottom margin, which is why the first gap is 48px). No margin is added on top of any entry. Account and Settings are reached by scrolling the one sidebar region | PASS measured | — |
| The heading keeps its fluid roles | apps/web/style.css | The section heading is 20px/600 (was 21px/600); the general `h2` is 24px/600; the page heading is the fluid `clamp(28px, 2.7vw, 38px)`/650 and measured 34.668px/650 at 1284×856. Manrope, the 14px base, the 12px/550 row title and the 12px secondary text are unchanged. The text-leaf table before → after differs in exactly one entry | PASS measured | — |
| Real pointer and focus actions | apps/web/style.css | A real `Tab` press puts the browser in the keyboard focus modality, then every visible transport control is focused: `document.activeElement` is that control, `:focus-visible` matches and the outline is 2px. A real `mouse.click` on the middle of the seek rail moves the range to the middle of the real track (the 4s fixture: 2.0s of 4.0s) | PASS measured | — |
| The source, the queue and the cursor are untouched | apps/web/src/main.rs | After the geometry change, navigating Home → Search → Settings → Library keeps the same `src` and the same `data-source-generation` of the single root media element; the document still holds exactly one `audio` element | PASS measured | — |
| The app size 80/100/150 keeps the composition | apps/web/style.css | The app's own size control (Settings → Appearance) was used: 4 × `Decrease app size` → 80%, `Reset to 100%`, 10 × `Increase app size` → 150%. At 768×1024, 1284×856, 1440×900 and 1728×1000 the document overflow is 0px, the body overflow is 0px and the control intersections are 0px². The app size is a CSS zoom on the document root, so the shell became a query container and the transport column follows the layout width (856px at a 1284px window and 150%) instead of the visual viewport | PASS measured | The app size does not move the viewport breakpoints for the rest of the application — only the player's composition follows the layout width. **The 44px target is NOT preserved at an 80% app size — see the next row** |
| The 44px target and the app size are two different things | apps/web/style.css, target/checks/dr04-density/dr04-density-matrix.json | The 44px target is a size in the layout coordinate system. The app size is a CSS `zoom` on the document root, so `getBoundingClientRect()` returns the **scaled** rectangle: at an 80% app size a 44×44 layout box measures **35.2×35.2** in the viewport. Measured in the live document over the 768/1284/1440/1728 windows: at an 80% app size **50 of 51** (768×1024) and **52 of 53** (the three wider windows) visible interactive targets measure below 44px, while at 100% the count is **0** and at 150% it is **0**. The browser zoom is a different mechanism: it shrinks the CSS viewport instead of the elements, so at 80/100/150% browser zoom the count is **0** at every window. The 44px guarantee therefore holds for the 100% and 150% app sizes and for every browser zoom, and does **not** hold for the 80% app size — the user has asked for a smaller interface, and the hit boxes shrink with it | PASS measured | The 80% app size is a deliberate user choice, not a defect of this package; the 44px target is a property of the 100% layout, and it is not restored at 80% |
| The browser zoom 80/100/150 keeps the composition | apps/web/style.css | The browser zoom was emulated as a device scale factor of 0.8, 1 and 1.5 over the 768/1284/1440/1728 windows, so the CSS viewport is the window divided by the zoom and the media queries agree with the layout again. At every combination the document overflow is 0px, the body overflow is 0px and the control intersections are 0px². A layout narrower than 760px is the mobile composition and keeps its own 120px band | PASS measured | Browser zoom 200% NOT_RUN |
| The mobile composition did not regress | apps/web/style.css | 320×700 and 390×844: 0 visible targets with a side below 44px, the band is still 120px, the physical 1px border is kept, the transport gap is still 8px and the control intersections are 0px² | PASS measured | The first track is still cut off at 320px; that is DR-06's scope and it was not changed here |
| Rust quality and web build | workspace | `cargo fmt --all -- --check` exit 0; `cargo clippy --locked --workspace --all-targets -- -D warnings` exit 0; `cargo test --locked --workspace -- --test-threads=2` (380 passed, 0 failed, 3 ignored); `cargo build --locked -p vela-web --target wasm32-unknown-unknown --release` exit 0; `scripts/build-web.sh` exit 0 (wasm-bindgen 0.2.129) into `dist/`; `cargo build --locked -p vela-server` exit 0 | PASS | The native client's WebKitGTK rendering, its physical audio and the Docker runtime NOT_RUN: no X server on this host |
| Browser suites | tests/browser | Chromium Playwright suite PASS (68 passed, 1 skipped — the opt-in live import, 4.7m) and Firefox Playwright suite PASS (68 passed, 1 skipped, 6.0m), both including the permanent `tests/browser/dr04-density.spec.mjs` (5 journeys) and the updated TR03 and TR04a in `tests/browser/prototype-transfer.spec.mjs`. The DR04 journey now also reads the queue and the cursor through the drawer's own projection (3 rows, 1 current occurrence, both unchanged across a Home/Search/Settings round trip), and the app-size matrix asserts the measured `zoom` against the requested percent so a broken size switch cannot leave it green. Firefox advances media time because the host's PulseAudio null sink (`vela_ci`) is up; without it the audio assertions fail on an unchanged baseline as well | PASS measured | screen reader, forced-colors and native WebKitGTK NOT_RUN |

Permanent coverage: `tests/browser/dr04-density.spec.mjs` owns its server, data
and sessions and measures the live rectangles: the target table of the idle and
the playing library, the tiled 44px transport and seek rows, the zero
intersection area, the hairline, the glyph sizes against the button sizes,
the 44px navigation step and the 528px total, the 20px section heading, the
fluid page heading, a real focus action on every transport control, a real
pointer click on the seek rail, the source-preservation round trip, the
80/100/150% app-size matrix at four viewports and the 320/390 mobile
regression. The before/after tables and the text-leaf table live in
`target/checks/dr04-density/` (`dr04-density-before.json`,
`dr04-density-after.json`, `dr04-density-desktop.json`,
`dr04-density-matrix.json`) and the screenshots in
`target/checks/dr04-density/shots/`. The capture harness is
`tests/browser/dr04-density.mjs` and the screenshot matrix is
`tests/browser/dr04-shots.mjs`.

### The decisive review round (Codex gpt-6.1-sol xhigh, 2026-10-22)

The decisive reviewer returned `REJECT` on the first pass with four blockers,
all of them documentation claims rather than code, and every one of them was
reproduced by the team lead before it was accepted:

- **`container-type: inline-size` was described as applying
  `contain: layout style inline-size`.** It does not. The property applies
  style and inline-size containment only, so the shell is not the containing
  block of its `position: fixed` descendants and creates no stacking context.
  Verified by measurement, not by reading: a probe page reported the fixed
  child anchored to the viewport (`right: 1400` with an 800px container), a
  negative `z-index` descendant escaping the container, and
  `computedStyle(shell).contain === 'none'`; the same probe with
  `contain: layout style inline-size` anchored the child to the container
  (`right: 800`) and confined the negative descendant. Real application at a
  150% app size: the shell box and the viewport coincide, which is the
  neutral case the limitation now states.
- **The `dvh` definition was corrected but its old conclusion survived.** The
  conclusion "the visible area is shorter than the shell" does not follow:
  the dynamic viewport already accounts for the toolbar. The claim is now
  withdrawn rather than restated.
- **`.sidebar .nav-item` was called 44×44.** The rule sets `width: 100%`; the
  measured rectangle is **181×44**. The step is 44px tall, which is what the
  528px total rests on.
- **The 152px stack was explained with two 8px gaps.** The artifact is
  74 + 8 + 70 = 152; the collapsed absolute row adds no gap because a flex
  `gap` only separates in-flow items.

Two non-blocking remarks were also adopted: the DR04 journey now reads the
queue count, the rows and the current occurrence from the drawer instead of
asserting them in a comment (verified non-vacuous: 3 rows, 1 current row, and
a deliberately altered queue fails the comparison), and the app-size matrix
asserts the measured `zoom` against the requested percent.

The second reviewer (`omp` reviewer, minimax-m3.1-flash, thinking max) did
**not** produce a verdict: it ran 131 minutes and hit its own 7200s timeout
(exit 124) in a read-only pass, after two earlier attempts in previous cycles
died to memory pressure. That is NOT_RUN, not an agreement.

The `::before` hairline and the 44px boxes are geometry, not announcements: a
screen reader was not run, `forced-colors: active` was not emulated, browser
zoom 200% was not exercised, the native WebKitGTK client, its physical audio
and the Docker runtime were not run (no X server on this host).

## DR-01 the glass and motion token layer — 2026-10-22

| Check | Paths | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- |
| Every role is declared once, and light re-points only four colours | apps/web/style.css; tests/browser/dr01-tokens.spec.mjs | The stylesheet is read with `node:fs` and the declarations are counted. The four glass colours are declared **twice** (the canonical dark root at `:89`–`:95` and the light root at `:217`–`:220`); the three blur radii, the two easing curves and the text alias are declared **once**; the eight durations are declared once in the canonical layer and once inside the `prefers-reduced-motion: reduce` block, and nowhere else. The light root is extracted with a brace-balanced scan and contains **no** blur, duration, easing or alias declaration — only the four colours. The existing global override is still present verbatim: `@media(prefers-reduced-motion:reduce){*,*:before,*:after{animation:none!important;transition:none!important;scroll-behavior:auto!important}}` | PASS measured | — |
| The roles resolve in the live document, in both themes | apps/web/style.css; tests/browser/dr01-tokens.spec.mjs | Read back with `getComputedStyle(document.documentElement).getPropertyValue(...).trim()` on a real account with real uploaded tracks. **Dark**: `--vela-glass-bg` `#131513d9`, blur `16px`/`12px`/`12px`, the eight durations `80ms 120ms 140ms 100ms 180ms 1800ms 150ms 140ms`, `--vela-ease-enter: ease-out`, `--vela-ease-state: ease-in-out`, `--vela-glass-text` `#f3f3ed` — the same string as `--vela-text`, and the same painted colour `rgb(243, 243, 237)` through a throwaway probe element. **Light**: `--vela-glass-bg` `#f4f7f3e6`, `--vela-glass-border` `#14271c14`, `--vela-glass-highlight` `#ffffffb3`, `--vela-glass-shadow` `0 18px 50px #21372924`; the alias resolves to `#14271c` = `rgb(20, 39, 28)`. The blur, the eight durations and the easing curves are **byte-for-byte identical** to the dark values. Chromium and Firefox report the same strings | PASS measured | — |
| The declared alpha | apps/web/style.css | Parsed from the two declared hex values: dark `#131513d9` → 0xd9 = 217/255 = **0.8510**, light `#f4f7f3e6` → 0xe6 = 230/255 = **0.9020**; both ≥ the directive's 0.82. This is arithmetic on the declaration, **not** a contrast measurement over artwork: no surface consumes the role, so no legibility over an image was measured and none is claimed | PASS measured | Contrast over artwork NOT_RUN — it is DR-02's scope, and no `backdrop-filter` was ever rendered |
| Reduced motion | apps/web/style.css; tests/browser/dr01-tokens.spec.mjs | `page.emulateMedia({ reducedMotion: 'reduce' })` on the live page: all eight duration roles read **0ms**; the two easing curves are unchanged (they are not durations). The existing global override still silences the components: the computed `transition-duration` of `.track-row` and `.button` is 0 in every part, while **before** the emulation the same two elements report 0.12s and 0.15s — so the zeroing is the override's work, not the absence of a transition | PASS measured | A JS or Web Animations driver was not exercised: the roles have no consumer yet, and DR-03 owns that test |
| The composition did not move | apps/web/style.css; tests/browser/dr01-tokens.spec.mjs | Measured on the live document: body `14px`; at 1284×856 `--vela-sidebar-width` 216px, `--vela-topbar-height` 75px, `--vela-player-height` 88px, `.player-bar` 88px (88±1); at 390×844 `--vela-mobile-player-height` 120px, `--vela-mobile-nav-height` 70px, the mobile player reserve 120px; the horizontal overflow is 0 in every frame. The same numbers were measured on the pre-edit baseline before the change | PASS measured | — |
| The rendered frames are unchanged | apps/web/style.css; target/checks/dr01-tokens | The pre-edit capture is real, not reconstructed: `dist/style.css` was temporarily replaced by `git show origin/main:apps/web/style.css` (SHA-256 `58d4be42…`) and captured, and `pre-edit-baseline.json` records `dr01RolesResolved = 0` plus the served hash. The after-capture (`after-edit.json`) served the shipped CSS (`48a5a011…`) and resolved all 18 roles. The raw frames differ by 116–148 pixels in one narrow strip, and the strip is the row-number column where `…01` and `…03` swapped: the default order is `ORDER BY t.id ASC` over random UUIDs, so that difference is sort order. The controlled A/B removes it — one live document, one account, one dataset, the shipped CSS, then the origin/main CSS injected last in the same document, then the shipped CSS twice as the control (`target/checks/dr01-tokens/ab/diffs.json`): shipped-vs-original differs by **23 pixels in the same bounding box** as the control's shipped-vs-shipped diff, and one pair is **0 pixels**. The stylesheet contributes nothing; the 23 pixels are the repaint of the swap itself | PASS measured | Two independent runs are not byte-reproducible (UUID row order); the claim is that the change renders nothing, not that separate runs are identical |
| Rust quality and web build | workspace | `cargo fmt --all -- --check` exit 0; `cargo clippy --locked --workspace --all-targets -- -D warnings` exit 0; `cargo test --locked --workspace -- --test-threads=2` (380 passed, 0 failed, 3 ignored); `cargo build --locked -p vela-web --target wasm32-unknown-unknown --release` exit 0; `scripts/build-web.sh` exit 0 (wasm-bindgen 0.2.129) into `dist/`; `cmp apps/web/style.css dist/style.css` — identical | PASS | The native client's WebKitGTK rendering, its physical audio and the Docker runtime NOT_RUN: no X server on this host |
| Browser suites | tests/browser | Run by the team lead on the packaged release build, after the package's own attempts exhausted the disk: Chromium **82 passed, 1 skipped, exit 0** (the opt-in live import, 5.1m) and Firefox **82 passed, 1 skipped, exit 0** (6.5m), both including the permanent `tests/browser/dr01-tokens.spec.mjs` (4 journeys: the one-definition count with the compact-declaration check, the roles in both themes, the reduced-motion block and the geometry/frames). The worker's first full Chromium attempt failed on `ENOSPC` while writing a trace and its retry lost the connection mid-run; those runs are recorded as failures, not as this result | PASS measured | screen reader NOT_RUN; `forced-colors: active` NOT_RUN; native WebKitGTK NOT_RUN (no X server) |

Permanent coverage: `tests/browser/dr01-tokens.spec.mjs` owns its server, its
account and its real uploaded tracks. It never launches a browser of its
own — the engine that runs the spec is the engine it measures — and it
writes `target/checks/dr01-tokens/<engine>/tokens.json`,
`…/<engine>/geometry.json` and `…/shots/<engine>/*.png`, so the two engines
never overwrite each other's evidence. The one-definition rule, the role
values, the alpha, the reduced-motion block and the geometry are all
asserted there, not asserted in prose.

DR-01 shipped a vocabulary, not a surface: when the package was written no
`backdrop-filter` was applied and no surface read a `--vela-glass-*` or
`--vela-motion-*` role (a grep over `apps/web/*.css` and
`apps/web/src/**/*.rs` found only the declarations and the spec that reads
them back). DR-02 has since consumed the roles in the player and the topbar;
this section still claims nothing about the glass working on its own.

## DR-03 round-4 reviewer verification — 2026-10-23

This review checks the held Rust slice and removes remaining fixture-clock
dependencies in its browser measurements. It changes only the two DR-03 test
scripts and these evidence documents. The production Rust files, including
`browser_player.rs` and `playback.rs`, match their incoming SHA-256 hashes.
Evidence root: `target/checks/dr03-round4-review/`. The older DR-03 table records
earlier runs; its full-suite and negative-mutation results were not repeated here.

| Executed command | Actual result | Artifact |
| --- | --- | --- |
| `cargo fmt --all -- --check` | exit 0; no output | `fmt.log` |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | exit 0; `Finished dev profile` | `clippy.log` |
| `cargo test --locked --workspace -- --test-threads=2` | exit 0; **392 passed, 0 failed, 3 ignored** across workspace test binaries; web **57 passed**, including **12** cover-model tests | `tests.log` |
| `cargo build --locked -p vela-web --target wasm32-unknown-unknown --release` | exit 0; `Finished release profile` | `wasm-release.log` |
| `scripts/build-web.sh` | exit 0; `Web assets built in dist/` | `build-web.log` |
| `cmp apps/web/style.css dist/style.css` | exit 0; identical, no output | packaged stylesheet compared directly |
| `cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings` | exit 0; `Finished dev profile` | `clippy-wasm.log` |
| `cd tests/browser && npx playwright test dr03-motion.spec.mjs --browser=chromium --output=/root/vela/target/checks/dr03-round4-review/pw-final-2` (then `pw-final-3`) | two consecutive runs before the boundary correction: **1 passed (19.3s)** and **1 passed (17.9s)**; exit 0 each | `chromium-final-2.log`, `chromium-final-3.log`; each run has its own `dr03-motion.json` and screenshots |
| `cd tests/browser && npx playwright test dr03-motion.spec.mjs --browser=firefox --config=/root/vela/target/checks/dr03-round4-review/isolated.config.mjs --output=/root/vela/target/checks/dr03-round4-review/pw-firefox-final-isolated` | before the boundary correction: **1 passed (19.8s)**; exit 0; config omits the unused shared server, retaining the spec's own isolated runtime | `firefox-final-isolated.log`, `firefox-final-isolated/dr03-motion.json`, `isolated.config.mjs` |
| `cd tests/browser && npx playwright test dr03-motion.spec.mjs --browser=chromium --config=/root/vela/target/checks/dr03-round4-review/isolated.config.mjs --output=/root/vela/target/checks/dr03-round4-review/pw-boundary-1` (then `pw-boundary-2`) | two consecutive final-file runs including the boundary correction: **1 passed (17.4s)** and **1 passed (19.7s)**; exit 0 each | `chromium-boundary-1.log`, `chromium-boundary-2.log`, per-run `dr03-motion.json`; `browser-final-source.sha256` |
| `cd tests/browser && npx playwright test dr03-motion.spec.mjs --browser=firefox --config=/root/vela/target/checks/dr03-round4-review/isolated.config.mjs --output=/root/vela/target/checks/dr03-round4-review/pw-firefox-boundary` | final file including the boundary correction: **1 passed (21.8s)**; exit 0 | `firefox-boundary.log`, `firefox-boundary/dr03-motion.json`; same source hash |
| `cd tests/browser && node dr03-render.mjs chromium` | exit 0; **100 PNGs**, seven sizes × dark/light × normal/reduce; all 28 combinations: **1** settled layer, **0** overflow; no-cover: **1** layer, **0** leaving | `render-chromium.log`, `render-chromium/render-chromium.json`, `render-chromium/*.png` |
| `node target/checks/dr03-round4-review/lifecycle-probe.mjs` | exit 0; actual outgoing animation: `aria-hidden=true`, `pointer-events:none`; reduce cleanup with **0** end events; held image released after logout: **0** layers; aborted image: **1** ready fallback layer | `lifecycle-probe.log`, probe source, `lifecycle/lifecycle.json`, `lifecycle/image-error.png` |
| `cd tests/browser && VELA_DR03_EVIDENCE=target/checks/dr03-review/chromium-1 npx playwright test dr03-motion.spec.mjs --browser=chromium --output=/root/vela/target/checks/dr03-review/pw-1` (then `chromium-2`, `chromium-3`, `chromium-final`), and the final file in Firefox with the preserved isolated config | Chromium **1 passed (16.7s)**, **1 passed (17.0s)**, and after the boundary correction **1 passed (16.8s)** and **1 passed (19.4s)**; Firefox on the final file **1 passed (20.3s)**; exit 0 each. The recorded evidence of the last Chromium run: bottom boundary focus `queue-move-up-6` in a seven-row queue (the "down" control of the last row is disabled), top boundary focus `queue-move-down-0`, identical-occurrence swap moves the focus from `queue-move-up-6` to `queue-move-up-5`, crossfade **2** layers / **1** leaving / **1** at the end / **1** audio element, rapid replacement **1** layer / **1** distinct audio element, shared artwork **1** ready layer at opacity **1**, reduce inside a held fade **1** layer / **0** leaving / opacity **1** | `target/checks/dr03-review/chromium-{1,2,3,final}/dr03-motion.json`, `target/checks/dr03-review/firefox-final/dr03-motion.json`, `target/checks/dr03-review/pw-*` |

Cargo used `CARGO_BUILD_JOBS=2`, sequentially. Browser runs used separate
`VELA_DR03_EVIDENCE` directories; the render used `VELA_DR03_RENDER` under the
same evidence root. Viewports: 320×700, 360×800, 390×844, 768×1024, 1284×856,
1440×900 and 1728×1000. Representative mobile menu/queue, desktop Library and
Cyrillic-title screenshots were visually inspected. Journeys 4b/4c retain their
real network holds and named stale-answer/reduce assertions. No new skip or
fixed-time wait was added. Existing named EXP/PLAY/RES/NET/AUTH/NP tests are
unchanged; this is presentation evidence for G05/G23, not another playback engine.

An intermediate run (`chromium-final-1.log`) after a concurrent helper update
failed during `browserContext.close` with `ENOENT` for a trace in the shared
`tests/browser/test-results` directory. No assertion failure appears in its
log. Its status remains **FAIL**; the two subsequent final-file passes use
separate output directories. Earlier snapshot runs also passed in both engines
(`chromium-1.log`, `chromium-2.log`, `firefox.log`); they are not the final-file gate.
A Firefox launch with the default config (`firefox-final.log`) exited 1 before
running any test because another runner occupied port 3217. The final isolated
Firefox run avoids that shared port and passed.

A P2 finding was closed by a concurrent test correction: journey 6b's hardcoded
move 3→4 did not reach the bottom of its seven-row queue. It now computes the
last index and asserts focus on that row's Up control. Both final Chromium
runs and the final Firefox run include the correction; player behavior is unchanged.
Full browser suites, Firefox render matrix, native GUI (no X server), physical
audio, screen reader, GPU-backed device and Docker runtime are **NOT_RUN in this
review**. No live runtime/data upgrade or commit was performed.

### Team-lead acceptance of the same final files — 2026-10-23

The reviewers' own runs above are not the gate the package was accepted on. The
team lead repeated every check itself on the corrected files, in one tree, with
`CARGO_BUILD_JOBS=2` and no competing Cargo process. Source hashes in
`target/checks/dr03-r4-final/source-final.sha256` — `cover_fade.rs`
`ac7a6e0b…`, `main.rs` `c2d3531e…`, `queue_drawer.rs` `19407963…`,
`browser_player.rs` `d950761c…`, `playback.rs` `d7a34d0c…`, spec `efb4b77e…`,
render probe `548095d0…`. Evidence root `target/checks/dr03-r4-final/`.

| Executed command | Actual result | Artifact |
| --- | --- | --- |
| `cargo fmt --all -- --check` | exit 0; no output | — |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | exit 0; `Finished dev profile` | `clippy.log` |
| `cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings` | exit 0 | `clippy-wasm.log` |
| `cargo test --locked --workspace -- --test-threads=2` | exit 0; **392 passed, 0 failed, 3 ignored** | `tests.log` |
| `cargo build --locked -p vela-web --target wasm32-unknown-unknown --release`; `scripts/build-web.sh`; `cmp apps/web/style.css dist/style.css` | exit 0; exit 0; identical, no output | `wasm-release.log`, `build-web.log` |
| `cd tests/browser && npx playwright test --browser=chromium --output=…/pw-full-chromium` | **90 passed, 1 skipped**, exit 0, 6.8m; `dr03-motion.spec.mjs` passed inside the suite at **16.2s** | `full-chromium.log` |
| `cd tests/browser && npx playwright test --browser=firefox --output=…/pw-full-firefox` | **90 passed, 1 skipped**, exit 0, 8.1m; `dr03-motion.spec.mjs` passed inside the suite at **17.3s** | `full-firefox.log` |
| `cd tests/browser && VELA_DR03_RENDER=…/render-chromium node dr03-render.mjs chromium`, then the same for `firefox` | exit 0 each; **100 PNG per engine**; all **28** combinations per engine: `layers 1`, `leaving 0`, `ready 1`, **`audio 1`**, **overflow 0**; empty library `layers 0`; no-cover `layers 1 / ready 0` with 1 icon and 0 images; Cyrillic title 202px wide, overflow 0 | `render-chromium/`, `render-firefox/`, `*.json`, `*.log` |
| `node target/checks/dr03-round4-review/lifecycle-probe.mjs` | exit 0; outgoing animation genuinely started and `paused`, `aria-hidden=true`, `pointer-events:none`; reduce during it → **0** end events; logout with a held picture → **0** layers, 0 in shell; aborted image → 1 ready fallback layer | `lifecycle-probe.log` |
| Sidebar reading from the 1284×856 screenshot, checked against the DOM | the 33px sidebar overflow does **not** clip a control: last utility item rect `683…727`, sidebar ends 768, player band starts 768, `elementFromPoint` on its centre hits that `nav-item` | `sidebar-probe/` |

Two screenshots were inspected by eye in addition to being measured: the
320×700 dark row-menu sheet over the no-cover library and the 1284×856 light
library with the long Cyrillic title. The hover paint on "Add to queue" in the
sheet is `:hover` (`style.css:1600`), where the pointer is left after the
trigger click, not a stored state.

Still **NOT_RUN**, honestly: screen reader, forced-colors, browser zoom 200%,
app-scale 150% for this slice, GPU-backed device / SwiftShader stand, native
WebKitGTK (no X server), physical audio output, Docker runtime, and a real
mobile browser with a dynamic toolbar. The earlier negative mutation of
`accept_ready` was **not** repeated by the team lead in this cycle; its recorded
run stands and the guard is unchanged (`ac7a6e0b…`).

## DR-03 the short answer that never delays the command — 2026-10-23

| Check | Paths | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- |
| Every changed surface answers in its own role, with a transform or an opacity and nothing else | apps/web/style.css; apps/web/now-playing.css; apps/web/queue-drawer.css; apps/web/home-editorial.css; tests/browser/dr03-motion.spec.mjs | Read with `getComputedStyle` on the live document. `.button` and `.icon-button`: `transition-property: "transform, opacity"`, `transition-duration: 0.08s, 0.08s` (the instant role). `.track-row`: `"opacity"`, 0.08s. `.catalog-card .catalog-cover` and `.home-album-card .catalog-cover img`: `"transform"`, 0.12s (the quick role). `.track-menu-panel`: `transition-property "transform, opacity"`, `animation-name "vela-menu-fade"`, `animation-duration 0.12s`; its content: `"vela-menu-enter"`, 0.12s. `.now-playing-dialog[open]`: `"np-enter"`, 0.14s (the overlay role). `#queue-drawer-dialog .queue-panel`: `"vela-drawer-enter"`, 0.18s (the panel role). `.player-eq i`: `height 10px`, `transform-origin bottom`, `animation-name "vela-player-eq"`, 0.8s. **No `height`, `width`, `top`, `left`, `background-position`, `background`, `color`, `box-shadow` or `filter` appears in any of the new transition properties or keyframes**; the equalizer's keyframe animates `scaleY` only and the placeholder's keyframe animates `opacity` only. The pressed control answers with `matrix(0.97, 0, 0, 0.97, 0, 0)` | PASS measured | — |
| The answer is painted in the nearest frame, and the command never waits for it | apps/web/src/cover_fade.rs; apps/web/src/main.rs; tests/browser/dr03-motion.spec.mjs | The row menu: `performance.now()` before the click, then the first animation frame that carries a visible opacity — **34.3ms** in Chromium, with the entrance at `currentTime 16.6ms` and the opacity at 0.22. The Play/Pause command with the menu open: the label is "Play" again **18.3ms** after the click, in the same frame, while the menu's entrance is still running. The number is measured from the event, not read from a CSS value | PASS measured | A GPU-backed stand and a real device are NOT_RUN (see KNOWN_LIMITATIONS) |
| The decoration touches nothing but the picture | apps/web/src/cover_fade.rs; tests/browser/dr03-motion.spec.mjs | With the row menu open and the track playing, the audio element is the **same node** (a marker attribute survives), its `src` is unchanged and its `data-source-generation` is unchanged (**3** before and after the decoration). The Play/Pause command keeps the same node and the same `src`; the generation moves only because the engine's own pause invalidates the pending source, which is the engine's contract and not the decoration's | PASS measured | — |
| The exit of an overlay is 0ms | apps/web/src/now_playing.rs; apps/web/src/main.rs; tests/browser/dr03-motion.spec.mjs | A `cancel` listener added after the app's own handler reads `dialog.open` in the tick of the event: **false** — the sheet has already left the top layer. 200ms later the sheet is still closed (the callback does not open it again) and the focus is back on the connected trigger ("Open now playing") | PASS measured | — |
| The artwork crossfade keeps two layers, one request and one audio element | apps/web/src/cover_fade.rs; tests/browser/dr03-motion.spec.mjs | Two real covers: the tracked `crates/media/tests/fixtures/bright-cover.flac` and a second real FLAC whose picture block carries the tracked `cover.png`, so two tracks really have two different artworks. On the change: **2 layers** (one of them leaving), **1** at the end, **1** `<audio>` element, and the resource timeline grows by exactly **one** entry — the request for the new cover. The outgoing picture is not fetched again. Three changes in a row: never more than **2** layers, **1** at the end, **1** audio element and **1** distinct audio element over 90 frames (the minimum count is 1). A track without a cover: **0** leaving layers and the placeholder is there | PASS measured | — |
| A moved queue row changes the order at once, settles and keeps the focus | apps/web/src/main.rs; apps/web/src/queue_drawer.rs; apps/web/queue-drawer.css; tests/browser/dr03-motion.spec.mjs | A real queue with five occurrences, three of them the same track. "Move track up" on the third row: the DOM order changed in the click's own tick, the destination row carries `queue-row-settle` with `--queue-settle-from: -4px`, the class is gone after the effect, the focus is on **"Move track up"** of `queue-move-up-1` — the moved row — the `src` and the `data-source-generation` are unchanged, one `<audio>` element, and the `.queue-row.current` mark still names the same occurrence (it follows the swap, because the engine remaps the cursor with the order). The three occurrences of the same track are still three rows. A second move right after the first leaves **one** settled row | PASS measured | — |
| Reduced motion is the same surfaces without a movement | apps/web/style.css; apps/web/now-playing.css; apps/web/queue-drawer.css; tests/browser/dr03-motion.spec.mjs | `emulateMedia({reducedMotion:'reduce'})`: on `.button`, `.icon-button`, `.catalog-card .catalog-cover`, `.track-menu-panel`, `.now-playing-dialog[open]`, `#queue-drawer-dialog .queue-panel`, `.player-eq i` and `.cover-fade-layer` — the `animation-name` is `none`, the animation and transition durations are **0**, there is **no running animation** (`getAnimations()` is empty) and the `::before`/`::after` pseudo-elements are `none` too. The equalizer keeps its own height (`scaleY(.4/.7/.5/.9)`, not the squashed playing base). Play, Escape and the drawer still work | PASS measured | A screen reader, forced colours and the native client are NOT_RUN |
| Rust quality and web build | workspace | `cargo fmt --all -- --check` exit 0; `cargo clippy --locked --workspace --all-targets -- -D warnings` exit 0; `cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings` exit 0; `cargo test -p vela-web` **57 passed, 0 failed** (12 of them the new cover-fade model); `cargo build --locked -p vela-web --target wasm32-unknown-unknown` exit 0; `scripts/build-web.sh` exit 0 (wasm-bindgen 0.2.129) into `dist/` | PASS | The native client's WebKitGTK rendering, its physical audio and the Docker runtime are NOT_RUN: no X server on this host |
| Browser suites | tests/browser | The DR-03 spec owns its server, its account and its real uploaded originals: **eleven** journeys (round 3), and the engine that runs the spec is the engine it measures. Chromium **1 passed**, Firefox **1 passed**. Team-lead Chromium runs of the repaired spec: **3/3 green** (19.1 s, 19.0 s, 20.0 s). The whole suite, re-run by the team lead on the round-3 tree: Chromium **90 passed, 1 skipped**, exit 0 (6.7m); Firefox **90 passed, 1 skipped**, exit 0 | PASS measured | A screen reader, a real GPU-backed browser and the native client are NOT_RUN |
| Journey 4b was blocked by the fixture clock, not by the crossfade guard | tests/browser/dr03-motion.spec.mjs; team-lead timeline probe | A `MutationObserver` on `.cover-fade` plus a 100 ms tick recorded every layer class, its `data-generation`, and the audio element of the held context. Unfixed tree: `d=3`, `t=0.00 … 2.96`, and at `t=4903ms` the root advanced to the next track while its picture was still held — a third identity was selected and the wait for the released outgoing picture could never settle (**2 failures in 3 Chromium runs**). Fixed tree (the held journeys pause with `#player-toggle` after the row click): **3/3 green** in Chromium | PASS measured | The probe measures this host's headless Chromium; a real device and a GPU stand are NOT_RUN |
| The negative control fires on the repaired spec | apps/web/src/cover_fade.rs (temporary mutation, reverted) | Dropping the `generation`/`key` guard in `accept_ready` so any answer is accepted as the arrival of the current layer: journey 4b fails in Chromium at `the leaving layer did not start to leave on somebody else answer` (expected **0**, received **1**), 15.1 s. The file was restored and hashes to `ac7a6e0b…`, identical to before the mutation | PASS measured | — |
| The render matrix is real, not a green compilation | tests/browser/dr03-render.mjs; target/checks/dr03-motion/render-chromium | Seven viewports (320×700, 360×800, 390×844, 768×1024, 1284×856, 1440×900, 1728×1000) × dark/light × normal/reduced in Chromium, each state reached by a real command (row click, menu trigger, queue trigger) and photographed after the effect settled: crossfade layers **1/1/1/1/1/1/1** in all four configurations, horizontal overflow **0** at every viewport, a track without a cover **1** layer and **0** leaving. Every file name carries the engine, the theme, the viewport and the motion preference. One viewport in the first run read **2** layers / **1** leaving because the 3-second fixture hit natural EOF mid-matrix; the probe now pauses the transport and waits for a settled surface before it reads, and the rerun is clean | PASS measured | The Firefox matrix of the current fixture is recorded separately below |
| The queue cursor follows the move — the fixture clock was the flake | tests/browser/music.spec.mjs; team-lead probe `eof-probe.mjs` | Every `play`/`ended`/`seeked` of `#vela-audio` and every change of `.queue-row.current` recorded around the two move clicks. Chromium and Firefox: the rows read `C..` before the move and `.C.` after it in the click's own tick — the cursor is never dropped. The two assertions that used to fail (`music.spec.mjs:113`, `:117`) fail because the tracked **1.0 s** `signal.flac` reaches **natural EOF** between the edits and the root advances the cursor by design. The same test fails on `origin/main` `d2bcbb0` in an isolated worktree (4 runs: 3 pass / 1 fail at `:117`; this branch: 2 pass / 2 fail), and `onended`/`advance` are unchanged from `origin/main` — **pre-existing, not a DR-03 regression**. The test now pauses with `#player-toggle` before the queue edits: **6/6** Firefox runs pass afterwards (before: 2 failures in 3 runs) | PASS measured | A screen reader and the native client are NOT_RUN |

Permanent coverage: `tests/browser/dr03-motion.spec.mjs` owns its server, its
account and its real uploaded originals, and the engine that runs the spec is
the engine it measures. The artifacts are
`target/checks/dr03-motion/<engine>/dr03-motion.json` and the screenshots
`now-playing-1284x856.png`, `queue-drawer-1284x856.png`,
`albums-1284x856.png`, `home-1284x856.png` and
`player-no-cover-1284x856.png`, so the two engines never overwrite each
other's evidence.

The durations, the event-to-paint number, the layer counts, the request
count, the focus and the reduced-motion state are all asserted in that spec,
not asserted in prose.

## DR-02 the underlap and the two glass bands — 2026-10-22

| Check | Paths | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- |
| The catalog runs under the two bands, and the first frame does not jump | apps/web/style.css; tests/browser/dr02-glass.spec.mjs | Measured on the live document at 1284×856, 1440×900, 1728×1000, 390×844, 360×800 and 320×780. The scroller now spans the whole viewport — `0…856` at 1284 and `0…844` at 390 — while the topbar keeps `0…75` (`0…63`) and the player `768…856` (`654…774`); the content column's bottom is the viewport bottom at every width. The content wrapper starts exactly at the topbar's bottom edge (75 / 63) and its first heading keeps its y: `topbar.bottom + --vela-content-top`. The one bottom reserve is the height of the bands the scroller floats over: 88 on the desktop, 190 on the mobile composition, and `.content-inner`'s padding is `reserve + 24` (112px measured) and `reserve + 20` (210px measured). One central scroller in every frame (`bodyScrollers` = `['main-scroll']`). At a scroll position computed from the first real row, a **real** track row intersects the topbar's rectangle and another intersects the player's, with the row's title read out of the DOM; at the end of the list the last row ends entirely above the player | PASS measured | — |
| Exactly two filters, at most three with one overlay open | apps/web/style.css; tests/browser/dr02-glass.spec.mjs | Every element and every `::before`/`::after`/`::backdrop` was walked in the live document and counted with `getComputedStyle(...).backdropFilter`, keeping only the visible ones (a pseudo counts only when it is generated, a `::backdrop` only while its dialog is open). Library: **`header.topbar` `blur(12px)` and `div.player-bar` `blur(16px)` — two**. Queue drawer open: those two plus `dialog#queue-drawer-dialog::backdrop` `blur(8px)` — **three**. Now Playing open: those two plus `dialog#now-playing-dialog::backdrop` `blur(14px)` — **three**; the two overlays never stack. No `.track-row`, grid, card or panel carries a filter | PASS measured | — |
| The labels on the glass are readable over the real cover and the real backdrop | apps/web/style.css; tests/browser/dr02-glass.spec.mjs; crates/media/tests/fixtures/bright-cover.flac | The bright-cover fixture is a **tracked repository fixture** (`crates/media/tests/fixtures/bright-cover.flac`, a real 3-second FLAC with an embedded 800×800 JPEG), so the journey cannot be skipped: a missing file now **fails** the test (`readFile` rethrows) instead of `test.skip`. Its artwork colour was read from the image itself: **rgb(246, 243, 224)**. The composited contrast (the label's own translucent stack over the backdrop, in overlay order) over that cover: the band captions **10.89:1** in the dark theme (10.895 measured) and **14.50:1** in the light theme; the transport and favourite glyphs **4.61:1** and **7.06:1**; the focus ring **10.89:1** and **14.50:1**. Over the placeholder of a track without artwork: 16.90 / 11.62 (text), 7.16 / 5.66 (glyphs). Over the page surface: 16.48 / 14.55, 6.98 / 7.08. The in-situ measurement — the bands hidden, the viewport photographed, the real painted pixels under each label sampled and composited — is never worse: the worst text pixel is **10.89:1** and the worst glyph **5.58:1** (Chromium) / **5.51:1** (Firefox). Thresholds: 4.5:1 for normal text, 3:1 for glyphs. 270 analytical cases per engine | PASS measured | A screen reader and a real device with a GPU are NOT_RUN |
| The effective rectangle keeps the last 44px target reachable | apps/web/style.css; tests/browser/dr02-glass.spec.mjs | The effective rectangle is `scroller.top + topbar (+ safe top)` to `scroller.bottom - reserve`: **75…768** at 1284×856, **63…654** at 390×844, **63…590** at 320×780 (the mobile reserve is 190, the mobile navigation is below the player). The last focusable target of the list — the row's "More actions" button, **44×44** — was focused and the browser scrolled it into view: it landed at 510…554 (1284), 394…438 (390) and 330.91…374.91 (320), i.e. whole and with more than 8px of clearance from both edges of the effective rectangle | PASS measured | — |
| The modes are static and opaque where they must be | apps/web/style.css; tests/browser/dr02-glass.spec.mjs | `emulateMedia({reducedMotion:'reduce'})`: the computed `transition-duration` and `animation-duration` of `.player-bar`, `.topbar` and their `::before`/`::after` pseudo-elements are **0s** everywhere, and the band is still glass (a static surface). `emulateMedia({forcedColors:'active'})`: both bands report an **opaque** background (`rgb(255, 255, 255)` under the forced palette), `backdrop-filter: none` and no shadow. `emulateMedia({media:'print'})`: `backdrop-filter: none` and `box-shadow: none` on both bands. The stylesheet also returns the opaque roles under `prefers-reduced-transparency: reduce` | PASS measured | `prefers-reduced-transparency: reduce` could not be emulated on this host (Playwright has no switch for it); the rule is present in the stylesheet and was read, not exercised. A forced-colours screen reader is NOT_RUN |
| Scrolling, the theme and the overlay keep the source and the position | apps/web/style.css; tests/browser/dr02-glass.spec.mjs | One real 30-second original, played from the Library. After a 320px scroll, after the theme switch to light and after the queue drawer was opened and closed, the `#vela-audio` element is the same node (a `data-` marker survives), there is still **one** `<audio>` element, the `src`, the resolved `currentSrc` and the `data-source-generation` are unchanged, and the position never rewinds | PASS measured | — |
| The shells' popups are not trapped under the bands | apps/web/style.css; tests/browser/music.spec.mjs | The first draft gave `.main-scroll` the plan's `z-index: 0`, which opens a stacking context. The music suite caught the consequence: with that context the "Add to playlist…" item of an open row menu is painted below the DR-05 receipt (the receipt intercepted the click, 108 retries). The A/B is real: the same test **fails** with the `z-index: 0` variant, **passes** with the shipped stylesheet (the scroller stays `position: relative` with `z-index: auto`, so it is still painted before the two bands and their glass still samples the content under them) and **passes** with the pre-package stylesheet. The row menu and the mobile action sheet are descendants of the scroller, so a context there would trap both | PASS measured | — |
| The frame budget | apps/web/style.css; tests/browser/dr02-frame-budget.mjs; docs/PERFORMANCE.md | Measured in a browser process the test runner does not instrument, against the **opaque** roles (`var(--vela-player-bg)` / `var(--vela-bg)`, `backdrop-filter: none`, no shadow), and the gate asserts the two runs paint differently: glass `rgba(19, 21, 19, 0.85)` + `blur(16px)`/`blur(12px)` vs fallback `rgb(16, 19, 15)`/`rgb(19, 21, 19)` + `filter: none`. Chromium, 1284×856, software rasterisation (SwiftShader): with the glass **598 frames in ten seconds, 3 frames over 33 ms (0.50 %), the median 16.7 ms, the longest 33.4 ms**; with the opaque fallback **602 frames, 0 over 33 ms, the same median, the longest 16.8 ms** → **+0.50 points**. Firefox: **602 frames, 0 over 33 ms** glass, **603 frames, 0 over 33 ms** fallback, the median 17.06 ms in both → **0.00 points**. Both are inside the 5-point budget the plan sets | PASS measured | A GPU-backed stand is **NOT_RUN**: this host has no GPU-backed browser (the renderer is SwiftShader in every launch mode). The budget is a local number, not a promise about any other device |
| Rust quality and web build | workspace | `cargo fmt --all -- --check` exit 0; `cargo clippy --locked --workspace --all-targets -- -D warnings` exit 0; `cargo test --locked --workspace -- --test-threads=2` **380 passed, 0 failed, 3 ignored**; `cargo build --locked -p vela-web --target wasm32-unknown-unknown --release` exit 0; `cargo build --locked -p vela-server` exit 0; `scripts/build-web.sh` exit 0 (wasm-bindgen 0.2.129) into `dist/`; `cmp apps/web/style.css dist/style.css` — identical. No Rust source changed in this package: the change is `apps/web/style.css` and the browser suite | PASS | The native client's WebKitGTK rendering, its physical audio and the Docker runtime are NOT_RUN: no X server on this host |
| Browser suites | tests/browser | The **whole** suite, re-run by the team lead after the review round: Chromium **89 passed, 1 skipped** (the opt-in live import), exit 0, 6.2m; Firefox **89 passed, 1 skipped**, exit 0, 7.6m. Both include the seven new DR-02 journeys and the updated `prototype-transfer.spec.mjs` (TR01/TR02/TR03/TR09), `dr06-mobile.spec.mjs` and `design.spec.mjs`. The DR-02 journeys alone: Chromium 7/7 (57.5s) and Firefox 7/7 (1.1m). The Playwright output went to `/dev/shm/pw-out-full-*` and `/dev/shm/pw-out-dr02-*`; the evidence is in `target/checks/dr02-glass/<engine>/` and `/dev/shm/vela-dr02-lead/<engine>/` | PASS | A screen reader, a real GPU-backed browser and the native client are NOT_RUN |

Permanent coverage: `tests/browser/dr02-glass.spec.mjs` owns its server, its
account and its real uploaded tracks, and it never launches a browser of its
own for the six journeys that use the page fixture — the engine that runs
the spec is the engine it measures. The seventh journey, the frame budget,
runs its measurement in a separate process on purpose: the Playwright test
runner attaches to every browser it starts, and that harness cost is worth
15.7–24.4 points of frames over 33 ms for the same page. The artifacts are
`target/checks/dr02-glass/<engine>/geometry.json`, `…/contrast.json`,
`…/filters.json`, `…/focus.json`, `…/modes.json`, `…/preservation.json`,
`…/performance.json` and `…/shots/<engine>/*.png`, so the two engines never
overwrite each other's evidence.

The underlap, the two filters, the composited contrast, the effective
rectangle, the modes, the preserved source and the frame budget are all
asserted in that spec, not asserted in prose.

## DR-07 keyboard readable queue drawer — 2026-10-21, second review 2026-10-22

| Check | Paths | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- |
| Drawer focus ring, light and dark | apps/web/queue-drawer.css | Real Tab press first, then the focused control, over the live drawer at 1284×856. Every measured control (close, queue track, move up, remove, volume rail, position rail) reports `:focus-visible` true, `outline: 2px solid`, `outline-offset: 3px`. Light `rgb(8, 117, 76)` on the panel `rgb(255, 255, 255)` = 5.74:1; dark `rgb(211, 233, 165)` on `rgb(28, 32, 29)` = 12.54:1. The ring stays inside the dialog rect on both sides. The design audit had measured the old hardcoded `#d3e9a5` on white at 1.315:1. Chromium and Firefox report the same numbers | PASS measured | forced-colors rendering NOT_RUN (the rule is present, the mode was not emulated) |
| Drawer keyboard order | apps/web/src/queue_drawer.rs | One forward walk with real Tab presses over the open drawer, recorded control by control: close, resume, previous, next, shuffle, repeat, volume rail, position rail, queue track 1, its down action, queue track 2, move up, remove, clear upcoming, sleep timer. The volume rail, the position rail, a queue row, the move action and the remove action are all in that order | PASS measured | The engines' own cycle differs: Chromium wraps inside the native modal dialog, Firefox stops at the last control |
| Rail geometry and visible names | apps/web/queue-drawer.css; apps/web/src/queue_drawer.rs | Live rects: volume 325×44 and position 325×44 in a 325px column at 1284×856, 1440×900 and 1728×1080; 299×44 in a 299px column at 360×800; 259×44 in a 259px column at 320×700. `display:block`, `min-width:0`. Both labels are visible, bound by `for` to the existing ids, 12px/500 | PASS measured | — |
| Announced rail values | apps/web/src/queue_drawer.rs | Real keypresses on the native rails. Volume: Home → `0%`, ArrowRight → `1%`, End → `100%`, Home → `0%`. Position: Home → `0:00 of 0:01` (the real one-second fixture), ArrowRight moves the rail to `0.1`, End → `0:01 of 0:01`, Home → `0:00 of 0:01`. The empty queue announces `Duration unknown` on a disabled rail whose technical maximum is `1` | PASS measured | A track longer than one second was not available: the only real fixtures are one second long, so the `m:ss` resolution cannot show a sub-second position |
| Focus return from the drawer | apps/web/src/queue_drawer.rs; apps/web/src/main.rs | After a real `Escape` the dialog is gone (`#queue-drawer-dialog` count 0) and `document.activeElement` is the connected `Show playback queue` button, in the empty queue and in both themes, in Chromium and in Firefox. The trigger is frozen by the root (`Surfaces::capture_queue_trigger`) and restored by the root's guarded path (`Surfaces::restore_queue_focus`); the drawer keeps no copy of that convention. The drawer and the row menu hold two independent frozen triggers, so an open row menu cannot take the drawer's trigger away | PASS measured | — |
| The `Open library` exit hands the focus back | apps/web/src/queue_drawer.rs; apps/web/src/main.rs | From the Home page with an empty queue, the real `Open library` button goes through the drawer's closer: the dialog leaves the top layer before the drawer leaves the DOM (`#queue-drawer-dialog` count 0), the library page loads (`.track-row`), and `document.activeElement` is the connected `Show playback queue` trigger — not the document body. Chromium and Firefox | PASS measured | — |
| The focus return after a re-render inside the drawer | apps/web/src/queue_drawer.rs; apps/web/src/main.rs | The countdown ticks every second and the drawer is re-rendered while the focus is inside it: after a real tick, a real `Escape` closes the drawer and `document.activeElement` is the connected `Show playback queue` trigger. Chromium and Firefox | PASS measured | — |
| The sleep timer's one announcement | apps/web/src/main.rs; apps/web/src/queue_drawer.rs | The real 5-minute timer in the live drawer: the drawer holds exactly one `role="status"` line and it is empty before a transition. Choosing `5 minutes` writes `Sleep timer set for 5 minutes.` into it, cancelling writes `Sleep timer canceled.`, and the expiry writes `Sleep timer reached zero and paused playback.` (the page clock was advanced by 301 s, so the real deadline was reached and the real playback pause ran). A tick does not change the line, and the tick does not add a second live region. The background behind the open modal is out of reach — the queue trigger behind it cannot take the focus, so the root's receipt is not in the accessible surface and the message is never in two places at once. The line is the drawer's own quiet text: 12px, `rgb(64, 89, 74)` on the light panel `rgb(255, 255, 255)` = 7.65:1, the same semantic muted role as the countdown's text | PASS measured | A screen reader was not run: the single live region, its role and its text are DOM-level guarantees, not a measured announcement |
| Sleep countdown is not announced every tick | apps/web/src/queue_drawer.rs | The real countdown of a real 5-minute timer in the live drawer: `role="timer"`, `aria-live="off"`, the accessible name `Pauses in 5:00` and the same string as visible text. It really ticks — the recorded name counts down from `Pauses in 5:00` (`Pauses in 4:58` in the Chromium run, `Pauses in 4:59` in the Firefox run) — and no ancestor of the countdown is a live region (no `role` in status/alert/log/marquee, no `aria-live` other than `off`): the collected `liveAbove` list is empty in both engines. Its text is the drawer's own quiet text: in the light theme `rgb(64, 89, 74)` on the panel `rgb(255, 255, 255)` = 7.65:1 in both engines. The start, the cancel and the expiry are announced once each by the root on the drawer's own status line — see the previous row | PASS measured | A screen reader was not run: the role, the politeness and the text are DOM-level guarantees, not a measured announcement |
| Now Playing → View full queue → Escape | apps/web/src/queue_drawer.rs; apps/web/src/main.rs | The real sheet opens the real drawer: after `View full queue` the `#now-playing-dialog` is gone and `#queue-drawer-dialog` is open; a real `Escape` closes the drawer and `document.activeElement` is the connected `Open now playing` trigger, in Chromium and in Firefox | PASS measured | — |
| The announcement line is a permanent live region | apps/web/src/queue_drawer.rs; apps/web/queue-drawer.css | The live region is in the DOM before the first transition: the open drawer holds exactly one `role="status"` node, its text is empty and it carries one text node and no element children. Choosing `5 minutes` writes `Sleep timer set for 5 minutes.` into that same node, which then carries exactly one text; the cancel and the expiry reuse the same node, and a tick does not touch it. The empty line measures 0px of height and 0px of margin, and the sleep row and the line share one wrapper whose height equals the row's own height (44px), so the empty line adds no space — at 1284×856, 1440×900, 1728×1080, 390×844, 360×800 and 320×700 | PASS measured | A screen reader was not run: the permanent region, its role and its text are DOM-level guarantees, not a measured announcement |
| The drawer freezes its own trigger | apps/web/src/main.rs | Four scenes, both engines. (1) The row menu is open, `Show playback queue` in the player bar is clicked and a real `Escape` follows: the drawer is gone, `document.activeElement` is the connected `Show playback queue` button and not the row's `More actions for …`; the menu is still open behind the drawer, as the anchored desktop panel it has always been. (2) The ordinary exit: the connected `Show playback queue` button. (3) `Now Playing` → `View full queue` → `Escape`: the connected `Open now playing` trigger. (4) Closing the row menu, and closing the playlist picker it opened: the connected `More actions for Signal` button. The drawer keeps its own frozen trigger slot; the row menu and the picker keep theirs | PASS measured | — |
| The two row roles are semantic, not a hardcoded pistachio | apps/web/queue-drawer.css; apps/web/style.css | The current row really paints the role: its computed background is `rgba(8, 117, 76, 0.05)` in the light theme and `rgba(211, 233, 165, 0.02)` in the dark theme, which are the values of `--vela-track-current` in each theme. The role values over the panel they sit on: light `#08754c0d` = 1.07:1 and `#08754c33` = 1.34:1 on the white panel, dark `#d3e9a505` = 1.05:1 and `#d3e9a533` = 1.74:1 on the dark panel. The hardcoded pistachio they replaced read 1.005:1 and 1.055:1 on the white panel | PASS measured | The two marks are decorative tints, not text and not the focus ring, and they stay below 3:1 in the light theme; the redesign of that marker is not in this package's scope |
| Reflow of the drawer | apps/web/queue-drawer.css | 1284×856, 1440×900, 1728×1080, 390×844, 360×800 and 320×700: the rails keep 44px and their column width (±1px), both labels stay visible, the document overflow is 0px at every width. The running sleep countdown stays inside the drawer at every width (Chromium 1147→1222 in a 914→1284 panel at 1284×856, 39→113 in a 16→320 panel at 320×700; Firefox 1145→1219 and 39→113), and neither the drawer, nor the sleep row, nor the document scrolls sideways (0px everywhere) | PASS measured | 200% browser zoom and a 150% app scale NOT_RUN |
| Rust quality and web build | workspace | `cargo fmt --all -- --check` exit 0; `cargo clippy --locked --workspace --all-targets -- -D warnings` exit 0; `cargo test --locked --workspace -- --test-threads=2` (380 passed, 0 failed, 3 ignored); `cargo build --locked -p vela-web --target wasm32-unknown-unknown --release` exit 0; `scripts/build-web.sh` exit 0 (wasm-bindgen 0.2.129) into `dist/` | PASS | The native client's WebKitGTK rendering, its physical audio and the Docker runtime NOT_RUN: no X server on this host |
| Browser suites | tests/browser | Chromium Playwright suite PASS (62 passed, 1 skipped — the opt-in live import, 4.7m) and Firefox Playwright suite PASS (62 passed, 1 skipped, 5.8m), both including the permanent `tests/browser/dr07-a11y.spec.mjs` (DR07) with the permanent live region, the four focus scenes and the two row roles | PASS | screen reader NOT_RUN |

Permanent coverage: `tests/browser/dr07-a11y.spec.mjs` owns its server, data and
sessions and measures the live focus ring (colour, width, offset, contrast
against the painted surface) in both themes, the drawer's real Tab order, the
live rectangles of both rails against their real column, the presence and
binding of both visible labels, the live `aria-valuetext` after real Home,
Arrow and End presses, the `Duration unknown` branch of an empty queue, the
focus return after a real Escape, the `Open library` exit and its focus
return, the focus return after a re-render inside the drawer, the sleep
countdown's live-region role, the sleep timer's one announcement line (the
start, the cancel and the expiry, with the page clock advanced to the real
deadline) and the inert background behind the open modal, the Now Playing →
`View full queue` → Escape path and the drawer geometry at six viewports. It
also measures the announcement line as a permanent live region (present and
empty before the first transition, exactly one text after it, and no space
taken while it is empty), the four focus scenes of the drawer's own trigger
(the ordinary exit, the row menu open behind the drawer, the Now Playing
sheet and the row menu and picker that keep theirs) and the two semantic
row roles with the contrast they really have over the panel.
Its screenshots and `dr07-evidence.json` land in
`target/checks/dr07-a11y-<engine>/` — `target/checks/dr07-a11y-chromium/` and
`target/checks/dr07-a11y-firefox/` — so an ordinary run of the two engines
never overwrites the other engine's evidence. `VELA_DR07_EVIDENCE` overrides
the whole path when a run needs a directory of its own; it is documented in
`tests/browser/README.md`.

Host note: Firefox needs an audio output to decode a media stream. This host
had no PulseAudio server, and without one Firefox's media element cannot
decode at all — the first Firefox run failed every playback test with
`audio.currentTime` stuck at 0 while the same build passed in Chromium. A
PulseAudio null sink was started for the run, exactly as the CI workflow
does (`pulseaudio --start`, `module-null-sink`, `pactl set-default-sink`),
and the Firefox suite then passed in full.

## DR-05 readable states and feedback stack — 2026-10-21

| Check | Paths | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- |
| Empty editorial/wave card contrast (light+dark, 1284×856 and 320×700) | apps/web/home-editorial.css; apps/web/src/home_editorial.rs | Live DOM contrast, two real scenes: 2 tracks/0 playlists (empty editorial) and 2 tracks/favorited/1 playlist (empty wave). Light title 15.71:1, eyebrow 15.71:1, subtitle 7.65:1, wave note 7.65:1. Dark title 15.11:1, eyebrow 15.11:1, subtitle 6.40:1, wave note 6.40:1. All ordinary text ≥4.5:1, all titles ≥3:1 | PASS measured | Screen-reader pass NOT_RUN |
| Feedback stack: recovery, Undo and ordinary receipt in one column | apps/web/src/main.rs; apps/web/style.css | Real aborted stream (root recovery budget exhausted: «Connection interrupted. Press Play to retry from your saved position.»), a real add-to-playlist (Undo receipt) and a real add-to-queue (ordinary receipt). DOM order recovery → Undo → ordinary. Chromium: pairwise intersection area 0px² at 1284×856, 390×844, 360×800 and 320×700; recovery↔Undo gap 8px, Undo↔ordinary gap 8px. Firefox: the same 0px² intersections and the same 8px gaps at the same four viewports. Stack↔player intersection 0px², stack↔mobile nav 0px², document overflow 0px, in both engines | PASS measured | Screen-reader announcement order NOT_RUN |
| Stack action targets | apps/web/style.css | Retry playback 143×48, Undo 63×44, dismiss 44×44 at every measured width | PASS measured | — |
| Stack bounded overflow | apps/web/style.css | A deliberately tall message injected into the live stack: at 320×700 the layer capped itself at 415px and at 390×844 at 559px, scrolled internally (`scrollTop` 40 after a wheel over a message, 132 after focusing the last action), kept a 0px² intersection with the player and the mobile navigation, and left the document overflow at 0px | PASS measured | Very long recovery text with a 150% app scale NOT_RUN |
| Now Playing recovery row | apps/web/now-playing.css; apps/web/src/now_playing.rs | `.np-connection` measured `display:flex`, `flex-wrap:wrap`, `gap:8px`, `padding:12px`, `p` `flex:1 1 240px`, button `min-height:44px`, `margin-top:0`. Chromium: the Retry is fully visible with 0px dialog overflow at 1284×856, 1440×900 and 1728×1000; the transport and the volume are visible at all three. Firefox: the Retry is fully visible at the same three viewports; the dialog itself still scrolls 109px/65px there because Firefox's taller secondary column does not fit the sheet — the recovery row itself is fully inside the fold | PASS measured | 720px height keeps the dialog's own scroll in both engines (by design) |
| EN system labels and count grammar | apps/web/src/{main,home_editorial,queue_drawer,shell}.rs | Live render: «2 tracks · 2 albums · 1 artist» and «1 private track»; queue «NOW PLAYING», «Saved queue», «No tracks queued. Add tracks from your library.»; idle player «Nothing playing / Choose a track»; editorial «Choose music for this space»; wave «No suitable tracks yet» / «Loading your mix…» | PASS measured | Full localization NOT_RUN (system labels only; real user names untouched) |
| Rust quality and web build | workspace | `cargo fmt --all`; strict `cargo clippy -D warnings` for vela-web/server/core/domain/media/imports/scanner with `--all-targets` PASS; release WASM build PASS; `scripts/build-web.sh` PASS; workspace tests PASS for the same crates | PASS | `cargo clippy/test --workspace` now run for the whole workspace including `apps/desktop`: the GTK development packages and `pkg-config` are installed, clippy exit 0 and every test result `ok`. The native client's own window, WebKitGTK rendering and physical audio stay NOT_RUN: no X server and no working PulseAudio server on this host |
| Browser suites | tests/browser | Chromium and Firefox Playwright suites PASS, including the new permanent `tests/browser/dr05-states.spec.mjs` (DR05) | PASS | Native GUI/WebKitGTK, physical audio, Docker runtime NOT_RUN |

Permanent coverage: `tests/browser/dr05-states.spec.mjs` owns its server, data
and sessions and asserts the live DOM's own contrast for the empty editorial
and wave cards in both themes, the three real feedback signals in one column,
their zero intersection and ≥8px gap at 1284×856, 390×844, 360×800 and 320×700,
the stack's zero intersection with the player and the mobile navigation, and the
44px height of every action in the stack. Its screenshots and `stack-state.json`
land in `tests/browser/test-results/dr05-states/`.

Ad-hoc stand artifacts from the same run: `/tmp/vela-dr05/artifacts/` —
`home-A-1284x856-{light,dark}.png`, `home-A-320x700-{light,dark}.png` (empty
editorial), `home-B-1284x856-{light,dark}.png`, `home-B-320x700-{light,dark}.png`
(empty wave), `stack-chromium-{1284x856,390x844,360x800,320x700}.png`,
`stack-firefox-{1284x856,390x844,360x800,320x700}.png`,
`np-chromium-{1284x856,1440x900,1728x1000,1284x720}.png`,
`stack-overflow-{320x700,390x844,1284x856}.png`, plus `report.json`,
`stack-chromium.json`, `stack-firefox.json` and `np-chromium.json` with the raw
measurements. The stand was an isolated `vela-server` on 127.0.0.1:52315/52316
with its own data directory and the real `crates/media/tests/fixtures` files,
not the 3219 audit stand. The bounded overflow was proven by injecting a
deliberately tall message into the live stack: at 320×700 the layer capped
itself at 415px and scrolled internally (`scrollTop` 40 after a scroll) with a
0px² intersection with the player and the navigation; at 390×844 it capped at
559px the same way.

## Distribution CI/CD 2026-10-20

Tagged source `eb0fb8cd059e2da03ef94abe9265d4a4d3505c63`, workspace0.12.8.
[Workflow 39479453423](https://github.com/Fynth/vela/actions/runs/39479453423) **SUCCESS**;
[public release](https://github.com/Fynth/vela/releases/tag/v0.12.8) has nine payloads/21 assets. Exact hashes and
publication evidence: [RELEASE_LEDGER_0.12.8](RELEASE_LEDGER_0.12.8.md).

| Evidence class | Executed result |
| --- | --- |
| Rust quality | fmt, strict workspace/all-target, native-QA and WASM Clippy PASS; release WASM build PASS; 374 Rust tests PASS/3 opt-in ignored, including 33 API integration tests |
| Packaging regressions | Python 38 PASS; actionlint 1.7.12 PASS; payload/size/hash/sidecar/source/version/full required-format gate PASS |
| Browser | Final CI Chromium 60 PASS/1 SKIP (4.3m), Firefox 60 PASS/1 SKIP (5.7m); existing live import opt-in is the sole skip; Firefox uses owned null audio output |
| Linux packages | Real optimized server extracted from archive: startup/assets/WASM/schema 13/shutdown/worker-fault PASS; desktop DEB/AppImage CLI smoke PASS |
| Windows package | Native MSVC EXE/ZIP build and `--version` PASS; GUI/audio/hardware NOT_RUN |
| Android package | Real Dioxus arm64 APK build, durable release signing (v2/v3) and manifest/identity/API policy checks PASS; device/background audio NOT_RUN |
| Arch/AUR | CI non-root makepkg, `.SRCINFO`, missing-library and CLI checks PASS; public AUR clone downloaded release tar, verified SHA256, built, and printed 0.12.8; `yay -Si vela-music-bin` PASS at0.12.8-1 |
| Docker | Image build PASS; runtime NOT_RUN |

Public [AUR package](https://aur.archlinux.org/packages/vela-music-bin) recipe commit
`10c82b3c5810d9b6a28a8497433d2e861468c887`. Logs and public build:
`target/checks/distribution/quality-0.12.8.log`, `aur-publish-0.12.8.log`,
`public-0.12.8/makepkg-public.log`; local Rust/Python/Clippy/WASM logs in the same
ignored directory. The failed v0.12.7 gate/tag is preserved; its cold-build fixture
bug was corrected before publishing 0.12.8. No failed check was reclassified as passing.
AUD-20 dependency maintenance warnings remain informational, not a clean strict audit.

G01–G24 still refer to the product mandate. This distribution work adds no new
scenario numbering or completion claim; named EXP/PLAY/RES/NET/AUTH/NP tests retain
their mappings below. G07 private-history coverage exercised the deferred listening
read fix in both browsers; TR03 exercises real favorite/source/EOF continuity.
New native GUI/physical audio/device/background checks were not run by this release;
historical Linux evidence keeps its dated source/runtime scope. Original data and
frozen runtime were not upgraded. See DISTRIBUTION for ABI/signing limits.

## AUD-09 measured performance

| Check | Paths | Commit | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- | --- |
| 100k stats/search cost | crates/media/src/{lib,search}.rs; migrations/0013_stats_covering_indexes.sql | `87b7acd` | stats p50 531->119ms, search p50 138->121ms (8 clients)/20ms (1 client) on isolated 100k fixture; covering indexes, parallel groups, two-phase search; identity equality proven; write cost +1.1ms/2000 inserts, +4.5MiB/100k; workspace25 suites, API33/33, Chromium56/1/0, Firefox56/1/0; migration12->13 on isolated live copy (quick_check ok) | PASS measured + optimized | Capacity/concurrency limits, other hardware profiles, live upgrade NOT_RUN | media/server |

## TR12 release ledger

| Check | Paths | Artifact | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- | --- |
| Release ledger | docs/RELEASE_LEDGER_2026-10-20.md; desktop tar; dist assets | SHA256 `58da18db…`, BUILD `4ceafdc` | Source-vs-artifact docs-only diff; package launch PASS; dist CSS/player byte-identical; release WASM and font/brand hashes; frozen runtime/data untouched; preserved `7fe1ca0` tar | PASS local artifact ledger | Public deploy/push, other platforms, hardware audio/SR/WebKit/full Wayland |

## AUD-05 cold offline launch

| Check | Paths | Commit | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- | --- |
| Cold offline launch | apps/desktop/src/{offline_profile,ui,offline,lib}.rs; native QA + harness | `3aeee09` | Non-secret0600 profile; startup falls through to read/play-only offline library on unreachable/expired/no session; real server off, Play advances one AudioEngine; desktop124/3 ignored; combined260 + offline26 + resume6 PASS | PASS native cold launch | Hardware/SR/full Wayland/other OS NOT_RUN |

## AUD-06 offline origin scoping

| Check | Paths | Commit | Evidence | Status | NOT_RUN |
| --- | --- | --- | --- | --- | --- |
| Origin-scoped offline | apps/desktop/src/{offline,ui}.rs; verify_native_client.py | `a913574` | Namespace<blake3(origin)>/<account>, manifest origin validation, legacy preserved-not-read; desktop118/3 ignored, strict clippy/fmt, native X11 peer/session260 checks PASS incl tampered-copy integrity | PASS native offline | AUD-05 cold launch, hardware/SR/other OS NOT_RUN |

## Current-source TR11 integration matrix — executed2026-10-20

Baseline `e4c425d` on `main`; docs-only diff, no source/Cargo/test/package change.
Host: Ryzen7 8845HS,16 logical CPUs,14GiB visible RAM, kernel7.2.10-1-cachyos;
cargo/rustc1.99.0, Python3.14.8, node26.11.1, Playwright1.64.0, Chromium156.0.8078.4,
Firefox157.0. Every run used an isolated temporary data directory, random loopback
ports and the suites' own fixture accounts; logins stayed on the real production
limiter. The frozen0.12.6 runtime (`vela-local-development.service`, MainPID185315,
0 restarts, health OK) and `data/` were not opened or modified.

| Check | Exact command | Executed result | Artifact |
| --- | --- | --- | --- |
| TR11 Rust format | `cargo fmt --all -- --check` | exit0, no diff | `target/checks/tr11/01-fmt.log` |
| TR11 strict Clippy, host workspace | `cargo clippy --locked --workspace --all-targets -- -D warnings` | exit0;8 crates printed `Checking` (domain/media/core/server/web/scanner/desktop/imports),11.44s | `02-clippy-workspace.log` |
| TR11 strict Clippy, WASM | `cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings` | exit0; `Checking vela-web` forced re-run2.55s | `03-clippy-wasm.log`, `07-clippy-wasm-fresh.log` |
| TR11 strict Clippy, native QA | `cargo clippy --locked -p vela-desktop --all-targets --features native-qa -- -D warnings` | exit0; `Checking vela-desktop` forced re-run2.00s | `04-clippy-native-qa.log`, `08-clippy-native-qa-fresh.log` |
| TR11 workspace tests | `cargo test --locked --workspace --no-fail-fast` | **345 passed,0 failed,3 ignored** in42s (18 real test binaries plus7 doc-test targets that run0 tests) | `05-cargo-test.log` |
| TR11 pinned WASM build | `VELA_WEB_PROFILE=dev VELA_WASM_BINDGEN=target/tools/wasm-bindgen-0.2.129-x86_64-unknown-linux-musl/wasm-bindgen scripts/build-web.sh` | exit0; CLI `wasm-bindgen 0.2.129` (lock also0.2.129); dev WASM20,070,213 B sha256`3e1f17c03a428e93c5e3092727dcc1e5d8124f495471ed9da5ce1b8c7889ac3f` | `09-build-web-dev.log` |
| TR11 release asset build | same script with `VELA_WEB_PROFILE=release` | exit0 after a real38.5s compile;3,529,914 B sha256`9d9251092581ff7ea6b8a137203a8fd652800bdcb1cf0a3b68c728061a72371f` | `10-build-web-release.log` |
| TR11 Chromium suite | `cd tests/browser && npx playwright test --browser=chromium` | **56 passed/1 skipped/0 failed in3.0m** (57 declared: music44, prototype-transfer11, design2) | `target/checks/tr11/browser-chromium.log`, `test-results-chromium/` |
| TR11 Firefox suite | `cd tests/browser && npx playwright test --browser=firefox` | **56 passed/1 skipped/0 failed in3.4m** | `target/checks/tr11/browser-firefox.log` |
| TR11 skip identity | `music.spec.mjs:308` `direct public downloadable audio URL imports without console access` | the only skip in both engines; `test.skip(!process.env.VELA_LIVE_IMPORT_TEST)` — opt-in live Internet import, not a product failure | both suite logs |
| TR11 visual/responsive review | `node target/checks/tr11/capture_visual.js {chromium,firefox}` on real isolated server+`dist` | 7 widths320/390/768/1024/1284/1440/1728×2 engines: document/body overflow0, one central scroller`#main-scroll`,1 audio element, player120px(mobile)/88px(desktop), topbar63/75, content scroller ends at player top, topbar visible,0 page errors;28 PNGs captured (2 opened and inspected:320 Home,1284 Library) | `target/checks/tr11/visual-{chromium,firefox}/{home,library}-*.png`, `visual-review.json` |
| TR11 band reserve (in-suite TR09 evidence) | TR09 test assertions executed in both engines; retained JSON is from the later Firefox run | reserve190=120 player+70 nav at320/390/671/760;88 on761/900/1024/1284/1440/1728; scroller bottom=player top at all10 viewports; nav0 height when sidebar shows; audio1 | `target/checks/prototype-transfer-web/tr09-composition-evidence.json` |
| TR11 startup/first response (release) | `VELA_SERVER_BIN=target/release/vela-server python3 target/checks/tr11/measure_startup.py target/checks/tr11/startup-release.json` | 3 fresh-DB runs: spawn→`/health`200 **21.89–28.57ms**, setup15.0ms, first`/api/me`0.35–0.41ms, first`/api/tracks`0.38–0.45ms, 2s PCM upload2.87ms, warm restart21.4–21.9ms, RSS12.8MiB→34.3MiB, SIGTERM3.1–7.1ms exit0, cookie survived restart | `startup-release.json` |
| TR11 startup/first response (debug) | same script on `target/debug/vela-server` | 3 runs: spawn→health330–368ms, setup309ms, `/api/me`0.68–1.25ms, `/api/tracks`0.89–1.81ms, RSS25.6→49.9MiB, warm312–324ms | `startup.json` |
| TR11 HTTP read benchmark | `python3 scripts/benchmark_http.py --threads8 --duration5 --tracks20 --output target/checks/tr11/http-bench-release.json` | release bin sha256`346b8ea2…` (rebuilt from this source,53.9s): `/health`44,201 req/5s p50 0.874/p95 1.379/p99 1.634ms; `/api/me`43,892 p50 0.882; `/api/tracks?limit=50`36,162 p50 1.069; `/api/search?q=Bench`29,751 p50 1.334; `/api/albums`41,545 p50 0.929; **0 errors**, SIGTERM exit0, temp tree removed | `http-bench-release.json`, `http-bench-release.log` |
| TR11 browser navigation timing | `node target/checks/tr11/browser-timing.js` | Chromium156: TTFB1.3–1.8ms, FCP120/120/164ms, app-ready152/161/191ms, WASM transfer+compile4.6–7.1ms of3,529,914 B, DOMContentLoaded57.3–98ms; route click→content Library56ms, Albums42ms, Home34ms;1 audio element, overflow0 | `browser-timing.json` |
| TR11 native X11 | `python3 scripts/verify_native_client.py --backend x11 --server target/debug/vela-server --test-peer --test-session --evidence target/checks/tr11-native` | **260 checks PASS**,15 complete PNGs (1280×840,1440×900,1728×900,640×700), **1 output stream on the owned `vela_native_test_null` sink, zero HTML media elements**,0 temporary originals after exit, 3 TLS peer connections, saved session resumed without password and cleared after sign-out, v2 BLAKE3 offline copy2,646,044 B | `target/checks/tr11-native/report.json` |
| TR11 native Wayland (window-only) | same wrapper `--backend wayland --window-only --evidence target/checks/tr11-native-wayland` | **5 checks PASS**,1 PNG **2556×2060** from viewport1278×1030 DPR2, normally tiled with no resize,0 temporary originals; backend=`wayland` | `target/checks/tr11-native-wayland/report.json` |

Named negative permission/race tests executed inside the above suites (not weakened,
no fabricated grants):

| Area | Suite and named tests | Result |
| --- | --- | --- |
| Revoked/expired session | browser `AUTH01 real revoked session stops the source and clears private playback UI`; Rust `auth::tests::setup_is_atomic_sessions_revocable_and_credentials_private`, `tests/api.rs authentication_csrf_private_catalog_and_artwork_are_enforced` | PASS both engines; api.rs33/33, core26/26 |
| Public metadata vs private audio | browser `PN01 explicit publication separates private playback, metadata, streaming and revocation`, `PN02 public preview rejects private origins…`; Rust `global_search_finds_owned_music_and_granted_metadata_without_audio_rights`, `publication_tests::publication_http_enforces_explicit_rights_private_ownership_and_every_media_grant` | PASS both engines; api33/33 |
| Playlist collaborator without media | browser first-run scenario (listener sees `Metadata only`, no `Play playlist`, share page has no `#vela-audio`, revoke removes it); Rust `viewer_cannot_edit_manage_access_or_play_and_can_leave`, `collaboration_conflicts_idempotency_reordering_sharing_and_revocation`, `playlist_rename_and_delete_are_owner_cas_idempotent_and_revoke_metadata` | PASS both engines; api33/33 |
| History/privacy consent | browser `G07 private history measures actual playback only after account and device consent, exports and erases`; Rust `private_listening_consent_idempotence_export_and_erasure_use_real_http`, `personal_mixes_use_only_current_consented_history_and_owned_feedback`, core`revocation_still_deletes_history_and_fails_late_writes_and_exports_closed`, `privacy_cas_is_atomic_and_clear_invalidates_inflight_updates`, media`concurrent_privacy_changes_never_mix_history_and_consent_snapshots`, `disabled_history_has_no_effect_and_performs_no_history_queries` | PASS both engines; core26+7, media63 |
| Stale replies/obsolete reads | browser `late older library responses…`, `pending album responses…`, `job filters ignore late replies…`, `global search debounces and discards obsolete queries…`, `late play rejections…`, `NP02 held genuine favorite replies…`, `RES03`, `NET02`, `PC02`, `TR09`/`TR04a` view races | PASS both engines |
| Account change/logout | browser `pending private responses are discarded when another account signs in`, `cancelled metadata responses stay closed across account changes`, `newer filter requests win and private suggestions clear across account changes`, `private search replies cannot populate another account after logout`, `private job replies cannot reappear after logout and another account signs in` | PASS both engines |

Corrections kept visible: the first WASM/native-qa Clippy invocations finished from
cargo cache in0.4s without re-checking a crate, so both were re-run after touching only
the crate-root mtimes (`git status` stayed empty and is empty now); the first HTTP
benchmark used the stale `target/release/vela-server` sha256`0fef9e58…` (2026-10-19
23:37), which was rebuilt from this source before the recorded run. Superseded logs
`03-clippy-wasm.log`, `04-clippy-native-qa.log`, `http-bench.json/.log` are retained,
not quoted as final.

TR11 verdict: **PASS for the executed source/browser/native-linux/measurement scope.**
NOT_RUN: physical speaker/hardware audio output (all audio was an owned null sink or
browser-decoded), screen reader/Orca, WebKit/Safari, Android/iOS, Windows/macOS,
full Wayland resize/peer/offline runs, Internet/public-CA federation, Docker/other OS,
packaged-runtime rerun and real-device touch/keyboard hardware. Native Wayland remains a
window-only subset; the measured numbers are one host, one workload and not capacity
claims.

## Current-source interaction/Hyprland repair — executed2026-10-19

Baseline91867f2, workspace0.12.6; source/debug evidence, not a runtime/package upgrade.
Exact changes/reproduction: [interaction review](UX_INTERACTION_REVIEW_2026-10-19.md).

| Check | Executed evidence | Scope/status |
| --- | --- | --- |
| UX01 | Real server/PCM upload, seek7s, Library/Home/current queue Pause/Play, same audio source and retained clock | PASS Chromium/Firefox; no mocked audio |
| Full browser regression | Chromium43 pass/1 live-import skip2.3m; Firefox43 pass/1 skip2.7m; DESIGN01 responsive/theme, PLAY/RES/NET/NP/privacy/auth retained | `target/checks/ux-toggle-{chromium,firefox}-final/`; browser sources unchanged by later native-only follow-up |
| Native X11 fixture |89 checks,10 complete PNGs,640/1280/1440/1728; pending click/row/queue/playlist continuity, two-origin TLS, mode capabilities, server-stopped offline actual EOF repeat;0 leftover originals | PASS `target/checks/ux-modes-native-x11/report.json`; WebKitGTK/Rodio/null sink |
| Actual native Wayland |5 checks, unresized normal Hyprland tiling, exact owned PID/xwayland=false; frameless controls, seek/row/source continuity/no overflow;1278×1030 DPR2→2556×2060 PNG;0 leftovers | PASS `target/checks/ux-modes-native-wayland/report.json`; distinct subset, not full peers/offline/physical output |
| Better Design | Two final scoped comprehension PASS; DOM web0critical/0serious/10moderate, native0critical/0serious/5moderate | `.better-design/ux-review.json`; inherited moderate spacing remains; native390 unsupported |
| Rust/tooling |293 pass/0fail/3ignored; fmt; strict workspace/native-qa/wasm Clippy; native debug/example/dev WASM builds;22 Python tests; JS syntax/CLI bounds | `target/checks/ux-modes-*.log`, `ux-toggle-web-build-final.log`; ignored/device checks not claimed |

G01–G24 mapping: reinforces G05 persistent player/queue; G18 Linux-client subset and
G23 responsive/theme/input subset remain PARTIAL. No screen-reader, physical sound,
mobile, device handoff, rooms or all-platform certification. Data/schema/runtime
unchanged. Initial failed browser/capture runs are retained/explained, not counted
as final passing evidence.

## Current-source Better Design UI — executed2026-10-19

Source baseline0170ec5, workspace0.12.6; separate from the historical packages below.
Exact changes/commands: [design review](DESIGN_REVIEW_2026-10-19.md).

| Check | Executed evidence | Scope/status |
| --- | --- | --- |
| DESIGN01 | Real isolated Rust server; committed FLAC+generated PCM upload; same authoritative browser audio/source through Home→Library; pause; sidebar group geometry and Settings reachability at1440×720; selected rendered text/primary-control contrast≥4.5 and fonts≥12; player/seek targets≥44; no horizontal overflow; real Home/Library/Playlists screenshots in both themes | PASS in Chromium and Firefox; web320/390/1440/1728; no global contrast/screen-reader claim |
| Full browser regression | Chromium42 pass/1 live-import skip2.3m; Firefox42 pass/1 live-import skip2.6m | `target/checks/design-contrast-{chromium,firefox}-final/run.log`; existing security/media/request-order tests retained |
| Strengthened DESIGN01 contrast | Every visible match of the selected selectors, including current/ordinary tracks;38 readings/theme/engine at320/390/1440/1728; dark minimum6.714:1, light4.536:1 | Separate Chromium5.8s/Firefox7.2s PASS; `target/checks/design-contrast-all-rows-{chromium,firefox}/`; this is not an all-screen WCAG audit |
| Native real-window fixture |74 checks,10 complete screenshots, actual DOM widths640/1280/1440/1728, navigation/source/position continuity, queue/favorite/playlist/folder/group/public peer/offline flows,0 abandoned originals | `target/checks/design-contrast-native-final-evidence/report.json`; PASS; X11/XWayland/WebKitGTK/Rodio null sink, not physical sound/native-Wayland certification |
| Better Design review |3 text comprehension PASS; web0 critical/0 serious/10 moderate, native0 critical/0 serious/4 moderate spacing findings | `.better-design/review.json`; original own tokens, UI/UX/review rules; native390 unsupported scope, inherited moderate spacing retained |
| Rust/tooling |291 passed/0 failed/3 ignored; fmt; strict workspace and WASM Clippy; native-qa example Clippy; native/WASM dev builds;22 Python tests; JS syntax | Exact logs under `target/checks/design-contrast-*`; builds/platform evidence remain distinct |

G01–G24 mapping: this increment reinforces G05 browser queue/player continuity and
the320px/theme/keyboard subset of G23. G23 remains PARTIAL: no screen-reader,
localization or complete reduced-motion/platform certification was added. Native
and public-catalog checks retain their separately documented scope; no handoff,
rooms, mobile or whole-product completion follows. No schema/data/runtime migration
was needed. Initial failed harness/session/selector/capture runs are preserved and
their corrections explained in the review.

Documentation reconciliation2026-10-19 (docs-only): current deployed server baseline
is the frozen0.12.6/schema12 runtime and the current native package is0.12.6; the
0.12.0–0.12.5 sections are dated historical checkpoints, not current state. Golden
G14/G15 are corrected to PARTIAL below (exact tested subset stated); see
[CURRENT_STATE.md](CURRENT_STATE.md) for the source/deployed/design split. No check
was executed for this reconciliation.

Source `d3bde0c`; archived package SHA256 above. Exact extracted0.12.0 binary/assets
served the original data directory after a verified backup/isolated restore; the
service is healthy with schema11 and all prior rows/media intact.

| Feature | Exact paths | Verified source | Executed evidence | Result | Next | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| Local package/defaults | scripts/package-release.sh; target/releases/vela-0.12.0-*.tar.gz | d3bde0c | Extracted startup/setup/schema11/assets/SIGTERM;19 frozen hash matches; preserved earlier release checksums | PASS | Docker/ARM64 unverified | main |
| Exact packaged browser regression | apps/web; tests/browser | d3bde0c package | Chromium41/41 1.9m + Firefox41/41 2.4m with licensed live import; sleep/playlist/queue/history/DSP/radio/publication regressions | PASS | Safari/mobile NOT_RUN | web/main |
| Backup/restore before upgrade | scripts/backup.py; target/upgrade-backups/before-0.12.0-20261018T161750Z.tar.gz | d3bde0c |0600 440MB backup; isolated restore with30 table count equality, media/artwork SHA, quick_check/FK0 | PASS | Encryption/offsite not claimed | main |
| Runtime switch | target/runtime/vela-0.12.0; enabled user unit; original data/ | d3bde0c | Graceful stop old PID; unit switched; health0.12.0/PID267247/0restarts; schema11/1user/20tracks/20analysis/0playlists; served hashes match | PASS | Boot/login behavior unverified | main/devops |
| Shutdown/worker recovery | scripts/verify_shutdown.py | d3bde0c package |10 clean SIGTERM/restarts exit0; import/scanner/analysis/listening worker DB failures stopped correctly | PASS | No device/Docker claim | main |

# Verified native Linux and web source slices0.12

Source slices: native `3a285e0`/hardening `05951fd`, web queue/playlist `6421f3d`.
The frozen0.11.0 server runtime and original data were not replaced; no0.12 package
was installed.

| Feature | Exact paths | Verified source | Executed evidence | Result | Next | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| Native Linux client transport/audio | apps/desktop/src/{client,audio,lib,ui,main}.rs; apps/desktop/style.css |3a285e0/05951fd |30 desktop tests (28 run,2 opt-in); strict Clippy `--all-targets --features native-qa`; fmt; owned-null-sink audio test; real WebKitGTK window against isolated fixture server/account/media | PASS for Linux library/queue playback | Packaging, secure persistence, favorites/playlists UI, offline/background | main/audio/client/UI |
| MPRIS2 observer | apps/desktop/src/mpris.rs |05951fd | Ignored real-session test under private `dbus-run-session` after bounding update/command channels: properties and play/pause/next/previous/seek/volume forwarding | PASS for Linux session bus | External controller/session hardware smoke | main |
| Native favorites | apps/desktop/src/{client,ui}.rs; examples/native-ui-qa.{rs,js} | 0.12 local | Owner favorite/unfavorite API, filtered Favorites view, unfavorite re-read to empty, Library restore, queue-state sync;30 desktop tests (28 run,2 opt-in) and233 workspace tests | PASS for Linux native scope | Packaged/background/offline remain | main |
| Native playlist lifecycle | apps/desktop/src/{client,ui}.rs; scripts/verify_native_client.py; examples/native-ui-qa.{rs,js} | 0.12 local + current debug server | Fixture creates a real two-entry playlist; native list/detail, create→rename→cancel/confirm-delete, fixture survives, entry add/reorder/restore/remove, three-entry streamable playback, shared queue2 entries, Library return;25/27 desktop tests | PASS for Linux native scope | Entry editing/folders/albums/artists remain | main |
| Native album/artist browse | apps/desktop/src/{client,ui}.rs; examples/native-ui-qa.{rs,js} | 0.12 local + current debug server | Real album groups/detail (`Real Fixtures · Vela Test`), Play album native audio, real artist groups/detail, Play artist native audio, Library return;25/27 desktop tests | PASS for Linux native scope | No editing/edition identity | main |
| Web sleep timer | apps/web/src/main.rs; tests/browser/music.spec.mjs | 0.12 local | Root countdown 5/15/30/60, real60s audio still playing before expiry, paused+status at zero, cancel/restart stays playing; Chromium41/41+Firefox41/41 | PASS for browser scope | No OS-level timer/native sleep | main |
| Native queue/favorite parity | apps/desktop/src/ui.rs; examples/native-ui-qa.{rs,js} | 0.12 local | Move-down follows current occurrence, move-up restores, Clear-upcoming keeps only current, player favorite API+queue sync;25/27 desktop tests | PASS for Linux native scope | No drag/drop | main |
| 0.12.6 session/offline integrity release/runtime | Cargo.toml/lock; target/releases/vela-0.12.6-*; target/runtime/vela-0.12.6; scripts/install-user-service.py | `7fe1ca0` | Package verify; exact Chromium41 pass1 skip1.9m, Firefox41 pass1 skip2.2m; backup restore-verified schema12; installer handover, health0.12.6,PID544183,0 restarts; counts intact; native0.12.6 SHA7abe39…; session/offline/integrity features included | PASS for Linux local runtime/package | No external publication; non-Linux not implied | release/main |
| 0.12.5 spectral-4/native folders release/runtime | Cargo.toml/lock; target/releases/vela-0.12.5-*; target/runtime/vela-0.12.5; scripts/install-user-service.py | `3eedf1b` | Package verify; exact Chromium41 pass1 skip1.9m, Firefox41 pass1 skip2.2m; backup restore-verified schema12; installer handover, health0.12.5,PID498859,0 restarts; counts intact; native0.12.5 SHAc6bd29… | PASS for Linux local runtime/package | No external publication; non-Linux not implied | release/main |
| 0.12.4 playlist folders schema12 release/runtime | Cargo.toml/lock; target/releases/vela-0.12.4-*; target/runtime/vela-0.12.4; scripts/install-user-service.py | `16034b9` | Package verify; exact Chromium41 pass1 skip1.9m; Firefox41 pass1 skip2.3m on clean rerun (first run flaked sleep timer); backup restore-verified schema11; explicit new-runtime migration to schema12, then installer handover; health0.12.4,PID463860,0 restarts; counts intact | PASS for Linux local runtime/package | No external publication; native folder UI absent; non-Linux not implied | release/main |
| 0.12.3 privacy/offline patch release/runtime | Cargo.toml/lock; target/releases/vela-0.12.3-*; target/runtime/vela-0.12.3; scripts/install-user-service.py | `6cd407c` | Package verify; exact Chromium40 pass1 skip1.9m, Firefox40 pass1 skip2.2m; private0600 backup restore-verified; graceful handover, health0.12.3, PID432696,0 restarts; counts/integrity unchanged; native0.12.3 SHA13d75a… | PASS for Linux local runtime/package | No external publication; spectral-1/2 rows retained; non-Linux not implied | release/main |
| 0.12.2 spectral-3 patch release/runtime | Cargo.toml/lock; target/releases/vela-0.12.2-*; target/runtime/vela-0.12.2; scripts/install-user-service.py | `e9d1954` | Package verify; exact Chromium40 pass1 skip1.9m, Firefox40 pass1 skip2.2m; private0600 backup restore-verified; graceful stop/installer handover, health0.12.2, PID382555,0 restarts; counts/integrity unchanged | PASS for Linux local runtime/package | No external publication; spectral-1/2 rows retained; native/mobile outside Linux not implied | release/main |
| 0.12.1 DSP patch release/runtime | Cargo.toml/lock; target/releases/vela-0.12.1-*; target/runtime/vela-0.12.1; scripts/install-user-service.py | `9aa062a` | Package verify; exact Chromium40 pass1 skip1.8m, Firefox40 pass1 skip2.2m; private0600 backup restore-verified; graceful stop, installer handover, health0.12.1, PID354876,0 restarts; counts/integrity unchanged | PASS for Linux local runtime/package | No external publication; spectral-1 rows retained; native0.12.0 package not repackaged | release/main |
| Local DSP tempo/chroma/key spectral-4 | crates/media/src/analysis.rs; crates/domain/src/audio_analysis.rs; apps/web/src/radio.rs; tests/browser/music.spec.mjs | 0.12 local | Normalized spectral-flux onset envelope, onset-interval median BPM, bounded confidence, extractor version bumps;22 analysis tests incl120 BPM click train, A440/C#4/two-tone chroma, C-major/A-minor Krumhansl triads and None on single-pitch/silence; Chromium Radio shows spectral-4 + Estimated tempo + Chroma pitch classes + Estimated key + heuristic disclaimer; full Chromium41 passed,1 live-import skip in2.2m | PASS for bounded descriptive tempo/UI/radio regression | No calibrated beat/chroma; Firefox not rerun; real-music tempo accuracy unmeasured | media/domain/web |
| Native TLS federation | apps/desktop/src/remote.rs; scripts/verify_native_client.py | 0.12.5 source | One-day local CA + IP-SAN cert; Python TLS terminator proxies the second peer; native reqwest validates the chain for catalog/stream;3 TLS connections; debug-only CA/transport overrides absent from release strings;28/30 desktop tests | PASS for loopback TLS certificate validation | Internet DNS/public CA/two physical hosts NOT_RUN | desktop/main |
| Native public peer playback | apps/desktop/src/remote.rs,ui.rs; scripts/verify_native_client.py; examples/native-ui-qa.{rs,js} | 0.12 local | Second isolated Vela instance: canonical public origin, explicit CC-BY stream publication, two isolated peers, save/remove origins in isolated private0600 config, 21-record public peer first page20 then preview cursor Load more appends the final record; unified search per-peer cursor Load more appends its 21st result with no duplicate IDs, unified search returns both origins, unreachable saved peer returns browse-only cached metadata with age, no play action and an explicit Retry live that remains cached when still unreachable, second-origin public playback, remote previous/next across both origins, credential-free;28/30 desktop tests | PASS for loopback debug transport | Internet TLS/two-host validation NOT_RUN | main |
| Native playlist folders | apps/desktop/src/{client,ui}.rs; examples/native-ui-qa.{rs,js} | 0.12.4 source | Owner folder create/filter/move/rename/delete via real APIs; unfiled keeper excluded by filter; delete unfiles and restores grouping;28/30 desktop tests | PASS for Linux native source | Source newer than packaged0.12.4 desktop archive; no folder drag/drop | desktop |
| Offline lifecycle/quota repair | apps/desktop/src/{offline,ui}.rs | `e225b63` | AUD-03/07/08/12: ticket+checking/Cancel before work, spawn_blocking same-fd BLAKE3 with per-block cancel, non-current removal remaps queue/order, per-await gate/epoch/user/origin guards, manifest cap/unique staging/quota+replacement delta/recoverable publish; desktop56 pass3 ignored; real native offline/server-stopped QA `target/checks/aud-offline-p1/` | PASS for source+native window basic flow | Cross-process save, disk-full/power-loss, explicit checking/Cancel harness assertions NOT_RUN | desktop |
| Desktop saved-session P0 repair | apps/desktop/src/{session_store,ui}.rs; scripts/verify_native_client.py | `3094754` | AUD-01/02/04/11: off-thread bounded keyring deletion after audio stop; transient/5xx/403 preserve item, only401 deletes; attempt generations; byte-bounded/kill+reap/serialized helper; redacted Debug; desktop43 pass2 ignored; real two-process Secret Service save/resume/sign-out QA `target/checks/aud-session-p0/` | PASS for source+Linux Secret Service | Not deployed to live0.12.6; no hung-real-D-Bus repro | desktop |
| History snapshot reads | crates/core/src/listening.rs | `48b2fa4` | AUD-13: history/export use one deferred snapshot, no BEGIN IMMEDIATE/prune on GET; hourly worker prunes; core26+7, API31; red/green 2.52ms vs10.03s SQLITE_BUSY under open writer | PASS for source | Retention worker delay not measured live; preferences/devices GETs still write | core |
| Native hardening review fixes | apps/desktop/src/{offline,session_store,ui}.rs; crates/core/src/playlists.rs | working slice | Keyring singleton/verified deletion/malformed cleanup; same-fd BLAKE3 offline verification + failed-manifest cleanup; transactional folder-owned move; two-process resume + server-stopped offline QA;28/30 desktop tests,31 API tests | PASS for reviewed fixes | Independent review produced no findings before stop; self-review fixes recorded; loopback/mobile limits unchanged | desktop/core |
| Firefox full suite (TR04a baseline) | tests/browser/music.spec boot helper + IN01 Search path | pending | Boot-state wait (setup OR sign-in) and fresh Search instead of stale Library; full Firefox49 passed/1 skipped/0 failed3.2m; targeted IN01 Chromium pass; full Chromium49/1/0 before test-only change | PASS Firefox suite | WebKit/Safari/native/package NOT_RUN | web/QA |
| TR10 native fonts/package | apps/desktop/{style.css,examples/native-ui-qa}; docs | `47fb518`+`d314a2c`; package `4ceafdc` | Manrope Latin/Cyrillic embedded as data-URI @font-face (OFL, weight200-800), native X11 QA260 checks/15 shots incl Cyrillic metrics/frameless controls/scale80-150/one AudioEngine; rebuilt0.12.6 tar SHA`58da18db…` launch PASS; prior `7abe39…` preserved previous/ | PASS native font+package | Real device/SR/hardware audio/other OS/Internet NOT_RUN | desktop |
| TR09 mobile/tablet | apps/web/{style,css,main}; prototype-transfer.spec | `8c23fb7` | <=760 one mobile composition, 761 tablet/desktop, short-height390x600, safe-area once, 44px, reduced motion, focus/transport ownership; fixed always-on overlay marker blocking Space/Arrow; Chromium56/1/0 + Firefox56/1/0 | PASS web TR09 | Device/SR/OS inset hardware/native NOT_RUN | web |
| TR08 settings hub | apps/web/{shell.rs,main.rs,style.css}; tests | `9262329` | Mobile More opens Settings directly; More-from-Vela real route list incl radio seed/admin Jobs; aria-current grouping; tests updated; Chromium51/1/0 + Firefox51/1/0 | PASS web TR08 | Device/SR/WebKit/native NOT_RUN | web |
| TR07 context menus/picker | apps/web/{main,player.js,style}; prototype-transfer.spec; music.spec | `9859bd0` | Frozen-target registry across More/right-click/Shift+F10/ContextMenu/touch sheet; real searchable modal add-to-playlist picker with CAS+receipt+Undo; focus trap/return; transport keys suppressed while overlay open; targeted2/2, Chromium51/1/0, Firefox51/1/0 | PASS browsers | Touch hardware/SR/zoom/WebKit/native/audio NOT_RUN | web |
| TR07b mobile action sheet | apps/web/{main.rs,style.css}; prototype-transfer.spec | `42f105c` | <=670 fixed sheet above player, backdrop, safe-area, 48px rows, internal scroll, focus owns Escape; desktop anchored190; full Chromium50/1/0 + Firefox50/1/0 | PASS sheet | Not focus-trapped; transport keys reach global; focus return/menu Undo/searchable overlay NOT_RUN | web |
| TR07a right-click menu | apps/web/{main.rs,style.css} | `c8ad235` | Signal-controlled row menu: right-click, More toggle, Escape/action close, aria-haspopup/expanded; root actions only; full Chromium49/1/0 | PASS menu interaction | Touch sheet/searchable overlay/menu Undo/Firefox NOT_RUN | web |
| TR07a track actions menu | apps/web/{main.rs,style.css} | `653c4b4` | Compact details menu per row with Play/Pause, queue, favorite, radio, metadata; root actions only; full Chromium49/1/0 | PARTIAL menu | Right-click/searchable overlay/touch/Undo/Firefox NOT_RUN | web |
| TR06b add receipt/Undo | apps/web/src/main.rs; style.css | `8d2ea14` | Receipt names track/playlist; Undo deletes exact added item with post-add revision+fresh mutation; full Chromium49/1/0 | PARTIAL/SUITE-GREEN | Undo click not separately asserted; Firefox NOT_RUN | web |
| TR06b playlist picker search | apps/web/{main.rs,style.css} | `b1f4ecf` | Live name filter for editable playlists in Add-tracks picker; existing add mutation only; full Chromium49/1/0 | PARTIAL picker search | Receipts/Undo/searchable overlay/Firefox NOT_RUN | web |
| TR06a web playlist detail art | apps/web/{main.rs,style.css}; PlaylistSummary.artwork_id | `cfe0255` | Detail header CatalogCover168/120 mobile with fallback; targeted6/6, full Chromium49/1/0 | PASS Chromium | Firefox full49/1/0; picker/Undo/PT matrix NOT_RUN | web |
| TR05a web borderless cards | apps/web/style.css | `56455ac` | Album/artist cards borderless padding0/border0, real square cover radius7, title12, hover -1/press .98; targeted DESIGN/UX/TR01-04a 6/6 and full Chromium49/1/0 | PASS Chromium | Firefox full after card change49/1/0; PT width matrix NOT_RUN | web |
| TR05a web compact rows | apps/web/style.css | `42a1e3f` | Title12/550, artist12, row64, playlist title13; DESIGN01 legibility ≥12 preserved; full Chromium49/1/0; targeted DESIGN/UX/TR01-04a pass | PASS Chromium | Firefox row change, album/artist cards, PT width matrix NOT_RUN | web |
| Cross-engine browser suite | apps/web/style.css range focus; tests/browser | `62d2fab` | Explicit range :focus ring for Firefox; full Chromium49 pass1 skip0 fail, full Firefox49 pass1 skip0 fail3.1m, targeted SCALE both; web30/API33/desktop113 | PASS Chromium+Firefox | WebKit/Safari/native package/PT matrix NOT_RUN | web |
| Web playlist artwork | apps/web/src/{experience,main}.rs; PlaylistSummary.artwork_id | `93138a1` | Home and Playlists cards render first authorized playlist artwork via CatalogCover with fallback; full Chromium49 pass1 skip0 fail; web/API/desktop gates pass | PASS web playlist art | Firefox full after this change49/1/0; native detail QA pass; package NOT_RUN | web |
| TR04b/TR07a web + native playlist art | apps/web/{home_editorial,queue_drawer,main,style}; apps/desktop/src/ui.rs | `df0290a`,`e876449`,`7aa0ae0` | Home editorial/My Wave and right queue drawer project one root player; playlist detail/list real artwork; Chrome full49 pass1 skip0 fail, web30, API33, desktop113 | PASS web + native source | Firefox/manual PT widths/package/native matrix for last header NOT_RUN | web/desktop |
| Playlist artwork projection | crates/domain/src/lib.rs; crates/core/src/playlists.rs | `2c5318f` | First authorized playlist entry artwork exposed as PlaylistSummary.artwork_id via owned/list/search correlated query with serde default; core26 + API33 pass | PASS backend | Client playlist cards not wired; external metadata-only grants unchanged | core/server |
| Native actual-next preview | apps/desktop/src/{ui.rs,playback reuse}, style.css | `a2b4f94` | Queue shows order.projection() explicit-next or first shuffle-deck pending with reason label; no mutation/play; native X11 peer QA pass | PASS native preview | Drag/insertion/cross-device NOT_RUN | desktop |
| Native Wayland window | scripts/verify_native_client.py --backend wayland --window-only; apps/desktop | working tree | No GTK titlebar, integrated controls reachable, row pause/resume retains source+position, no horizontal overflow, backend=wayland, zero temp originals | PASS Wayland window-only | Full Wayland resize/peer/offline/package NOT_RUN | desktop |
| Native paused queue restore | apps/desktop/src/{queue_store,ui}.rs | `350d72a` | Private versioned snapshot; restore only same origin+account when queue empty, paused/no autoplay, no notice override; save on transitions/sign-out; one AudioEngine; native X11 peer/session QA pass | PASS native restore | Position resume starts at0; cross-device/handoff/TR07 NOT_RUN | desktop |
| Native storage controls | apps/desktop/src/{media_cache,ui}.rs | `ab7bb19` | Usage/clear only published cache pairs; Settings Storage panel count/size/Refresh/2-step clear; offline/staging untouched; unit test + native X11 peer/session QA; desktop tests pass | PASS native storage | Download manager/priority/TR07 drawer/Wayland NOT_RUN | desktop |
| TR05a native detail rows | apps/desktop/src/ui.rs | `8294802` | Album/artist detail uses .native-track grid: start, real art, title/artist, album, duration, row no; play_owned_context unchanged; native X11 peer/session QA pass, desktop111/3 ignored | PASS native detail | Library/search restyle, TR05 web, PT06/07/10, package NOT_RUN | desktop |
| TR04a Home filters | apps/web/{experience,icons,style}; prototype-transfer.spec | `1b70b7a` | Header+3-tab tablist over real DTOs; editorial honest absent (IMAGE_BLOCKED); real shelf<=6 borderless art; real Play new additions; playlists grid/empty; no fake/stale content; one root player; full Chromium49 pass1 skip0 fail, web27 | PASS web TR04a | Editorial/wave images, My Wave visual, Firefox/native/package NOT_RUN | web |
| TR03 web player | apps/web/src/main.rs; style.css; prototype-transfer.spec | `456dec3` | One root element;51px art/title/artist/album/quality; transport+44px seek/aria; source/queue/volume/mute; responsive88/mini; targeted TR01/02/03 3/3, DESIGN01/UX01 2/2, web27 | PASS web TR03 targeted | Full suite one login-limiter fail; Firefox/PT matrix/package NOT_RUN | web |
| TR03 native player | apps/desktop/src/ui.rs; style.css; native QA | `4a29a59` | 88px band:48px artwork, honest metadata, transport44px hit boxes/32px face, seek10px tabular+aria, Source/Queue/Mute/volume, mute restore; one AudioEngine; source/queue/position survive Queue/theme/scale; native X11 QA204 checks/14 screenshots incl owned null-sink audio, desktop108/3 ignored | PASS native TR03 | Web TR03 source-generation8→10 on keyboard seek unresolved; TR07 drawer/Wayland/package/other widths NOT_RUN | desktop |
| TR01/TR02 shell foundation | apps/web/{style.css,index.html,src/*}; apps/desktop/{style.css,src/ui.rs}; apps/server/src/lib.rs; scripts/build-web.sh; font assets | `d687026`,`6269ff0`,`d0e5584` | Namespaced roles, self-hosted Manrope+allowlist/copy, web216/75/88 one-offset shell targeted Chromium DESIGN01/UX01/TR01/TR02 4/4 across13 widths+80/150% zoom; native X11 153 checks 216/75/88, single scroller, 640 destinations, playback continuity | PASS targeted web + native X11 shell | Home/player/topbar search/mobile/TR09/Wayland/packaging/native font NOT_RUN | web/desktop/server |
| Native authorized artwork | apps/desktop/src/{client,artwork,artwork_cache,ui}.rs | `0f9f12b` | ART01-08/10 source: authenticated HEAD+GET,8MiB/MIME/magic, denial, private0600 cache atomic/200/64MiB, bounded store, data-URL covers+fallback; desktop96 pass3 ignored incl real-server auth/cache reuse; ART01 real 2288x937: 640x498 data-image covers in library+player, square cover, sign-out clears, re-sign-in restores; ART02 PARTIAL (favorites+queue visual only; album/detail/search NOT_RUN); ART11 PARTIAL (2288 dpr1 dark + 1280 light queue; high-DPI/light player NOT_RUN) (`target/checks/artwork-visual-2288/`) | PASS ART01, PARTIAL ART02/ART11 | Denied/foreign visual, long-scroll memory, offline/MPRIS art NOT_RUN | desktop |
| Native settings/app scale | apps/desktop/src/{presentation,ui}.rs; style.css | pending | SCALE01/03/07/08/12/SET01/04/13 source: private versioned device store0700/0600,80-150+theme, first-frame apply, Settings/−+/slider/reset/repair, no audio/queue touch; desktop108 pass3 ignored; native peer/session QA pass | PASS for source+native flow | Visual/HiDPI/640/Orca/disk-full NOT_RUN | desktop |
| Web app scale/appearance | apps/web/src/{preferences,settings,main}.rs; style.css; tests/browser | `1618e63` | SCALE01/02/07/09: versioned device prefs80-150+theme, boot apply, Settings panel/reset/status, one CSS zoom, malformed recovery, no tokens; web27 tests, targeted Chromium SCALE01 pass4.2s | PASS for Chromium targeted | Full suite/Firefox/screen-reader/native scale NOT_RUN | web |
| Native saved session | apps/desktop/src/{session_store,client,ui}.rs; scripts/verify_native_client.py | 0.12.5 source | Opt-in checkbox stores opaque token/origin in unique QA Secret Service item; second native process resumes via `/api/me` without password; sign-out clears item; envelope has no password/username/CSRF;28/30 desktop tests | PASS for Linux Secret Service | Windows/macOS/mobile stores, biometrics NOT_RUN | desktop/main |
| Native offline copy/playback | apps/desktop/src/{offline,ui,client}.rs; apps/desktop/examples/native-ui-qa.{rs,js}; scripts/verify_native_client.py | `7609a76`+working slice | Explicit download of two tracks via validated originals; account-scoped0600 audio+v2 BLAKE3 manifests checked before playback and by the startup sweep; harness stops real fixture server, health probe fails, first offline original plays to advancing media, Next/Previous move within the newest-saved offline queue, confirmed removal leaves one pair;28/30 desktop tests | PASS for Linux one-track offline playback | No offline queue/removal/sync, mobile/background | desktop/main |
| Native portable package0.12.6 | scripts/package-desktop-linux.sh; scripts/verify_desktop_package.py; apps/desktop | `7fe1ca0` | Exact manifest/modes, `--version`=0.12.6, ldd, styled sign-in launch, normal close exit0, zero temp files; SHA256 `7abe39…`; includes folder UI, offline v2 BLAKE3 manifests + sweep, cache age/Retry live, pagination, opt-in session resume | PASS for portable tar/launch | deb/AppImage/signature/physical audio NOT_RUN | main |
| Native portable package0.12.1 | scripts/package-desktop-linux.sh; scripts/verify_desktop_package.py; apps/desktop | `a3cb52c` | Exact manifest/modes, `--version`=0.12.1, ldd no missing, real GUI sign-in launch+styled screenshot, normal close exit0, zero temp files; SHA256 `67850b30…` | PASS for portable tar/launch | deb/AppImage/signature/physical audio NOT_RUN | main |
| Native portable package | scripts/package-desktop-linux.sh; scripts/verify_desktop_package.py; apps/desktop |692c083 app/current scripts | Exact manifest/modes, `--version`=0.12.0, ldd no missing, real GUI sign-in launch+styled screenshot, normal close exit0, zero temp files; SHA256 4d6c67bb… | PASS for portable tar/launch | deb/AppImage/signature/physical audio NOT_RUN | main |
| Responsive native window/source lifecycle | apps/desktop/examples/native-ui-qa.{rs,js}; apps/desktop/{style.css,src/client.rs,src/lib.rs} |05951fd |15 named checks at1280×840 and640×700: real WAV/FLAC/MP3 playback, pause/seek, navigation continuity, duplicate queue, reorder, element bounds, no overflow, sign-out; zero temporary files during held transfer/after exit; close-while-playing variant passed | PASS for recorded Linux scope | Physical output, screen reader, packaged window | main/QA |
| Failure found and fixed | apps/desktop/style.css; src/client.rs |05951fd | First compact run reproduced queue grid rows overflowing640px; independent review reproduced a named temp file during preparation. Element-level assertion and unnamed `O_TMPFILE` source retained; complete reruns passed | FAILED then PASSED | Preserve failed evidence | main/reviewer |
| Owner playlist rename/delete + queue truthfulness | crates/domain/src/lib.rs; crates/core/src/playlists.rs; apps/server/src/lib.rs; apps/server/tests/api.rs; apps/web/src/{main,client}.rs; tests/browser/music.spec.mjs |6421f3d | Real TCP owner/editor/outsider/CSRF/unknown/stale/replay/share-revocation/track-preservation test; full workspace233 passed/0 failed/2 opt-in; strict native+WASM Clippy; fmt;22 Python; Chromium40/40 in2.5m and Firefox40/40 in3.3m with live licensed import | PASS for source web/API scope | Folders/smart rules/invitations/media grants; exact package rerun | main/web |

Evidence paths are under ignored `target/checks/playlist-*` and `native-0.12*`;
`native-0.12-window2` preserves the failed compact run. These are Linux window/audio
and source-browser tests, not mobile background, Apple/Windows, installer, exact
packaged-asset or physical-audio certification.

# Verified0.11 package/runtime checkpoint

Source/application `fe0484d62f1a2ae33a8c25ee901d4ed6b9568f06`.

| Feature | Exact paths | Verified source | Executed evidence | Result | Next | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| Explicit publication/grants/revocation/peer policy | crates/domain/src/network.rs; crates/core/src/network.rs; crates/media/src/publications.rs; apps/server/src/network.rs; migrations/0011_explicit_publications.sql | fe0484d | Rust200; strict native+WASM; API owner/CSRF/CAS; two real independent origins + hostile/slow peer; DNS/redirect/capacity and legacy10 preservation | PASS for profile1 | Full signed federation/adapters remain unfinished | main/media/QA |
| Actual Dioxus Network/root-player workflows | apps/web/src/network.rs; tests/browser/music.spec.mjs | fe0484d exact package | Chromium39/Firefox39 with licensed live import, PN01/PN02, all prior regressions; screenshots/physical abort/actual Range/revoke | PASS for web scope | Native/Safari/screen-reader/Internet TLS NOT_RUN | web/main |
| Frozen release integrity/defaults/recovery | scripts/package-release.sh; scripts/verify_package.py; scripts/verify_shutdown.py | fe0484d | Extracted schema11/default-private/assets/SIGTERM;19 matching binary/static hashes; retained release SHA checks;20 restarts+4faults | PASS | Docker/ARM64 platform validation unavailable | main |
| Original data and persistent supervision | scripts/backup.py; scripts/install-user-service.py; tests/test_user_service.py; data/ | fe0484d | Fresh stopped backup/isolated restore/all old SQL rows+media preserved; original10→11;22Python; actual unit installation/recheck, enabled/PID78464/0restarts | PASS | No boot/logout/native-audio inference | devops/main |

All exact commands/evidence paths, unexecuted scope and current original-data
counts are in PROGRESS. One listening preference/device record existed before
the final backup; private consent values are identical before/after migration,
with0sessions. New public catalog remains disabled with0publications. G01/G24
are verified to this original-data upgrade scope; G11/G14/G15/G22 remain the
specific read-only/rights subsets below, not full federation golden acceptance.

# Public catalog0.11 source integration

Historical source gate before the exact package checks above.200 Rust tests, strict native/all-target+WASM Clippy, release native/WASM,
fmt,22 Python, JS syntax/diff and locked dependency policy passed.20 real clean
restarts and4 independent worker faults passed. `target/checks/publication-*` holds
commands/logs; the failed NUL projection run is preserved separately from corrected
whole-workspace evidence. Source Chromium39/39 passed in2.5m, licensed live import; screenshot/evidence paths
are in PROGRESS. No native/public-TLS/ActivityPub compatibility claim.

| Feature | Exact paths | Latest verified source | Evidence command | Result | Next | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| Origin/private publication/independent grants | core/network.rs; media/publications.rs; domain/network.rs; migration0011; server/network.rs | fe0484d | cargo test --locked --workspace -- --test-threads=2:200 | PASS | Exact package/regressions | main/media |
| Two independent origins + hostile peer | server/src/network/tests.rs | fe0484d | Same Rust run: actual B HTTP preview calls actual A HTTP, Range/HEAD/revoke, slow/oversized/spoofed identity/grants/IDs | PASS for read-only profile | Standards signatures/replay/blocks remain NOT_RUN | main |
| Controlled DNS/redirects and stream capacity | imports/egress.rs; server/network/tests.rs | fe0484d | Real two-listener no-dial redirect controls, barrier cancellation retaining4DNS slots,8 public bodies drop/reserved owner capacity | PASS | Public load measurements | main/reviewers |
| Network UI/source guards | web/network.rs; tests/browser/music.spec.mjs | fe0484d | Chromium39/39 with live import, PN01/PN02; main inspected Network1440dark/320light, no overflow | PASS for source Chromium | Exact packaged two-engine runs | web/main |
| Same-build persistent user service | scripts/install-user-service.py; tests/test_user_service.py; docs/USER_SERVICE.md | fe0484d + frozen902dd47 runtime |22 Python incl13 installer; actual --replace-transient --install and --dry-run recheck | PASS | Preserve pinned upgrade process | devops/main |

Independent readonly QA signed off the implemented permission/projection/peer/DNS/
UI boundaries and checked retained logs. No release-blocking defect found; exact
package/two-engine acceptance still follows. Private playlist sharing remains
metadata-only even though separate public publication now has explicit audio grants.

Latest mandate mapping: G01/G24 additive10→11 legacy-row and backup safeguards;
G11 independent origin/catalog + permitted original public stream subset; G14
bounded hostile/slow/SSRF/metadata subset (forged actors/signature replay NOT_RUN);
G15 removal/stream-revocation subset (no federated recommendation/cache);
G22 declaration/provenance/unknown-rights/private defaults subset. PN01/PN02 are
new named acceptance; preserve all EXP/PLAY/PLNAV/RES/NET/AUTH/NP/PC/history/DSP
regressions. No entire golden scenario becomes green from a partial subset.

# Verified0.10 package/runtime checkpoint

Application/source 902dd47f9dca2d9b6f405cd50c21bf4dab5a73d8. Exact extracted binary passed37Chromium +37Firefox
with live licensed import; static bytes match frozen package before/after runs.
Screenshots8 size/theme combinations per engine; main inspected desktop/phone.
Packaged20clean restarts +4worker DB faults, default/schema10/assets/SIGTERM,
all preserved release checksums and9frozen hashes pass. Fresh stopped0.9backup
restored with every SQL row/media hash; original20tracks/assets and20analysis
rows preserved after9→10, quick_check ok/FK0, no auto consent/exclusions.
Current supervised loopback0.10 PID34675 uses original data,0restarts.
Source gates176Rust/strict native+WASM/fmt/9Python/JS/deny pass. Evidence paths
and exact limits are in PROGRESS; no native/Safari/screen-reader/Docker claim.

# Personal discovery0.10 source evidence

Source/application902dd47; final archive/runtime evidence is recorded above and in PROGRESS.

| Feature | Exact paths | Verified source | Executed evidence | Status | Next / owner |
| --- | --- | --- | --- | --- | --- |
| Private mix/service/feedback | crates/media/src/recommendations.rs; apps/server/src/recommendations.rs; migrations/0010_recommendation_exclusions.sql |902dd47 |176 Rust tests, strict native/all-target+WASM Clippy, real TCP, legacy9upgrade, owner/consent/retention/concurrency | PASS | Preserve gates / main |
| Web peer preview cursor | apps/server/src/network.rs; crates/domain/src/network.rs; apps/web/src/network.rs | 0.12.6 source | PreviewPeer.cursor validated, UUID-checked, forwarded to the peer catalog URI and checked for continuation; fixture test records the outbound cursor and rejects non-UUID; web Load more revalidates origin/dedupes by ID; Chromium41 pass1 skip2.0m; Firefox41 pass1 skip in three clean runs (2.6m/2.9m pre-fix, 2.9m at HEAD3a3f463) — still non-regression only (no successful-preview browser scenario); network lib tests4, API31 | PASS for source/Chromium/Firefox non-regression | Not deployed to live0.12.6 runtime; no browser success-path scenario (server fixture covers cursor) | server/web |
| Playlist folders | migrations/0012_playlist_folders.sql; crates/{domain,core}/src; apps/server/src/lib.rs; apps/web/src/main.rs; tests/browser/music.spec.mjs | 0.12.3 source | Owner-scoped folder list/create/CAS rename+delete, CAS playlist move, unfiling bumps playlist revisions; API folder test; Chromium42 scenarios (41 pass1 skip2.2m) and Firefox42 (41 pass1 skip2.6m) incl create/move/filter/rename/delete at320px | PASS for source/Chromium | Not deployed to live0.12.3 schema11; native folder UI absent | core/server/web |
| Listening device grants/revocation | crates/core/src/listening.rs; crates/domain/src/listening.rs; apps/server/src/listening.rs; apps/web/src/listening.rs; tests/browser/music.spec.mjs | 0.12.2 source | Owner-scoped GET devices (counts/last activity), DELETE with privacy-revision CAS, cascade sessions, revision bump, stale409/unknown404; G07 seeds a second real grant, lists it, revokes with confirmation at320px, and history/DSP/export erasure still pass; Chromium40 passed1 skip2.4m | PASS for Chromium source run | Firefox not rerun; packaged runtime not upgraded | core/server/web |
| Home/root playback/exclusions | apps/web/src/personal_mixes.rs; tests/browser/music.spec.mjs |902dd47 |Chromium37; G07 rendered actual rotation/erase; PC01 real playback/hide/undo/restore/export/catalog race; PC02 held read cancellation | PASS for source run | Preserve exact package two-engine gates / main |
| SQL performance | media/src/recommendations.rs; examples/benchmark_recommendations.rs | SHA256 in final resource JSON |10k/100k/1m actual release + baseline projection/plan equality; raw samples/hardware/scope in PERFORMANCE | PASS within recorded scope | Tied timestamps/dense history and calibrated quality / main |
| Visual/platform scope | tests/browser screenshots; PLATFORM_MATRIX | Final screenshots pending | Source shots inspected; corrected capture for CSS transition settling; Docker daemon and in-app browser absent | PASS for recorded browser scope | Native/Safari/screen-reader remain unverified / main |

Preserved failed runs remain FAILED, not passing evidence. Original20tracks and20analysis
rows were backed up/restored; frozen0.9 runtime restored separately from0.10 candidate.
Private defaults were unchanged. Full native clients and public federation remain NOT_RUN.

# Current mandate evidence

## Golden-scenario coverage map (2026-10-18 checkpoint; G14/G15 reconciled2026-10-19)

Mapping of all24 golden scenarios to executed evidence and honest status. Historical
EXP/PLAY/PLNAV/RES/NET/AUTH/NP regression IDs and their browser scenarios are the
executable substitutes where a golden cannot run on this host.

- G01 migration/rollback: EXECUTED (0.10→0.12.6 chain; per-step backup/restore/counts).
- G02 web upload/FLAC/artwork/playlist/stream: EXECUTED (browser suite + API tests).
- G03 URL import/cancellation/SSRF: EXECUTED (jobs + malicious URL + importer tests).
- G04 personalized Home/rotation: EXECUTED for owned signals; **G10-style fabricated
  "Daily Mix" is intentionally absent** — onboarding/discovery is honest, not fake.
- G05 queue/mini-fullscreen/Now Playing races: EXECUTED (EXP/PLAY/NP browser scenarios).
- G06 concurrent playlist revisions/view-only/revocation: EXECUTED (browser + API).
- G07 private history consent/exactly-once/export/delete/erasure: EXECUTED (G07).
- G08 DSP on sine/silence/click/licensed songs: EXECUTED for synthetic fixtures and
  real FLAC/MP3 decode; STFT/RMS/tempo/key descriptors with version/confidence.
- G09 track radio from analyzed local tracks: EXECUTED (measured/tag/fallback reasons).
- G10 onboarding + meaningful "Daily Mix": PARTIAL — real qualified listening drives
  rotation/discovery; there is no scheduled/branded mix; not claimed as equal parity.
- G11 A↔B federation opt-in/discovery/playable public track: EXECUTED for two isolated
  instances (native+web) on loopback; EXECUTED for TLS via debug CA; **Internet/public
  DNS/two physical hosts NOT_RUN.**
- G12 cross-server search origin/availability/partial results: PARTIAL — unified native
  search with per-peer cursor pagination and cached fallback exists; merged global
  cursor and progressive partial-results UI are absent.
- G13 cross-server playlist + B disconnect status: NOT_RUN (design checkpoint; folders
  and metadata are local-only; remote playlist references are not created).
- G14 forged peer/replay/SSRF/huge object abuse: **PARTIAL — executed subset only**
  (reconciled2026-10-19): public-DNS/redirect validation, SSRF address rejection,
  oversized/128KiB body and capacity limits, hostile/slow peer, metadata-only
  projection, spoofed origin/ID and grant mismatch, revocation, via network/security
  tests and PN01/PN02. **Absent/NOT_RUN:** HTTP-signature actor authentication, forged
  actor identity and signed-request replay (no signature/replay protocol exists), and
  Internet public-CA/two-host validation. Not full G14 acceptance.
- G15 federated public recommendation/revocation/no cache leak: **PARTIAL — executed
  subset only** (reconciled2026-10-19): explicit publication revocation → catalog/
  stream404 and public-first/private-default behavior (publications/two-node tests;
  web revoke flow). **Absent/NOT_RUN:** a federated/public recommendation model (none
  exists) and cross-node cache-leak/tombstone validation; native cached peer metadata
  is browse-only with no play action and no background refresh.
- G16 Android APK on device/emulator: BLOCKED (no Android SDK/NDK/Gradle on host).
- G17 iOS simulator/device: BLOCKED (no macOS/Xcode).
- G18 desktop packages on each platform in CI/physical: PARTIAL — Linux tar + WebKitGTK
  launch EXECUTED per release; Windows/macOS installers BLOCKED; remote CI NOT_RUN.
- G19 queue/device handoff phone↔desktop: NOT_RUN (no second platform).
- G20 collaborative room/participants/latency: NOT_RUN (collaborator grants tested).
- G21 100k benchmark/audio-usable-under-load: EXECUTED for metadata/rotation benchmarks
  and document scope; a fresh 100k multi-server run NOT_RUN on this host.
- G22 licensed provenance/uncertain-rights-private/takedown: EXECUTED (publications,
  revocation, invalidation paths).
- G23 keyboard/screen reader/Reduced Motion/localization/320px/theme: PARTIAL —
  keyboard/Reduced Motion/theme/320px executed; localization and screen-reader runs
  NOT_RUN.
- G24 update/backup restore/graceful shutdown/resume/failure recovery: EXECUTED per
  release (backup restore, SIGTERM, restart, failure-recovery tests).

Summary of honest gaps: G10 full parity, G12 merged cursor/partial UI, G13 cross-server
playlists, G14 forged-actor/signature-replay acceptance, G15 federated recommendation/
cache-leak validation, G16/G17/G19 platform evidence, G21 fresh 100k multi-server run,
G23 localization/screen-reader. Everything else is EXECUTED to the evidence scope
stated above, not beyond it.

The latest master mandate has24 golden scenarios. Historical G01–G14 numbers in older docs/tests belong to the earlier UX brief and do not automatically satisfy the new meanings. Preserve named regression IDs EXP/PLAY/PLNAV/RES/NET/AUTH/NP. New history uses G07-private-history, DSP G08-audio-analysis and radio G09-track-radio.

| Feature | Exact paths | Latest verified source | Executed evidence | Status | Next / owner |
| --- | --- | --- | --- | --- | --- |
| Baseline regression | workspace; tests/browser/music.spec.mjs | f6cc845 / app ac31d74 | cargo test --locked --workspace -- --test-threads=2:110; real Chromium33 with licensed live import | PASS | Repeat after integration / main |
| Data safeguard | scripts/backup.py; data; target/upgrade-backups | f6cc845 | New before-listening-20261017T085256Z.tar.gz restored in disposable dir; schema7,1user,20tracks/assets,1artwork,0playlists; quick_check ok, FK0; listening-backup.json | PASS | Preserve backup and frozenruntime / main |
| Private history service/API | core/src/listening.rs; domain/src/listening.rs; migration0008; server/src/listening.rs |bc50564 | Full workspace157; real TCP consent/CAS/dedup/ownership/retention/export/delete; final Chromium35 + Firefox35 including actual playback/RepeatOne/erasure/stale export | PASS | Preserve regressions, add personal ranking / main+core+web |
| Audible clock/outbox/UI | web/src/audible.rs,listening.rs,browser_player.rs,radio.rs |bc50564 | Strict native/WASM Clippy, final real-browser playback/muting/history-to-radio, bounded queue/session checks | PASS | Device registry/offline remain separate / main+web |
| DSP/worker/radio | media/src/analysis.rs,analysis_service.rs,discovery.rs; migration0009; server/src/analysis.rs |bc50564 |10 extractor/9 worker/7 radio tests + real TCP worker+originals+ACL + browser G08/G09;20 benchmark runs, immutable hashes | PASS | Personal recurring mixes and scale/quality evaluation / main+DSP+core |
| Process lifecycle | server/src/main.rs; scripts/verify_shutdown.py |bc50564 |20 real SIGTERM/restarts; four independent worker DB failures stop process | PASS | Preserve regression / main |
| Browser tool availability | Browser skill runtime | This session | getForUrl returned unavailable; documented browsers.list returned[] | BLOCKED in-app | Existing isolated Playwright harness is available / main |
| Native/federation | PLATFORM_MATRIX; FEDERATION_PROTOCOL; NETWORK_RIGHTS | No implementation | Host/tool probes only; no two-instance or native audio test | NOT_RUN | Implement independent milestones; no completion claim / main |

Logs remain in ignored target/checks/listening-*. Do not delete target: it also holds original private backups, release archives and active frozen runtime snapshots. Final commands/counts/source commit/runtime and remaining limitations are recorded in PROGRESS; preserved failed attempts are not passing runs.


Exact local package/runtime checkpoint: source bc50564979a9bcead7ed6667d6ae673df374e265; frozen extracted binary passed another35 Chromium/35 Firefox scenarios, setup/schema9/assets/SIGTERM and all release checksums. Original data was backed up/restored/logically compared, then upgraded in place without reset:1 user,20 tracks/assets,1 artwork,0 playlists; integrity/FK pass. Supervised loopback0.9.0 PID114613, eight served asset hashes match, no auto consent/analysis. Evidence: listening-upgrade-{backup,runtime}.json and listening-upgrade.log. Reinspect runtime on resume.
