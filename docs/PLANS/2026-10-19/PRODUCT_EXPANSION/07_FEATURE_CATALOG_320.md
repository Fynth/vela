# 320 feature implementation briefs

Every entry is PROPOSED, including an extension of existing functionality. N means
a new outcome; E means extend/preserve an existing foundation. Neither means shipped.
P0: everyday correctness/accessibility; P1: mature daily use; P2: advanced experience;
P3: optional research/integration. EP numbers refer to delivery packages. Platform
support is conditional on the real adapters in those packages; web compilation does
not certify native background playback. Shared subsystem recipes supply storage,
authorization, cancellation and migration details for every row.

The entries are granular acceptance outcomes, not320 independent apps or320 promises
for the next release. Each implemented entry must retain its ID in the acceptance
ledger. Do not create decorative controls for unimplemented entries.

## A. Dependable playback — EX001–EX020

Owners/files: desktop audio.rs/ui.rs/client.rs; web browser_player.rs/playback.rs;
server stream/range; media/range.rs. One root output authority per client.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX001 | P0/E | Consistent row transport across all contexts; resolve occurrence-aware commands through root state | Repeated current-row activation holds source/time; another duplicate selects separately | EP05 |
| EX002 | P0/E | Pending action can be paused/canceled; completion reads current intent and source generation | Held source completion never autoplays after Pause/logout | EP05 |
| EX003 | P1/N | Progressive start on supported formats through bounded authorized source adapter | Playback begins before full transfer without weakening documented integrity fallback | EP07 |
| EX004 | P0/E | Same-session reconnect with retained seek and bounded retry budget | Real disconnect recovers near bookmark; no new listen session or retry loop | EP07 |
| EX005 | P1/N | Source diagnostics drawer names waiting/auth/transfer/verify/decode stages | Displayed stage matches actual worker events; no raw paths/tokens | EP07 |
| EX006 | P1/N | Precision seek and optional measured waveform from a versioned analysis artifact | Scrubbing seeks correct source; unavailable waveform keeps text/range control | EP08 |
| EX007 | P2/N | A–B practice loop with finite validated marks in current track | Loop survives Pause, clears on track replacement, cannot seek outside duration | EP08 |
| EX008 | P1/E | Sleep at duration, current-track end or album end through root coordinator | Exact selected boundary pauses; queue edits recompute or explain invalid context | EP05 |
| EX009 | P0/E | Audio focus/headset interruption policy through each native engine | Call/route change pauses safely; resume obeys explicit setting | EP21 |
| EX010 | P0/E | Actionable autoplay/decode/offline/auth failures with separate recovery actions | Real failure fixtures produce correct action; no generic endless spinner | EP07 |
| EX011 | P1/N | Capability-negotiated optional transcoding, original preferred | Unsupported codec uses bounded preset worker; lossless-compatible direct play remains original | EP08 |
| EX012 | P1/N | Track/album loudness normalization with validated gain and clipping protection | Known gain fixtures match output measurements; album intent is retained | EP08 |
| EX013 | P2/N | Graphic/parametric equalizer with bypass and safe bounded presets | Invalid filters rejected; bypass restores reference output without resetting queue | EP08 |
| EX014 | P3/N | Optional headphone crossfeed as an explicit native DSP effect | Measured channel response and clipping tests pass; unsupported web target omits it | EP08 |
| EX015 | P1/N | Gapless playback for verified engine/format pairs | Joined test signals have measured boundary behavior; capability matrix names exceptions | EP08 |
| EX016 | P2/N | User-controlled crossfade and optional analysis-aware transition experiments | One mixer authority, bounded overlap, no transition across unsupported album boundaries | EP08 |
| EX017 | P1/N | Actual source/output quality inspector: original/transcoded, codec, sample/channel path | Unknown output parameters remain unknown; no false bit-perfect badge | EP08 |
| EX018 | P2/N | Mono/channel balance accessibility controls via supported output adapter | Both channels remain audible in reference fixture; no gain clipping | EP08 |
| EX019 | P1/N | Safe output-switch preview with rollback and capped volume | Failed output change retains prior playable device; no surprise full-volume start | EP20 |
| EX020 | P1/N | Separate music/podcast/audiobook resume/speed semantics | Spoken-word position persists independently; live streams do not expose fake finite seek | EP26 |

## B. Queue as a useful listening workspace — EX021–EX040

Owners/files: shared playback.rs, native ui.rs, web root/session.rs; future typed
queue journal in a cohesive shared client boundary. Stable occurrence IDs required.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX021 | P0/N | Durable native queue/context/position bookmark scoped to account and origin | Cold restart restores paused with duplicates intact and no unauthorized request | EP05 |
| EX022 | P1/N | Named saved listening sessions distinct from playlists | Switching saves previous snapshot and restores target paused without source collision | EP05 |
| EX023 | P1/E | Visible manual-next/context/autoplay queue segments | Playing next obeys explicit priority and updates actual-next preview atomically | EP05 |
| EX024 | P1/N | Side-effect-free actual shuffle-next preview | Opening queue never consumes entropy/history or changes subsequent playback | EP05 |
| EX025 | P1/N | Drag reorder with keyboard alternatives and occurrence identity | Duplicate entries remain distinguishable; current source/time is unchanged | EP05 |
| EX026 | P1/N | Multi-select queue actions with bounded Undo | Remove/reorder receipt restores exact occurrences, not title-matched replacements | EP05 |
| EX027 | P1/N | Save queue as playlist, including origin references and unavailable markers | Playlist round-trip preserves order/duplicates without exporting stream credentials | EP13 |
| EX028 | P1/N | Restore replaced queue through one bounded local recovery snapshot | Explicit Play-context replacement can be undone without resurrecting another account | EP05 |
| EX029 | P2/N | Album blocks movable as groups while preserving disc/track order | Group move keeps internal order and current occurrence binding | EP05 |
| EX030 | P2/N | Time-budget queue builder using known durations and bounded selection | Result approaches requested duration; unknown lengths are visibly excluded/estimated | EP14 |
| EX031 | P1/N | Queue search/jump-to-current without replacing context | Filter affects display only; Next still follows authoritative order | EP05 |
| EX032 | P1/N | Unavailable policy Ask/Skip/Stop with explicit reasons | Fully unavailable queue stops after bounded attempts, not an infinite skip loop | EP07 |
| EX033 | P1/N | Per-occurrence remove-current decision Stop/Skip/Keep | Selecting an option executes explicit transport semantics and logs one mutation | EP05 |
| EX034 | P2/N | Session traversal history separate from private collected history | Previous works with history collection disabled and does not create consent | EP05 |
| EX035 | P1/N | Append album/playlist without accidental queue replacement | Current source remains; duplicate policy is a visible deliberate choice | EP13 |
| EX036 | P2/N | Queue snapshot comparison after remote edits or handoff | Diff names added/removed/reordered occurrences without leaking other devices' private queues | EP16 |
| EX037 | P1/N | Mixed-origin queue with source availability badges | Colliding UUIDs from different origins never resolve to the wrong file | EP19 |
| EX038 | P2/N | Pin a next occurrence without enabling repeat | Pinned next survives fair shuffle updates and is consumed once | EP05 |
| EX039 | P2/N | Bounded queue checkpoint export/import without secrets | Oversized/malformed refs rejected; media reauthorized at playback | EP05 |
| EX040 | P1/E | Accurate end-of-context summary with Restart/Choose next/optional continuation | Repeat-off ends once; explicit restart begins a new logical selection | EP05 |

## C. Cache and real offline ownership — EX041–EX060

Owners/files: desktop media_cache.rs/offline.rs/client.rs; planned web storage
adapter; source/download services. Cache is not an offline permission grant.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX041 | P0/E | Storage inspector separates cache, pinned, staging, artwork and metadata | Summed measured usage reconciles with disk within documented overhead | EP06 |
| EX042 | P0/N | Editable byte/entry cache quotas with active-source protection | Lowering budget never closes decoder lease or deletes pinned copies | EP06 |
| EX043 | P0/N | Clear cache preview and safe account namespace cleanup | Only allowlisted cache entries removed; music/downloads/other account unaffected | EP06 |
| EX044 | P1/N | Prepare actual next using one bounded speculative lease | Queue/shuffle/source changes cancel stale work; no second audio output | EP07 |
| EX045 | P1/N | Single-flight shared transfer for Play/Prefetch/Save | Concurrent consumers cause one authorized body transfer and independent cancellation | EP07 |
| EX046 | P1/N | Resume downloads with strong validator and Range | Changed representation restarts safely; partial old/new files never concatenate | EP09 |
| EX047 | P1/N | Download album/playlist with aggregate and per-item states | One failed item produces Partial, not Available offline | EP09 |
| EX048 | P0/E | Verified cold offline startup independent of reachable server | Restart without network browses permitted local account copy and plays natively | EP09 |
| EX049 | P1/N | Offline artwork/lyrics/credits packaged with authorized downloads | Airplane-mode details open without hidden network fetch | EP09 |
| EX050 | P1/N | Wi-Fi/metered/roaming rules for speculative and explicit transfers | Unknown/metered state blocks speculation under conservative policy | EP09 |
| EX051 | P1/N | Download priorities: current, explicit user choice, background sync | Large background job cannot starve an interactive track | EP09 |
| EX052 | P1/N | Keep selected playlists offline by revision diff | Only added/changed assets transfer; removed entries get documented retention policy | EP09 |
| EX053 | P1/N | Local integrity Verify/Repair center using same descriptor checks | Corruption rejected; repair failure preserves last healthy published pair | EP06 |
| EX054 | P1/N | Low-space plan with exact reclaim candidates and free-space reserve | Preview excludes active/pinned data; disk-full produces recoverable failure | EP06 |
| EX055 | P2/N | Download quality variants with actual encoding identity | Switching future quality never relabels old lossy bytes as original | EP09 |
| EX056 | P1/N | Download pause/resume/cancel with atomic publication semantics | Pause survives restart where supported; canceled staging cleaned safely | EP09 |
| EX057 | P2/N | Smart unpinned eviction with documented recency/access costs | Reopening an active entry cannot race eviction into a broken source | EP06 |
| EX058 | P1/N | Accessible offline-only filter and missing-copy explanation | Offline view excludes unavailable bytes while keeping reversible metadata references | EP09 |
| EX059 | P2/N | Portable download manifest without credentials or unrestricted URLs | Import resolves origins/assets and rechecks rights before any new transfer | EP09 |
| EX060 | P1/E | Cross-account/cache isolation and explicit sign-out retention policy | New account sees no prior artwork/queue; chosen local removal scope is enforced | EP06 |

## D. A rich personal library — EX061–EX080

Owners/files: domain DTOs, media/catalog.rs/filters.rs, core library services,
web/desktop catalog views; additive migrations coordinated by integrator.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX061 | P1/N | Multiple named libraries with explicit membership and selector | Search/counts/streaming respect selected and authorized libraries | EP10 |
| EX062 | P1/E | Pinned album/artist/playlist shortcuts with per-account ordering | Broken refs become clear unavailable items, never a different object | EP10 |
| EX063 | P1/N | Library grid/list toggles and stable column preferences | Scale/narrow view retains title/play/actions without horizontal clipping | EP10 |
| EX064 | P1/N | Recently played/added/never played/forgotten views with honest scope | History-dependent views disable/fallback when current consent is off | EP15 |
| EX065 | P1/N | Album editions and release groups without collapsing distinct masters | Existing playlist track refs survive grouping migration | EP10 |
| EX066 | P1/N | Multiple assets per recording with compatible quality choice | Selected asset has correct MIME/size/validator; recording and file IDs differ | EP10 |
| EX067 | P1/E | Multi-disc/compilation/Various Artists presentation | Musical ordering and album-artist credit stay correct under pagination | EP10 |
| EX068 | P2/N | Album/artist/track collections with typed membership | Collection access cannot grant underlying audio permissions | EP10 |
| EX069 | P1/N | User tags and reusable colored labels as optional organization | Typed owner-scoped tag filtering does not alter embedded source tags | EP10 |
| EX070 | P1/N | Bulk favorite/tag/add-to-playlist with explicit selection scope | Visible-page versus all-matching selection is never ambiguous | EP10 |
| EX071 | P1/N | Rating model separate from Favorite and recommendation exclusions | Rating export/import round-trips and does not silently change hidden choices | EP10 |
| EX072 | P2/N | Listen-later album shelf with completion control | User can mark manually without fabricating counted listening time | EP15 |
| EX073 | P1/N | Library folders/collections navigation with bounded nesting | Moves preserve IDs and reject cycles; keyboard reorganization works | EP10 |
| EX074 | P1/N | Expanded indexed size/duration/last-played/quality filters | Query plan remains bounded; unknown metadata has explicit matching semantics | EP11 |
| EX075 | P1/N | Alphabet jump with Unicode-aware normalized ordering | Cyrillic/Latin/diacritics fixtures sort predictably without rewriting names | EP11 |
| EX076 | P1/N | Duplicate review separates exact file and possible recording match | Similarity never triggers automatic original deletion | EP12 |
| EX077 | P1/N | Missing/moved source repair preserving catalog identity | Re-scan matches stable asset/recording evidence and retains playlist refs | EP12 |
| EX078 | P2/N | Personal notes/bookmarks on albums and tracks | Private notes export safely and never become public profile data by default | EP10 |
| EX079 | P2/N | Reversible catalog archive/hide distinct from file deletion | Archived items can be restored with stable references | EP10 |
| EX080 | P1/N | Library health review for missing art/tags/unplayable assets | Every issue links to a real authorized repair action or documented limitation | EP12 |

## E. Search and navigation — EX081–EX100

Owners/files: media/search.rs/filters.rs; server search transport; web client/
experience.rs/shell.rs; native client.rs/ui.rs. Never search private foreign catalogs.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX081 | P1/E | Exact-first ranked track/album/artist/playlist search | Known exact fixtures outrank weak matches; bounds and auth remain | EP11 |
| EX082 | P1/N | Accent-insensitive lookup with original display text | cafe/café resolve as expected; original tags remain unchanged | EP11 |
| EX083 | P2/N | Optional transliteration aliases across scripts | Alias matches show original name; no silent ID merge by spelling | EP11 |
| EX084 | P2/N | Conservative typo suggestions with score cutoff | Exact results preserved; short query does not trigger expensive fuzzy scan | EP11 |
| EX085 | P1/N | Keyboard-complete search suggestions and clearable history | Arrow/Enter/Escape work; disabled history leaves no retained query list | EP11 |
| EX086 | P1/N | Search by composer/credits/recording IDs when known | Only indexed existing authorized fields are searchable | EP11 |
| EX087 | P1/N | Saved searches with validated filters and sort | Reopen reevaluates current rights/data, not stale stored results | EP11 |
| EX088 | P1/N | Scope All connected/This server/Downloaded | Unavailable adapters cannot freeze local response or imply extra rights | EP19 |
| EX089 | P1/N | Bounded concurrent remote result groups with origin labels | Slow peer times out separately and partial results remain usable | EP19 |
| EX090 | P1/N | Recent context/back-forward navigation with scroll anchor | Going album→artist→Back restores selection/scroll without restarting audio | EP24 |
| EX091 | P1/N | Global command palette for real actions and destinations | Unsupported commands omitted; typing never triggers playback shortcuts | EP24 |
| EX092 | P1/N | Search within queue/playlist/library as distinct filters | Scope never silently changes selected playback context | EP11 |
| EX093 | P2/N | Genre/decade browse pages from actual metadata facets | Empty facets hidden; unknown dates are not guessed into decades | EP11 |
| EX094 | P2/N | Mood browsing from explicit tags before experimental inference | Manual/provider/estimated labels are distinguished and reviewable | EP14 |
| EX095 | P1/N | Stable deep links with origin/object type identity | Opening foreign/private ref reauthorizes and shows useful denied state | EP19 |
| EX096 | P1/N | Find settings using labels, aliases and categories | “scale/масштаб” finds one actual reachable control without server query | EP03 |
| EX097 | P2/N | Advanced filter builder with suggestions and count preview | Bounded typed expressions reject arbitrary SQL and pathological nesting | EP11 |
| EX098 | P2/N | Search result explanation: title/tag/credit/source match | Explanations use actual matched fields, not generated guesses | EP11 |
| EX099 | P1/N | Row selection that survives lazy loading/virtualization | Keyboard focus does not jump to a reused unrelated DOM row | EP24 |
| EX100 | P1/N | Clear error/retry state per search origin and request generation | Old query errors/results cannot replace a newer search/account | EP11 |

## F. Metadata and music context — EX101–EX120

Owners/files: media/metadata.rs/analysis.rs/catalog.rs; domain; native artwork
adapter and web metadata.rs; provider traffic stays in controlled workers.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX101 | P1/N | Authorized native album artwork with bounded decode/cache | Foreign artwork denied; oversized/decompression-bomb fixture fails safely | EP12 |
| EX102 | P1/E | Catalog metadata editor with field provenance and revision | User-edited title survives rescans and uncertain enrichment | EP12 |
| EX103 | P1/N | Bulk metadata edit preview with per-track conflicts | No partial silent overwrite; original files remain unchanged | EP12 |
| EX104 | P2/N | Optional MusicBrainz matching with ambiguity review | Rate-limited suggestions show evidence; uncertain match requires acceptance | EP12 |
| EX105 | P2/N | Cover selection/crop with source and rights metadata | Uploaded/permitted art only; derived sizes remain authorized | EP12 |
| EX106 | P1/N | Structured artist roles, composers and recording credits | Multiple contributors preserved; missing names are not invented | EP10 |
| EX107 | P2/N | Artist biography with provider provenance and manual override | Untrusted markup sanitized; unauthorized lookup disabled | EP12 |
| EX108 | P1/N | Plain embedded/local sidecar lyrics extraction | No external lyric scraping; text available without timing claims | EP12 |
| EX109 | P1/N | LRC synchronized lyrics with seek and timing fallback | Malformed/out-of-order timestamps bounded; plain fallback always works | EP12 |
| EX110 | P2/N | Personal lyric offset correction per recording/asset | Offset does not rewrite someone else's source or drift another edition | EP12 |
| EX111 | P2/N | Manual lyric editor and explicit local write-back choice | Original tag write requires separate authorization and backup | EP12 |
| EX112 | P1/N | Artwork palette extraction for contrast-safe accents | Worker bounded; text colors validated rather than sampled blindly | EP12 |
| EX113 | P2/N | Exact duplicate grouping by content identity in review UI | Same-file group never conflates remaster/live versions | EP12 |
| EX114 | P2/N | Recording similarity hints via metadata IDs/fingerprint adapter | Confidence/provenance exposed; no automatic deletion | EP12 |
| EX115 | P1/N | Edition comparison for codecs, dates, tracks and tags | Different mastering/encoding remains visible and selectable | EP10 |
| EX116 | P2/N | Booklet/liner notes attachments with safe rendering | Size/type limits and independent authorization; no active remote HTML | EP12 |
| EX117 | P1/N | User-change history and per-field metadata rollback | Revision conflict preserves newer edits and documents rollback outcome | EP12 |
| EX118 | P2/N | Original-tag write-back as explicit admin-capability job | Review diff/backup; read-only sources reject cleanly; source failure preserves catalog | EP12 |
| EX119 | P1/N | File/audio info distinguishes container from actual codec | Unknown properties remain unknown; lossy audio never labeled lossless by filename | EP08 |
| EX120 | P2/N | Release/recording identifier inspector and repair suggestions | Stable local IDs retained while external identifiers are corrected | EP10 |

## G. Playlists and curation — EX121–EX140

Owners/files: core/playlists.rs, domain, server routes, web/native playlist views;
typed rules and provenance additions are additive and owner/revision scoped.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX121 | P0/E | Stable playlist-entry provenance through duplicate/concurrent reorder | Exact occurrence remains bound; current audio survives another user's edit | EP13 |
| EX122 | P1/N | Playlist descriptions and cover editing with revisions | Update conflict cannot erase a collaborator's committed change | EP13 |
| EX123 | P1/E | Pin/folder/rename/duplicate playlist workflows | Duplicate preserves entries, not collaborator secrets or share tokens | EP13 |
| EX124 | P1/N | Smart playlist editor with bounded typed nested rules | Preview matches committed evaluation; no unrestricted SQL | EP13 |
| EX125 | P1/N | Smart rule limits by track count/duration/bytes | Unknown duration/size handling explicit; query remains bounded | EP13 |
| EX126 | P1/N | Convert dynamic playlist to static snapshot | Saved entries correspond to one authorized evaluated revision | EP13 |
| EX127 | P1/N | M3U/M3U8 import with unresolved-entry review | Local paths never become arbitrary filesystem reads; duplicates retained | EP13 |
| EX128 | P1/N | Portable JSON/M3U export with origin metadata | No credentials/long-lived stream URLs; Unicode/order round-trip tested | EP13 |
| EX129 | P2/N | Authorized service playlist metadata matching | Unresolved songs remain explicit; no commercial-stream audio extraction | EP13 |
| EX130 | P1/N | Playlist membership inspector for a track | Lists only visible playlists and supports actual add/remove actions | EP13 |
| EX131 | P1/N | Duplicate-entry review with Keep both/Remove selected | Recording versions not deleted; operation is revisioned and undoable when safe | EP13 |
| EX132 | P2/N | Playlist history/diff and revision-safe restore | Restore creates a new revision, never rewrites audit history | EP13 |
| EX133 | P1/E | Collaborator invitations with accept/expiry/revoke | Invitation grant does not grant track streaming automatically | EP17 |
| EX134 | P1/N | Realtime collaborator changes with revision cursor | Dropped/duplicate events reconcile from authoritative snapshot | EP17 |
| EX135 | P2/N | Owner transfer with explicit acceptance | Current owner retains control until accepted; revoked invitation cannot transfer | EP17 |
| EX136 | P2/N | Follow playlist updates separate from editing rights | Unfollow stops subscription; source rights rechecked for playback | EP19 |
| EX137 | P1/N | Per-playlist search/sort with original order restore | Temporary sort does not silently rewrite shared entry order | EP13 |
| EX138 | P2/N | Setlist/album-sequence mode with annotations | Notes private by default; transport uses actual ordered media refs | EP13 |
| EX139 | P2/N | Coverage report for local/remote/offline/missing entries | Totals derive from current capabilities and show partial availability honestly | EP19 |
| EX140 | P1/N | Conflict recovery and uncertain-write reconciliation | Retry with same mutation ID does not duplicate additions | EP13 |

## H. Discovery with real accessible music — EX141–EX160

Owners/files: media/recommendations.rs/discovery.rs/analysis_service.rs; core listening;
web personal_mixes.rs/radio.rs; future native adapters. No GPU/cloud requirement.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX141 | P1/E | Explainable content/history/favorite recommendation reasons | Reasons cite actual eligible signals; disabled retained history is never queried | EP14 |
| EX142 | P1/N | Durable daily mixes from real taste/tag clusters | Snapshot refresh timestamp truthful; all emitted IDs resolve to current authorized media | EP14 |
| EX143 | P1/N | Weekly exploration with novelty/diversity/exclusions | Sparse catalog returns honest small/empty set, never mislabeled repeated “unseen” tracks | EP14 |
| EX144 | P1/N | Forgotten favorites and private repeat rotation | Suppression obeys current consent and avoids rapid-skip inflation | EP14 |
| EX145 | P1/E | Track/artist/album/playlist radio with preview | Actual seed/context retained; fewer candidates explained, rights never inferred | EP14 |
| EX146 | P2/N | Multi-seed radio and user-controlled familiar/explore balance | Bounded anchor count and deterministic test fixtures | EP14 |
| EX147 | P2/N | Explainable path between two sonically related seeds | Versioned descriptor graph yields real playable steps, no hallucinated bridges | EP14 |
| EX148 | P2/N | Album-first discovery preserving continuous releases | Album candidate diversity evaluated without shuffling movements/discs | EP14 |
| EX149 | P1/N | Onboarding taste choices drawn from current catalog | Every artist/genre option has actual eligible tracks or labeled unavailable status | EP14 |
| EX150 | P1/E | Not interested/less like this with undo and reset | Feedback receipt bound to selected item; navigation cannot apply it to another track | EP14 |
| EX151 | P1/N | Transparent freshness/coverage panel for recommendations | Unique candidates/repeat fraction measured locally without unnecessary telemetry | EP14 |
| EX152 | P2/N | Daypart/context mixes from user-chosen settings | No inferred location/emotion collection; deterministic fallback for missing tags | EP14 |
| EX153 | P2/N | Newly added versus verified new releases as distinct feeds | Import date never relabeled release date | EP14 |
| EX154 | P2/N | “More from this contributor/label/era” exploration | Relationships come from known authorized metadata, not invented biography | EP14 |
| EX155 | P2/N | Opt-in shared-taste mix from permitted participant signals | No raw private history revealed; each participant's playback authorization checked | EP18 |
| EX156 | P2/N | Optional external recommendation IDs mapped to accessible catalog | Unmatched IDs labeled unavailable; API failure leaves local engine working | EP19 |
| EX157 | P2/N | User-curated discovery stations with constraints | Typed filter/seed config exportable and bounded, no arbitrary executable rules | EP14 |
| EX158 | P1/N | Durable refresh jobs with revision/policy invalidation | Privacy revocation invalidates stale materialized sets and late writes | EP14 |
| EX159 | P3/N | Optional local embeddings evaluated against deterministic baseline | Measured quality/resource improvement required; model/assets/license explicit | EP14 |
| EX160 | P3/N | Optional spoken session introductions from approved local/authorized speech | Off by default, marked generated, stoppable, music works without speech/model | EP26 |

## I. Private history, statistics and reflection — EX161–EX180

Owners/files: core/listening.rs and domain/listening.rs; server listening transport;
web audible.rs/listening.rs; native monotonic-clock adapter only after consent design.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX161 | P0/E | Separate account/device listening consent with clear effective collection state | Upgrade/sign-in/profile import never enrolls collection | EP15 |
| EX162 | P1/N | Native qualified listening using advancing media time and monotonic corroboration | Seek/download/paused time cannot count; one session survives source recovery | EP15 |
| EX163 | P1/N | Private session mode that suspends future collection explicitly | Already retained data is neither erased nor used while disabled without allowed policy | EP15 |
| EX164 | P1/E | History search/filter by actual track/date/context with bounded pages | Owner-only data and stable cursor behavior under retention changes | EP15 |
| EX165 | P1/N | Daily/monthly listening minutes based on qualified intervals | Overlapping event retries deduplicate; timezone/daylight transitions tested | EP15 |
| EX166 | P1/N | Top tracks/albums/artists with named period and scope | Aggregates derive from actual eligible sessions, not browser queue visits | EP15 |
| EX167 | P2/N | Discovery timeline of first qualified plays | Import date/manual mark not confused with first listen | EP15 |
| EX168 | P2/N | Genre/era/source distribution with Unknown buckets | Incomplete metadata does not fabricate precise percentages | EP15 |
| EX169 | P2/N | Private month/year story and optional share preview | Share is separate explicit publication; hidden/private titles excluded unless selected | EP15 |
| EX170 | P1/E | Account-bound machine-readable history export | Held export checks current privacy epoch/session before delivery | EP15 |
| EX171 | P1/N | Selective history removal and full erase-and-disable | Transaction invalidates late outboxes; recommendation caches do not repopulate erased evidence | EP15 |
| EX172 | P1/E | Listening-device inventory and per-device revocation | Old device events cannot return after revised consent | EP15 |
| EX173 | P2/N | Listening goal/bookmark reminders without engagement pressure | Opt-in local reminders; no fabricated streaks or loss penalties | EP24 |
| EX174 | P2/N | Album completion as an explicitly defined personal metric | Skips and unknown durations reported; manual completion remains distinct | EP15 |
| EX175 | P2/N | Optional scrobbling outbox with idempotency and provider receipts | No duplicate play submissions after reconnect; provider deletion limits disclosed | EP19 |
| EX176 | P2/N | Import user-owned listening exports with provenance | Untrusted old events do not become public popularity or rights proof | EP15 |
| EX177 | P1/N | History retention controls and previewed pruning | Cutoff and affected records shown; transaction bounded and recoverable | EP15 |
| EX178 | P2/N | Aggregation rebuild/status and source-of-total explanation | Rebuild matches retained fixture history and never counts deleted sessions | EP15 |
| EX179 | P2/N | Listening notes linked to a private session/bookmark | Notes do not publish with a playlist share or remote device presence | EP15 |
| EX180 | P1/N | Consent-aware recommendations/statistics dependency inspector | Turning collection off clearly shows which views change and why | EP15 |

## J. Sharing and social interaction — EX181–EX200

Owners/files: core playlists/network services, future profile/follow services within
cohesive crates; server transport; web/native shared action/presentation layers.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX181 | P1/N | Optional profiles with display name/avatar/biography and field visibility | New profiles private; markup and uploaded images sanitized/bounded | EP17 |
| EX182 | P1/N | Follow requests/accept/reject/unfollow with explicit identity origin | Same username on two servers never becomes one actor | EP17 |
| EX183 | P1/N | Block and mute with separate enforcement | Block prevents requests/control/discovery where promised; mute only changes delivery/display | EP17 |
| EX184 | P1/N | Share sheet exposing metadata/play/download/edit as separate capabilities | Viewing shared playlist never grants audio implicitly | EP17 |
| EX185 | P1/E | Expiring/revocable links with token-scoped audit | Revoked link fails new access; no credential leaked into referrers/logs | EP17 |
| EX186 | P1/N | Invitation inbox with pending/expired/accepted/revoked states | Repeated acceptance mutation is idempotent and independently authorized | EP17 |
| EX187 | P2/N | Private album/track recommendations to selected friends | Recipient can inspect metadata only as permitted; unavailable audio explained | EP17 |
| EX188 | P2/N | Optional activity feed of explicitly shared changes | No private listening events enter feed by default or by federation enablement | EP17 |
| EX189 | P2/N | Follow curated playlists/collections with update notifications | Subscription never silently downloads or republishes referenced music | EP19 |
| EX190 | P2/N | Opt-in listening presence with expiry and audience controls | Offline/stale devices stop appearing live after TTL | EP16 |
| EX191 | P2/N | Reactions/bookmarks on permitted shared items | Mutation scopes checked; private reactions never appear in public counters | EP17 |
| EX192 | P1/N | Shared-with-me library with visible grant and expiration | Revocation removes unauthorized access on next check without claiming byte recall | EP17 |
| EX193 | P2/N | Group collections with named roles and moderation | Role change invalidates stale edit controls; each mutation reauthorizes | EP17 |
| EX194 | P2/N | Shared mix invitation with explicit contribution consent | Joining exposes only approved taste signals, not full private history | EP18 |
| EX195 | P2/N | Participant availability matching before shared playback | Everyone sees actual playable/missing state and authorized alternatives | EP18 |
| EX196 | P1/N | Notification center for real durable social/job events | Read/unread state survives restart without leaking another account's messages | EP17 |
| EX197 | P2/N | Scoped reports/abuse controls for public submissions | Evidence retention and admin visibility defined; no unrestricted remote fetch | EP19 |
| EX198 | P2/N | Safe social share cards generated from explicitly visible metadata/art | Preview before export; restricted art/history never included automatically | EP17 |
| EX199 | P1/N | Owner visibility preview: “view as invited user/public” | Projection uses server permission evaluator, not optimistic client filtering | EP17 |
| EX200 | P2/N | Portable social/profile export with revocation caveats | Secrets excluded; cross-origin follower migration requires proof and acceptance | EP19 |

## K. Devices, handoff and shared rooms — EX201–EX220

Owners/files: future device/session domain/service, one root playback coordinator,
server WebSocket transport; each native audio adapter reports actual capabilities.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX201 | P1/N | Named stable device registry separate from listening consent records | Device revoke disables commands without confusing privacy device grants | EP16 |
| EX202 | P1/N | Presence heartbeat/expiry with reconnect snapshot | Stale device becomes unavailable; connection count is not invented activity | EP16 |
| EX203 | P1/N | Capability-aware device picker and This device label | Unsupported seek/volume/download operations are disabled with reason | EP16 |
| EX204 | P1/N | Explicit acknowledged queue/position handoff | B starts accepted occurrence near target position; A pauses only after safe acceptance | EP16 |
| EX205 | P1/N | Remote play/pause/next/previous with sequence/idempotency | Duplicate/reordered commands cannot apply twice or control another account | EP16 |
| EX206 | P2/N | Remote volume with device-specific safe limits | Unsupported hardware volume not presented as effective; explicit change acknowledged | EP16 |
| EX207 | P1/N | Per-device independent listening plus saved user bookmarks | Starting B does not automatically interrupt A without an explicit transfer | EP16 |
| EX208 | P1/N | Incremental queue/progress sync and conflict indicator | Offline edits reconcile by occurrence/revision; stale cursor cannot replace newer state | EP16 |
| EX209 | P2/N | Secure QR/device pairing with short expiry and approval | QR contains narrow one-use invitation, not a reusable account password | EP16 |
| EX210 | P1/N | Device/session revocation and active-command audit | Revoked socket/token cannot reconnect or replay commands | EP16 |
| EX211 | P2/N | Create/invite/join/leave/end shared listening room | Actual participant list/roles; joining does not auto-enable microphone or speakers | EP18 |
| EX212 | P2/N | Room queue suggestions/votes/host control policy | Host can limit mutation rate; idempotent commands preserve consistent order | EP18 |
| EX213 | P2/N | Room playback synchronization with clock estimates | Measured drift/reconnect behavior within declared target; no “sample-perfect” claim | EP18 |
| EX214 | P2/N | Rejoin room from authoritative snapshot after disconnect | Old events discarded by sequence; duplicated join cannot create phantom participant | EP18 |
| EX215 | P2/N | Host transfer and room expiry with explicit acceptance | Host loss follows documented timeout policy; private room links expire | EP18 |
| EX216 | P2/N | LAN speaker discovery as separate trusted local integration | No broadening public importer SSRF policy; user opts into discovery | EP20 |
| EX217 | P2/N | Cast/AirPlay/DLNA adapter capability survey and selected implementation | Rights/provider/network/real hardware acceptance required per adapter | EP26 |
| EX218 | P3/N | Multiroom native synchronization via a tested protocol adapter | Drift, buffer and failure metrics recorded; unsynchronized outputs not marketed as sync | EP26 |
| EX219 | P1/N | Hardware/media-key commands share root application semantics | Bluetooth/lock-screen buttons match UI behavior and do not start second engines | EP21 |
| EX220 | P2/N | Per-device output/quality/network profiles | Profile switch previews actual changes and excludes privacy consent | EP16 |

## L. Authorized multi-server network and identity — EX221–EX240

Owners/files: desktop remote.rs/peers.rs/client.rs; web network.rs/client.rs;
core/network.rs and server/network; future standards adapters remain opt-in.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX221 | P1/E | Saved server connections with canonical origin and verified capabilities | Two equal UUIDs on different servers remain distinct | EP19 |
| EX222 | P1/N | Secure per-server account switching without cross-session data | Delayed old-origin response cannot enter new account/library/player | EP19 |
| EX223 | P1/N | Unified search with bounded fanout/deadlines and partial results | One malicious/slow origin cannot freeze or poison other groups | EP19 |
| EX224 | P1/N | Authorized source resolver with local/remote/encoding alternatives | Chooses permitted source; metadata cache never becomes stream authorization | EP19 |
| EX225 | P2/N | OpenSubsonic client adapter with extension negotiation | Real client/server fixture covers supported requests and explicit missing extensions | EP19 |
| EX226 | P2/N | OpenSubsonic server compatibility profile | Only documented implemented routes advertised; authentication/scopes tested | EP19 |
| EX227 | P2/N | WebFinger actor lookup and server identity preview | HTTPS/DNS/size/rebinding safeguards, no private address disclosure | EP19 |
| EX228 | P2/N | ActivityPub music-object interoperability profile | Versioned mappings/errors, unsupported activity rejection and two-instance proof | EP19 |
| EX229 | P2/N | Signed delivery with replay/ownership validation | Forged, stale, duplicate and foreign-object updates rejected | EP19 |
| EX230 | P1/E | Explicit lawful publication with separate discover/stream/download grants | Enabling network publishes nothing automatically; revocation blocks new transfers | EP19 |
| EX231 | P2/N | Provenance/license/attribution inspector on public items | Owner declaration labeled unverified when appropriate; no inferred redistribution rights | EP19 |
| EX232 | P2/N | Server/user blocklists and bounded moderation queue | Blocked origins cannot bypass via redirect/alias or repeated actor lookup | EP19 |
| EX233 | P2/N | Remote deletion/update notifications and cache tombstones | Local playlist keeps clear unavailable ref; no unauthorized byte mirroring | EP19 |
| EX234 | P2/N | Private-library access requests/approval/expiry | Follow/metadata permission does not imply audio copy/download | EP19 |
| EX235 | P2/N | Opt-in independent public server directories | Registration explicit; private servers never auto-listed; directory not mandatory | EP19 |
| EX236 | P1/N | Optional portable Vela identity through self-hostable OIDC | Issuer+subject proof links accounts; local-only use remains fully functional | EP19 |
| EX237 | P2/N | Secure identity link/unlink and lost-provider recovery | Existing server admin/local recovery remains; email equality cannot merge accounts | EP19 |
| EX238 | P2/N | Account/library organization migration with dry-run manifest | Portable refs and settings copied without credentials or unauthorized media | EP25 |
| EX239 | P2/N | Remote peer health/circuit breakers and metadata freshness | Offline cached browse labeled stale; retries bounded per peer | EP19 |
| EX240 | P2/N | Federation/admin audit with redacted event diagnostics | Event size/delivery failures visible without passwords/private library disclosure | EP25 |

## M. Native applications and platform quality — EX241–EX260

Owners/files: current apps/desktop; future apps/mobile/platform adapters only after
contracts. Dioxus presentation remains shared; native services own OS audio lifecycle.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX241 | P0/E | Linux Wayland/X11 geometry, decorations and display scaling | Normal tiled/floating/maximized fixtures show correct bounds without changing user compositor config | EP20 |
| EX242 | P1/N | Native Settings/account/library feature parity for daily tasks | Listener does not need a browser to change scale or manage a saved queue | EP03 |
| EX243 | P1/N | Desktop drag/drop files and native picker through authorized upload service | File validation/limits/cancel apply exactly as web ingestion | EP20 |
| EX244 | P1/N | Desktop tray/close behavior with explicit keep-playing choice | Closing cannot silently orphan an output thread; Quit stops cleanly | EP20 |
| EX245 | P1/E | MPRIS metadata and actual supported remote-source controls | Property/command state matches effective source; no fake remote availability | EP20 |
| EX246 | P1/N | Linux package/installer/update flow with exact artifact validation | Fresh install/upgrade/rollback preserve data and verified runtime hashes | EP25 |
| EX247 | P1/N | Android Media3 service and Dioxus bridge | Real background audio survives activity destruction with one active player | EP21 |
| EX248 | P1/N | Android focus/noisy/headset/lock-screen/notification support | Real device call/unplug/media-key scenarios pass | EP21 |
| EX249 | P1/N | Android offline sources/background downloads and quotas | Airplane-mode cold playback works; interrupted transfer resumes safely | EP21 |
| EX250 | P2/N | Android Auto adapter with safe limited browsing | Actual approved integration/build tests; no promise from regular Android UI | EP21 |
| EX251 | P1/N | iOS AVFoundation/AVAudioSession native bridge | Background category, interruptions and route changes verified on Apple hardware | EP22 |
| EX252 | P1/N | iOS Control Center/remote commands and lock-screen details | Paused/seek/play metadata match authoritative session | EP22 |
| EX253 | P1/N | iOS offline downloads/file protection/lifecycle recovery | Downloaded playlist plays in airplane mode after cold launch | EP22 |
| EX254 | P2/N | CarPlay surface when entitlement/toolchain permits | No entitlement claim before approval and actual interface validation | EP22 |
| EX255 | P1/N | Windows playback packaging/media-session integration | Windows runner compile plus actual playback/installer checks recorded separately | EP23 |
| EX256 | P1/N | macOS playback/Now Playing/packaging integration | Apple build/signing/runtime scopes explicit; no Linux-derived success claim | EP23 |
| EX257 | P1/N | PWA installability and offline metadata shell | No claim of guaranteed suspended-browser background transfers/audio | EP09 |
| EX258 | P2/N | Small headless/native endpoint for an owned speaker device | Single supervised output with secure pairing; no compulsory new server process | EP26 |
| EX259 | P1/N | Deep-link/file association routing to existing running app | New link does not launch a second competing audio session | EP20 |
| EX260 | P1/N | Platform compatibility/diagnostics matrix in-app | Reports actual build/backend/capabilities; missing features show real reason | EP04 |

## N. Settings, accessibility and interaction delight — EX261–EX280

Owners/files: web root/shell/styles, native root/window/styles; proposed device/
account preference services. See detailed settings/scale documents and S001–S120.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX261 | P0/N | App size presets/stepper/reset with platform-specific scaling adapter | SCALE01–14 pass; no audio remount, double-DPI or inaccessible recovery | EP02 |
| EX262 | P0/N | Searchable Settings with stable categories and deep-linked rows | Keyboard/user aliases find actual controls; admin results hidden from members | EP03 |
| EX263 | P0/E | Persisted System/Dark/Light themes with validated legacy migration | Restart/sign-out do not flash or overwrite the user's chosen appearance | EP01 |
| EX264 | P1/N | Independent text enlargement and density profiles |200% text retains controls and readable layout; compact never defeats target-size floor | EP03 |
| EX265 | P1/N | Appearance previews and bounded artwork accents | Actual UI reflects chosen value; foreground contrast remains validated | EP24 |
| EX266 | P0/N | Setting scope/effective/locked/pending-restart feedback | UI never says Saved for an unacknowledged or rejected operation | EP01 |
| EX267 | P1/N | Export/import preferences with compatibility preview | No tokens/history/share secrets; sensitive consent skipped by default | EP03 |
| EX268 | P1/N | Reset row/category/presentation with exact exclusions | Reset theme cannot erase downloads/account or enable public visibility | EP03 |
| EX269 | P1/N | Remappable commands with shortcut help and reserved-key protection | Browser/IME/screen-reader commands remain usable | EP24 |
| EX270 | P0/N | Complete focus/keyboard/context-menu/long-press parity | All primary actions reachable without pointer; no hidden trapped overlay | EP24 |
| EX271 | P1/N | English/Russian localization with plural/date/RTL readiness | Long labels and mixed script metadata fit high-scale/narrow layouts | EP24 |
| EX272 | P0/E | Reduced motion and low-power presentation controls | Motion off removes decoration, not content or completion feedback | EP24 |
| EX273 | P1/N | Optional compact mini-player/focus listening surface | Shares root engine and source; closing view never starts another output | EP20 |
| EX274 | P1/N | Customizable Home sections and navigation pins | User hides/reorders actual sections; restore defaults is reversible | EP24 |
| EX275 | P1/N | Reversible action receipts/Undo center for safe mutations | Undo tied to actual receipt/revision; expired action is honestly unavailable | EP24 |
| EX276 | P1/N | Accessible error center with copyable redacted diagnostics | No credentials/private paths exported; errors remain actionable after toast disappears | EP04 |
| EX277 | P1/N | Granular notification destinations/quiet hours/content privacy | Opt-in system permission; no sound or lock-screen title disclosure by default | EP24 |
| EX278 | P2/N | User-defined safe presets for low-data/album/large-text sessions | Preview changes; privacy, identity and public grants excluded | EP03 |
| EX279 | P0/N | Screen-reader semantics and throttled transport announcements | Real assistive-technology navigation tested, not inferred from aria-label presence | EP24 |
| EX280 | P1/N | Safe display/startup recovery path without server contact | Corrupt settings or unusable scale can be reset without deleting library/session secrets | EP02 |

## O. Ingest, enrichment and supported sources — EX281–EX300

Owners/files: imports source.rs/egress.rs; media ingestion/metadata; scanner;
web imports/scanner/jobs; native transfer client. Every remote adapter uses egress policy.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX281 | P1/E | Unified Add Music Upload/URL/Scan/Storage entry | Every offered source opens a real flow; unsupported adapters absent | EP20 |
| EX282 | P1/E | Batch drag/drop upload with per-file outcomes and aggregate progress | One bad file does not erase valid successes; dedup/quota enforced | EP20 |
| EX283 | P1/N | Resumable large uploads with authorized upload sessions | Chunk replay/idempotency/checksum/quota/restart tests pass | EP09 |
| EX284 | P1/E | Rich URL preview of actual assets/destination/known size | Source metadata not presented as validated embedded tags before ingestion | EP26 |
| EX285 | P1/N | Safe ZIP import with compressed/uncompressed/file-count/depth limits | Traversal/symlink/bomb/nested archive fixtures rejected before publication | EP26 |
| EX286 | P1/N | Trusted WebDAV source connector with scoped stored credential | Internal endpoint permission separate from ordinary URL SSRF policy | EP26 |
| EX287 | P2/N | S3-compatible source with scoped bucket/prefix capabilities | Credentials never exported; listing/download bounded and path-safe | EP26 |
| EX288 | P2/N | Internet Archive allowed-asset adapter | Enumerate only downloadable permitted assets; unavailable/restricted items explained | EP26 |
| EX289 | P2/N | Open-license catalog adapters with source/license attribution | Per-source policy/evidence retained; public domain not guessed from host name | EP26 |
| EX290 | P1/E | Read-only folder rescan with snapshots and preserved edits | Moves/retags do not lose playlist identity; originals never deleted | EP12 |
| EX291 | P2/N | Watched folders with debounce and bounded reconciliation | Bursts coalesce, restart resumes scan safely, filesystem errors remain visible | EP26 |
| EX292 | P1/N | Metadata conflict review after import/rescan/enrichment | Existing explicit edits retain precedence until user accepts a diff | EP12 |
| EX293 | P1/N | Import destination/quota estimate and duplicate-aware preview | Unknown size labeled unknown; quota race independently rechecked at write | EP26 |
| EX294 | P1/N | Durable import retries and restart recovery with clear stages | At-most-once publication despite repeated job attempts | EP26 |
| EX295 | P2/N | Import rules for destination/tags without executable scripts | Typed bounded rules, dry-run preview and owner authorization | EP26 |
| EX296 | P2/N | Podcast RSS subscriptions as distinct content domain | Feed/episode egress bounded; enclosure download/resume not confused with music | EP26 |
| EX297 | P2/N | Internet radio favorites with live-stream-specific controls | No fake duration/seek/offline-complete status; reconnect bounded | EP26 |
| EX298 | P1/N | Permitted playlist metadata import with match review | Unmatched entries preserved visibly; no DRM or restricted-service stream extraction | EP13 |
| EX299 | P1/N | Unified own transfer/job history with privacy-safe diagnostics | Only real worker receipts shown; no source URL/token in aggregate task DTO | EP04 |
| EX300 | P1/N | Duplicate-safe retry/cancel/cleanup from native and web | Cancellation after publication explained; no destructive rollback of other consumer's asset | EP26 |

## P. Trust, operations, portability and extensibility — EX301–EX320

Owners/files: core/config/database/auth/jobs; server transports; backup/install/
package/verify scripts; documentation/CI. No mandatory extra services.

| ID | Priority/type | Experience and implementation | Acceptance | Package |
| --- | --- | --- | --- | --- |
| EX301 | P0/N | Typed policy-aware server Settings with safe active/pending projection | Operator locks cannot be bypassed by generic preference API | EP04 |
| EX302 | P1/N | In-app backup creation/status and isolated restore validation | Original runtime/data not overwritten during validation; media hashes/FKs checked | EP25 |
| EX303 | P1/N | Guided restore/recovery with explicit irreversible boundary | Concrete preview/backup/approval before destructive data replacement | EP25 |
| EX304 | P1/N | Complete data portability export of catalog/favorites/playlists/settings | Versioned documented formats exclude account secrets and unauthorized media | EP25 |
| EX305 | P1/N | Session inventory/password rotation/revocation flows | CSRF/current authorization enforced; revoked session cannot issue new media commands | EP04 |
| EX306 | P2/N | Optional passkeys or OIDC login after reviewed account recovery | No custom authentication protocol; local admin recovery retained | EP19 |
| EX307 | P1/N | Quota/resource profile editor with measured effective limits | Lowering quota blocks writes instead of deleting music; workers remain bounded | EP04 |
| EX308 | P1/N | Analysis job scheduling/power policy and actual job-center projection | Versioned extractor cache preserved; shutdown/restart/error state accurate | EP04 |
| EX309 | P1/N | Redacted health/stream/job diagnostics and downloadable support bundle | Tokens/paths/private metadata excluded by allowlist, not fragile string replacement | EP25 |
| EX310 | P1/N | Performance and reliability baseline suite for representative libraries | Reports actual hardware/workload/p50/p95/p99/RSS without fabricated superiority | EP25 |
| EX311 | P1/N | Safe update check/staged local upgrade/rollback workflow | Exact artifacts verified, pre-upgrade restore check, explicit restart approval | EP25 |
| EX312 | P1/N | Container/ARM64/native package matrix and honest support UI | Build versus run versus hardware evidence remains separate | EP25 |
| EX313 | P1/N | Scoped API keys and user-visible integration grants | Hash-at-rest/revoke/scope tests; no permanent unrestricted share credential | EP04 |
| EX314 | P2/N | Optional authorized webhooks with controlled egress/redaction | SSRF/retry/idempotency limits; only explicitly selected events disclosed | EP26 |
| EX315 | P2/N | Extension interfaces for import/metadata/catalog/recommendations | Compile-time/admin-reviewed integrations first, no unrestricted binary plugin runner | EP26 |
| EX316 | P1/N | Audit and privacy control center explaining data flows | Every integration lists purpose/destination/revoke/export behavior | EP04 |
| EX317 | P1/N | Accessibility/codec/client capability status in release notes and app | Status derives from evidence, not platform marketing support | EP25 |
| EX318 | P2/N | Optional Home Assistant/automation integration with narrow scopes | Automation cannot bypass ownership or trigger unapproved public playback | EP26 |
| EX319 | P1/N | Release-level golden-scenario ledger with exact artifact provenance | Passing compile alone cannot mark an entire user journey complete | EP25 |
| EX320 | P0/N | Continuous source/runtime/schema/rights truthfulness checks | Existing data survives every upgrade; docs never mark proposals as implemented | EP00 |

## Crosswalk to the older240-entry catalog

The old F IDs remain historical proposal references. Consolidate them into one
authoritative accepted backlog by category, then attach exact F references to an EP
ticket when implementation begins. Do not advertise320+240 as560 unique features.

| Earlier Player Reboot category | This catalog |
| --- | --- |
| A Playback | EX001–EX020, EX241–EX260 |
| B Queue | EX021–EX040, EX201–EX220 |
| C Cache/offline | EX041–EX060 |
| D Search/navigation | EX081–EX100 |
| Library/catalog and metadata proposals | EX061–EX080, EX101–EX120 |
| Playlist/curation proposals | EX121–EX140 |
| Discovery/statistics proposals | EX141–EX180 |
| Social/rooms/device proposals | EX181–EX220 |
| Network/identity proposals | EX221–EX240 |
| Settings/design/accessibility proposals | EX261–EX280 |
| Import/operations/portability proposals | EX281–EX320 |

Existing implemented foundations are preserved, not re-counted as new achievements.
The earlier plan's exact section names/IDs remain in its source document; use this
crosswalk as outcome grouping, not an assertion that every previous item is identical.
