# Behavior and state contracts for the transfer

Keep the approved visual hierarchy while preserving Vela's stronger production
behavior. This file is a proposed contract, not a new API specification already in
the source. Reuse the actual root/services and named tests.

## 1. State ownership

| State | Authority | Presentation may do |
| --- | --- | --- |
| Authentication/session/CSRF | Existing account/session services | Dispatch authenticated requests; never persist credentials in new UI stores |
| Queue/order/cursor/source intent | Existing web root/playback domain or native root+AudioEngine | Read projections and dispatch typed commands |
| Actual media phase/time/output | HTMLMediaElement/browser coordinator or native AudioEngine | Render actual state; do not infer Playing from click or download completion |
| Current route/back-forward | Root navigation service | Request navigation with a stable destination/view snapshot |
| Search query/filter/cursor | One root/page request context | Edit query and display current authorized result |
| Favorites/playlists/revisions | Existing server-authoritative services | Optimistic reversible state only with safe failure/reconciliation |
| Art | Existing authorized image adapter/cache | Show pending/ready/missing/denied/failure without another loader/token |
| Scale/theme | Existing device preferences | Change through the validated service; no duplicate localStorage/config file |
| Private history | Existing consent/revision-qualified listening service | Read permitted history and explicitly dispatch privacy actions |
| Dialog/menu/drawer | Root overlay lifecycle | Own focus/layer/close affordances; never own another player |

Keep local UI drafts, durable device preferences and server state separate. Root
refactoring must not create parallel copies of `playing`, queue or auth in each page.

## 2. Transport command table

| User action | Required behavior |
| --- | --- |
| Play on another track | Resolve exact origin/asset and displayed playback context; start via existing root |
| Current occurrence Play/Pause | Toggle requested transport; retain source/seek/queue/order, including preparation-pending cancellation |
| Another duplicate queue occurrence | Select that exact occurrence; do not collapse by track ID |
| Play album/playlist/mix | Explicit context command with real ordered eligible items and stated replacement/append semantics |
| Open album/artist/Now Playing | Inspect/navigate without an implicit source start |
| Play next | Insert with established explicit-next priority; no current-source reset |
| Add to queue | Append a new occurrence; duplicates allowed under existing bounds |
| Manual Next | Existing manual-next policy, distinct from natural EOF under Repeat One |
| Previous | Existing seek-restart/history policy; changing the visible page does not change it |
| Shuffle | Existing fair deck/history, not random selection from a shrinking track-ID set |
| Repeat | Actual supported Off/All/One; state/label consistent across every surface |
| Seek | Preview while scrubbing, one committed supported seek, clamp/reject unknown/unseekable cases |
| Mute | Retain prior audible volume and restore it; do not always restore a hard-coded 65% |
| Navigation/theme/scale/overlay | No source generation change or output reinitialization solely from this action |

The prototype's `playing` flag is insufficient to replace Vela's requested/actual/
loading/recovering/error state. A stale Play promise cannot replace a newer selection.
At EOF, a new explicit replay can be a new logical listen under the existing history
contract; a visual pause/resume is not a new listening session.

## 3. Context identity

Track DTO IDs are only one layer. Preserve origin/server, physical source ref,
recording/group identity where known, queue occurrence, playlist entry, playback
context, expected revision and account/lifetime ticket. Row indices and titles are
never sufficient identities after sorting, pagination, edits or duplicate insertion.

Opening a context menu freezes its actual target. Right-click never starts playback.
Visible More, Shift+F10/context-menu key and touch sheet share the command registry.
Keep native browser menus for editable/selected text and unrelated surfaces. Reuse
the [detailed menu plan](../PRODUCT_EXPANSION/INTERACTION_PLAYBOOK/02_CONTEXT_MENUS.md)
with the new palette/geometry, not its older visual direction.

The plus at the end of a track row has an explicit destination. In a normal library
row it means Add to queue; inside a playlist add-picker it means Add to this playlist.
Use distinct accessible names and receipts. Add-to-playlist selection is a real
searchable picker, not an empty button that logs or silently chooses the first list.

## 4. Home data binding

| Reference section | Production source and fallback |
| --- | --- |
| Editorial story | An actual owner-selected/pinned curated playlist/collection ref; empty state can open a real chooser |
| Three story indicators | Actual configured stories; fewer targets mean fewer meaningful selectors, not dead dots |
| My Wave | Existing eligible private mix/radio results, true reasons and explicit context Play |
| Again in headphones | Actual consent-qualified recent/repeat album candidates where supported; otherwise rename honestly to Your albums |
| Recently in collection | Actual added-at order, bounded first page, no fabricated play statistics |
| Favorites count | Actual server count with correct scope, not `loaded_page.len()` presented as a global total |
| Sidebar playlists | Actual accessible lists/roles/counts, no prototype names seeded into user data |

Do not use retained-but-disabled history for ranking, suppression or a “recent” badge.
If current history consent is off, show catalog/favorite-driven shelves with honest
labels. My Wave can be an on-demand mix; no false daily/weekly freshness claim.

The approved decorative coast/wave art can identify an editorial module. It never
stands in for an actual media cover or proves a playlist exists. No automatic mood
classification fromRMS/FFT or unstated external AI.

## 5. Search and navigation

One query model backs topbar and Search. Preserve the current 300ms debounce,
immediate explicit submission where supported, physical abort and independent account/
origin/query/generation/lifetime checks. Enter while IME composition is active does
not submit. A cleared query cancels pending work and renders the appropriate browse
state. Old responses cannot refill the field or replace results after account switch.

Back/Forward stores destination+object ref+filters+scroll/focus anchor. It is separate
from queue history and source context. Keep playback active while a page request is
pending. Restore by stable item, not stale numeric row index. Announce actual route
change with useful focus behavior, not a full-page remount of the audio element.

Ctrl/⌘K focuses search unless a modal/IME/assistive key contract owns the event. Search
query synchronization must not refetch both topbar and page clients independently.
No unsupported users/podcast/global-streaming search categories appear as decorative tabs.

## 6. Catalog and list scope

Genre chips reflect actual facets. Unknown genre is not silently turned into Ambient.
Sort is server-wide with stable tie-break/cursor behavior; client sorting the first 50
rows is not Artist sort for a 100k-track library. Preserve all current advanced filters
under a compact Filters surface instead of deleting them to match a smaller demo.

“Listen all” needs explicit scope. The safe first slice may say Play loaded tracks;
the full target uses a bounded resolved query-context continuation, not an unbounded
download of every track. Counts and late-page failures remain honest. Never truncate
a big playlist silently while displaying a full count and claiming complete playback.

Album/artist detail uses real grouping IDs, edition/disc/track order and current data.
Existing user metadata edits and playlist references survive a visual refactor.
Album-save and artist-follow are different from track-favorite; missing services need
accepted backend work, not deceptive heart/follow state on a representative track.

## 7. Queue drawer and overlays

Queue is a modal right drawer in this reference: background inert, Close/Escape/focus
return, own scroll and a bounded header/footer. Audio continues while underlying UI
is inert. If the current coordinator supports it, provide a real Pause/Resume in the
current block; do not make all transport unreachable while the modal is open.

Queue display reads a side-effect-free projection of actual playback order. Opening
the drawer must not consume shuffle state. Manual-next priority, context and autoplay
segments remain distinguishable where supported. Remove targets an occurrence; after
removing/moving another entry, the current source/time stay stable. Current removal
uses the existing explicit policy, never an accidental index shift to another song.

Now Playing uses the root-coordinated native HTML dialog contract. Child context
menus/pickers must be interactive in the active top layer. Close child overlays before
owner unmount. Do not combine a second legacy global focus trap with native dialog
inertness. Dismissal disposes timers, reads/listeners and focus state; an already
committed mutation is not undone just because its picker closed.

## 8. Favorites, playlist mutations and Undo

Use one real entity mutation, shared projection and receipt. Known failure can safely
reverse an optimistic change; an unknown timeout must reconcile before retry. Preserve
the intended mutation ID across retries. Do not generate another write ID per rerender.

Playlist creation trims whitespace, validates length/role and confirms actual server
success. Update sidebar and list from the authoritative result. Add/remove/reorder
uses entry identity and revision/CAS; stale replies never lower the current revision.
Collaboration does not grant media/art/download rights by granting metadata access.

Undo is a new authorized inverse of the acknowledged receipt; do not restore a stale
whole-playlist snapshot over someone else's edit. A temporary toast should not be the
only place to recover a durable failed operation. No toast for every harmless click.
Existing folder, share, invite and export flows remain reachable under the new style.

## 9. Upload, cache and source feedback

The prototype's local-object-URL import is not the server workflow. Add Music opens
actual Upload/URL import/Scan/connected-storage capabilities that exist. Allow a real
file picker/drop target, batch results, limits, duplicate receipts, jobs and actionable
errors. File-chooser cancel is not a failed import. Do not discard selected valid
files because one member fails without an explicit batch policy.

Use existing stream/cache/prefetch mechanisms. The design transfer does not implement
preload by fetching a whole audio file on hover or add a second downloader. Current
native authorized complete-file preparation and integrity policy remain truthful.
Ready next requires an actual prepared lease; buffered ranges require adapter data.
Source stages can be disclosed quietly; errors and cancel/pause intent stay accessible.

Art pending should not skeletonize known song text or disable authorized Play. Missing,
denied and failed art exit pending state. Native image credentials stay out of DOM;
use the existing bounded data-URL/store adapter. Account/origin changes invalidate
private art/menu/results even when filenames or titles happen to match.

## 10. Settings, history and less visible routes

Mobile More opens Settings directly. A compact “More from Vela” list within Settings
links actual Albums/Artists/Favorites/History/Radio/Downloads/Add Music/Network and
authorized Jobs/Admin. Desktop keeps sidebar/direct routes plus an equivalent way
to find remaining destinations. Do not force listeners through server diagnostics.

Keep app scale 80–150 in 5-point steps,100 reset, Dark/Light/System, persistence/recovery
status and current boot-time application. Settings uses actual services, not static
“Dark” text or nonfunctional toggles. Save failures show unsaved state without
discarding an unreadable preference document or changing monitor/browser scaling.

History is private and opt-in under current account AND device consent/privacy
revision. Preserve qualified advancing media-time accounting, export/erase and late
outbox suppression. Empty history, disabled history, erased history and temporarily
offline history need different truthful copy. A play click or cover animation never
becomes a listen. Network/server health remains separate from history consent.

## 11. Input and accessibility rules

Every meaningful action has a visible/keyboard path. Nonmodal menus do not trap Tab;
modal dialogs/sheets do. Space transport is disabled while a button, input, select,
contenteditable, IME or active modal handler owns it. Page key handlers never steal
browser zoom, editing or assistive shortcuts. Escape closes only the appropriate
active layer and returns focus to a stable live target.

Keep 44px touch targets as the product goal; desktop reference icons can be smaller
inside distinct nonoverlapping hit boxes. A focus ring, icon, label or actual semantic
state communicates selection in addition to tint. Long labels may wrap/reflow without
covering controls. Reduced motion retains all state/actions. Skeleton bars and
equalizer pixels are decorative and not individually announced.

## 12. Failure and stale-work rules

Every async operation captures account/session, origin, object/context, query/generation
and view lifetime. Physically cancel superseded reads and independently reject late
completion. Closing/navigating does not resurrect an old dialog, export, share link or
error toast. Switching account clears private visible results and pending targets.

Retry binds the exact safe operation with bounded backoff/deadlines and independently
rechecked rights. Permission expired, server offline, missing file, unsupported codec,
storage full and revoked share are different states. Do not replace a denied source
with a same-title recording or grant remote audio because a cached title is visible.
Record unresolved implementation gaps honestly instead of shipping decorative recovery.
