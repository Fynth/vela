# Context menus that behave like a finished music application

## 1. Scope and entry points

Provide the same entity actions through right-click on desktop, a visible More
button, keyboard invocation and a touch action sheet. Reuse the original Vela SVG
family and existing root/application actions. No second audio engine, duplicate
favorite service or unrestricted native file/URL opener is introduced.

Intercept a context event only on a recognized Vela entity/action surface with a
valid target. Preserve browser/OS menus for text inputs, selected text, editable
metadata, external links and unrelated blank space. Do not suppress context menus
globally on body/WebView. Preserve host modifier behavior; Firefox can bypass a
custom context event with Shift+right-click, as documented by MDN.

Suggested product decision: right-click focuses its target without playing it.
If it is outside the current selection, target only that item; if it is inside a
multiselection, operate on the frozen selected set. Preserve selection until a new
selection action, not merely until the menu closes. Phase 1 may ship single-item
menus before multi-select; do not imply bulk semantics before they exist.

## 2. Open/close lifecycle

State: Closed → Opening(target,anchor,ticket) → Open → optional Picker/Confirm →
Dispatching command → Closed with receipt. Only one root action-menu stack is active.
Opening another target disposes old hover/typeahead/measurement/request state.

The ticket captures menu instance, stable target reference, origin/account/session,
view lifetime and selection/provenance snapshot. Queue occurrence/playlist entry
IDs are required when the action is occurrence-specific; track UUID/title/row index
alone cannot identify a repeated entry. Keep the target immutable while a submenu
or async picker is open. Background selection changes must not redirect an action.

Close on Escape, click/tap outside, completed action, new route/account/origin,
target removal, invoking another menu, lost owning view or incompatible overlay.
Window blur closes sensitive popups. On scroll/zoom/resize either reposition to the
same measured anchor or close safely; do not leave a floating menu detached from
the row. Successful service mutation can finish after presentation closes, with
its own independent session-safe receipt; closing never promises server rollback.

Return focus to the More trigger/target row when still present. If virtualization
or deletion removed it, use the next stable nearby row or section heading. Do not
scroll the entire library to the top or focus a recycled unrelated DOM element.

## 3. Target and capability projection

Build menu items from target type plus actual support/permission state. Visibility
and availability are independent: omit meaningless unsupported features, but show
a useful denied/unavailable action with a concise reason when it teaches recovery.
Use a closed command registry; never infer access from membership in a playlist.

| Target | Context needed | Examples of meaningful actions |
| --- | --- | --- |
| Owned track | Origin/account/track ID and source capabilities | Current Play/Pause, Play next, Add to queue, Favorite, playlist picker, album/artist, details |
| Queue occurrence | Queue revision and occurrence ID | Play/Pause current, Move next/up/down, Remove occurrence, details |
| Playlist entry | Playlist ID/revision and entry ID | Play permitted entry, remove/reorder entry when allowed, details |
| Album | Album/edition ref, actual tracks/grants | Play album, Shuffle album, Add next/last, save/download when supported, details |
| Artist | Artist ref and actual playable catalog scope | Open, Shuffle available, Radio when supported, favorite/follow when real |
| Playlist | Playlist ref/role/revision | Play available, edit/duplicate/export/share/delete appropriate to role |
| Pinned download | Local manifest/origin/account/asset identity | Play offline, verify/repair, remove local copy, details |
| Remote/cached metadata | Origin/object ref and current live authorization state | Open origin/details, Retry live, permitted Play; cached browse alone grants nothing |
| Multiple items | Stable refs and count plus capability intersection | Add next/last, add to playlist, favorite, safe supported batch action |

## 4. Default single-track menu

Use a concise first level, with roughly 8–12 useful actions in at most four short
groups. Omit empty groups and unnecessary separators.
Do not create a scrollable catalog of every future feature on an ordinary song.
Full details, share and output selection can open their own surfaces.

| Order/group | Label | Required behavior |
| --- | --- | --- |
| Listening | Play or Pause/Resume | Current occurrence uses actual root intent; another target starts its explicit displayed context |
| Listening | Play next | Insert with established explicit-next priority, not replace current context |
| Listening | Add to queue | Append a distinct occurrence; current source/time untouched |
| Organize | Add to favorites / Remove from favorites | Real owner mutation; busy/receipt semantics, correct current label |
| Organize | Add to playlist… | Open the dedicated searchable playlist picker with this exact target |
| Organize | Save for offline / Remove offline copy… | Only when implemented and authorized; removal preview distinguishes server original |
| Explore | Go to album / Go to artist | Resolve actual known refs; missing tags do not create fake objects |
| Explore | Start track radio | Only after actual radio capability/seed eligibility; context Play remains a new queue command |
| Explore | Track details | Known metadata/encoding/origin and permitted credits; unknown fields stay unknown |
| Share | Share… / Copy Vela link | Actual share flow or stable object link; never a raw long-lived media credential |

Reduce this set according to actual context/capabilities. “Remove from queue” acts on
an occurrence, while “Remove from playlist” acts on an entry: never use one ambiguous
Remove command for both. Deleting original media is not a casual context-menu action.

## 5. Keyboard and accessibility

Use one consistent APG-style popup-menu behavior, validated with actual assistive
technology. The table states the implementation contract; it is not a new menubar.

| Key/action | Result |
| --- | --- |
| More button, context-menu key or Shift+F10 | Open on the focused entity |
| Up/Down | Move within items; wrap consistently |
| Home/End | First/last item |
| Enter/Space | Invoke item; open submenu when applicable |
| Right/Left | Enter submenu/return to parent; direction adapts to RTL |
| Escape | Close current submenu, then root menu; restore focus |
| Tab/Shift+Tab | Close and continue outside, not an endless menu focus trap |
| Printable text | Bounded typeahead among action labels; no playback shortcut |

Use named menu/menuitem/check/radio/disabled semantics, expanded state on submenu
triggers, noninteractive separators and an explicit label identifying the target.
Disabled items may receive menu focus for their explanation but never dispatch.
Do not use a native disabled button in a way that removes a needed explanation.
The menu is not a modal dialog. A subsequent picker/confirmation uses its own correct
focus model. Suppress global transport shortcuts while a menu/picker handles keys.

## 6. Positioning, scale and top layers

Anchor pointer invocation to event client coordinates; keyboard invocation uses the
focused row/More bounds. Clamp to visible viewport with an 8px inset, flip horizontally
and vertically near edges, and constrain height with internal scroll. A default
width near 256–300 CSS units is a starting token, not a fixed minimum that breaks
small/high-scale layouts. Labels wrap or truncate accessibly; text size remains legible.

Use actual viewport/anchor geometry in the chosen rendering coordinate system. Never
multiply client coordinates blindly by devicePixelRatio. Account for current CSS
zoom/native WebView zoom, visual viewport, scroll and safe areas. Recompute after
font load/scale changes; tests must click actual item rectangles, not guessed pixels.

A high z-index outside an open native HTML dialog cannot solve top-layer/inertness.
Mount the overlay host inside the active owning dialog layer, or use an actually
supported top-layer primitive with verified focus behavior. When Now Playing opens
a menu, its target, focus return and background inertness must remain correct.
Avoid a global portal that becomes unclickable behind the modal while visually shown.

Submenus use bounded intent delay, e.g.150–200ms for hover-only opening/closing, with
safe pointer travel toward the submenu. Keyboard activation is immediate. Keep at
most one submenu depth; deeper workflows become a named picker/dialog. Hover never
executes a command, starts audio or changes playlist membership.

## 7. Searchable Add to playlist picker

This is a real picker/dialog or combobox/list, not a menu role containing arbitrary
interactive fields. Opening carries immutable target refs and count. Show eligible
real playlists, search, recent destinations where legitimately retained, and create
new playlist when the account has that actual capability.

States: loading, ready, no eligible playlists, no search match, adding, success,
permission changed, revision conflict and transport uncertainty. Existing bounded
playlist APIs can seed the first slice; never invent a nonexistent global search or
silently pretend the first 200 are every possible playlist without a bound disclosure.

Selecting a destination sends one idempotent revision-aware command. Keep target/
destination names visible while adding. A new destination search cannot redirect
an in-flight add. Known failure preserves context and offers retry; uncertain result
reconciles before repeating. Success confirms actual inserted count and offers Open
playlist or safe Undo where the receipt supports it. Duplicate handling is explicit.

## 8. Multi-selection and batch commands

Phase 2 introduces checkbox/keyboard range selection and a contextual action bar.
Right-click inside it targets the frozen selection; outside it opens that single
row. Keep total and scope explicit: selected visible items differs from all matching
results. Do not apply actions to unloaded unknown rows without an accepted query-
selection server contract and permission evaluation.

Compute action capability intersection plus per-item exceptions. For a mixed set,
show actual eligible/denied counts where known; do not silently omit failures and
report complete success. Start with a bounded selected-ID count, e.g.200; server
contracts may impose smaller chunks. Define transactional versus partial semantics
before adding a bulk action. Do not loop unbounded individual HTTP writes from UI.

Queue insertion preserves selected visual order/duplicates and MAX_QUEUE bounds.
Favorite batch is reversible per acknowledged target. Download jobs obey quota/
concurrency. Dangerous removal needs exact scope/dry-run and explicit confirmation.
Late partial receipts cannot mutate another account or a newly selected set.

## 9. Touch, trackpad and native integration

More is always discoverable; long-press is an additional route. Suggested long-press
baseline 450ms with cancellation on movement beyond about 10 CSS units, scrolling,
pointercancel, selection gesture or app blur. Use a dedicated drag handle so reorder
does not compete with long-press on the whole row. Preserve native scroll and text
selection; do not apply touch-action:none globally.

On small touch viewports open an accessible action sheet with the same command IDs,
target summary,44-unit targets, safe areas and explicit Close. Modal sheets trap
focus; nonmodal menus do not. A drag dismissal, Escape, Back and Close share cleanup.
Do not dismiss unsaved drafts silently. Tablet/desktop switches to popover if space
allows. OS/native context menu integration is optional and must preserve this command
registry and capability model rather than diverging from the web client.

## 10. Safety, stale state and receipts

Do not keep a menu open across logout/account switch or reuse a target after row
recycling. Queue/playlist revisions can change while open: resolve stable identity
at invocation, reject stale/removed targets or show a conflict. Never guess by index.
The server independently authorizes every media/share/edit operation.

Copy actions use a reviewed clipboard adapter; failure offers selectable safe text.
Copy link must not expose session tokens, CSRF, signed stream URLs or private paths.
“Show in folder” is meaningful only for an actual permitted local file capability,
not the server's private storage path. No shell command comes from a song title.

## 11. Acceptance requirements

| ID | Required observation |
| --- | --- |
| MENU01 | Right-click opens without playing; default browser menu remains outside recognized surfaces |
| MENU02 | Visible More and keyboard reach the same actions/target as pointer invocation |
| MENU03 | Current Play/Pause and explicit next/last preserve established queue/audio semantics |
| MENU04 | Duplicate queue/playlist occurrences dispatch by stable identity after reorder |
| MENU05 | Outside click/Escape/route/logout/target removal close and dispose deferred work |
| MENU06 | Focus returns correctly, including virtualized/deleted targets and Now Playing |
| MENU07 | Edge/zoom/HiDPI/scroll positioning is clickable and inside actual visible viewport |
| MENU08 | Submenu intent works without hover activation; keyboard navigation immediate |
| MENU09 | Disabled/unsupported/denied capabilities explain truthfully; forced action reauthorized |
| MENU10 | Native dialog/top-layer menu works without losing background inertness or player continuity |
| MENU11 | Playlist picker loads/searches real data, handles empty/failure and uses correct semantics |
| MENU12 | Add-to-playlist uses mutation identity/revision; repeated activation cannot duplicate writes |
| MENU13 | Uncertain timeout reconciles before retry; receipt/count/Undo reflects actual outcome |
| MENU14 | Multi-selection scope/capability intersection and partial outcomes are explicit |
| MENU15 | Right-click outside selection targets one row; inside targets frozen set |
| MENU16 | Touch long-press cancels on scroll; drag handle does not accidentally open sheet/play |
| MENU17 | Text/IME/global shortcuts/RTL/assistive navigation remain functional |
| MENU18 | Copy link/clipboard failure uses no credentials or private media/file paths |
| MENU19 | Context action remains bound through held replies, account/origin changes and recycled rows |
| MENU20 | Dark/light/reduced-motion/current app scale and long localized labels pass screenshots |
| MENU21 | Actual metadata-only collaborator/public-cache negatives remain enforced |
| MENU22 | No duplicate audio owner, global context suppression, event-listener leak or idle animation loop |
