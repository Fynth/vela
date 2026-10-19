# Vela interaction and product experience playbook

English specification for a future implementation agent. Planning only, 2026-10-19.
Initial inspection anchor: `b619253` plus preserved native appearance work in the dirty tree.
Concurrent-work reconciliation anchor: `8c339e3`; see the evidence note below.
No application code, stylesheet, dependency, data or runtime is changed by this task.

## The requested outcome

Make Vela feel alive, responsive and thoughtfully interactive. Start with precise
loading placeholders and a complete right-click/keyboard/touch action system, then
add useful direct manipulation, reversible feedback, queue tools and music discovery
interactions. Motion should explain state and respond to input; it must not hide
network delays, simulate progress or turn every page into identical decorated boxes.

This supplement contains 96 granular interaction briefs, 11 delivery packages, detailed
state/command contracts and acceptance matrices. It expands the existing 320-entry
catalog; it is not 96 unrelated features to add to a marketing counter. Cross-references
identify overlap. No proposal is marked implemented by being written here.

## Documents

1. [Honest loading, shimmering skeletons and progressive reveal](01_LOADING_AND_SKELETONS.md).
2. [Context menus, searchable action pickers and touch sheets](02_CONTEXT_MENUS.md).
3. [Direct manipulation and complete product interaction recipes](03_PRODUCT_INTERACTIONS.md).
4. [96 implementation-ready interaction briefs](04_IDEA_CATALOG_96.md).
5. [Motion, visual feedback and accessible presentation](05_MOTION_AND_VISUAL_LANGUAGE.md).
6. [Dioxus integration, state ownership and capability contracts](06_INTEGRATION_CONTRACTS.md).
7. [Delivery order and exact QA requirements](07_DELIVERY_AND_ACCEPTANCE.md).
8. [Copyable prompt for the next agent](08_AGENT_PROMPT.md).

Parent plans remain useful: [main index](../README.md), [native master plan](../14_NATIVE_CROSS_PLATFORM_MASTER_PLAN.md),
[settings](../03_SETTINGS_EXPERIENCE.md), [architecture](../08_ARCHITECTURE_AND_DATA.md)
and [320-feature catalog](../07_FEATURE_CATALOG_320.md).

## Current facts to preserve

- Web `experience.rs` already renders generic loading placeholders for initial Home
  and Search; `personal_mixes.rs` has placeholders too. style.css uses a 1.4s opacity
  pulse and already has reduced-motion overrides. Refine these components instead
  of claiming skeleton loading is entirely absent.
- No central entity context-menu/right-click implementation was found in the
  inspected web/native source. Existing individual actions and root callbacks are
  the starting point, not permission to duplicate business logic.
- Native authorized artwork was implemented in `0f9f12b`: artwork.rs/client.rs and
  ArtworkCover have bounded request/presentation state and data-URL rendering.
  Loading/denied/failed/missing currently share a neutral visual fallback. Extend
  those distinct states; do not repeat the older claim that native always ignores art.
- Web device appearance was implemented in `1618e63` with 80–150% scale and saved
  System/Dark/Light. This is a documented deviation from the earlier proposed 75–200%.
  Respect actual validation/storage and do not silently widen it for menu positioning.
- Native scale/Settings work was initially dirty/in flight in ui.rs, lib.rs, style.css,
  examples/native-ui-qa.js and untracked presentation.rs. It was independently committed
  as `eb36048` during this documentation task. It now provides the actual 80–150%
  device store and Settings; preserve it rather than implementing a competing store.
- Concurrent `9c1ba4f`/`8c339e3` added and narrowed native artwork evidence: ART01
  has an actual 2288×937 window result; ART02 and ART11 are PARTIAL. Detail/search,
  high-DPI and denied/foreign-art visual checks remain unverified as recorded in
  QA_EVIDENCE. These are another workstream's historical results, not runs here.
- Unrelated prompt.txt is not read or changed.
- The source and frozen runtime are separate. Latest dated artwork/scale tests are
  historical; no browser/native run, accessibility certification or performance
  measurement is performed by this documentation pass.

## Experience rules

1. A loading shape represents real pending content; a progress number represents
   measured work. Unknown media is not silently replaced with fake song names/art.
2. Known titles/controls remain usable while unrelated data refreshes. A current
   audio buffer operation does not skeletonize the entire library.
3. Right-click, visible More, keyboard invocation and touch sheet share one target-
   bound command model. No decorative action or implicit permission grant.
4. Current-row Play/Pause, queue occurrences and explicit context Play retain their
   established meanings. Menus and motion never own audio or recreate the source.
5. Every gesture has an accessible alternative, a cancellation path and a bounded
   resource cost. No surprise autoplay from hovering or opening a preview.
6. Reset/Undo/retry are real transactions with honest limits. A failed request is
   not a successful animation; an uncertain timeout is not a confirmed rollback.
7. Keep original Vela mint/dark/light tokens and SVG family. Use artwork shelves,
   typography and rows; containers are added for real grouping, not as decoration.

## Recommended first release

Deliver geometry-matched row/artwork skeletons with reduced motion and race guards;
single-item track context menus with Play/Pause, Play next, Add to queue, Favorite,
Add to playlist, Go to album/artist and actual track details; keyboard/visible-More
parity; then acknowledged action receipts/Undo and a real playlist picker.

Do not delay these daily improvements until rooms, DSP models or federation finish.
Do not show room/radio/download/share actions when their necessary source rights,
backend or platform support is absent. Later recipes make those richer flows precise.

## Evidence and sources

Better Design MCP UI/UX principles guide the proposal. Review rules and scoped
copy checks apply to documentation only; no rendered UI is created or approved here.
The receiving agent must run the actual DOM/screenshot/platform review loop.

Primary references were checked on 2026-10-19:

- [WAI-ARIA menu patterns](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/) and
  [menu button](https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/): semantic/keyboard foundation; do not introduce a desktop menubar merely because the reference includes one.
- [WAI-ARIA combobox](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/): a searchable playlist picker is a distinct widget, not a text field crammed into an action menu.
- [MDN contextmenu event](https://developer.mozilla.org/en-US/docs/Web/API/Element/contextmenu_event): invocation and browser differences; retain visible and keyboard alternatives.
- [WCAG pause/stop/hide](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html),
  [flash threshold](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html),
  [interaction animation](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html): constrain decorative motion and provide static/reduced alternatives.

Timing, menu contents, product recipes and resource budgets below are proposed Vela
decisions, not claims that these standards mandate a particular animation duration.

## Documentation validation and preservation receipt

Executed Python document checks PASS: 9 Markdown files; 96 unique ordered IX briefs;
18 LOAD, 22 MENU, 16 ACT, 12 MOT and 12 FLOW acceptance rows; 11 IN package definitions;
54 relative document links across this package and the three updated indexes;
26 consistent tables inside the package. No undefined EX-range reference, unfilled
placeholder or unmatched code fence was found. This validates document structure,
not the future application's behavior.

Better Design scoped comprehension checks PASS with zero findings for these proposed
copy samples: “Your library / Loading tracks…”, “Added to queue / Plays after the
current track. / Undo / Open queue”, and “Add to playlist / Choose a playlist for
this track. / Create playlist / Cancel”. Menu-keyboard, whole-screen comprehension,
rendered spacing, screenshots, audio, accessibility and platform tests remain NOT_RUN
by this task. Review guidance was loaded; no rendered visual approval is claimed.

Preservation comparison covers 448 non-Markdown working-tree files, including the
initially untracked presentation.rs. Of these 446 have identical SHA256 values;
the two native QA driver files changed in independent commit `9c1ba4f`. The initially
dirty native Settings code was independently committed as `eb36048`; this task did
not author/stage those source changes or the later artwork QA/document corrections.
Only the playbook and its parent README/PROGRESS/ROADMAP indexes are this delivery.
No application builds/tests, data/runtime changes, external publication or subagent
delegation occurred. Unrelated prompt.txt was not read, staged or changed.
