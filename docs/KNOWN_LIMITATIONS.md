# Unsupported and unverified functionality

## DR-02 the underlap and the two glass bands — 2026-10-22

The package lets the catalog run under the topbar and the player and paints
those two bands with the DR-01 glass roles. The measurements are in
[QA_EVIDENCE](QA_EVIDENCE.md) and [PERFORMANCE.md](PERFORMANCE.md); what was
**not** measured is here.

- **The frame budget is a software-raster number, and a GPU-backed stand is
  NOT_RUN.** The renderer on this host is SwiftShader in every launch mode
  (`WEBGL_debug_renderer_info`), so the +0.50 points (Chromium) and 0.00
  points (Firefox) the glass costs here say nothing about a browser that
  composites on a GPU. The plan's 5-point budget is met on this stand and is
  not claimed for any other device. The Chromium gate run read three frames
  over 33ms in a ten-second scroll; one sample cannot say whether they
  belong to the filter or to the software rasteriser.
- **The runner-vs-plain-process difference was measured against the
  filter-only fallback, not the opaque one.** The diagnostic run of
  2026-10-22 read 0.00 points in a plain process (602 frames, the median
  16.7ms, the longest 16.8ms) and 15.7–24.4 points inside the Playwright
  test runner for the same page, browser, driver, server and session. Its
  fallback column removed only `backdrop-filter`, so that A/B isolates the
  harness, not glass against opaque. The budget gate itself now compares
  the glass with the opaque roles and asserts the two painted surfaces
  differ.
- **The contrast in `design.spec.mjs` is a bound, not a sample.** For a
  surface that carries a `backdrop-filter` the suite composites the text's
  own stack over **white and over black** (the extremes of sRGB) and asserts
  the worse of the two, because any cover can scroll under the band; for
  every other surface it keeps the original reading against the first opaque
  ancestor. The real-artwork measurement is the separate journey in
  `dr02-glass.spec.mjs`, which uploads a real FLAC with a real 800×800 cover,
  reads the artwork's own colour and also samples the painted pixels under a
  hidden band. A colour outside the sRGB extremes (a wide-gamut image) is
  not modelled by either.
- **The opaque fallback is not automatic.** It is what a reader with
  `prefers-reduced-transparency: reduce` or `forced-colors: active` gets,
  what `print` gets, and what an engine without `backdrop-filter` gets. No
  CSS feature exists that asks a browser whether it can afford a blur, so a
  slow device keeps the glass until the reader asks for less transparency.
- **Firefox renders the blur differently from Chromium.** Both engines
  pass the same journeys — two filters, the effective rectangle, the
  composited contrast, the modes and the preserved source — but the frames
  are not expected to be pixel-identical, and they are not compared.
- **`prefers-reduced-transparency: reduce` was not exercised.** Playwright
  has no switch for it on this host, so the rule was read in the stylesheet
  and not run. `forced-colors: active`, `prefers-reduced-motion: reduce`
  and `print` were run in the live document.
- **A screen reader was not run.** The bands changed their surface and
  their caption colour; the roles, the names, the live regions and the
  focus order are unchanged, but nothing here is a screen-reader claim.
- **The native client is not covered.** `apps/desktop/style.css` keeps its
  own `--vela-*` vocabulary and was not touched: the native underlap and the
  native glass are DR-08's scope, and nothing here says a native window
  renders a glass layer.
- **The underlap is a layout change, not a data change.** No API, no
  migration, no manifest, no static asset and no `media_assets` row was
  touched; the two bands are styled by the existing `apps/web/style.css`,
  which `scripts/build-web.sh` repackages into `dist/`.

## DR-01 the glass and motion token layer — 2026-10-22

DR-01 declares the vocabulary and applies none of it. The measurements are
in [QA_EVIDENCE](QA_EVIDENCE.md); what was **not** measured is here.

- **The glass does not exist yet, as of DR-01.** When DR-01 was written no
  surface read `--vela-glass-*`: no `backdrop-filter` was applied and no
  floating layer was rendered. DR-02 has since applied the roles to the
  player and the topbar and measured them over artwork; this section
  describes the state DR-01 shipped, not the current one.
- **The alpha is arithmetic, not contrast.** 0.851 (dark) and 0.902 (light)
  are the alphas the two declared hex values state. No contrast ratio was
  measured against any image, and none is claimed: the quiet 10px
  secondary text over a 0.851 background is exactly the case DR-02 has to
  measure with primary text on the composited surface.
- **The durations are not used by any component.** `--vela-motion-*` and
  `--vela-ease-*` have no reader, so the reduced-motion block that zeroes
  them changes no rendered transition today. The block is there for the
  packages that will read them, and it does not replace the existing
  global `animation:none` / `transition:none !important` overrides: a zero
  token cannot stop a JS or Web Animations driver, so DR-03 still has to
  exercise the media query itself.
- **The pixel comparison has a noise floor, and one raw frame pair differs
  by more than it.** The default library order is `ORDER BY t.id ASC` over
  random UUIDs, so two runs of the same flow show the same rows in a
  different order; the frames are therefore not bit-reproducible between
  independent runs. The pre-edit and after-edit captures differ by 116–148
  pixels in one narrow strip, and that strip is the row-number column where
  `…01` and `…03` exchanged places: it is sort order, not styling. The only
  clean comparison is the controlled A/B in one live document, where the
  stylesheet is swapped in place and the rows cannot move; there the
  shipped-vs-origin/main diff is 23 pixels in the same bounding box as the
  control's own shipped-vs-shipped diff, and one pair of frames is
  byte-identical. That is what "the change renders nothing" means here — it
  is not a claim of byte-identical screenshots between two independent runs.
- **The full browser gate was closed by the team lead, not by the package.**
  The worker's own full Chromium run died with `ENOSPC` while Playwright
  wrote a trace (the host had ~6.7 GB free), and its retry lost the server
  connection mid-suite with 43 scenarios unrun; the worker's full Firefox
  run never finished. Both suites were then re-run by the team lead with
  the Playwright output redirected away from the full disk, and both
  passed; the failed attempts are recorded as failures, not folded into
  the passing count.
- **The native client is not covered.** The native tokens are DR-08's
  scope: `apps/desktop/style.css` keeps its own `--vela-*` vocabulary and
  was not touched, and nothing here says a native window renders a glass
  layer.
- **A screen reader, `forced-colors: active`, physical audio and Docker
  were not run** on this host: there is no X server, and the DR-01 roles
  have no consumer to announce.
- The design system's historical mint/sage table and the prototype
  transfer remain history. They are not the source of the DR-01 hex
  values; the source is the current `apps/web/style.css`.

## DR-06 the mobile composition — 2026-10-22

The package moves the music into the first phone screen. The measurements are
in [QA_EVIDENCE](QA_EVIDENCE.md); what was **not** measured is here.

- The safe-area numbers are **SIMULATED**. This host has no cutout and no
  device, so the 34px bottom inset, the 24px top inset and the 16px side
  insets were emulated through the shell's own `--vela-safe-*` variables —
  the same variables `env(safe-area-inset-*)` feeds. The geometry is the
  geometry a notched device would lay out, but no hardware was involved, and
  a real home indicator, a real cutout and a real landscape rotation were
  not exercised.
- No real mobile browser was used. The measurements come from a headless
  Chromium and a headless Firefox at fixed viewports. A mobile browser with a
  dynamic toolbar, a URL bar that collapses, an onscreen keyboard and a
  device-pixel ratio other than 1 were not tested. The shell already divides
  its viewport units by the app zoom, but the interaction between a dynamic
  `dvh` and the keyboard was not observed.
- The idle sentence of the mini-player reads in full at 100% app size on a
  320px phone, and it **wraps** instead of being cut: the two engines measure
  the same Manrope text differently (Chromium 87px, Firefox 89.87px at
  12px/600) and the title path is 88px there, so the sentence takes two lines
  of 18px in Firefox and one in Chromium. The band still keeps its 60px of
  content above the 44px seek rail. The **current** track's title keeps the
  standard ellipsis instead, and the full name stays in the DOM and in the
  `title` attribute.
- The plan's `.section-toolbar { margin-bottom: 0 }` was applied to the
  **library's** section toolbar — the pre-list the plan measures — and not
  to every section toolbar on the phone. The other pages keep their section
  rhythm: their toolbars are followed by content that has no margin of its
  own, so a global zero would have pulled every heading onto its first card.
  The library's own toolbar, the search row and the filter block were
  measured; the other pages' spacing was not re-measured after the change.
- The 761..850px band has no queue control in the player bar. That is the
  existing composition — the mini queue is hidden below 900px and the
  desktop right zone below 850px — and this package did not change it. The
  queue stays reachable from the Now Playing sheet and the drawer, and the
  band was not re-designed here.
- The 150% evidence is the application's own app size, a CSS `zoom` on the
  document root. A real 150% system font scale, a browser zoom of 200% and
  `forced-colors: active` were not exercised, and a screen reader was not
  run: the announcement claims about the default library order are DOM facts
  — the node is in the DOM, its `role` is `status`, its text is unchanged —
  and not observed announcements.
- The 80% app-size evidence reads the row targets as `rect / zoom`. The app
  size is a CSS `zoom` on the document root, so `getBoundingClientRect()`
  already reports the zoomed space; dividing once returns CSS pixels. A
  check that multiplied by the zoom instead counted it twice and failed a
  correct 44px target at 28.16px — the assertion, not the UI, was wrong.
- The native client's WebKitGTK rendering, its physical audio and the Docker
  runtime were not run: this host has no X server.

## DR-04/DR-05/DR-07 audit round — 2026-10-22

The five defects of the independent audit were closed with source changes and
with measurements of the live document in Chromium and in Firefox: the
duplicate time formatter is gone, the drawer has no closer of its own, every
declared rhythm role is consumed with the value it already had, the size map
and the volume paragraph say the same thing, and the root receipt is a
permanent live region.

The second audit of the same tree found three more point defects, all closed
here: the recovery status and the Undo receipt are permanent live regions
like the ordinary receipt, the drawer and the root share one lookup of the
drawer's native dialog, and the Now Playing trigger limitation below says
what the code does. What was *not* measured is stated here:

- No screen reader was run. The live-region claims are DOM facts — the node
  is in the DOM before the first message, only its text changes, the empty
  row is out of the flow — and not observed announcements. A person using
  a screen reader is the only judge of how the rows are actually read.
- `forced-colors: active` was not emulated. The focus rules are unchanged,
  but the rendering of the collapsed receipt and of the 44px boxes in forced
  colors was not observed.
- Browser zoom 200% was not exercised; the application's own 80/100/150%
  size was, and the browser zoom is a separate mechanism.
- The native client's WebKitGTK rendering, its physical audio and the Docker
  runtime were not run: this host has no X server.
- The Now Playing sheet does not restore the focus itself: the native
  dialog's own focus restore hands it back to the element that held the
  focus when the sheet opened. The application freezes that element only to
  donate it to the queue drawer, so that the drawer's own exits return the
  focus to the sheet's opener. The frozen element is whatever held the focus
  at that moment — the code refuses only the document body — so on a platform
  where a pointer click does not focus a button the sheet's own opener is
  *not* the frozen element: the previously focused control is (the player's
  queue button, for example), and the drawer would hand the focus back to
  that control instead of to the sheet's opener. The browser's own focus
  restore is the fallback only when the focus was on the document body
  before the click. Chromium and Firefox focus a clicked button on Linux,
  and the path was measured in both; no other platform was measured.
- The shell is a query container (`container-type: inline-size`). The side
  effect of that property is narrower than it is often described:
  `container-type: inline-size` applies **style** and **inline-size**
  containment, and it does **not** apply layout or paint containment. The
  shell is therefore *not* the containing block of its `position: fixed`
  descendants and it does *not* establish a stacking context. This was
  measured, not assumed: in Chromium a `position: fixed` child of a
  `container-type: inline-size` element is anchored to the **viewport** (its
  right edge is the viewport's right edge, not the container's), a negative
  `z-index` descendant escapes the container and paints below the
  container's own background, and `getComputedStyle(shell).contain` reads
  `none` with `container-type: inline-size`. The same probe with
  `contain: layout style inline-size` behaves differently on both counts
  (the fixed child is anchored to the container and the negative descendant
  stays inside it), which is what shows the two cases apart. The root's
  feedback stack, the mobile track-menu backdrop and panel and the native
  dialogs with their own fixed panels are therefore fixed layers of the
  **viewport**, and the shell's own `overflow: hidden` does not clip them.
  Style containment does scope counters and quotes to the container, which
  this application does not rely on.
  The consequence that was measured and is not a defect: the shell is
  exactly `100dvh / zoom` tall, so at 100% and 150% the container box and the
  viewport coincide, and the DR-04 and DR-05 measurements found no overlap
  between the stack, the player and the mobile navigation. A browser with a
  dynamic toolbar was not measured. `dvh` is the *dynamic* viewport size —
  it tracks the viewport as the toolbar shows and hides — and it is **not**
  the largest size in the `vh` family (`lvh` is); the dynamic viewport already
  accounts for the toolbar, so the visible area is not automatically shorter
  than the shell, and no claim is made about that case.
- The shell's container query is evaluated one layout late in Firefox when
  the app size (a CSS `zoom` on the document root) and the window width
  change together: at a 150% app size in an 800px window the volume was
  still drawn until the page was reloaded, while Chromium hid it at once.
  On a settled layout — a reload with the size kept — both engines agree:
  the volume yields below 600px of layout width, the queue and the mute
  stay. The permanent journey measures the settled layout, and the
  difference is recorded here rather than hidden.

## DR-04 dense desktop control with real 44px targets — 2026-10-22

The package was measured as geometry: the live rectangles of every visible
control, the intersection area between the desktop transport and the seek
range, the height of the player band, the navigation step and the type of the
headings were read from the running document in Chromium and in Firefox. A
screen reader was not run, so the announcement of the new targets is a
source-level guarantee, not a measured one. `forced-colors: active` was not
emulated: the rules that paint the focus ring exist and are unchanged, but the
forced-colors rendering of the 44px boxes was not observed. Browser zoom
200% was not exercised; the application's own 80/100/150% size was, and the
browser zoom is a separate mechanism. The native client's WebKitGTK rendering,
its physical audio and the Docker runtime were not run: this host has no X
server.

Three deliberate exceptions to the desktop density are recorded rather than
hidden:

- The coarse-pointer composition keeps its 8px transport gap and its 19px
  glyphs. It is the accepted touch adaptation, and the desktop composition
  owns the 4px gap and the 15/17px glyphs. The two are not the same
  measurement, and the "236px transport" number belongs to the fine pointer.
- The card and icon surfaces (the job card, the import preview, the job icon)
  keep their 9px radius. They are surfaces, not controls, and only the
  control, input, button and track-row radii moved to 8px.
- The Home title's fluid `clamp(30px, 3.1vw, 45px)` reaches 45px, which is
  above the 19–42px range of the design directive. That is a documented source
  exception and this package did not change it.

The app size also scales the **hit boxes**. The 44px target is a size in the
layout coordinate system, but `getBoundingClientRect()` returns the scaled
rectangle, so at an 80% app size a 44x44 layout box measures **35.2x35.2** in
the viewport. Measured over the 768/1284/1440/1728 windows, at an 80% app
size **50 of 51** and **52 of 53** visible interactive targets measure below
44px; at 100% and 150% the count is **0**. The browser zoom is a different
mechanism - it shrinks the CSS viewport instead of the elements - so at
80/100/150% browser zoom the count is **0** everywhere. The 44px guarantee
belongs to the 100% layout; a user who asks for an 80% interface gets smaller
hit boxes with it, and this package did not change that.

The app size is a CSS `zoom` on the document root. It scales the layout but it
does not move the viewport media queries, so at a 150% app size in a 1284px
window the content is laid out in 856px while the breakpoints still answer
"desktop". This package made the *player* follow the real layout width: the
shell became a query container, so the transport column is sized by its own
content below 1100px of layout width and the control intersections are 0. The
rest of the application still answers to the visual viewport, which is a
property of the app-size feature and belongs to the package that owns it.
Below 600px of layout width the player's volume yields first — the queue and
the mute stay, and the volume remains reachable in the queue drawer and in the
Now Playing sheet — because the desktop transport and the whole utility zone
do not fit side by side in that width.

The mobile composition was not redesigned here. At 320px the first track is
still cut off by the mini-player band; that is DR-06's scope, and this package
kept the 120px band, the 8px transport gap and the 44px navigation targets
unchanged.

## DR-07 keyboard readable queue drawer — 2026-10-21, second review 2026-10-22

The drawer's focus ring, its two rails, their visible names, its Tab order and
the focus return after Escape were measured against the live DOM in Chromium
and in Firefox; both engines report the same ring, the same rectangles and the
same focus return. A screen reader was not run: the accessible names, the
announced values and the focus returns are a source-level guarantee, not
a measured announcement. The `forced-colors: active` rule uses the system
`Highlight` role, but the mode itself was not emulated, so the painted result
in a forced-colors desktop is unverified. A 200% browser zoom and a 150% app
scale were not exercised for this package.

The two real media fixtures of this repository are one second long, so the
`m:ss` position format cannot show a sub-second position: a rail at `0.1`
still announces `0:00`. That is the format's own resolution, not a wrong
reading. A track with a real length would exercise the same code path
unchanged, but it was not available on this host.

The root's own player seek input was not touched. It already carries the
accessible name `Playback position` and the same `m:ss of m:ss`
`aria-valuetext`, but an unknown duration is announced there as `0:00 of 0:00`
instead of the drawer's `Duration unknown`. The two surfaces therefore do not
say the same thing about an unknown length; only the drawer was in this
package's scope.

Four of the drawer's exits hand the focus back to a connected trigger:
Escape, the close button, the Now Playing sheet's `View full queue`, which
opens the drawer from the sheet and hands the focus back to the sheet's own
`Open now playing` trigger, and the `Open library` action, which is drawn
only while the queue is empty and now goes through the same closer: the
native dialog leaves the top layer before the drawer leaves the DOM, and the
navigation to the library does not strand the focus on the document body
(measured in Chromium and in Firefox). The focus round trip itself is the
root's: the root freezes the control that opened the drawer and restores it
through its own guarded path, so the drawer keeps no second copy of that
convention.

The sleep countdown is a `timer` with an explicit `aria-live="off"`: it
ticks every second, and a polite live region would read "4:58", "4:57",
"4:56" without a pause. Its accessible name carries the remaining time, so
a person who navigates to it still hears that the timer runs. The start, the
cancel and the expiry are one meaningful message each, written by the root
into one `role="status"` line inside the drawer, because the modal
background is inert while the drawer is open and the root's receipt cannot
say anything from there. The root decides where the message goes: the line
inside the modal while the drawer is open, the root's receipt while the
drawer is closed, never both at once, so the message is never announced
twice. The consequence is honest and small: a sleep message that is
produced while the drawer is open lives only in the drawer and is gone when
the drawer closes — the cancel in particular no longer leaves a receipt in
the feedback stack. The countdown's own role, the one status line and its
text are DOM-level guarantees; that a screen reader says them was not
measured, because no screen reader was run.

At 320px the queue row's title is truncated to a single character by the
fixed 44px row actions next to the 38px cover. That is the pre-existing
narrow-row layout, not a regression of this package, and it was not redesigned
here.

The drawer's announcement line is a permanent live region: it is in the DOM
before the first transition and only its text changes, so the announcement
depends on a text change and not on the insertion of the region together with
its content. The empty line is collapsed to zero height without a margin and
takes no space, which was measured in both engines at six viewports. That a
screen reader speaks the changed text of a region that was already there was
not measured: no screen reader was run.

The drawer and the row menu keep two different frozen triggers, so a row menu
that is open behind the drawer no longer takes the drawer's focus return. The
menu itself is not closed by the drawer: at desktop widths it is the
anchored panel it has always been (it has no backdrop there), it is inert
while the modal drawer is open, and it stays open behind the drawer. That is
the pre-existing surface behavior, not a regression, and the redesign of that
anchored panel was not in this package's scope.

The current row's tint and the next occurrence's outline are now the
project's own roles, `--vela-track-current` and `--vela-track-next`, with a
value per theme. Measured over the panel they sit on, the light theme's
current row is 1.07:1 and its outline 1.34:1 against the white panel (the
hardcoded pistachio they replaced read 1.005:1 and 1.055:1), the dark
theme's 1.05:1 and 1.74:1. Both are decorative tints, not text and not the
focus ring, and they stay below the 3:1 that a non-text indicator would
need: the row that is also the next occurrence is marked, but it is not
announced by contrast, and the redesign of that marker was not in this
package's scope.

The native desktop client, its WebKitGTK rendering, its physical audio and the
Docker runtime were not run: this host has no X server. The browser suites
needed an audio device for Firefox, so a PulseAudio null sink was started for
the run; the sink is a test environment, not a product capability, and no
physical audio output exists here.

The drawer's trigger is frozen from `document.active_element()` at the moment
the drawer opens. Chromium and Firefox on Linux focus a button on click, so
both measured engines return the focus to the right control. A WebKit engine
that does not focus a button on click would freeze a different control, or
freeze nothing at all and the focus would not return. The row menu does not
have that exposure: it resolves a stable element id instead. Only the two
measured engines were checked, so this stays unverified for WebKit/Safari
until the native parity package runs against a real WebKitGTK window.

## DR-05 readable states and feedback stack — 2026-10-21

The English dictionary covers the system's own labels in the editorial card,
the wave card, the queue drawer, the idle player and the library/sidebar
counts. It is not a localization framework: real playlist, track and artist
names stay in the script their owner gave them, and the rest of the interface
still carries English strings that were outside this package's scope. A
Russian-speaking account therefore sees a mixed-language shell, exactly as
before this change, only with the touched states consistent.

The contrast fix is scoped to the two empty card variants. The image-backed
card keeps its light-on-artwork colours and its shade, and its contrast
against the artwork image was not re-measured: the measurement walks opaque
ancestors and cannot composite text over artwork.

The feedback stack was measured in Chromium and Firefox with a genuine
aborted stream, a real add-to-playlist and a real add-to-queue. A screen
reader was not run, so the announcement order of the three live regions is a
source-level guarantee, not a measured one. The layer's own bounded overflow
was proven with a deliberately tall message — it capped itself, scrolled
internally by wheel and by keyboard focus, and never reached the player or
the navigation — but a real recovery message at a 150% app scale was not
exercised.

The Now Playing recovery row was verified at 1284×856, 1440×900 and 1728×1000
with a normal track title. At a 720px dialog height the Retry is still below
the fold and the dialog keeps its own scroll; that is the documented fallback
for long titles and large text, not a measured regression. The artwork's height
reserve was re-derived from the dialog's real height budget, so a very tall
title or a large app scale can still push the recovery row into that scroll.
Firefox lays out the sheet's secondary column taller than Chromium does, so
the dialog still scrolls 109px at 1284×856 there; the recovery row itself is
fully inside the fold in both engines.

The GTK development packages and `pkg-config` are now installed on this
host, so `cargo clippy --locked --workspace --all-targets -- -D warnings`
and `cargo test --locked --workspace -- --test-threads=2` both complete for
the whole workspace, `apps/desktop` included. The native desktop client was
compiled and unit-tested, but its WebKitGTK rendering, its window and its
physical audio were not run: this host has no X server and no working
PulseAudio server. The Docker runtime was not run either.

## Empty library state — 2026-10-21

The empty-library Home state is verified by source, unit tests, strict Clippy
and a release WASM build only. No browser session, screenshot or screen-reader
pass was run, so the 320/360/390 px and desktop layouts, the keyboard focus
order and the 44 px target are source-level guarantees (the reused
`.button` rule already carries `min-height:44px` and the global
`button:focus-visible` outline), not measured evidence. A real import in
flight was exercised through the projection unit tests, not through a live
download: the byte counters come from the server's own job rows and a task
with an unknown total renders no progress bar at all.

The empty state only appears once the server confirmed the library is empty
(`/api/stats` reports zero tracks and zero albums). Until that count arrives
Home shows its bounded placeholders and the personal-mixes section keeps its
normal read, so the first paint never claims an empty collection nor hides a
real one. Uploads and completed imports refresh `/api/stats`, so the state
clears as soon as real music exists; a browser session was not run to confirm
that transition live.

The «Your next listening chapter» section is suppressed for an empty library.
It is a browser-web feature only and remains absent from the native client, as
recorded below.

## Distribution pipeline 2026-10-20

Distribution 0.12.8 is public with EXE/tar.gz/DEB/AppImage, a durably signed arm64 APK and AUR `vela-music-bin` 0.12.8-1 (resolved through yay). Windows EXE/ZIP is portable, unsigned by Authenticode and requires WebView2. Ubuntu 24.04+ DEB/tar/AppImage uses host GTK3/WebKitGTK4.1/ALSA/libudev/libxdo.so.3; AppImage does not bundle these libraries. AUR carries a private compatible libxdo.so.3 because current Arch uses libxdo.so.4. Native packaging/CLI smoke is not GUI/device/audio/background certification. Docker image build passed; runtime was not run. APKs built without repository signing secrets remain explicit ephemeral previews. See [DISTRIBUTION](DISTRIBUTION.md) and [release evidence](RELEASE_LEDGER_0.12.8.md); dated platform rows below retain their historical scope.

## Interaction/window source repair2026-10-19

The newer [interaction review](UX_INTERACTION_REVIEW_2026-10-19.md) supersedes the
earlier NOT_RUN statement below only for the tested normal tiled Hyprland Wayland
subset: frameless window, reachable controls, row pause/resume, seek/source continuity
and no horizontal overflow. Full peers/offline and responsive native checks remain
separate X11 fixtures with a null sink. Physical sound, other compositors, screen
readers and non-Linux platforms remain unverified.

Web/native current-row transport, pending native intent and native per-entry group/
playlist playback are repaired. Context Play remains separate. Native public sources
still have no shuffle/repeat; offline supports sequential playback and repeat-one,
not shuffle/repeat-all. Controls/MPRIS reflect these limits. Public MPRIS metadata/
transport remains unsupported. Stable duplicate playlist-entry provenance during
concurrent reorder needs further work.

The frozen server and already running user desktop were not upgraded. Restart the
updated desktop binary to see the frame change. Progressive native playback,
next-track prefetch, durable native queue restore, artwork and handoff remain work.
Exact passing runs are in PROGRESS; this is not whole-product UX certification.

## Contrast UI verification2026-10-19

The newer Better Design implementation replaces the earlier muted current-source
palette and improves layout/control sizing; see [dated review](DESIGN_REVIEW_2026-10-19.md).
This is source/debug fixture verification, not a live frozen-runtime upgrade.
Measured text/control contrast and DOM geometry are scoped regressions, not a WCAG
or screen-reader certificate. Native artwork still uses neutral placeholders;
native audio remains complete-original preparation rather than progressive playback.
Native QA uses X11/XWayland under Hyprland with an owned floated window and null
sink; native-Wayland tiling, physical audio, iOS/Android/Windows/macOS and Docker
were not validated by this design pass. Device handoff/prefetch remain roadmap work.

Source audit2026-10-19: [outstanding findings](PLANS/2026-10-19/01_AUDIT.md) and
[ordered repair regressions](PLANS/2026-10-19/02_REPAIR_PLAN.md) include native
keyring/lifecycle/offline issues still awaiting fixes. Documentation reconciliation
2026-10-19 (docs-only): use [CURRENT_STATE.md](CURRENT_STATE.md) for the
source/deployed/design split. Historical statements below that conflicted with the
implemented native offline copies, opt-in Secret Service session and playlist folders
are corrected in place; older sections remain as dated history. Proposed fixes/design
are not completed features, and no code defect found by the audit is fixed by this
reconciliation.

Current0.9 adds explicit private account/device listening consent, bounded cumulative events/history/export/delete, optional durable local PCM/FFT analysis and bounded owner-only track radio. Exact executed checks are in PROGRESS. Collection is conservative and can undercount short/throttled/background playback; it cannot verify physical sound. Pending browser summaries are plaintext, capped100/128KiB, may drop the oldest at capacity and clear on logout/opt-out. History caps10000 retained sessions/account and32 device grants; Settings lists active grants and revokes one with privacy-revision CAS (deleting its sessions), while bulk device management is only the existing delete-all-and-disable path. Reads currently take a short SQLite writer lock to prune; contention is not benchmarked. DSP uses at most a120-second prefix by default, cooperative deadlines and no process memory sandbox; unsupported decoders fail safely. There is no per-job analysis cancel/pause UI; shutdown cooperatively cancels/drains and requeues interrupted work. The unified Jobs page still projects imports/scans only; analysis has its own real status/counts on Radio. Real decoder fixtures cover WAV/FLAC/MP3; Opus analysis is unsupported even where a browser plays it. Tempo is a bounded onset-regularity estimate, not a validated beat annotation; chroma is nearest-semitone pitch-class energy; key is an uncalibrated Krumhansl-profile heuristic (no tuning/chord transcription), and there is no genre/mood inference, LUFS/normalization or exhaustive million-track similarity index. Radio considers at most500 newest other owned tracks, with metadata fallback; personal daily/weekly mixes and history-based ranking remain absent. These additions do not establish native or federated functionality.

The0.5 listener increment adds a real Home, separate local Search, five-tab phone navigation where More opens Settings and the remaining destinations live in its More from Vela list, and next/last queue actions. This is not complete consumer-app parity. Home uses current additions/favorites/albums/playlists, not listening history or personalized mixes. Search returns at most8 per category without pagination; group/playlist matching is ASCII case-insensitive literal substring, track matching is FTS. No typo/transliteration/Unicode collation, search history, remote search or group-query latency guarantee exists. The sidebar shows at most6 playlist shortcuts. Version0.7 adds paused account-bound context/position/preferences restoration and bounded browser recovery; route changes retain active audio. Browser seek precision and autoplay policies still apply. Version0.6 adds bounded fair shuffle traversal/history, explicit-next priority and repeat/previous transitions; transient shuffle history rebuilds on reload, while version0.7 retains per-account shuffle/repeat preferences; this is not a durable device journal. During shuffle the visible queue retains saved order with an explicit explanation, not a randomized upcoming preview. TR07 now provides a real row action registry (visible More, right-click, Shift+F10, the
ContextMenu key and the phone action sheet), a searchable add-to-playlist dialog with
receipt and Undo, and a right-side queue drawer over the one root player. A persistent
right Now Playing panel and a complete design-system component suite remain
partial/absent. Version0.8 adds the full root-coordinated Now Playing modal.

Chromium checks cover320/768/1440/1920px dark/light and keyboard More dismissal/containment, not screen-reader certification or all browser/OS combinations. The in-app browser runtime reported no available browser; isolated Playwright Chromium was used against actual native/WASM builds. A later0.8 QA follow-up also ran all33 scenarios in Playwright Firefox157 on Linux; see PROGRESS for the precise scope. One requested parallel reviewer hit the environment usage limit without delivering code; implementation/integration continued in the main agent.

Version0.6.1 verifies playlist route/object/session generation guards, post-mutation revision monotonicity and share-link view isolation; this does not complete invitations/media grants or all legacy client workflows.

This is an early functional slice. The complete master roadmap is not delivered. Consult PROGRESS for executed checks; source/build recipes are not proof of platform runtime success.

- A Linux native desktop package exists with explicit private offline copies (max100/2GiB,0600, account-scoped), an offline Previous/Next queue and confirmed per-copy removal; saved originals play with the server stopped. Offline v2 manifests verify BLAKE3 before playback and a bounded startup sweep flags
corrupt copies; there is no content/entitlement sync, automatic download, bulk storage
manager, Android/iOS hardware validation, device transfer, remote control, PWA cache or background audio. Linux supports explicit opt-in Secret Service session persistence (opaque token only); Windows Credential Manager, macOS Keychain, Android Keystore and biometric unlock are absent.
- Explicit public publication and read-only public-catalog consumption exist (0.11 profile; see "Public catalog0.11 scope"). The native client saves canonical origins, searches them per-origin with cursors and browse-only cached fallback, and plays granted public streams; that remote path is verified only over loopback/private test CA, not Internet public-CA or two physical hosts. The web Network view is a single-origin first20 preview. There is still no ActivityPub-style federation, profiles/follows/activity feed/rooms, automatic peer discovery/WebFinger, HTTP signatures, merged global cursor, connected-server library adapter or OpenSubsonic routes. The live0.12.6 runtime has discovery disabled and0publications; no private content is public without an explicit per-track declaration.
- No direct-reference external libraries, WebDAV/S3/ZIP/Archive.org/feed adapters, cloud playlist metadata adapters or arbitrary commercial-service downloading. Direct HTTP/S audio, browser batch uploads and administrator-configured read-only folder snapshots are implemented ingestion paths. Browser preview knows filename/MIME/size, not pre-downloaded embedded album tags/artwork/license information.
- No FFmpeg transcoding, quality presets, ReplayGain, verified gapless/crossfade, playback-speed control or cross-device session-position synchronization. The web sleep timer pauses this tab’s player only; it is not an OS-level or cross-device timer. Uploaded uncommon containers/codecs may not play in the browser. No full-frame decode verification; later corrupt frames may survive metadata validation.
- Persisted album/artist groups and dedicated screens use trimmed names with SQLite ASCII NOCASE and album-artist/year keys (ADR0004). Unknown years remain provisional separate groups; Unicode case folding, verified editions/recordings, multiple encodings, genres/composer catalog, batch metadata editing and the remaining sort/filter dimensions (duration, size, last-played/play-count) remain planned. Genre/format/year/technical minimums and title/newest orders are implemented for Library/Favorites. Catalog pages50/max100 use UUID keyset cursors; new randomUUID additions may appear before a cursor. Album pages follow disc/track order but edits between pages can move tracks. Queue contains selected pages, not the whole catalog. Missing album tags remain available through Library rather than an invented album.
- Playlists are capped at1000 entries and200 per owner. Named viewer/editor grants and expiring metadata-only links exist; owners can CAS-rename or permanently delete a playlist (entries, collaborator grants and share links cascade; tracks/assets/streams remain) with inline confirmation. But there is no separate media sharing, invitations/acceptance, owner transfer, smart-rule model, activity/revision feed or SSE updates. Owner collaborator list/revocation and member-leave controls exist; stream access remains owner-only. Queue/playlist order uses accessible move-up and move-down controls and Clear-upcoming semantics, not drag/drop. A committed delete cannot be replayed back into existence and delivered bytes/licenses cannot be recalled.
- Exact BLAKE3 duplicates are handled; logical recording/master/live/remix duplicate grouping, acoustic fingerprints, enrichment/ambiguous review, smart playlists, personal recurring recommendations, lyrics, aggregated listening dashboards, general user export/collections/podcasts/internet radio are not implemented. Private listening export/totals and local track radio now exist separately as described above.
- Imports restart from zero rather than resuming byte offsets. Processing cannot cancel once atomic publication starts; automatic retries are limited to transient failures. Pause is unsupported. Conservative pre-download quota checks can reject a duplicate at a completely full budget. Progress uses active-job polling, not SSE.
- Jobs combines only actual durable imports and administrator-owned scans; browser uploads remain in the transfer dialog. No all-user admin queue, task pause, scheduler, SSE or scan retry-in-place exists. Scan again creates a new pass and source removal removes its history while retaining music. Idle pages need Refresh to discover work started elsewhere; hidden pages suspend network polling. Action flags are snapshots and mutations may reject a changed state/quota/source. State-filtered histories are live, so an explicit retry can move a row between filter groups while paging. No task metadata is an audio-access grant.
- Managed budget accounts referenced physical audio only; per-file limits bound staging/artwork, but global thumbnail/orphan accounting and reconciliation remain work. A hard crash between publication and DB commit can leave retained orphan content. No garbage collector deletes source files.
- Current database layout supports one application process and one local SQLite data directory. An OS lock enforces this. Network filesystems/cluster modes are unsupported. Fixed resource limits have not established safe user counts; benchmark scope is explicitly documented.
- Container runtime/build, ARM64, WebKit/Safari, stock-browser/OS combinations and OS system controls and actual mobile hardware must be validated separately; only checks in PROGRESS count. CI has been configured but is not remotely executed/published by this session.
- No independent security audit, backup encryption, encrypted local browser catalog or public production deployment. No software updates performed on external servers.

- **Dependency gate2026-10-19: FAIL for the advisories check.** `cargo-deny` rejected three INFO Unmaintained notices — `zvariant5.15.1` RUSTSEC-2026-0370, `zbus_names4.3.5` RUSTSEC-2026-0371 and `zvariant_derive5.15.1` RUSTSEC-2026-0372 — through `vela-desktop → mpris-server0.10.0 → zbus5.19.0`. Bans/licenses/sources passed. No ignore/exception was added for these three; they are maintenance notices, not published runtime vulnerabilities, and no exploitation is established. Evidence: `target/checks/audit-20261019/dependencies.log`; `deny.toml` keeps its earlier reasoned ignores (paste/GTK/fxhash), none of which covers these notices. Older "dependency policy passes" sentences and dated logs are historical and predate this advisory database. Generated license notices include packaged texts and hash-verified pinned upstream texts for most crates;15 packages currently have metadata-only notices (down from12; provenance in docs/third-party-license-texts). Full grants were not found at their inspected exact commits. External redistribution needs those notice gaps reviewed and a resolved dependency gate. No release was externally published.

- Catalog-only per-track metadata editing is implemented; original tag write-back and bulk edits are unavailable. The editor uses native HTML dialog behavior; non-Chromium accessibility/platform behavior still needs separate validation. Metadata revisions are local-server records; cross-server/offline metadata editing is not supported.

- Folder scanning currently requires Linux and operator-configured local storage roots (default data/sources). It copies audio into managed storage, using additional disk space; it does not stream directly from external directories. There is no filesystem watcher/scheduled scan, arbitrary path browser, source write-back or deletion synchronization. Symlinks/hardlinks/special files are skipped; symlinked root ancestors are rejected. Limits are32 configured sources,32 directory levels,1M visited entries and100k candidate files per pass by default. File signatures are best-effort; deliberate same-signature changes are not cryptographically revalidated. Kernel I/O on broken/network filesystems cannot be forcibly cancelled. Folder scans are administrator-only; ordinary users can upload/import instead.

- Library genre filtering compares complete trimmed tags with ASCII NOCASE; it does not split multi-genre strings or implement Unicode collation. Format labels can identify containers rather than verified codecs; numeric thresholds do not prove losslessness. Unknown years/audio properties do not match explicit bounds. Facets suggest at most100 genres/32 formats and report truncation; an exact genre can still be typed. Title/newest cursors are live: concurrent edits/additions can move page boundaries. Applied conditions persist in the current app session and clear on account changes; they are not an offline sync feature. Album/artist musical ordering is unchanged. First-page year filtering, empty combined matches and grouped facets showed higher SQL costs in the
measured100k-owner workload. AUD-09 (`87b7acd`) measured and fixed the two dominant read
costs on that fixture: `/api/stats` p50 531->119ms and `/api/search` p50 138->121ms (8 clients,
20ms at one client) through covering indexes, an index-only aggregate and a two-phase
search; search identity is unchanged. Concurrency stays bounded by the 4-connection SQLite
pool, so per-endpoint RPS remains a client-bound lower bound, and concurrent-write, cold-cache
and other hardware profiles are still unmeasured.

- Browser0.7 bookmarks store private catalog metadata locally in plaintext, bounded4MiB/1000 entries, not audio/credentials. Storage denial/quota/clearing can prevent persistence; it does not stop playback. Time updates checkpoint about every5s, with pause/seek/hidden/pagehide flushes; abrupt OS/process termination can lose the latest checkpoint. Restore is paused and fetches no media until explicit action. Legacy fallback is bounded and may lose position/preferences when a new snapshot is invalid. No offline playback or cross-tab/device conflict journal is provided.
- Reconnect uses three delays,10s HEAD deadlines and an8s progress watchdog; final failure needs explicit retry. Unsupported files can exhaust recovery without a precise codec diagnosis. Session revocation is detected on denied requests, not by erasing already buffered bytes. Autoplay restrictions can require another Play gesture after asynchronous metadata/recovery. Physical audio output, Safari, background suspension and native lock-screen services remain unverified.

- Now Playing0.8 is a native browser modal, full-height/scrollable on phones, with a five-entry saved-order preview. It is not an always-visible right sidebar, randomized shuffle prediction, lyrics view, file download grant or native engine. Source properties reflect metadata, not measured decoder output; no new latency/gapless/audio-quality certification follows. Dialog/native-focus behavior and Media Session callbacks are tested in isolated Chromium and Playwright Firefox157/Linux, not Safari, screen readers or physical headset/lock-screen hardware. Playwright used its Ubuntu24.04 fallback Firefox on this CachyOS host; this does not certify all stock Firefox builds/OS combinations. Very long text, enlarged fonts and short viewports can require vertical scrolling; controls remain reachable.

## On-demand personal mixes0.10 scope

Exploration caps at500 newest eligible nonhidden owned tracks and12 combined
favorite/recent-qualified anchors. It may return empty after anchor/played
suppression; it does not backfill with mislabeled repeats. Rotation shortlists
at most500 retained qualified tracks before diversity. Neither is a whole-
library similarity index, scheduled snapshot or calibrated preference model.
Explicit hidden-track choices cap at1000/account and are distinct from history:
history erasure does not reset them; Home has restore-all and UUID-only export.
Data-only updates may wait for one bounded10s feedback/export request; true
privacy/session changes invalidate immediately. Remote preference revisions
require refreshing privacy settings before another mismatched mix can display.
No public catalog, collaborative model, native client or offline entitlement is
implied by these server/browser features.

Personal candidate ordering uses added_at DESC/id ASC; the existing newest index
uses id DESC. Very large equal-timestamp batches may need extra sorting. The
current scale benchmark uses unique added timestamps and at most50 qualified
fixture sessions, so tied-time and10000-session latency remain unmeasured. Stable
repeated inputs return stable mixes; Refresh reloads current data rather than
pretending to regenerate an independently scheduled or random taste model.


## Public catalog0.11 scope

Public discovery is optional and off by default; no original account music is
published by upgrade. Only explicit per-track CC0/BY/BY-SA declarations are supported.
Evidence is owner-only and unverified; artwork is not public. Public tracks require
a positive known duration of at most7days; unusual years become unknown and public
text has bounded/control-free projections without changing owner tags. Stream/download grants
are separate API policy, not DRM or a guarantee recipients cannot save streamed
bytes. Revoking access cannot revoke received licenses or recall copies. Retained
revoked records count toward quotas; no purge/edit UI is implemented (revoke and
republish creates a new ID). Public metadata follows current owner catalog edits.

The Network view previews one HTTPS DNS origin's first20 published tracks with a
revalidated cursor "Load more" (server validates the cursor per page and the web client
dedupes by track ID); it does
not save peers, fan out search, follow users, mirror music, proxy audio or integrate
remote tracks into the root queue. Public links open the granted origin separately.
The native desktop Public view is separate: it saves origins, fans out search with
per-origin cursors/cache fallback and plays granted public streams through its own
engine; the web Network view remains one first20 preview.
No IP origins, custom HTTPS ports, signed inbox/activity, HTTP signatures, replay
protection, peer blocklist, WebFinger, remote cache/tombstones or Funkwhale compatibility
are implemented. Two-node tests use a production-absent exact test hostname pin;
Internet TLS/public deployments and standards interoperability remain unverified.
OS DNS resolution cannot be forcibly canceled, but occupied workers retain bounded
admission until actual completion. Public-catalog load/latency remains unmeasured.

Persistent user supervision starts with the existing Linux user manager, normally
at login; boot-before-login/logout behavior is unverified and no linger is enabled.
The installer is a same-build handover, not an updater; schema upgrades require a
verified backup and the separate upgrade process. Native clients/offline/device
handoff and Apple/Android hardware validation remain unfinished.

## Native Linux client0.12.x scope

**Updated2026-10-19:** this section previously described only the first0.12 slice;
the0.12.5/0.12.6 package adds explicit offline copies, opt-in Secret Service resume
and native playlist folders, documented below and in
[NATIVE_CLIENT_CONTRACTS.md](NATIVE_CLIENT_CONTRACTS.md). The audit's native
AUD-03/07/08/12 are fixed in source `e225b63` (off-thread cancellable prepare,
current-vs-other removal remap, per-await origin/epoch guards, quota/staging/
recoverable publication); concurrent two-process and disk-full/power-loss boundaries
plus explicit checking/Cancel harness assertions remain unexecuted. AUD-05 (cold
offline launch) is fixed in `3aeee09` and AUD-06 (origin-scoped offline namespace) in
`a913574`; queue snapshots canonicalize their origin in the adjacent fix.
AUD-01/02/04/11
are fixed in `3094754`, AUD-13 in `48b2fa4`.

Offline routing regressions observed during the repair are now fixed in source
`03b2523`: natural EOF advances the device queue instead of server prepare (Repeat-one
keeps replay), MPRIS Next/Previous use the offline-aware handlers, Play offline is no
longer restricted to Library/Favorites, and `cancel_all` cancels `folder_gate`.

The0.12.6 Linux x86_64 desktop application is a real Dioxus/WebKitGTK window with a
Rodio/CPAL output thread and MPRIS2 observer, verified against an isolated fixture
server/account/media. A portable x86_64 tar package has an exact-manifest/ldd/version check and a real
styled-window launch/close smoke. It is not an installer, AppImage/deb/rpm/Flatpak,
signed artifact, auto-updated client or release-ready desktop distribution. The window was
created only on this Hyprland/X11 host; GNOME/KDE/Wayland variants, screen readers,
physical audio output, headphone buttons and system sleep/resume are not certified.

Playback downloads the complete authorized original into a private temporary file
before decoding; there is no progressive streaming, gapless, crossfade, ReplayGain,
quality selection or measured time-to-audible claim. The250MiB/one-source bound and
preparation progress/cancel/retry are real, but large files incur preparation time
and temporary disk use. Explicit per-track offline copies **exist** in the0.12
Linux client, with v2 BLAKE3 manifests in0.12.6
(100 tracks/2GiB, account-scoped0600 audio, digests verified before
playback and by a bounded startup sweep, newest-saved offline Previous/Next queue,
confirmed per-copy removal; QA played with the fixture server stopped). There is
still no separate download-grant entitlement, background/automatic download or sync,
bulk storage manager, or verified cold launch/airplane-mode playback without a prior
in-process sign-in (AUD-05). By default sign-in is required each launch and
credentials/CSRF stay memory-only. An explicit opt-in **Remember this session**
stores only the opaque session token and canonical origin in the Linux freedesktop
Secret Service; password/username/CSRF are never stored, and resume still calls
`/api/me` (0.12.5+). Windows Credential Manager, macOS Keychain, Android Keystore and
biometrics are absent. Source commit `3094754` fixes AUD-01/02/04/11: sign-out stops
audio/clears private state first and deletes the keyring item off-thread with a
bounded deadline; transient/network/5xx/403 errors preserve the saved item (only a
genuine 401 rejection deletes it); secret-tool calls are byte/deadline bounded,
serialized and kill+reaped, and SavedSession Debug is redacted. The real two-process
Secret Service QA passed. The frozen live0.12.6 runtime predates this fix. Sign-out
clears the queue/catalog and stops native audio, but no cross-device session, device handoff or
remote-control protocol exists.

The native UI covers Library, Favorites, Albums, Artists, Playlists, Public and Queue,
transport, theme, sign-in and sign-out.
Favorites are implemented natively (filtered view, per-row favorite/unfavorite and
queue-state synchronization). Native playlists support real list/detail metadata, per-entry `can_stream`
labels, playback of the authorized subset, and owner create/CAS-rename/confirm-delete.
Albums and artists are browsable natively with read-only grouped detail and playback
of the owner-authorized tracks. Playlist entry add/reorder/removal is native; playlist folders are native since0.12.4/0.12.5
(owner-scoped create/filter/move/rename/delete with CAS), while album/artist
metadata editing, search history, settings/privacy, listening history and
recommendations remain browser-web features only. Native listening events
are not sent, so consent/history cannot be inferred from native preparation or
decoding. MPRIS requires a working session D-Bus and reports bounded public metadata
only; a failed bus degrades to in-app controls. Android Media3, iOS AVFoundation,
Windows SMTC and macOS Now Playing remain unimplemented/blocked on their real
toolchains, and Linux evidence cannot certify them.
