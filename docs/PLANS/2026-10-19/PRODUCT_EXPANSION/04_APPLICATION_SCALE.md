# Application scale: exact implementation specification

Priority P0. Proposed feature EX261. Deliver EP02 after EP00/EP01. This is not an
instruction to modify Hyprland configuration, force a monitor scale or replace the
native audio engine. No scale implementation has been written in this package.

## User-visible behavior

Settings → Appearance → App size offers presets75/90/100/110/125/150/175/200%, a
minus/plus stepper, a labeled slider and Reset to100%. Internal value is an integer
percentage,75–200 inclusive, in5-point steps; invalid stored/imported values fall
back to100 rather than reaching the renderer. Default100% means the app follows
normal host scaling. Initial preview is the actual application, not a screenshot.

Live changes preserve playback source, cursor, queue, position, requested intent,
session, active dialog, input draft and focus. Do not remount App or recreate the
WebView/audio element. Do not resize the native window to fake enlarged content.

Persist presentation scale per installation/browser origin, shared across accounts
on that installation because it contains no account data. It must work before
sign-in and without server contact. Do not put scale into private history/device
consent tables. Optional future per-window overrides are ephemeral; opening another
window starts from the installation default.

Keep App size, Text size and Interface density independent. Text enlargement is an
accessibility contract; Compact density must not shrink touch controls below the
declared minimum. System monitor DPI is not another saved app preference.

## Four coordinate spaces

1. Native/compositor physical pixels: capture and rasterization.
2. Window logical units: host window geometry and pointer dispatch.
3. Layout CSS units after the selected engine zoom model.
4. User text size/font accessibility adjustments.

Do not infer app zoom from `devicePixelRatio` alone. Browser/page zoom and monitor
scale can both affect it. Record host scale, actual viewport, requested/effective
app percentage and measured control rectangles independently in QA evidence.

Example, not a measurement: a device at2× monitor scale with App size125% should not
have125% multiplied into monitor settings, then multiplied again into CSS. Apply
the application preference exactly once through the selected platform adapter.

## Desktop engine decision

Pinned local source evidence: dioxus-desktop0.7.10 exposes DesktopService.webview and
set_zoom_level. Its convenience method logs zoom failure and returns no result.
Wry0.53.5 WebView.zoom accepts a factor and returns Result; Android is explicitly
unsupported by that API. See research S19/S20, checked against Cargo.lock and local
crate source. Do not upgrade Wry independently just to obtain a different signature.

Preferred Linux desktop spike: a UI-thread presentation adapter wraps the existing
WebView's Result-returning zoom call. Validate percent before converting to a finite
factor. Return typed Applied/Unsupported/Failed status to preferences. The adapter
owns no media/account/HTTP state. Do not report successful persistence as proof that
the renderer accepted a value. If readback is unavailable, distinguish accepted
operation from a QA-verified visual result; do not manufacture an effective readback.

Compare its rendered reflow, focus rectangles, dialog positioning, scroll anchoring,
44-unit targets and pointer hit testing in real WebKitGTK on native Wayland and X11.
Inspect current uncommitted DMABUF work independently; do not equate enabling a
renderer option with verified refresh rate or smooth scale changes.

If native zoom fails the spike because supported OS versions or breakpoint behavior
cannot meet accessibility requirements, use the bounded semantic-token adaptation
described for web below. Record that decision and test both paths. Never stack a
WebView zoom factor and CSS zoom for the same App size value.

## Web/PWA decision

JavaScript must not attempt to change browser page zoom or block Ctrl/Cmd+plus,
minus or0. Browser zoom remains the user's own accessibility control. App size is
an additional preference available inside Vela. A web PWA is still subject to this
rule; do not assume installation grants desktop-native zoom APIs.

Preferred web baseline: bounded semantic layout/text tokens driven by an app-scale
variable, using relative units for text, icons, insets and row geometry. Reflow uses
actual available content width/container queries and robust minmax/flex wrapping;
do not assume multiplying rem changes viewport media-query breakpoints. Fixed
overlays, player heights and safe areas must use the same token system.

CSS zoom may be evaluated as an alternative only after exact Chromium/Firefox/
WebKit behavior and the no-double-scaling contract are tested. Do not use root
transform:scale with inverse viewport width/height tricks: visual placement, layout,
hit testing and scroll bounds diverge. The shipping adapter must be explicit; an
unsupported fallback cannot silently show a selected percentage that changes nothing.

## Mobile decision

Respect Android font/display preferences and iOS Dynamic Type/accessibility limits.
Do not call unsupported Wry Android zoom. Start with text size and density tokens,
safe areas and native touch targets. Expose full App size on a mobile target only
after its actual bridge/rendering implementation passes its platform matrix.
Never scale Media3/AVFoundation transport clocks or system media metadata.

## Keyboard, pointer and recovery

Desktop-native: Ctrl/Cmd+plus or equals increases5 points; minus decreases5;0 restores
100. Normalize keyboard layouts and numeric keypad keys. Register one application
command path, not both WebKit default behavior and a second DOM handler. Validate
which modifiers are owned by the OS first. Exclude password/text/IME composition
events when a host editing command conflicts. Do not add a Ctrl+wheel interceptor
by default; scrolling should remain predictable.

Web: leave browser shortcuts alone. Offer App size stepper/reset in Settings and
an optional distinct remappable app command that does not shadow browser zoom.
Native key bindings may be customized later with conflict detection.

High scale must not strand Reset outside the visible area. Put it alongside the
percentage in an adaptive settings header. Provide a native startup “Reset display
settings” recovery command and an in-app keyboard-reachable recovery route; this is
separate from Reset account. If a future backend change needs a risky renderer
restart, require explicit confirmation with a timeout fallback. Ordinary live zoom
changes need no repeated confirmation dialog.

## Persistence contract

Store a small versioned device preference document in the existing platform's
appropriate private config area; web uses a bounded allowlisted local storage
document. Use atomic replacement and cancellation-safe writes on desktop, off the
UI thread. Only presentation fields are allowed here. No cookie, token, path to
music, queue, history or origin-specific private object enters this document.

Load and validate before the first visible app frame when possible, including
signed-out state. Initialization failure produces100% with a non-blocking explanation.
Apply live changes immediately, coalesce writes while dragging and flush final value
on blur/pointer release. A write failure says “Applied for this session; could not
save” and does not reset a listening user's playback.

## Implementation steps and file ownership

1. EP00: fingerprint dirty work, record actual runtime/backend, capture baseline
   Settings and player geometry; preserve all existing row-toggle regressions.
2. EP01: define validated presentation preference values and a device-store contract;
   evolve a cohesive existing module or one small shared boundary, not a new crate farm.
3. Add a desktop presentation adapter near `apps/desktop/src/window.rs`; keep its
   startup load hook in main/root integration under main ownership.
4. Add native Settings route and a root-owned preference service in ui.rs; extract
   presentation only when this reduces coupling. The service survives route changes.
5. Add web Settings category/scale controls through existing root/shell; evolve
   CSS tokens and include every packaged stylesheet in the server's exact allowlist.
6. Migrate existing web theme into validated preferences without discarding it.
   Desktop's current theme defaults should be preserved until a saved choice exists.
7. Test before/after snapshots, active audio identity/time, settings search, keyboard,
   persistence failure and recovery. Repeat on normal tiled Wayland without forced float.
8. Update docs/platform/evidence and package only after exact built assets are checked.

## Acceptance cases

| ID | Scenario and expected result |
| --- | --- |
| SCALE01 | Fresh startup100%, change125%, restart paused state: saved scale loads before private requests; no startup loop |
| SCALE02 | Play real30s fixture; change75/100/125/150/200 and theme; same audio source/occurrence with advancing position |
| SCALE03 | Native Wayland and X11 at monitor1×/1.25×/2× where available; app scale applied exactly once; report untested monitor modes |
| SCALE04 | Move a live window between differently scaled outputs; text/pointer/dialogs remain aligned and app preference unchanged |
| SCALE05 | Web native browser zoom200/400% plus app settings100/125/200%; reflow and reachability; browser Reset is not intercepted |
| SCALE06 | Text enlargement200%, long English/Russian/German labels,320px-equivalent content, no hidden essential action |
| SCALE07 | Corrupt/NaN/negative/oversized/out-of-step imported values; safe normalization or rejection before renderer call |
| SCALE08 | Storage denied/disk full; temporary applied status, no false Saved, next start recovers safely |
| SCALE09 | Keyboard-only Settings search→scale→reset, focus retained; modal menus anchored correctly after scaling |
| SCALE10 | Pointer hit-test center/edges of transport, drag handle, slider, context menu and scrollbar at75/200%; no offset clicks |
| SCALE11 | Measure touch-target size in the declared host logical units. If shrinkage takes controls below44 units, enlarge hit regions, adapt density or restrict the lower bound on that platform; never silently accept undersized controls |
| SCALE12 | Sign-out/account switch/offline launch preserves only device presentation, not previous account's private data |
| SCALE13 | Unsupported backend returns explicit reason; UI never claims changed effective scale when operation fails |
| SCALE14 | Screen reader observes meaningful percentage/value changes without a torrent of announcements during slider drag |

Performance goals are acceptance targets, not measured results: no new audio-source
request caused by scale; input feedback within100ms on the recorded reference host;
no persistent idle animation loop. Measure layout/render frame costs and retain
baseline traces before making any “smooth at144Hz” claim.
