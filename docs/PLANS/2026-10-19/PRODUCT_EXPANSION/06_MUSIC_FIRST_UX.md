# Music-first UX and visual implementation plan

Retain Vela's original bright mint identity and existing high-contrast dark/light
tokens. This plan is an evolution of the real application, not a replacement mockup.
All new surfaces are proposed; earlier screenshots are baseline evidence only.

## Visual direction

Let music imagery, typography and listening context carry the interface. Use a dark
continuous canvas, a quieter navigation surface, expressive legitimate artwork,
and the persistent player as the stable visual anchor. Light theme is designed with
equal care, not inverted dark pixels. Primary mint remains an action/current-track
color, not a border around every object. Status colors remain semantically distinct.

Use five compositions deliberately: artwork shelves for albums, typographic lists
for tracks, editorial sections for discovery, compact identity rows for people and
devices, and divided setting rows for preferences. Avoid using a rectangular panel
component as the default container for every heading, sentence and button.

Preserve existing tokens until measured changes are justified. Add semantic tokens
for compact/comfortable/touch density, app/text size, panel widths, menu surfaces,
artwork-derived accents, destructive states and motion. No remote font dependency,
unlicensed celebrity art or generated demo catalog is required. Album colors may
influence an accent wash while text stays on contrast-tested surfaces.

## Layout contract

Desktop: one primary navigation/library area, one scrollable content canvas, optional
right Now Playing/Queue, persistent bottom player. Resize panels within constraints;
hide/collapse optional panels before essential content becomes unusable. Keep scroll
ownership explicit. Navigation history restores content state and scroll without
recreating audio. A listener should never traverse raw server health to reach music.

Phone/narrow effective width: bottom destinations for implemented Home/Search/Library,
mini-player above safe-area navigation, fullscreen player and accessible sheets.
Context menus become sheets when space requires; every drag gesture has buttons.
At high app/text scale, native desktop can use narrow presentation without falsely
claiming to be a tested mobile client.

## Shared action model

Track, album, artist and playlist actions derive from one capability resolver:
metadata-view, artwork-view, stream, download, edit, share and remote-control are
independent. A context menu observes these results; mutations reauthorize server-side.
Do not fetch a restricted file just to determine whether a menu should show Download.

For current occurrence use Pause/Resume. For another occurrence use Play. Header
Play starts the named context. Play next and Add last remain distinct commands.
Selection, activation, long press, double click and Enter must have documented
semantics. Use item-origin identity, not title equality, for every command.

## Screen specifications

### Home

Open with one useful path to actual accessible music, not a static promotional hero.
Return listeners see Continue listening, recent contexts and real additions. Eligible
history-based rotation and materialized mixes disclose scope/refresh. New listeners
see Add music or browse known accessible catalogs, never fabricated recommendations.

Allow section reorder/hide/reset from a quiet Customize action. Avoid twenty empty
sections. A section's Play action builds that exact queue; context menus offer next,
save and explanation where supported. Keep content stable while a feedback receipt
is pending; a late response cannot move the target beneath the pointer.

### Library

Filters: tracks/albums/artists/playlists/downloaded and explicit library/server scope.
Persist view/sort/columns by scope. Pinned shortcuts are real refs. Show actual counts
with known scope, and “more available” instead of pretending a page is the whole
library. Support multi-select, select-all-visible versus select-all-query distinction,
keyboard range selection, typed filters and a clear active-filter reset.

Large libraries need keyset pagination and virtualization with stable focus/row
identity. An unloaded row must not claim queue membership. Batch destructive actions
preview selected scope and never delete original files by default.

### Search and browse

Search starts immediately on valid input with debounce/cancellation, exact-first
ranking and accessible suggestions. Clearly label Tracks/Albums/Artists/Playlists;
add people, podcasts and servers only when their actual backends exist. Make origin
and playable/unavailable states explicit. Remote partial results stream into their
own groups without replacing local selection or freezing input.

Genre/decade/mood navigation uses real tags, trusted metadata or clearly labeled
manual categories. No confident mood inference from a scalar RMS descriptor. Search
history is optional, scoped and clearable. Saved searches store typed expressions,
not duplicated giant result sets.

### Album and artist

Album: legitimate cover, title/artist/year/release type, disc/track ordering, total
known duration, Play/Shuffle, save/download when authorized, editions/assets and
credits disclosure. Preserve album sequence when choosing a track; do not queue the
entire artist by accident. Unknown dates/credits stay unknown.

Artist: real image or typographic fallback, available albums/singles/appearances,
local top tracks with named scope, follow/favorite, radio from actual candidates.
Biographies require permitted provider/provenance. Avoid invented monthly listeners,
tour dates or “verified” badges. Small catalogs get honest limited radio semantics.

### Playlists and collections

Playlist headers show owner, visibility and actionable media availability. Description,
cover and folders are editing flows, not always-open creation forms. Reordering uses
stable entry IDs, drag handle plus keyboard Move actions and revision/CAS feedback.
Conflict UI preserves user intent and offers a refetched diff, never silent overwrite.

Share sheet distinguishes who can view metadata from who can play/download each
source. Show expiration/revoke/pending invitations. A metadata-only collaborator
gets a helpful unavailable action, not a broken Play. Smart playlists have rule
preview, explanation/count bounds and “Save static copy”; users edit rules rather
than manually dragging a dynamic result into a contradictory order.

### Player and queue

Bottom player always has current title/art, real requested/actual transport feedback,
seek and queue access. Fullscreen Now Playing is a presentation of the same state.
Loading is a finite prepare/retry operation with cancel/retry where appropriate,
not a spinner hiding an indefinite failure. Quality describes the actual source.

Queue separates previous/current/manual-next/context/autoplay as real model segments.
Saved order and actual shuffled next must be labeled. Reorder/remove/swap never
replace current source. Clear upcoming and Clear entire queue are separate actions;
removing current offers explicit stop/skip semantics. Save queue creates a playlist
without exposing credentials or stale remote stream URLs.

Lyrics are an optional right/fullscreen view with plain/LRC fallback, correct line
timing, click-to-seek only on seekable media and an easily recoverable follow-scroll
mode. Large artwork/lyrics layouts must not conceal transport at high text scale.

### Downloads and storage

Show actual Saved/Downloading/Waiting for Wi-Fi/Verifying/Partial/Failed states and
known byte counts. A collection with one failed file is not fully available offline.
Separate pinned downloads, evictable cache, staging and metadata/art. Clear cache
must never delete pinned music. Offer Verify/Repair with bounded work and actionable
storage estimates. Offline cold start cannot depend on a successful server login.

### Add Music and jobs

One Add music entry: Upload, From URL, Scan configured library, Connect trusted
storage when supported. Preserve progress while navigating. Preview actual downloadable
assets, destination, known size/rights and duplicates. Failures name recoverable
causes without leaking server paths/tokens. Retry must not double-publish assets.

Jobs are a projection of actual workers. Uploads/downloads/analysis need explicit
integration contracts before appearing as “completed” rows. Ordinary users see own
tasks; administrator queues remain separate. Cancel cannot promise undo after atomic
publication; the UI must distinguish request cancellation from completed persistence.

### Devices and rooms

Devices show actual names/platform/capabilities, expiry-based presence and a clear
This device label. Selecting a device opens Transfer/control options; it never
automatically steals output. Confirmed handoff resolves destination authorization,
prepares the source, acknowledges acceptance and only then stops the source player.

Rooms show participants, roles, current track, source availability and shared queue.
Unauthorized participants see why a track is unavailable and may suggest alternatives.
Host rights do not grant media rights. Joining never silently starts speakers or
broadcasts personal history. Reconnect uses sequenced authoritative snapshots.

### Network, profiles and identity

Network explains connected sources through real catalogs and permission states,
without exposing private addresses/user lists. Manual server lookup, optional public
profile discovery and library access requests are separate steps. An offline peer
can show a stale metadata cache clearly, with no implied playable grant.

Profile publication, presence, follows and shared listening are opt-in. Optional
portable identity links multiple server accounts after proof from each issuer;
it does not merge their credentials, library ownership or device sessions silently.

### Settings and administration

Follow the dedicated settings/scale specifications. Avoid an enormous form of every
possible feature. Display implemented listener controls first, advanced options by
search/disclosure and server management only to administrators. Copy uses outcomes,
not environment variable names. Recovery/reset remains available without a server.

## Interaction details to test everywhere

- Menus close on Escape/outside action; focus returns to the invoking row.
- Native dialogs/sheets retain focus, allow keyboard dismissal and do not leave
  hidden overlays intercepting pointer input after navigation.
- Hover is inside hover-capable media queries; focus reveals the same actions.
- Current-track state uses icon/text as well as color; stale network data cannot
  update a new route/account or a replaced occurrence.
- Reversible actions offer bounded Undo tied to an acknowledged mutation. No false
  Undo for expired grants, deleted originals or already distributed content.
- Toasts confirm useful changes without moving the track list; errors have durable
  retry/detail access. Do not stack a toast for every slider tick or audio time update.
- Loading placeholders preserve geometry and represent real work. Empty and filtered-
  empty states differ, with an actual next action.
- Progress, percentage and play state announcements are throttled and meaningful;
  screen readers must not hear the play clock every200ms.

## Design validation instructions for the receiving agent

Use Better Design principles before implementation and review rules afterward.
Review exact before/after content in dark/light,320/390/tablet/1440/1728/ultrawide,
high scale and long localized text. Use real fixture media and legal artwork.
Run comprehension on actual visible copy and spacing on captured DOM, not source
guesses. Preserve the existing passing row-toggle, modal, consent and stale-response
regressions. Fix serious/critical findings; record remaining moderate findings and
explicit unverified screen-reader/hardware/platform scope. No public preview or
external publication is authorized by this documentation task.
