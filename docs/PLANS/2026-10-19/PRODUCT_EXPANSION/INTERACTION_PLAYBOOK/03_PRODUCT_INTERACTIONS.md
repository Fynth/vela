# Product interaction recipes, beyond decoration

Each recipe combines a clear listener outcome, concrete flow, integration boundary
and acceptance. They expand existing product-catalog outcomes rather than adding
another contradictory backlog. Ship their supported subset in dependency order.

## 1. A track row that responds correctly

Keep a distinct play target, readable title/artist, favorite, duration and More.
Hover/focus may reveal secondary actions without moving columns or making essential
actions disappear from touch. Current intent uses Pause/Resume, actual engine state
adds preparing/buffering/playing text/icon evidence. A playing indicator is a simple
bounded animation, never a purported measured spectrum if no analysis data exists.

Single click selects/opens according to current row policy; play button is unambiguous.
Double click behavior is documented and cannot cause two independent source starts.
Row click must ignore child links/buttons/sliders/menu triggers. Title/album/artist
links have distinct targets and do not unexpectedly start music when navigating.

Give immediate pressed feedback, then actual receipt/error. A failed favorite mutation
must not leave a heart permanently filled. Every related row/player/queue projection
reconciles the same track identity, including duplicated occurrences.

## 2. Action receipts and bounded Undo

Examples: “Added after the current track”, “Added to Evening”, “Moved 3 tracks”,
“Removed this download”. Use a compact root receipt area above the player without
shifting the list; screen-reader updates are polite except urgent failures.

For reversible actions retain an acknowledged operation ID, target set, expected
revision and safe inverse. Start an optional 8s receipt display; hover/keyboard focus
pauses dismissal so the action remains usable. History of recent receipts can be
opened later if an actual action-center implementation exists. Time-limited UI Undo
does not imply an operation can be undone forever.

Avoid snapshot rollback overwriting unrelated concurrent edits. Reorder Undo is a
new semantic move/CAS mutation, not restoring an old entire playlist blindly. Queue
Undo is bounded to the same account/session. For unknown transport outcome, say
“Checking whether it was added…” and reconcile, not “Failed” followed by duplicate add.
Destructive originals/grant revocation/externally delivered data do not get fake Undo.

## 3. Multi-select with clear scope

Show checkboxes on selection mode/focus and support Shift range plus accessible
Select all visible. Display selected count and context. If “all matching results”
is later supported, require an actual server query-selection contract and clear
count/revision semantics; selecting a page must not silently select the whole library.

The contextual bar offers only capability-compatible actions: next/last, favorite,
playlist, download or remove where actually supported. Preview mixed eligible and
unavailable members. Keyboard users can leave selection without losing playback.
Clear selection on account/origin change; a delayed write receipt targets its frozen
set, never the current newly selected rows.

## 4. Queue drag with a real insertion preview

Use a dedicated handle, stable occurrence IDs and a clearly visible insertion line.
The drag image is the selected title/cover/count, not a full screenshot including
private unrelated content. Bounded autoscroll near container edges; Escape/pointer
cancel restores the original layout and sends no mutation. Drop resolves stable
neighbor IDs/revision and produces one reorder command.

Keep the active source and position. A current occurrence moves visually while its
identity stays active. The actual-next preview updates only after a successful local
structural model update/accepted shared command. A merely hovered drop target does
not consume shuffle state. Move up/down and “Move next” remain keyboard alternatives.

Dragging into a playlist/sidebar can open a lightweight destination confirmation
after a valid hover delay. Do not silently create a playlist, publish music or move
an original file. Cross-origin items retain source references and rights checks.

## 5. A temporary listening shelf

Offer “Keep for this session” as a reversible personal staging area for album/track
refs while browsing. It is distinct from the actual queue and from a saved playlist.
The user collects a few ideas, then explicitly chooses Play next/Add all/Create
playlist/Discard. Show real count/duration where known and origin availability.

Store a small bounded account+origin-scoped reference list without credentials;
never retain another account's items after switch. Do not request audio merely by
adding a shelf ref. The shelf is optional and collapsible, avoiding another permanent
sidebar or a new mandatory step before listening. Reuse shared queue/playlist commands
when the user commits it.

## 6. A listening session that fits the available time

“Build a 45-minute session” opens a bounded planner from an album, playlist or selected
tracks. Show the actual selected candidates, known total, unknown-duration count and
an honest target range. Let the user pin opening/closing tracks, preserve albums as
blocks, choose diversity and shuffle preview. Recompute without starting audio.

Use deterministic bounded selection/ranking on accessible candidates, not an LLM
inventing song IDs. Exact target length is not guaranteed. “Play session” explicitly
replaces/appends after a clear choice and preserves a recoverable previous queue.
This is a useful planning tool, not a promise that the user's external commute lasts
exactly 45 minutes or permission to collect location data.

## 7. Seek previews and a forgiving progress rail

Hover/focus/drag shows the target timestamp and actual known duration. Pointer drag
uses a larger hit area; key steps follow the existing seek preference. Keep the
preview separate from committed position until user release/keyboard action.
Escape cancels an in-progress scrub where supported. Time updates must not fight
the held thumb or fire an HTTP seek on every raw pointer pixel.

Optional buffered ranges are shown only if the adapter reports them. A waveform
requires actual measured/versioned data, not a random decorative graph. Previewing
a timestamp never autoplays. On unseekable live sources, expose correct transport
instead of a fake finite rail. Keep screen-reader values useful without rapid noise.

## 8. Music details as a navigable trail

From a track, reveal actual album/edition, artist/contributor, genre, recording/file
and origin details with compact drill-ins and Back restoring scroll/focus. Keep
credits distinct from text guesses in an artist string. The trail should let a
listener explore “more from this pianist/producer/label” only when those credits
actually exist and are authorized.

An album edition compare sheet shows real track order, known mastering/release data,
codec/size/quality and availability. It does not merge different recordings or claim
that a larger file sounds better. Optional audio comparison is a later explicit
single-engine interaction with a saved return state; never hover-autoplay a preview
or accidentally count preview gestures as qualified listens.

## 9. Radio and mix workbench

Before committing a generated queue, show real preview candidates and compact controls
for familiarity, artist variety and session length. Each change runs bounded owner-
scoped ranking with cancellation/generation checks. Preserve current playback while
previewing. Pin a seed or remove an actual candidate with a reversible explanation.

“Why this track?” discloses actual tags/favorites/current-consent history or measured
signals. “Less like this” uses the existing feedback receipt model. Do not manufacture
mood labels from RMS or publish private taste while collaborating. A sparse library
returns fewer tracks and explains which constraint can be relaxed.

## 10. Offline readiness as an inspectable product state

An album/playlist can show “8 of 10 saved” and open a per-item panel with real bytes,
verification, artwork and failures. Separate Waiting for network, Downloading,
Verifying, Partial, Ready and Repair needed. A decorative green tick is not offline
readiness. Future “Prepare for offline” plans estimate actual missing bytes and
respect source download permission/quota.

Let users reorder pending transfers within admitted worker policy, retry one failed
item, stop background speculation and preview safe reclaim. Do not remove pinned
music because a temporary connection fails. Cached metadata/covers alone cannot
pretend the audio will work in airplane mode.

## 11. Personal listening map and rediscovery

Offer a quiet timeline/heatmap of actual qualified private listening, plus forgotten
favorites and “Albums I have not finished” when the retained data supports those
claims. Hover/focus reveals real counts/date ranges; click opens the filtered actual
tracks. Unknown metadata/listening coverage stays explicit.

This is opt-in, account-scoped and respects disabled history and erasure. Do not
compute completion from clicking Play or downloading a file. Manual album completion
is separate from measured listening. Export/share always previews selected visible
data; no automatic public yearly card or fabricated popularity counters.

## 12. Conflict handling that keeps the user's work

When another collaborator changes a playlist, show a small actual revision update
and a diff anchored by entry IDs. Preserve the listener's active source and any
unsaved draft. Offer Apply compatible move/Reload/Keep draft according to the actual
service contract. Do not replace a whole list from a stale client snapshot.

Optional presence avatars name real connected editors and expire; they must not imply
listening presence without consent. A metadata collaborator still cannot play private
tracks. “Updated elsewhere” is useful only when driven by actual revision events or
an explicit refreshed snapshot, not fake realtime decoration.

## 13. Fast navigation without losing place

Maintain Back/Forward context, row selection, filters and scroll anchors. A quick
album/artist/details peek should be a nonmodal drawer if it does not change the task;
a full new listening context uses a route. Opening/closing either leaves playback
alone. Multiple scroll regions need explicit boundaries and restored focus.

A command palette runs only actual app commands and authorized search. Show concise
shortcut hints in menus/help, not permanent keyboard labels on every track. Never
intercept text editing, IME, browser zoom or screen-reader commands to play music.

## 14. Space to focus on the music

Optional Focus view emphasizes actual current art/title, transport and lyrics/queue
toggle, with quiet source/quality details. Desktop mini-player is another presentation
of the same coordinator, not a second output or untracked window. Closing it returns
to the app under explicit user close behavior.

User-controlled artwork accent and scene transitions use original Vela tokens and
contrast safeguards. A low-power/reduced-motion setting can retain color/identity
without perpetual animation. No audio-reactive spectrum without actual signal data,
and no full-window bloom/blur merely to appear sophisticated.

## 15. Helpful recovery without a support-console detour

An unavailable track can expose Retry live, Try authorized local asset, Open source
details, Keep in queue or Skip according to the actual resolver. Show the reason:
origin offline, permission expired, local file corrupt or unsupported format are
different states. Do not substitute a similarly titled track silently.

An optional source-preparation detail panel shows actual auth/cache/transfer/verify/
decode timings and safe error codes. Keep secrets/private paths out of copy/export.
It is a diagnostic tool under disclosure, not the first screen every listener sees.

## 16. Privacy and sharing lenses

In a share workflow show “Who can see this?” and “Who can play these tracks?” as
separate projections of the real permission model. A “View as invited user” preview
must use authorized server evaluation, not client-side imagination. Missing grants
offer an actual access request or clear unavailable status, never a broken Play.

Transfer/device/room controls show real destinations, participants and acknowledgements.
No automatic device takeover, unsolicited speaker start, fake friend-online dot or
audio redistribution by virtue of joining a room.

## Recipe acceptance

| ID | Required result |
| --- | --- |
| ACT01 | Row/title/child action/menu activation never dispatch conflicting playback commands |
| ACT02 | Receipt/Undo follows actual revision/command and cannot overwrite unrelated changes |
| ACT03 | Selection and batch scope remain explicit through paging/rights changes |
| ACT04 | Drag/drop cancels cleanly and preserves current source/occurrence/shuffle invariants |
| ACT05 | Session shelf stores safe bounded refs and commits only on explicit action |
| ACT06 | Time-budget planner previews actual candidates and honest known/unknown duration |
| ACT07 | Scrub preview/commit/cancel is stable and unseekable sources remain truthful |
| ACT08 | Metadata trail/edition comparison uses actual identifiers and preserves navigation/audio |
| ACT09 | Mix preview/feedback preserves privacy, real eligibility and stale-request guards |
| ACT10 | Offline indicators equal verified per-item state, not metadata/cache availability |
| ACT11 | Listening views rely on actual consent-qualified data and respect erase/opt-out |
| ACT12 | Collaboration conflict retains drafts and uses stable entry/revision semantics |
| ACT13 | Back/palette/peek restores focus and scroll without resetting audio |
| ACT14 | Focus/mini-player shares one root engine and has explicit lifecycle/close policy |
| ACT15 | Recovery displays actual source cause and never silently changes to an unrelated track |
| ACT16 | Sharing/device/room projections do not infer media/presence permissions |
