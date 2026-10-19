# Loading that communicates the actual work

## 1. Separate content loading from audio preparation

The user's proposed “blocks shimmering as if loading” is a skeleton placeholder.
Implement a subtle traveling light or slow pulse inside the future content's exact
geometry. It should feel like a page taking shape, not a flashing advertisement.
Use it only for real pending work. Existing Vela pulse classes are the starting point.

| Actual state | Visual treatment | What must remain usable |
| --- | --- | --- |
| First catalog response, no data | Matching row/shelf skeleton and one loading status | Navigation, search input, persistent player |
| Same authorized catalog refreshing | Existing rows retained; small section update indicator | Reading, selection, current playback and unrelated actions |
| A new search/query context | Clear obsolete results; skeleton matching the new result layout | Query editing/cancel; no stale old-query actions |
| Pagination/Load more | Append a small skeleton tail, retain existing rows/scroll | Loaded tracks and keyboard focus |
| Metadata ready, artwork pending | Only artwork slot shimmers; actual title/artist/action stays | Play, queue and context menu |
| Audio source resolving/transferring | Row/player prepare status and actual known byte progress | Pause pending intent, source cancel/retry and navigation |
| Decoder/output waiting | “Preparing audio” status, indeterminate without fake percent | Stop/pause intent and safe source replacement |
| Buffering on a known track | Preserve cover/time/title; clear buffering status | Seek/pause where actually supported |
| Download/import processing | Real worker stages/bytes in task row | Real pause/cancel/retry only where worker supports them |
| Failed/denied/missing data | Actionable error or honest missing fallback, no shimmer loop | Recovery/navigation; authorized existing data where still valid |

Shimmer does not mean prefetch exists. A visible “Ready next” marker requires an
actual prepared/verified next-source lease, not an animation or cached metadata.

## 2. State machine and timing decisions

Use a typed pending operation with request identity, context, start time, previous
authorized content and real completion/error/cancel. Suggested baseline timing:

- First 0–150ms: keep a stable reserved shell; avoid a distracting flash for a cache hit.
- Beyond 150ms with no data: reveal the appropriate skeleton; do not delay real data
  to make the animation visible. Cached results may appear immediately.
- After roughly 2s: keep or add a concise explanation of the actual pending action.
- After 8s: offer a real cancel/retry/connection explanation appropriate to the service.
  Elapsed time alone is not proof of failure; obey its actual bounded deadline.
- On success: activate actual content immediately; optional 100–140ms reveal does not
  delay actions or preserve an invisible overlay intercepting clicks.
- On failure/cancel: remove pending visuals promptly and show the actual next state.

Never enforce a minimum fake wait. Do not “finish” a progress bar on a timeout.
An automatic retry can remain pending only under the existing bounded retry policy;
the animation must not hide retry exhaustion. Cached content that fails reauthorization
must be cleared, not retained as a convenient refresh fallback.

## 3. Skeleton geometry specification

Use the same row/grid templates, responsive tokens and aspect ratios as loaded
components. A skeleton is a skin on the real layout contract, not a separate rough
mockup with unrelated heights. Decorative bars have no labels or focus targets.

| Surface | Proposed placeholder layout |
| --- | --- |
| Track table | Cover-sized square, title/artist bars, optional album/duration bars, reserved action column; exact row height |
| Album/playlist shelf | Artwork ratio and two short caption bars; no extra surrounding dashboard panel |
| Artist shelf | Actual artist shape and caption; do not imply a portrait is already known |
| Album detail | Stable header artwork/title space plus disc/track row skeletons; header actions wait for capabilities |
| Search | Separate track rows and known category groups; unknown groups are not fabricated with precise counts |
| Queue | Short tail/current-prepare indicator; never hide known current/previous occurrences |
| Artwork only | Existing square/circle bounds with neutral base and subtle shimmer; no layout shift when decoded |
| Playlist picker | Small list skeleton inside picker, search/Close remain active |
| Settings | Prefer actual known values plus pending marker; use skeleton only for genuinely unknown server fields |

Choose a deterministic set of bar-length proportions, e.g.72/58/86/64%, based on
placeholder position. Do not call random() on every render or use arbitrary song
names. Patterns must be stable across hydration/re-render and scale with text/density.
Avoid fake duration bars that look like measured audio waveform or download progress.

Default visible counts are layout decisions, not catalog facts: a compact viewport
might reserve 5–7 track rows or 3–4 cover tiles. Compute from the visible region and
the current component's row size, with a hard cap on rendered placeholders. At most
one viewport plus a small tail; never render a thousand animated rows for a page.

## 4. Shimmer and static alternatives

Proposed primary effect: one low-contrast highlight traverses a local placeholder
group over 1.6–2.2s. Keep its base close to the actual surface and the highlight a
small luminance change. No black/white alternation, saturated flashes, glow bloom
or repeated scale/bounce. A slow low-amplitude pulse is an acceptable fallback.

Respect System/Reduced motion and a future explicit “Loading animations” preference.
Reduced means stable neutral shapes plus text status, not disappearing feedback.
After 5s of optional decorative looping, switch to static placeholders and a visible
status unless the user explicitly keeps full motion. Animation can never be the only
evidence that work continues. Validate actual flash/motion safety rather than treating
a slow duration as an automatic accessibility certificate.

Stop decoration offscreen, in hidden tabs/windows, on unmount and when the operation
settles. Prefer one bounded animation layer per visible group to independent expensive
filters on every row. Do not permanently promote every skeleton/image to a GPU layer.
Benchmark compositor and memory cost; native WebKit behavior requires actual evidence.

## 5. Progressive reveal and stale content

Home sections may settle independently. Reveal successful real sections without
waiting for the slowest unrelated feed; failed sections get scoped Retry. Avoid
moving a focused card beneath a user's pointer when a late section arrives. Preserve
anchors, reserve stable section dimensions and insert without viewport jumps.

Image decoding is a separate completion from HTTP fetch. Keep the slot reserved,
show a loaded cover only when decode is ready, and make an actual decode error
distinct from “no artwork”. The current native ArtworkStatus can be adapted to its
existing pending/loading/ready/denied/failed/missing projections. Preserve its bounded
queue/cache and origin/account/session guards; do not create an image loader per row.

Same-context background refresh may retain previously authorized rows. A new query,
origin, account, privacy epoch or revoked grant cannot borrow that permission. An
old delayed request may not reintroduce skeletons, clear a newer error or replace
new content. Dispose all timers and captured targets when their lifetime expires.

## 6. Audio-specific row feedback

Keep the selected title/art visible. The play button reflects requested intent, so
the listener can pause while preparation runs. Next to it, show a small pending ring
or text “Preparing”; do not animate a playing equalizer until actual engine state
supports it. If byte length is known and actual transfer progress exists, display it
in the player/task detail; otherwise use an indeterminate label.

Loaded/buffered progress, playback position and download completion are different
visual quantities. A future dual seek rail may show buffered ranges only when the
adapter reports them reliably. Full-file native verification may have received all
bytes but still be verifying/decoding; do not show “Ready” before its real receipt.

## 7. Accessibility and performance contract

Set busy state on the region whose contents are pending. Announce one concise status,
not every skeleton bar or shimmer tick. Decorative shapes are hidden from assistive
technology; skeletons have no fake focusable Play buttons. Error/action targets are
real controls. Do not make the entire page inert while one section loads.

Show available controls during refresh. Reserve space for loading button content so
labels changing to “Adding…” do not move surrounding controls. Repeated activation
must not duplicate a write. Give the user the real final receipt or failure.

## 8. Acceptance requirements

| ID | Expected observation |
| --- | --- |
| LOAD01 | Real delayed catalog response shows matching rows, not a generic spinner-only blank page |
| LOAD02 | Under 150ms success appears promptly without an artificial minimum animation delay |
| LOAD03 | Loaded and skeleton row/cover geometry match across current scale/density/breakpoints |
| LOAD04 | Same-context refresh preserves readable authorized rows, focus and active audio |
| LOAD05 | New query/account/origin clears obsolete private results and rejects late completion |
| LOAD06 | Artwork-only pending does not erase known title/artist or disable valid playback |
| LOAD07 | Failed/denied/missing art exits shimmer and displays its actual fallback/retry policy |
| LOAD08 | Real byte progress is distinguished from audio verification/decode/buffering |
| LOAD09 | Pause during source preparation wins over delayed completion; no fake playing animation |
| LOAD10 | Load-more tail preserves scroll and only actual result count is announced |
| LOAD11 | Reduced motion renders static placeholders and still communicates status |
| LOAD12 | Decorative loop stops after policy timeout, offscreen/hidden/unmount and settled work |
| LOAD13 | Screen reader hears one useful loading/result/error update, no animated-bar noise |
| LOAD14 | Partial Home success remains usable; one failed section has scoped recovery |
| LOAD15 | Retry/cancel bind to current request and cannot affect another view/account |
| LOAD16 | Long labels/text enlargement/current app scale keep placeholders and controls aligned |
| LOAD17 | Bounded placeholder/layer count and measured idle/load CPU/memory; no perpetual render loop |
| LOAD18 | All successful/failed screenshots and actual-request timing traces retained; no mocked completion |
