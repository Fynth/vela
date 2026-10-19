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

Web sidebar scrolls as one region on short windows. Native shell is one two-column
grid with one content scroll owner and one bottom player row; count sidebar width
once. Web Library starts with real tracks, concise title and real Play action.
Native create forms are details/summary disclosures. Success notices float above
the player and are dismissible, without moving the collection. Mobile track actions
use a second row: the increased height is deliberate to retain accessible targets.
Dark/light colors change atomically; only transform/opacity/shadow feedback animates.
Respect reduced motion and existing modal/focus/player lifecycles.

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

Desktop: constrained central canvas with library shortcuts and bottom transport. Mobile: five44px-minimum navigation targets, mini-player above navigation, safe-area padding and a focus-contained More sheet; avoid a compressed ten-item desktop menu. Cards reflow at320px, tablets and desktop/ultrawide without hiding essential actions. Keep the single root audio element mounted through all these states.

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
