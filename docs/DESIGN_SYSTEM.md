# Original Vela design system

## Approved prototype transfer target 2026-10-19 — proposed, not implemented

The user selected [the Vela prototype](https://prototyp-opal.vercel.app/) as the
new visual target. The [transfer specification](PLANS/2026-10-19/PROTOTYPE_TRANSFER/README.md)
and [DeepSeek handoff](PLANS/2026-10-19/PROTOTYPE_TRANSFER/START_HERE_DEEPSEEK.md)
define Manrope, charcoal/pistachio roles, compact desktop composition, independent
content scrolling and the mobile mini-player/navigation layout. They supersede
older visual proposals for this transfer; existing executed behavior/privacy/audio
contracts and actual Light/System/80–150% preferences remain authoritative.

This is documentation only. Source tokens below describe their dated implementations,
not the newly shipped prototype design. Do not blindly overwrite current --sidebar
(length), --muted (text) or --accent (action) with prototype roles of different types.
Use the namespaced mapping and real asset/package/native-window acceptance in the
handoff. No new rendered spacing or application/platform validation is claimed here.

## Transport/window interaction repair2026-10-19

Keep the contrast tokens below. Current-row Play/Pause reflects root requested
intent and preserves the source, position and queue. Context-level Play starts the
album/playlist/mix/radio; duplicate occurrences remain separate. Linux integrates
44px accessible window controls into its existing toolbar, with no GTK title strip.
Other platforms retain system decorations. Source-specific unsupported modes are
disabled and explained. See [interaction review](UX_INTERACTION_REVIEW_2026-10-19.md)
and [ADR0024](DECISIONS/0024-row-transport-and-linux-window.md) for actual browser,
native Wayland/X11 evidence and limits. No new kit, font or audio owner is introduced.

## Current-source contrast increment2026-10-19

The implemented web/Linux native UI now follows [ADR0023](DECISIONS/0023-contrast-and-listening-layout.md)
and the [executed design review](DESIGN_REVIEW_2026-10-19.md). Source baseline0170ec5;
this is not an upgrade of the frozen user runtime. Existing data-theme selection
and original Vela assets remain authoritative; no React kit/font service was added.

| Semantic role | Dark | Light |
| --- | --- | --- |
| Background | #0b0e0d | #f4f7f3 |
| Surface | #151b18 | #ffffff |
| Raised surface | #25312b | #e4ece6 |
| Hover/current surface | #263e32 | #d7e9de |
| Main text | #f5faf7 | #14271c |
| Secondary text | #bac9c0 | #40594a |
| Quiet text | #9dafa3 | #4c6555 |
| Separators | #354b3e | #c1d1c5 |
| Input/control outline | #739581 | #668572 |
| Primary accent | #66efbc | #08754c |
| Primary action text | #092c1e | #ffffff |

Use semantic variables (`--raised/--hover` in native and
`--surface-raised/--surface-hover` in web), not per-screen competing palettes.
Main labels/body are14–15px, secondary track/source text12–13px, headings28–42px.
Keep the system font stack, clear focus outlines and original SVG icons. Main
transport/sliders/track actions use44px targets; the native four-action column is176px.
Standard page inset24px, web phone16px; component gaps mainly8/12/16/24/32px.
Inherited minor deviations remain measured rather than disguised as a perfect scale.

## DR-04 density, rhythm roles and the measured size map — 2026-10-22

The density work keeps the interface small and makes the *targets* real. The
rhythm roles are declared once in the existing `:root` of `apps/web/style.css`,
before the compatibility aliases, and they name the values the composition
already used; they are not a second palette, a second scale or a new layer.
Every role below is consumed through `var(...)` in the file that declares it,
and the table names the surfaces that consume it:

| Role | Value | Where it is the authority (consumed in `apps/web/style.css`) |
| --- | --- | --- |
| `--vela-space-1` | 4px | the desktop transport gap (`.player-buttons`), the player's zone gap below 1100px |
| `--vela-space-2` | 8px | the mobile and coarse-pointer transport gap |
| `--vela-space-3` | 12px | the sidebar step gap, the track grid gap |
| `--vela-space-4` | 16px | the player bar's zone gap below 1100px |
| `--vela-space-6` | 24px | the sidebar's group gap, the Home track grid gap |
| `--vela-space-8` | 32px | the Home section margin |
| `--vela-radius-control` | 8px | the navigation item, the buttons, the icon buttons, the inputs and the selects |
| `--vela-radius-cover` | 6px | `.artwork`, `.catalog-cover`, `.playlist-art`, the Home artwork |
| `--vela-radius-panel` | 12px | the Home hero panel |
| `--vela-target-size` | 44px | the navigation step, the sidebar playlist |
| `--vela-heading-section` | 20px | the section toolbar heading |

Two surfaces that the earlier version of this table named are **not** the
source of a role: the editorial feature and the My Wave panels live in
`apps/web/home-editorial.css` and keep their own 12px literal there, and the
player's controls are 44px boxes with their own rules rather than a spacing
step.

Most substitutions replace a literal with the identical value, but not all of
them, and the differences are deliberate parts of this package. These are the
surfaces that actually consume a role, with the value each one had before:

| Surface | Role | Before | Now |
|---|---|---|---|
| `input`, `select` | `--vela-radius-control` | `border-radius: 9px` | 8px |
| `.button` | `--vela-radius-control` | `border-radius: 9px` | 8px |
| `.icon-button` | `--vela-radius-control` | `border-radius: 8px` | 8px (unchanged, now named) |
| `.sidebar .nav-item` | `--vela-radius-control` | `border-radius: 8px` | 8px (unchanged, now named) |
| `.artwork` (library cover) | `--vela-radius-cover` | `border-radius: 5px` | 6px |
| `.home-artwork .artwork`, `.listening-cover .artwork`, `.catalog-cover`, `.playlist-art` | `--vela-radius-cover` | `border-radius: 7px` | 6px |
| `.home-hero` | `--vela-radius-panel` | `border-radius: 20px` | 12px |
| `.section-toolbar h2` | `--vela-heading-section` | `font-size: 21px` | 20px |
| `.sidebar .nav-item` | `--vela-target-size`, `--vela-space-3`, `--vela-radius-control` | `height: 45px`, `gap: 14px`, `border-radius: 8px` | 44px, 12px, 8px |
| `.sidebar-playlist` | `--vela-target-size` | `min-height: 40px` | 44px |

The other role consumers are the spacing steps, and those are unchanged:
the sidebar step gap was 13px and is 12px through `--vela-space-3`, the
transport gap was 17/12/10px and is 4px through `--vela-space-1`, and the
track grid gap was 13px and is 12px through `--vela-space-3`. The editorial
feature and My Wave panels live in `apps/web/home-editorial.css` and keep
their own 12px literal there; the player's controls are 44px boxes with their
own rules rather than a spacing step.

The visual effect is one pixel on the corners of fields, buttons and covers,
one pixel on the section heading, one pixel on the sidebar step, and the
12px → 44px growth of the playlist rows. It was measured, not assumed: the
live composition's computed radii, font sizes and row heights were read in
Chromium and in Firefox after the change.

Measured on the live web composition (Chromium and Firefox, 1284×856, 100%
app size, real account, real uploaded tracks):

| Surface | Measured |
| --- | --- |
| Sidebar navigation item | 181×44, step 44px, 12 items = 528px, real 24px between groups |
| Player band | 88px tall (`y768..856`), the 1px top border replaced by a hairline that takes no vertical space |
| Transport row | `y768..812`, 44px tall; five 44px controls and four 4px gaps = 236px |
| Seek row | `y812..856`, 44px tall; the range is 44px, the drawn rail 3px |
| Play button / face | 44×44 hit box, 32px face |
| Transport / utility glyphs | 15px and 17px |
| Volume | 65×44 above 1100px of viewport width, 44×44 from 1100px down to 760px, and not in the player bar below 760px (the mobile composition) or below 600px of layout width (the container query) |
| Section heading | 20px/600; the general `h2` stays 24px/600 |
| Page heading | fluid `clamp(28px, 2.7vw, 38px)`/650, measured 34.668px at 1284 |
| Body / row title / secondary | 14px, 12px/550, 12px |

The intersection area between the Play button, the four transport buttons, the
three utility buttons, the player's favorite and the seek range is 0 at 768,
1284, 1440 and 1728px, at the 80/100/150% app sizes and at the 80/100/150%
browser zoom.

The transport composition follows the width the content really has. The app
size is a CSS `zoom` on the document root, so the viewport media queries keep
reporting the visual width while the content is laid out in the narrower
zoomed width; the shell is therefore a query container
(`container-type: inline-size`, `container-name: vela-shell`) and the
transport switches to the content-sized column
(`minmax(0,1fr) auto minmax(0,1fr)`, `center max-width: none`) below 1100px
of layout width. Below 600px of layout width the player's volume yields
first: the queue and the mute stay, and the volume remains reachable in the
queue drawer and in the Now Playing sheet. The same is true below 760px of
viewport width, where the mobile composition leaves the whole utility zone
out of the player bar; the volume is in the drawer and in the sheet there as
well. Measured: 65×44 at 1284, 44×44 at 1000, and at 900px with a 150% app
size (600px of layout width) the volume is `display: none` while the queue
and the mute are still on screen. Chromium and Firefox agree on a settled
layout; Firefox re-evaluates this container query one layout late when a
zoomed root is resized, which is recorded in
[KNOWN_LIMITATIONS](KNOWN_LIMITATIONS.md).

Conscious exceptions, kept because they are optical or platform values rather
than rhythm values:

- The base cover radius is 6px (`--vela-radius-cover`): the library track
  cover, the player bar's cover, the catalog cover and the playlist art all
  measure 6px. The measured exceptions are the Now Playing sheet's cover
  (16px, 12px below 680px), the album card's cover (7px) and the mobile
  player bar's cover (5px); the avatar and the vinyl stay at 50% and the
  modal sheets keep their own radius.
- The card and icon surfaces (job card, import preview, job icon) keep their
  9px radius; they are not controls.
- 1px borders, the 3px seek rail, the 2/5px badge padding and the 8px quality
  badge stroke are optical values, not members of the whitespace scale.
- The 44px target is a size, not a step in the whitespace scale; it is never
  rounded to 32px or 48px.
- The Home title's fluid `clamp(30px, 3.1vw, 45px)` reaches 45px. That is a
  documented source exception relative to the 19–42px directive, not a licence
  to change the other headings.
- The coarse-pointer composition keeps its own 8px transport gap and 19px
  glyphs, and the mobile composition keeps its 120px band and 4px transport
  gap (DR-06). The desktop composition owns the 4px gap and the 15/17px
  glyphs.
- The app size is a CSS zoom on the document root: it scales the layout but
  does not move the viewport media queries. The player's composition follows
  the layout width through the shell's query container, but the rest of the
  application still answers to the visual viewport. That is a property of the
  app-size feature, not of this size map.
- The `input`/`select` padding is 10px 12px: the horizontal value moved to the
  scale, the vertical one keeps the ≥44px total target.

## DR-06 the mobile composition's measured numbers — 2026-10-22

The phone band has one boundary (760/761) and one override. The numbers below
are the rendered ones, measured in Chromium and in Firefox against a real
account with real uploaded tracks, not the declared ones.

| Surface | Measured on the phone band | Note |
| --- | --- | --- |
| `--vela-content-top` | **16px** | the desktop content top is 24px; the 63px topbar is unchanged |
| `.page-heading` | 83.91px at 320px, 44px at 360 and 390px | the title and the real Play action share one row from 360px |
| `.library-section .section-toolbar` | 100px | the search field takes its own row; the other pages keep their own toolbar rhythm |
| `.library-filters` | 44px | the default library order leaves the flow on the phone |
| `.track-header` | **36px** | one row; the empty actions heading is `display: none` |
| `.track-row` | **113px** | 44 + 8 + 44 of targets, 8px of padding above and below, the 1px rule |
| Track grid | `44px minmax(0,1fr) 44px`, 8px gap | the actions are their own row, `grid-column: 2/-1` |
| Player band / navigation band | 120px / 70px | unchanged; the reserve is 190px |
| Transport | Play, Next and Queue 44×44, 4px apart (140px) | the idle title is 12px/600, the subtitle 10/400; the current title keeps its 12px role and its 42px cover |

The idle sentence wraps instead of being cut: the two engines measure the
same Manrope text differently (Chromium 87px, Firefox 89.87px at 12px/600)
and the title path is 88px at 320px, so the sentence takes two lines of 18px
in Firefox and one in Chromium. The band keeps its 60px of content above the
44px seek rail. The current track's title keeps the standard ellipsis and
its full name in the DOM and in the `title` attribute.

The default library order is a **fact, not a control**: the summary carries
`data-default-order="true"` only while the applied filters equal
`TrackFilters::default()`, and on the phone that sentence leaves the flow
with the existing `.sr-only` technique — it stays in the DOM with its
`role="status"` and its text. An active filter or sort has no marker, is
shown and may wrap.

The empty editorial card and the empty wave take the height their own text
needs on a phone; the image-backed feature keeps its 288px and the
image-backed wave its 160px.

## DR-01 the glass and motion vocabulary — 2026-10-22

DR-01 is the **token layer only**. It declares one vocabulary of depth and
response for the floating web layers and applies none of it: there is no
`backdrop-filter`, no new transition and no changed surface in this
package. The visible glass and the motion that consumes these roles belong
to DR-02 and DR-03. Nothing below may be read as "glass is implemented".

The roles live in the existing canonical `:root` of `apps/web/style.css`,
next to the roles they belong with, and the light theme re-points only the
colours:

| Role | Value | Declared at | Consumer |
| --- | --- | --- | --- |
| `--vela-glass-bg` | `#131513d9` (dark) / `#f4f7f3e6` (light) | `apps/web/style.css:89` / `:217` | the two floating bands — the player and the topbar (DR-02) |
| `--vela-glass-blur` | `16px` | `apps/web/style.css:90` | the desktop player band (DR-02) |
| `--vela-glass-blur-mobile` | `12px` | `apps/web/style.css:91` | the mobile player band (DR-02) |
| `--vela-glass-blur-topbar` | `12px` | `apps/web/style.css:92` | the topbar band (DR-02) |
| `--vela-glass-border` | `#ffffff14` (dark) / `#14271c14` (light) | `apps/web/style.css:93` / `:218` | the player's hairline and the topbar's bottom edge (DR-02) |
| `--vela-glass-highlight` | `#ffffff0f` (dark) / `#ffffffb3` (light) | `apps/web/style.css:94` / `:219` | the top-edge inset highlight of both bands (DR-02) |
| `--vela-glass-shadow` | `0 18px 50px #00000059` (dark) / `0 18px 50px #21372924` (light) | `apps/web/style.css:95` / `:220` | the one soft outer shadow of the player band (DR-02) |
| `--vela-glass-text` | `var(--vela-text)` | `apps/web/style.css:96` | the captions of both bands (DR-02) |
| `--vela-motion-instant` | `80ms` | `apps/web/style.css:97` | the pressed control — `.button`, `.icon-button`; the pressed card — `.catalog-card`, `.listening-card`, `.playlist-card`; the row's own opacity — `.track-row`; the pressed wave control — `.home-wave-play:active` (DR-03) |
| `--vela-motion-quick` | `120ms` | `apps/web/style.css:98` | the covers that answer a hover — `.catalog-card .catalog-cover`, `.playlist-card .playlist-art`, `.listening-card .listening-cover`, `.home-album-card .catalog-cover` and the cover images inside them; the row menu and the touch sheet — `.track-menu-panel`; the wave control's hover — `.home-wave-play` (DR-03) |
| `--vela-motion-overlay` | `140ms` | `apps/web/style.css:99` | the Now Playing sheet's entrance — `.now-playing-dialog[open]`; the transport's hover — `.now-playing-dialog .np-toggle`; the one placeholder entrance — `.track-skeleton-card .track-skeleton-art` (DR-03) |
| `--vela-motion-exit` | `100ms` | `apps/web/style.css:100` | **no consumer** — the exit of an overlay is 0ms by design: the dialog leaves the top layer in the tick that closes it, and the outgoing cover layer belongs to the artwork crossfade (DR-03) |
| `--vela-motion-panel` | `180ms` | `apps/web/style.css:101` | the queue drawer's entrance — `.queue-panel` (DR-03) |
| `--vela-motion-skeleton` | `1800ms` | `apps/web/style.css:102` | **no consumer** — the placeholder is a static surface (DV-14) and the one entrance it has reads the overlay role; this stays the reserved budget of a real long pending (DR-03) |
| `--vela-motion-reorder` | `150ms` | `apps/web/style.css:103` | the settle of a moved queue row — `.queue-row-settle` (DR-03) |
| `--vela-motion-artwork` | `140ms` | `apps/web/style.css:104` | the artwork crossfade — `.cover-fade-layer.is-ready` and `.cover-fade-layer.is-leaving` (DR-03) |
| `--vela-ease-enter` | `ease-out` | `apps/web/style.css:105` | every entrance — the row menu, the touch sheet, the Now Playing sheet, the queue drawer, the incoming cover layer, the placeholder entrance (DR-03) |
| `--vela-ease-state` | `ease-in-out` | `apps/web/style.css:106` | every state change — the pressed control and card, the row, the hovering cover, the transport hover, the outgoing cover layer, the queue settle (DR-03) |

The eight durations are zeroed for `prefers-reduced-motion: reduce` in one
block at the end of the same file (`apps/web/style.css:1889–1900`): the
easing curves are not durations and are not touched there. That block does
not replace the existing global `animation:none` / `transition:none
!important` overrides (`apps/web/style.css:284`, `:668`, `:1127`, `:1147`,
`:1442`, `:1557`) or the component files' own reduce blocks: a zero token
cannot stop a JS or Web Animations driver, so DR-03 still has to exercise
the media query itself.

**The source of the colours is the current stylesheet, not this document.**
The palette primitives and the semantic dark roles are
`apps/web/style.css:42–85`; the light theme is the separate validated
adaptation `:root[data-theme="light"]` at `apps/web/style.css:176–224`.
The historical mint/sage table further up this file, the 2026-10-19
prototype transfer and the system-font paragraph below are history: they
are **not** a source of hex values for new roles, and the mint table in
particular does not match the shipped palette. The DR-01 colours are the
shipped charcoal/pistachio roles and their light adaptation, and the
`--vela-glass-text` alias points at the existing `--vela-text` role
instead of introducing a new foreground.

**The type is the current self-hosted Manrope** (latin and cyrillic
subsets, `font-display: swap`, `apps/web/style.css:9–26`); DR-01
introduces no new font stack and no new asset.

**Alpha 0.851 (dark) and 0.902 (light) is a property of the declaration,
not a contrast claim.** The declared background is `#131513d9` and
`#f4f7f3e6`; 217/255 and 230/255 are the alphas those two hex values
state. This says nothing about legibility over artwork, and the contrast
over an image is DR-02's work — it is not claimed here.

**The glass roles have two consumers; the motion roles have their first
consumers in DR-03.** DR-02 (below) reads `--vela-glass-*` for the two
floating bands — the player and the topbar — inside one `@supports` block.
DR-03 reads the eight `--vela-motion-*` durations and the two
`--vela-ease-*` curves for the short answers of the web surfaces: the
pressed control, the hovering cover, the row menu, the Now Playing sheet,
the queue drawer, the artwork crossfade and the settle of a moved queue
row. Two roles still have **no** consumer and say so in the table above:
`--vela-motion-exit` (the exit of an overlay is 0ms by design) and
`--vela-motion-skeleton` (the placeholder is a static surface).

The [ADR0023](DECISIONS/0023-contrast-and-listening-layout.md) decisions
stay in force and are not revisited here: the contrast-qualified roles are
unchanged, the web application still has exactly one audio authority and
one scroll owner, and the shell's layout roles are untouched.

The measured evidence (real account, real uploaded tracks, Chromium and
Firefox, 1284×856 and 390×844, both themes) is in
[QA_EVIDENCE](QA_EVIDENCE.md); the permanent gate is
`tests/browser/dr01-tokens.spec.mjs`.


## DR-02 the two floating glass bands — 2026-10-22

DR-02 is the first package that gives the DR-01 vocabulary a surface. The
shell keeps its grid, its sidebar, its one HTMLMediaElement and its one
scroller; what changes is that the catalog now **continues under** the
topbar and the player instead of stopping above them.

**The geometry is declared once** (`apps/web/style.css`, the DR-02 section
at the end of the file, before the DR-01 reduce block):

| Rule | Value | Why |
| --- | --- | --- |
| `.main-content` | `grid-row: 1 / -1` (desktop and mobile) | the content column spans every shell row: the sidebar keeps column 1 row 1, the player keeps row 2, the mobile navigation keeps row 3 |
| `.main-scroll` | `margin-top: calc(-1 * (var(--vela-topbar-height) + var(--vela-safe-top)))` and the same value as `padding-top` | the one scroller starts under the topbar and the first heading keeps its old y — the frame does not jump |
| `.main-scroll` | `scroll-padding-top: calc(topbar + safe-top + 8px)`, `scroll-padding-bottom: calc(var(--vela-bottom-reserve) + 8px)` | a focused target lands inside the effective rectangle between the two bands with 8px to spare |
| `.content-inner` | `padding-bottom: calc(var(--vela-bottom-reserve) + 24px)` (mobile `+20px`, short height `+16px`) | the one content reserve: the last row can always be scrolled above the bands. No page adds a spacer of its own |
| `.main-scroll` | `position: relative` and **no** stacking context of its own | a positioned box with `z-index: auto` is painted before the two bands, so their glass samples the content that scrolls under them. The scroller must not open a stacking context: a row's action menu and the mobile action sheet are its descendants, and a context here would trap them below the player and below the DR-05 receipt |

**The effective visible rectangle** is `scroller.top + topbar (+ safe top)`
to `scroller.bottom - reserve`: 75→768 at 1284×856, 63→654 at 390×844.
`--vela-bottom-reserve` is unchanged — 88 on the desktop, 120+70+safe-bottom
on the mobile composition — and it is now the height of the bands the
scroller floats over rather than a gap above them.

**The glass is progressive.** The opaque surfaces stay the base; one
`@supports ((backdrop-filter:blur(1px)) or (-webkit-backdrop-filter:blur(1px)))`
block paints the two bands:

- the player and the topbar take `--vela-glass-bg` and `--vela-glass-text`;
- the topbar blurs with `--vela-glass-blur-topbar` (12px), the desktop player
  with `--vela-glass-blur` (16px), the mobile player with
  `--vela-glass-blur-mobile` (12px);
- the player's DR-04 hairline keeps its one pixel and takes
  `--vela-glass-border`; the topbar's bottom edge takes the same role;
- both bands carry the `--vela-glass-highlight` inset on their top edge;
- the **one** soft outer shadow (`--vela-glass-shadow`) belongs to the
  player. The topbar has no second outer shadow.

**The collection stays opaque.** The sidebar, the mobile navigation, the
rows, the grids, the cards, the panels and the inner surfaces of the dialogs
carry no filter: a blur over a long list is a jank and a readability cost.
The other filtered surfaces belong to overlays that are not open with the
collection: the `::backdrop` of the queue drawer (8px), of Now Playing
(14px) and of the metadata dialog (6px), plus the `.playlist-picker-backdrop`
element (6px). Measured: **two** filters are visible with the catalog, and
**three** with the queue drawer or Now Playing open (the metadata dialog and
the playlist picker were not opened in that run).

**The captions on a glass band use `--vela-glass-text`, not the quiet
role.** Over a light cover the quiet role (`#798372`) falls to about
3.07:1 on the composited surface, which is a failure. The hierarchy on the
bands is therefore carried by the **size and the weight** — the title 12px/600,
the artist, the time and the quality format 10px/500, the album and the
quality caption 10px/400 — and never by the opacity of the text.

**The modes are explicit.** `prefers-reduced-motion: reduce` keeps the static
glass and sets `transition:none` / `animation:none` on both bands and their
pseudo-elements. `forced-colors: active` and
`prefers-reduced-transparency: reduce` return the bands to their opaque
roles (`--vela-player-bg`, `--vela-mobile-player`, `--vela-bg`) and remove
the blur. `print` removes the blur and the shadow. Without the
`@supports` query the two opaque bands are the fallback. No new hex value
and no new role was introduced: the bands read the DR-01 vocabulary only.

Web sidebar scrolls as one region on short windows. Native shell is one two-column
grid with one content scroll owner and one bottom player row; count sidebar width
once. Web Library starts with real tracks, concise title and real Play action.
Native create forms are details/summary disclosures. Success notices float above
the player and are dismissible, without moving the collection. Mobile track actions
use a second row: the increased height is deliberate to retain accessible targets.
Dark/light colors change atomically; only transform/opacity/shadow feedback animates.
Respect reduced motion and existing modal/focus/player lifecycles.

## DR-03 the short answer that never delays the command — 2026-10-23

DR-03 is the first consumer of the eight motion roles. It changes **how a
surface answers**, not what a surface does: every command, every state
change, every focus move and every audio command happens in the tick that
carries the event, and the effect is a separate, short moment on top of it.
No playback, queue, session or ownership rule changed.

**The durations are the DR-01 roles, and every effect moves `transform` or
`opacity` only.**

| Surface | Effect | Role |
| --- | --- | --- |
| `.button`, `.icon-button` pressed | `scale(.97)` on the control | `instant` 80ms |
| `.catalog-card`, `.listening-card`, `.playlist-card` pressed | `scale(.97)` on the card | `instant` 80ms |
| `.track-row` | its own opacity | `instant` 80ms |
| the cover inside a card (fine pointers only) | `translateY(-2px)`, the cover image `scale(1.03)` | `quick` 120ms |
| `.track-menu-panel` (desktop) | the panel fades in, the content `opacity 0→1`, `scale(.98→1)`, origin `top right` | `quick` 120ms |
| `.track-menu-panel` (touch sheet, ≤760px) | the panel fades in, the content `opacity 0→1`, `translateY(4px→0)` | `quick` 120ms |
| `.now-playing-dialog[open]` | `np-enter`: `opacity 0→1`, `translateY(4px→0)` | `overlay` 140ms |
| `dialog.queue-drawer[open] .queue-panel` | `opacity 0→1`, `translateX(4px→0)` | `panel` 180ms |
| `.cover-fade-layer` | the artwork crossfade, opacity only | `artwork` 140ms |
| `.queue-row-settle` | the settle of a row that has already moved, `translateY(±4px→0)` | `reorder` 150ms |
| `.player-eq i` | the equalizer: a fixed 10px box, `scaleY` only | 800ms, state-bound |

**The exit of an overlay is 0ms.** `dialog.close()`, the release of the
inertness and the return of the focus are not deferred: the sheet leaves
the top layer in the tick that closes it. This is a deliberate deviation
from the usual modal fade-out — the lifecycle and the focus contract of the
application are older than the effect, and a copied dialog is not an
acceptable way to keep one.

**A card is a hit box and it does not travel.** The cover inside it is
what answers, and only under `(hover: hover) and (pointer: fine)`, so a
touch device has no stuck hover. A pressed control answers with a scale
that does not move layout. The button's old hover lift is gone with it: a
control that moves under the pointer is a control the reader aims at by
mistake, so the hit box stays still and the answer is the press.

**The row menu keeps the space the shell reserved for it.** The panel
fades in and its content answers inside it (a scale on the desktop, a
four-pixel rise on the touch sheet), so the sheet still ends exactly at
the player's top edge and never covers the transport — not even for a
frame. The panel's own box is the anchor, and the anchor does not move.

**The equalizer and the placeholder are state, not decoration.** The
equalizer keeps one 10px box per bar and animates `scaleY` only, so it
never changes a layout; the placeholder is a static surface with one
140ms opacity entrance, and the `background-position` shimmer is gone
(DV-14).

**The artwork crossfade keeps two layers and one request.** The picture
changes only after the new one is really there, the outgoing layer leaves
with the same role, and the state lives in the component: an account change
or a sign-out drops it with the component, so a previous account's picture
can never reach the screen.

**A moved queue row settles, and the focus follows it.** The order changes
in the tick of the command; the receipt that says so is a presentation
fact, it dies with the next structural edit, and the focus that the
rebuilt row lost is handed to the same control of the destination row.

The measured evidence is in [QA_EVIDENCE](QA_EVIDENCE.md) and the permanent
gate is `tests/browser/dr03-motion.spec.mjs`.

## Earlier visual baseline and proposals

Proposed evolution2026-10-19: [Listening Atlas visual specification](PLANS/2026-10-19/06_VISUAL_SYSTEM.md)
and its [local design board](PLANS/2026-10-19/design/vela-atlas-board.html).
The proposal includes original seed-based geometry, Cyrillic/Latin typography,
semantic tokens and responsive concept captures. It is not applied to application
CSS; the existing shipped design documented below remains the current implementation.

The earlier sage/forest/cream palette used dark background#101716,
surfaces#17201e/#1d2825, accent#9ddbc4, text#f0efe8 and light ivory#f4f2eb,
surface#fffcf5, accent#21775b, text#182c25. These are historical colors, superseded
in current-source UI by the contrast tokens above. Preserve original Vela branding,
legitimate fixture artwork and existing components; no proprietary assets or new
bitmap dependencies.

Use the system sans-serif stack,4/8px spacing rhythm,12–16px readable controls/body,24–40px headings,9–16px radii and restrained borders. New UI uses original inline SVG with consistent24px geometry/stroke; decorative SVG is hidden from assistive technology and buttons carry accessible names. Artwork is square, lazy-loaded where possible, with native vector fallback rather than fabricated covers. Existing technical format labels describe actual reported properties, not assumed losslessness.

Desktop: constrained central canvas with library shortcuts and bottom transport. Mobile: five44px-minimum navigation targets, mini-player above navigation, safe-area padding, and a More tab that opens Settings where a focus-contained More from Vela list reaches the remaining destinations; avoid a compressed ten-item desktop menu. Cards reflow at320px, tablets and desktop/ultrawide without hiding essential actions. Keep the single root audio element mounted through all these states.

Motion is short opacity/transform feedback, disabled under prefers-reduced-motion. Focus rings must remain visible, dialog focus returns to its trigger, errors cannot rely only on color, and the playing state has text/icon evidence. Skeletons reflect a real pending request; empty states offer actual Add music/search/library actions. Do not promise WCAG/screen-reader certification from a screenshot.

Design references studied: [Spotify's library/canvas/playing layout rationale](https://newsroom.spotify.com/2023-06-20/spotify-desktop-experience-redesign-your-library-now-playing-views-customize/) and [Apple's explicit next/queue interactions](https://support.apple.com/en-ae/guide/iphone/ipha4521ef7d/ios). These inform independent information architecture, not copied assets. [Dioxus resource lifecycle](https://dioxuslabs.com/learn/0.7/essentials/basics/resources/) informs cancellation; the current exact-session/generation pattern is retained.

Baseline screenshots are preserved under ignored target/checks/experience-baseline-screens. Final review must include actual fixture music at320px, tablet,desktop and ultrawide, dark/light, navigation/player continuity and keyboard More dismissal. Record artifact paths and findings in PROGRESS; no visual/platform verification is inferred from a compile.

Playback0.7 uses an unobtrusive fixed recovery status above the persistent player, with readable text and a real Retry playback action. At320px the message wraps above the mini-player/navigation; both themes use the established surface/text/focus tokens. Queue provenance is one ellipsized text line, with the full label retained in the DOM. Inspect actual exhausted-retry states on desktop and phone rather than injecting decorative alerts.

Now Playing0.8 reuses sage/forest/ivory tokens in now-playing.css: a bounded two-column native desktop dialog, square real cover, readable track heading,44px targets and60px primary transport. The phone view fills100dvh, preserves safe-area padding, wraps long text and scrolls to real details/queue actions. Light/dark, reduced motion, keyboard containment/return and increased text size are acceptance checks. The cover fallback is the original Vela SVG, never invented album art. Native sliders retain keyboard operation. Keep the modal in the browser top layer; do not imitate it with a second legacy focus trap or make background playback controls disappear.

Desktop artwork is constrained by viewport height so seek, transport and volume remain visible at normal text sizes on900px and720px-height screens. Small source sizes use explicit B/KiB/MiB/GiB labels instead of rounding a nonempty file to0.0MB.

The0.8 QA follow-up exercises the same dialog/focus/transport/theme/viewport assertions in Playwright Firefox157/Linux. Chromium and Firefox screenshots are retained separately; Firefox may render the backdrop without Chromium's blur treatment. Dimmed inert background, contrast, layout and real controls remain the contract. No stock Safari or screen-reader certification follows.


Radio0.9 uses the same forest/ivory tokens, original neutral artwork, a bounded seed hero, dense track rows with explicit reasons, and a secondary optional analysis panel. No invented cover art or genre/mood confidence is shown. Private history rows link actual playback to related track radio; Settings separates account/device consent and explicit erasure. Source/status/measurement details use existing surfaces and accessible controls. The first actual radio screenshots exposed a div-based placeholder inheriting the flexible description rule; the selector now names the description explicitly, and screenshot checks assert square artwork at all eight size/theme combinations. Component screenshots keep the privacy panel reviewable without capturing long unrelated administrator scan histories. Final artifact paths and engine results are in PROGRESS.

## Personal Home shelves0.10

PersonalMixes renders inside Home after its hero/toolbar and before additions,
with the existing compact listening history alongside it. Reuse cover/spacing/
button tokens; do not create another player or navigation shell. Exploration and
rotation have actual playable cards with short reasons and scoped next/last/radio/
hide actions. Coverage, algorithm and refresh details live in a disclosure rather
than dominate the listening flow. Pending privacy or preference writes clear
stale actionable cards; successful Undo and explicit confirmation remain visible.
Responsive assertions/screenshots are executed separately from source presence.

Executed0.10 visual evidence: exact package37scenarios in each Chromium/Firefox,
320/768/1440/1920 dark/light viewport shots in target/checks/personal-packaged-
{chromium,firefox}-screens. Disable CSS transitions only during screenshot capture
to avoid intermediate theme colors; full functional playback/race tests retain
normal app behavior. Main inspected actual desktop/phone images. Square-cover
and document-width assertions pass; screen-reader/Safari/native remain unverified.


## Network0.11

Use existing Vela navigation, surfaces, type, icons and theme tokens. The Network
page separates administrator discovery policy, owner publication declarations,
retained records and peer preview. Administrator controls are absent for ordinary
members. Every rights/audio permission starts unchecked, metadata-only state is
explicit, changing the declaration clears rights confirmation, and revoke asks
for an inline confirmation with a Keep action. The original root player remains
active across navigation. External public links display origin and actual grants.
Responsive layouts collapse form/card columns, preserve labels and focus styles,
wrap provenance text and support320px plus light/dark/reduced motion. PN02 captures
actual320/1440 theme states; screenshot inspection and exact results belong in
QA_EVIDENCE. No licensed cover art is fetched to decorate public records.
