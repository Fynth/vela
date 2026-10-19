# Dioxus integration and command contracts

## 1. Reuse the actual application

Web roots: main.rs, experience.rs, browser_player.rs, playback.rs, now_playing.rs,
personal_mixes.rs/radio.rs/listening.rs and shell.rs. Existing root actions already
coordinate transport, favorites, queue and navigation. Native root: ui.rs with
audio/client/artwork/offline/cache/remote services and current presentation work.

Build a small coherent interaction layer, not another API/client/audio subsystem.
Conceptual modules may be interaction state, command projection, context-menu/
picker presentation and loading primitives. Names are illustrative; create a file
only when it reduces coupling and has an actual consumer. Shared pure value types
may be reused like the existing playback module or moved into a cohesive client
boundary only after real duplication justifies it.

No database access in components, no new framework, no copied service token into
DOM, and no extra HTMLMediaElement/Rodio/Media3/AVFoundation owner.

## 2. Proposed command and target model

| Record | Required information and invariants |
| --- | --- |
| EntityRef | Canonical origin/server ref, object type and stable object ID; account ownership context separate |
| OccurrenceRef | Queue session/revision and occurrence identity; never only track ID/index |
| PlaylistEntryRef | Playlist/revision and entry ID, with independently evaluated media availability |
| ActionContext | Surface/context, exact target or frozen selected refs, invoking anchor/focus handle, account/origin/lifetime ticket |
| ActionAvailability | Supported/denied/unavailable/busy with safe reason and allowed recovery; this is a hint, not authorization |
| Command | Closed typed action kind and validated arguments, expected revision/mutation ID where needed |
| CommandReceipt | Actual acknowledged target/count/revision/outcome, safe Undo description or reconciliation state |
| PendingContent | Real operation identity, context, current lifecycle, prior allowed snapshot, optional actual progress/stage |
| OverlaySession | Root menu/picker/dialog identity, owner layer, target ticket, focus-return strategy and cleanup handles |

These are design contracts, not existing DTO names. Keep framework-free values free
of Dioxus/HTTP dependencies. Root/services translate commands to existing authorized
handlers. Do not put raw URLs, paths, credentials or arbitrary SQL in action arguments.

## 3. Capability projection and dispatch

Menu construction should be synchronous from known current capabilities whenever
possible. Do not wait for every remote server to open a menu. Unknown support can
show a meaningful disabled reason or a separate resolve action; it must not show
fake enabled commands. The same projection drives visible More and touch sheet.

At invoke, check current account/session, target identity/lifetime, request generation,
source adapter support and local queue bounds. Server writes/streams independently
recheck rights. A target still visible after revocation is not proof it remains playable.
Every structural queue change synchronously updates PlaybackOrder and stable refs.

Play-current toggles transport; explicit Play album/radio/mix starts that context.
Opening menus/details/skeletons/peeks never invokes either. Queue occurrences and
playlist entries are distinct command targets. Unknown/removed targets produce a
fixed safe explanation, not best-effort title matching.

## 4. Async reads, writes and receipts

Menu/picker reads capture session, origin, query, menu instance, target, generation
and lifetime. Physically cancel superseded reads; independently reject late replies.
Closing a menu cannot reopen it from a delayed load or reveal the previous account's
playlist names. Native artwork keeps its existing store/session gates.

An initiated mutation belongs to a service command, not to a soon-unmounted menu
component. Closing the menu cancels presentation work, not a fictional rollback of
an already committed server write. Receipt projection checks current account/privacy/
lifetime before rendering private names. Ordinary catalog refresh can be deferred
while feedback is pending, but account/privacy changes invalidate immediately.

Generate one mutation ID per intended operation and preserve it through uncertain
retry. Do not generate another ID on every busy-state rerender. Known failure may
rollback a reversible optimistic value; unknown timeout must reconcile first.
Undo is a new independently authorized operation referencing the acknowledged
receipt/revision, not a global stale-state restore.

## 5. Loading component contract

Presentation receives a real operation state, layout variant, visible placeholder
count, motion policy and optional concise status. Components own no HTTP task and
do not start loading because they have mounted. The operation/service owns cancellation
and completion. Timer cleanup belongs to the component lifetime and request ticket.

Skeleton geometry reuses real layout tokens/templates. Stable bar proportions are
deterministic, not rerender random values. Cap placeholders by visible viewport.
Avoid a global loading boolean that hides the whole app whenever any API is pending.
Known data can remain visible only in the same currently authorized context.

## 6. Overlay host and geometry adapter

One root overlay stack coordinates menus, pickers, sheets and Now Playing ownership.
Choose owner layer deliberately. A menu inside a native HTML dialog uses a host
that is actually interactive in its top layer. Closing the owning dialog closes
children before unmount. Focus return and cleanup are consistent across Escape,
outside pointer, Back, navigation and account replacement.

Geometry measurements are on the presentation thread, bounded and scheduled around
actual open/resize changes; do not measure every row each audio clock tick. Menu
positioning uses one defined CSS/viewport coordinate adapter validated at actual
scale/DPR. Use a supported observer and dispose it; do not multiply all dimensions
by DPR or copy a CSS-transform trick from an unrelated framework.

Open native bindings remain target-specific. On web protect native browser context
and zoom behavior. On desktop do not expose unrestricted WebView/native commands
through the context menu. Mobile sheets share commands but not identical window math.

## 7. Performance and resource budgets

| Area | Proposed bounded behavior, to measure before final tuning |
| --- | --- |
| Menu mount | One active root menu, one submenu depth; render actual limited actions only |
| Menu async work | At most one current picker query/resolve per overlay ticket; cancel obsolete work |
| Placeholder count | At most a visible viewport plus small tail; no huge animated DOM list |
| Motion | No app-level per-frame state writes for CSS-only decoration; static when hidden/settled |
| Geometry | Measure opening/anchor/viewport changes, not every pointer pixel or playback tick |
| Batch selection | Initial bounded IDs, e.g.200; actual server limits and result semantics govern chunks |
| Receipts | Bounded small per-account session list with no credentials/private path exports |
| Images | Reuse existing bounded artwork request/presentation/cache; no duplicated loader/cache authority |
| Playback | Current source takes priority; no menu/animation operation changes source generation alone |

Measure cold/open menu interaction, heavy list scroll, source preparation and idle
work separately. Local menu feedback should be immediate; an initial target is
within 100ms on the reference host, not a measured claim. Do not prefetch audio on
hover to make menus look responsive or attribute network throughput to animation.

## 8. Styling and packaging

Reuse current tokens/icons and stylesheets initially. If a separate CSS/JS asset is
justified, add the exact server allowlist and release-asset verification entry as
required by AGENTS. Native include/package paths require their own evidence.
Do not implement web-only event wiring and declare native/mobile parity from compilation.

## 9. Storage and migrations

The first skeleton/menu slice should need no schema migration. Add new state only
when a real persisted feature requires it: account-safe listening shelf, menu-related
preferences, receipts or saved session refs. Prefer current presentation service for
device settings and existing mutation/revision services for shared data.

No backend tables solely to animate loading. No private menu targets, search queries,
song names or listening data sent to external design/analytics tools by default.
Persistent user-data changes remain additive and require existing backup/restore
discipline. Device preferences contain no token, share credential or audio URL.

## 10. Suggested file ownership for later implementation

Integrator owns main.rs/ui.rs, shared action wiring/contracts, Cargo/migrations,
asset allowlists and builds. A presentation contributor can own isolated menu/
skeleton/picker components/styles. A playback contributor can own agreed pure
occurrence/command tests without editing both roots concurrently. QA/security can
own fixtures/read-only review/tests with the lead scheduling real runs.

This documentation task does not delegate agents. A future authorized task may use
actual available concurrency, with no overlapping root edits and at most 2 Cargo
compiler jobs. Review and integrate every contribution before declaring a feature.
