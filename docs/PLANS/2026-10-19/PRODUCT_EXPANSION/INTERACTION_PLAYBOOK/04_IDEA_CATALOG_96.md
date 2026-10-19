# 96 concrete interaction briefs

All rows are PROPOSED outcomes or refinements, not executed features. P0 is core
correctness/accessibility; P1 is daily product quality; P2 is advanced interaction.
EX references link outcomes to the parent 320-entry catalog; do not count overlap
twice. Apply the detailed recipes/contracts/QA to each row. Ship only actual supported
actions, with truthful capabilities and an accessible alternative to each gesture.

## A. Loading and progressive reveal — IX001–IX008

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX001 | P0 | Geometry-matched song-row skeletons using actual list columns and row tokens | Real delayed list settles without moving action columns or current focus | EX270 |
| IX002 | P1 | Group-level subtle shimmer with static/reduced/low-power fallback | No flashing/per-row animation storm; work completion stops it | EX272 |
| IX003 | P1 | Artwork-only pending skin on existing native/web image state | Known song info remains usable; missing/denied art never loops as Loading | EX101 |
| IX004 | P1 | Independent Home section reveal with scoped retry | One slow mix does not blank a loaded album shelf or move a focused target | EX274 |
| IX005 | P0 | Background-update treatment retaining same-context authorized rows | Refresh is visible without replacing content with fake empty state | EX100 |
| IX006 | P1 | Pagination skeleton tail sized to visible list | Previously loaded rows/scroll stable; actual received count announced | EX099 |
| IX007 | P0 | Source preparation feedback distinct from download/verification/playback | Requested Pause wins; equalizer does not pretend pending audio is playing | EX002 |
| IX008 | P1 | Fixed-size action buttons while showing Adding/Saving receipt state | No neighbor jump or duplicate write during repeated input | EX266 |

## B. Contextual actions — IX009–IX016

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX009 | P0 | Right-click song menu through a root target/command registry | Opens correct origin/item without playing; no global browser suppression | EX270 |
| IX010 | P0 | Visible More and Shift+F10/context-key parity | Same actual actions available without pointer or hidden gesture | EX269 |
| IX011 | P1 | Searchable Add to playlist picker preserving exact target refs | Real destination lookup, mutation receipt and no stale-target redirect | EX130 |
| IX012 | P1 | Selection-aware action menu with eligible/denied count | Mixed-source multi-select cannot silently claim all actions succeeded | EX070 |
| IX013 | P1 | Context-specific queue/playlist Remove wording and scope | Removes exact occurrence/entry, never an ambiguous server original | EX033 |
| IX014 | P1 | Mobile long-press/More action sheet with cancellation | Scroll/drag do not accidentally play/open; focus and safe areas work | EX270 |
| IX015 | P1 | Rights-aware disabled reasons and actual recovery actions | Metadata-only item cannot play via forced menu dispatch | EX184 |
| IX016 | P1 | Safe Copy Vela link/Copy metadata actions with clipboard fallback | No stream token/private path; failed clipboard leaves selectable safe text | EX095 |

## C. Direct manipulation and receipts — IX017–IX024

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX017 | P1 | Transient action receipt anchored above the persistent player | Confirmed outcome does not reflow the track list or leak after logout | EX275 |
| IX018 | P1 | Revision-safe Undo for add/remove/reorder where supported | New unrelated edits remain; unavailable inverse is explained honestly | EX275 |
| IX019 | P1 | Selection bar with explicit visible-versus-query scope | Keyboard range selection and selection exit preserve playback | EX070 |
| IX020 | P1 | Track/album drag into queue with insertion preview | One explicit drop command; cancel sends no mutation | EX025 |
| IX021 | P2 | Drop into playlist destination with target preview | Real eligible playlist receives exact items once, without original-file movement | EX035 |
| IX022 | P2 | Temporary listening shelf for ideas before queue commitment | Bounded safe refs, no hidden audio fetching or cross-account carryover | EX022 |
| IX023 | P1 | Favorite feedback synchronized across row/player/queue | One acknowledged mutation updates all same-track projections; failure rolls back safely | EX071 |
| IX024 | P2 | Recent action list with receipts and safe repeat/open actions | Replaying a command requires explicit new intent and current authorization | EX275 |

## D. Queue as an interactive tool — IX025–IX032

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX025 | P1 | Actual-next queue panel with saved-order toggle | Opening it never consumes shuffle state or promises unavailable prefetch | EX024 |
| IX026 | P1 | Visual segments for manual next/context/autoplay | Segment contents match authoritative scheduling and reorder constraints | EX023 |
| IX027 | P1 | Dedicated drag handles, autoscroll and keyboard move commands | Current occurrence follows stable ID; audio/time remain unchanged | EX025 |
| IX028 | P1 | Keep album together toggle in a session planner | Disc/track order retained; incompatible current queue behavior explained | EX029 |
| IX029 | P2 | Duration planner with pinned start/end and known-time range | Result comprises actual authorized tracks and discloses unknown lengths | EX030 |
| IX030 | P1 | Save queue/session with ordered duplicate preview | Saved playlist/bookmark has no credentials and round-trips occurrences | EX027 |
| IX031 | P1 | Unavailable queue card with retry/skip/keep choices | Source resolver reason correct; no silent replacement by same-title song | EX032 |
| IX032 | P2 | Queue-change diff before replacing an ongoing session | Explicit commit/Undo preserves previous context under bounded policy | EX028 |

## E. Search and navigation — IX033–IX040

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX033 | P1 | Keyboard search suggestions grouping actual tracks/albums/actions | Debounce/cancel and exact target selection survive fast typing | EX085 |
| IX034 | P1 | Command palette for implemented navigation/transport/organization | No unsupported command or collision with text/IME/browser zoom | EX091 |
| IX035 | P1 | Reversible filter chips with visible result scope | Clearing one chip preserves others and current audio | EX097 |
| IX036 | P1 | Back/Forward with scroll and selected-row restoration | Virtualization does not focus a reused unrelated row | EX090 |
| IX037 | P1 | Album/artist/details peek drawer with Open full action | No hover autoplay; drawer uses actual metadata and closes cleanly | EX090/EX106 |
| IX038 | P2 | “Matched by…” explanation on expanded search results | Real title/tag/credit field named; no generated imaginary relevance | EX098 |
| IX039 | P1 | Search scope tabs for local/downloaded/connected when implemented | Slow remote origin cannot block local results or erase their selection | EX088 |
| IX040 | P1 | Jump to currently playing row/context from persistent player | Correct occurrence/origin highlighted without fetching a fake whole queue | EX031 |

## F. Artwork, library and metadata exploration — IX041–IX048

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX041 | P1 | Album cover reveal using reserved image bounds and actual decode readiness | No flash/layout shift; denied art remains private | EX101 |
| IX042 | P1 | User-controlled artwork accent wash on current album/player | Contrast preserved; no fake palette or perpetual animation | EX112 |
| IX043 | P1 | Playlist mosaic from permitted real member art with deterministic layout | Metadata-only shares cannot expose private covers | EX122 |
| IX044 | P1 | Edition compare sheet with actual track/encoding/release differences | Different masters stay distinct; no invented quality verdict | EX115 |
| IX045 | P2 | Contributor trail: song→artist/role→available related works | Only known indexed credits/authorized catalog returned | EX106 |
| IX046 | P1 | Missing-art/tag/duplicate repair queue with diff preview | Explicit edits preserved; no automatic original deletion | EX080 |
| IX047 | P2 | Album listen-later shelf with explicit completion action | Manual mark separate from qualified listening statistics | EX072 |
| IX048 | P2 | Pin/folder organization with inline rename and safe cancel | Stable refs, cycle rejection and keyboard alternatives | EX073 |

## G. Discovery workbench — IX049–IX056

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX049 | P1 | Mix preview before changing the active queue | Real accessible candidates; preview leaves current audio untouched | EX145 |
| IX050 | P2 | Familiarity/variety/length controls with actual bounded reranking | Stale preview rejected; limited library gives honest smaller result | EX146 |
| IX051 | P1 | “Why this?” disclosure tied to true ranking reasons | Disabled retained history is not queried or exposed | EX141 |
| IX052 | P1 | Less like this/Hide with successful-operation Undo | Feedback target cannot drift during refresh/navigation | EX150 |
| IX053 | P2 | Pin/replace a real candidate while preserving the session's constraints | Replacement respects rights/exclusions and does not fabricate new media | EX157 |
| IX054 | P2 | Audio-similarity journey between actual seed tracks | Versioned measured/tag fallback clearly distinguished | EX147 |
| IX055 | P1 | Freshness/coverage inspection for actual materialized sets | Actual refresh date/candidate count, no fake daily update badge | EX151 |
| IX056 | P2 | Album-first exploration mode preserving complete releases | Album continuity honored and per-track context remains explicit | EX148 |

## H. Offline and transfer interactions — IX057–IX064

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX057 | P1 | Album/playlist readiness summary with real item/byte counts | Partial/corrupt audio never labeled fully saved | EX047 |
| IX058 | P1 | Expandable transfer rows separating waiting/download/verify/error | Values reflect actual worker state; Retry not decorative | EX056 |
| IX059 | P1 | Prepare-offline preview showing missing bytes/art/rights/storage | Unknown size/permission visible before action | EX049 |
| IX060 | P1 | Interactive safe storage reclaim preview | Active leases/pins/server originals excluded from cache clearing | EX054 |
| IX061 | P2 | Pending transfer priority reorder under worker limits | Current Play outranks speculation; order change acknowledged | EX051 |
| IX062 | P1 | Metered/offline policy banner with one real override choice | User's explicit override scope and expiry are clear | EX050 |
| IX063 | P1 | Corrupt-copy repair action on the affected item | Failed repair preserves a healthy published version where available | EX053 |
| IX064 | P2 | Verified “Ready next” badge tied to actual prepared lease | Revoked/canceled/stale prefetch removes readiness before playback | EX044 |

## I. Playback focus and personal reflection — IX065–IX072

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX065 | P1 | Expanded seek hit area with timestamp preview/commit/cancel | No seek storm or fake waveform; drag stays stable under time updates | EX006 |
| IX066 | P1 | Now Playing focus view with art/transport/details toggle | Shares the same engine and returns focus when closed | EX273 |
| IX067 | P2 | Lyrics follow-scroll with manual-reading pause and Resume following | Actual permitted timed/plain lyrics and reduced-motion fallback | EX109 |
| IX068 | P1 | Clear sleep boundary preview: timer/track/album | Changes to context update or invalidate end boundary honestly | EX008 |
| IX069 | P2 | Personal history timeline/heatmap with real filtered drill-in | Uses current consent-qualified history; unknown dates/coverage explicit | EX165 |
| IX070 | P2 | Forgotten-favorite shelf with explanation and fresh play context | Real retained signals; no stale history use after opt-out | EX144 |
| IX071 | P2 | Album-completion view with transparent measured/manual distinction | Playing/downloading alone never counts as completed listening | EX174 |
| IX072 | P1 | Safe quality/source inspector under disclosure | Original/transcoded/output unknowns accurate; secrets redacted | EX017 |

## J. Collaborative and connected interactions — IX073–IX080

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX073 | P1 | Playlist revision diff and recoverable conflicting move | Stable entry IDs, preserved drafts, no silent stale overwrite | EX132 |
| IX074 | P2 | Actual collaborator presence/changing state with expiry | No fictional realtime indicators or implied listening presence | EX134 |
| IX075 | P1 | Share permission lens separating visible and playable tracks | Server projection and actual media authorization agree | EX199 |
| IX076 | P2 | Device handoff preview showing actual destination readiness | Source continues until accepted transfer; animation follows acknowledgement | EX204 |
| IX077 | P2 | Room suggestion/vote tray with role/permission feedback | Real participants, bounded votes, no unauthorized audio relay | EX212 |
| IX078 | P2 | Per-origin partial-search status and explicit retry | Cached browse labeled stale; local selection usable during peer failure | EX239 |
| IX079 | P2 | Shared playlist availability diff when a source goes offline | Missing refs retained; reconnect does not silently copy audio | EX139 |
| IX080 | P1 | Invitation inbox with real pending/expired/accepted/revoked state | Repeated acceptance idempotent; stale link cannot grant access | EX186 |

## K. Settings and discoverability — IX081–IX088

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX081 | P1 | Settings search jumps/highlights the exact actual row | Highlight static/reduced alternative; no lost focus after scale | EX262 |
| IX082 | P0 | Preference receipt shows applied/temporary/next-track/restart state | No false Saved for rejected/unsaved change | EX266 |
| IX083 | P1 | Before/after preview for safe appearance/preset changes | Explicit apply/reset and persistence; no privacy opt-in hidden inside preset | EX278 |
| IX084 | P1 | Contextual keyboard hints in menus/help | Commands match platform/support and do not clutter every row | EX269 |
| IX085 | P1 | Optional concise first-use tips triggered by real empty task | Dismiss persists safely; no forced walkthrough or repeated nagging | EX274 |
| IX086 | P1 | Loading/motion/press-feedback preferences in existing schema/service | System reduced-motion constraints respected; no parallel preference store | EX272 |
| IX087 | P1 | Row/menu density and target-size preview | Actual app-scale/text-size hit regions verified | EX264 |
| IX088 | P1 | Accessible display recovery always reachable from Settings | Current implemented scale bounds retained; no duplicated browser zoom handling | EX280 |

## L. Reliable polish and recovery — IX089–IX096

| ID | Priority | Implementation idea | Acceptance | Parent outcome |
| --- | --- | --- | --- | --- |
| IX089 | P1 | Actual request/source-stage diagnostic peek for slow start | Auth/cache/transfer/decode timing measured separately, no private paths | EX005 |
| IX090 | P1 | Recoverable action failure with preserved input/target | Retry independent of dismissed toast and still bound to original command | EX276 |
| IX091 | P1 | Persistent error inbox for relevant unresolved own operations | Only actual errors/receipts; no duplicated fictional job state | EX196 |
| IX092 | P1 | Empty/filtered-empty/offline-empty states with different real actions | Guidance matches actual capabilities instead of generic Refresh everywhere | EX058 |
| IX093 | P1 | Focus/hover/pressed/current states that never shift layout | Touch/keyboard retain all functions without hover | EX270 |
| IX094 | P1 | Overlay stack manager for menu/picker/dialog/sheet lifetimes | No invisible blocker or focus trap after navigation/account change | EX270 |
| IX095 | P1 | Low-power/offscreen animation policy with bounded observer cleanup | Hidden/settled interface stops decorative work; no listener leak | EX272 |
| IX096 | P0 | Interaction acceptance ledger tied to actual backend/platform evidence | Proposed/partial/verified states distinct; no success from animation alone | EX319 |

## Accepting a brief

For each accepted IX entry record source owners, API/state requirements, actual
capabilities, loading/empty/error/cancel/unsupported states, keyboard/touch behavior,
resource bounds, relevant LOAD/MENU/ACT/MOT tests, screenshots and source/artifact
identity. If a prerequisite is absent, deliver a smaller complete slice or keep the
entry deferred. Do not create a “coming soon” control just to check a row.
