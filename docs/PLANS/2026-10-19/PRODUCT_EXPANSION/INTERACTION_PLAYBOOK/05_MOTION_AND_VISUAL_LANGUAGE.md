# Motion and visual feedback specification

These are proposed Vela tokens/behaviors, not a new animation library or measured
performance claim. Keep the current original mint dark/light system and existing
SVG family. Extend actual components; do not replace Dioxus with a second framework.

## 1. Visual compositions

Use rows for songs/settings, borderless artwork shelves for albums, editorial text
and varied section width for discovery, and real overlay surfaces for menus/pickers.
A skeleton may contain simple blocks because it represents content geometry; loaded
content must not inherit a giant empty card shell merely to match that placeholder.

The player is visually stable across routes. Current track is recognizable by label/
icon and restrained color, selection by a separate treatment, pointer hover by a
quiet surface, and focus by an unmistakable outline. Do not overload mint to mean
current, success, selected, warning and every border at once.

Artwork-derived color is optional and bounded; preserve readable text/controls.
Artist, album, playlist mosaic and account/avatar are distinct shapes/content types.
No perpetual glass blur, glowing border, bouncing icon or gradient headline is needed
to communicate that a playlist is ready.

## 2. Proposed motion tokens

| Token | Starting value | Usage and constraints |
| --- | --- | --- |
| Instant feedback |0–80ms | Pressed/active state follows actual input immediately |
| Quick transition |100–140ms | Local icon/surface/content reveal; no delayed command |
| Overlay entrance |120–160ms | Opacity and at most 4px travel from actual anchor |
| Overlay exit |80–120ms | Pointer blocker/focus lifetime disposed immediately, not after decorative delay |
| Panel transition |160–220ms | Short route/peek/queue reveal if supported without layout jank |
| Reorder settle |120–180ms | Stable-item translation after actual reorder, not duplicated ghost rows |
| Shimmer cycle |1600–2200ms | Low-contrast group highlight, stops/static after policy timeout |
| Hover intent |150–200ms | Submenu/peek opening only; click/key remains immediate |
| Success receipt |Up to 8s UI display | Pause dismissal on focus/hover; actual operation state separate |
| Touch long press |About 450ms | Cancel on move/scroll/pointercancel; More remains available |

Use a simple shared ease-out for entrances and symmetric/ease-in-out for subtle
status transitions. Do not turn a timeline into twenty unrelated spring systems.
Reduced motion removes travel/scale/looping decoration; immediate state/color/text
feedback remains. Final timings are tuned from actual devices and user task tests.

## 3. Button and icon feedback

Button press changes surface/outline without moving neighboring hit targets. A
small visual scale effect may be tested inside a fixed hit box, but is unnecessary
if it causes text blur or repeated-click instability. The event dispatch occurs on
the real input contract, not after an animation completes.

Favorite may briefly acknowledge an accepted/reversible optimistic change using
existing heart geometry. No confetti for ordinary library housekeeping. Play/Pause
icon changes observe root intent; errors/actual engine state remain visible.
Loading content stays inside a fixed button shell so its width does not jump.

Hover styles live behind hover/fine-pointer conditions and have keyboard equivalents.
Selected/pressed/current/focus states take precedence over hover; moving the pointer
away must not erase current-track meaning. Essential actions remain available on touch.

## 4. Cover reveal and contextual color

Reserve actual aspect ratio and rendition bounds. Decode first, then optionally
reveal over 100–140ms. Do not animate cover dimensions or run a full-page transition
whenever a tiny cached row image appears. Repeated current artwork references share
the existing authorized request/presentation service.

When the track changes, the current artwork/title swap follows the accepted source/
selection state, not a stale HTTP completion. A brief crossfade between images is
presentation only; do not imply audio crossfade is implemented. Stop animation in
hidden/offscreen surfaces and release old images promptly.

## 5. Menus, sheets and drawers

Menus emerge from pointer/trigger geometry with a restrained surface and readable
separators. A context-menu arrow/pointer decoration is optional; correctness at the
viewport edge is more important. Do not use blur over thousands of moving list rows.

Modal sheets explicitly dim/inert background; nonmodal menus do not pretend to be
dialogs. A drawer should not obscure the current player or quietly create a second
scroll trap. Keyboard users must be able to open/close without perceiving motion.
An opening/closing animation is interruptible: a second action always wins.

## 6. Reordering and spatial feedback

Use one lightweight drag ghost and one insertion line. Other rows may translate
to show the potential drop, but their stable identity and focus remain intact.
No repeated network mutation while a pointer hovers. Show pending/committed/conflict
states after drop based on the actual local/server command semantics.

Cancel returns to the prior order and sends no new command. Reduced motion uses
an insertion line and immediate stable order update. Autoscroll is bounded and
restricted to the owning list, never the whole window unexpectedly.

## 7. Loading, success, error and unavailable language

Each state has readable text and optionally an icon; do not communicate error only
through red or loading only through movement. Never animate fake progress to 90%.
Use determinate values only from actual bytes/items; explain stages separately.

Success feedback names the real outcome and actual affected scope. Errors preserve
input/selection and offer an available recovery path. Missing image/metadata is a
neutral state, not a red failure banner. A denied remote grant explains access
without revealing private owner details.

## 8. Motion preferences and power behavior

Integrate with the current presentation preference service/schema, not another
standalone store. The parent registry's motion/loading settings remain proposals
until wired. Suggested effective policy: system reduced motion constrains decoration;
explicit user reduction is honored; hidden windows/offscreen placeholders stop;
available platform power-saving signals can reduce discretionary work.

A “Full motion” choice must not override a system accessibility restriction silently.
No network or waveform decoding is justified solely to animate a loading state.
Do not use an always-running Rust/JavaScript render timer where CSS/static state
will do. Recheck compositor behavior on actual WebKitGTK rather than assume GPU
promotion is free or that DMABUF guarantees a particular refresh rate.

## 9. Rendering and accessibility review

Review same content before/after, including long metadata, missing covers, current
and selected rows, menu/picker open, loading/failed/offline states. Test dark/light,
current implemented 80–150% app scale, text enlargement and future larger scales only
after the validated range changes. Do not claim unsupported scale combinations passed.

Native window frames and platform conventions remain outside the animation system.
Do not add another native titlebar or modify Hyprland rules to disguise a layout issue.
No new font/illustration/icon package is needed for this interaction increment.

## 10. Motion acceptance

| ID | Required observation |
| --- | --- |
| MOT01 | Press/hover/focus/current styles do not shift geometry or erase one another |
| MOT02 | Reduced motion preserves content/actions/status while removing decorative movement |
| MOT03 | No flashing/high-contrast repeated pulse; actual rendered safety checked |
| MOT04 | Shimmer/playing indicators stop according to hidden/offscreen/settled policy |
| MOT05 | Overlay close cannot leave invisible blockers or delay a new action |
| MOT06 | Artwork/title reveal follows current target generation and preserves aspect/contrast |
| MOT07 | Reorder/cancel is stable with duplicate IDs and keyboard alternative |
| MOT08 | Button pending label keeps width and cannot double-submit |
| MOT09 | Receipt timeout pauses on focus and does not claim unacknowledged success |
| MOT10 | Source/position/queue remain unchanged by purely visual motion/theme/scale |
| MOT11 | Actual browser/native traces show bounded layers/work with no idle render loop |
| MOT12 | Current scale/text/locales/RTL/AT and dark/light states have explicit verified/unverified evidence |
