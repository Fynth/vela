# Delivery sequence and adversarial interaction QA

No application tests in this file have been executed by this documentation task.
The receiving agent uses isolated real servers/media and records actual results.
Preserve existing named playback/auth/privacy/playlist/request-order regressions.

## 1. Eleven bounded delivery packages

| Package | Prerequisites | Complete outcome | Acceptance scope |
| --- | --- | --- | --- |
| IN00 | None | Current-source/dirty-work inventory, real rendered baseline, action/state/capability map | Source/runtime separation, existing artwork/scale recognized, no user-data changes |
| IN01 | IN00 | Shared pending-state projection and geometry-matched catalog/art skeletons | LOAD01–18, MOT02–04; actual delayed/failed reads |
| IN02 | IN00 | Closed command/target registry and single-item track More/right-click/keyboard menu | MENU01–10/17–22; real root transport/favorite/queue actions |
| IN03 | IN02 | Searchable playlist picker, real add receipts and safe Undo/reconciliation | MENU11–13, ACT02; existing CAS/permission tests |
| IN04 | IN02/IN03 | Touch sheet, multiselection and capability-aware batch contracts | MENU14–17; keyboard/touch and partial outcomes |
| IN05 | IN02 | Queue insertion preview/drag/move/actual-next presentation with stable occurrences | ACT03/04, IX025–032 and shared PlaybackOrder property tests |
| IN06 | IN03/IN05 | Temporary listening shelf and time-budget session planner | ACT05/06; real catalog/known durations, no preview autoplay |
| IN07 | IN02/IN03 | Metadata trail, edition peek, mix preview/explanation/feedback | ACT08/09/13; actual authorized catalog/recommendation services |
| IN08 | Existing transfer/download foundations | Offline readiness/storage/priority/recovery interactions | ACT10/15; actual source/worker receipts and server-stopped validation |
| IN09 | Required history/social/device services | Private listening views, conflict/share/device/room interactions | ACT11/12/16; real consent/participants/rights, no decorative future controls |
| IN10 | Continuous after IN00 | Cohesive motion/preferences/accessibility/performance/packaging and final handoff | MOT01–12, actual UI review, source/artifact/platform evidence |

IN04 batch support may follow a later backend package; do not block a complete
single-item menu on it. IN05 true actual-next display requires side-effect-free
queue preview, not index+1 guesses. IN08/IN09 ship only the portions whose real
services exist. Future download/rooms features stay deferred, not fake menu items.

Recommended first integration: IN00→IN01+IN02→IN03, with IN10 gates at every step.
User-visible result: real stable loading, contextual Play/Pause/next/last/favorite/
playlist/details and accessible input parity. Only then expand complex manipulation.

## 2. Existing evidence to retain

- UX01/current-row transport: current click pauses/resumes without replacing source.
- PLAY/RES/NET/NP: queue/shuffle/repeat/source recovery/seek/modal and account races.
- Playlist authorization/CAS/revocation and private metadata-only sharing negatives.
- Private listening/feedback/retained-disabled-history and stale-session tests.
- Native artwork authorization/cache/store generation tests, current scale/appearance
  storage/recovery tests and actual source-specific mode limits.

Do not modify old tests to accept a new regression in order to make a visual suite
green. A changed product contract needs an explicit documented decision and meaningful
replacement assertions, not fewer real backend checks.

## 3. Real test fixtures

Use lawful committed FLAC/MP3/image fixtures and generated long PCM for timing/seek.
Create isolated users A/B with private libraries, a metadata-only collaborator,
permitted and denied artwork, playlists with repeated track entries, unknown/missing
metadata, long mixed-script titles and unavailable remote references.

Exercise actual requests: hold/release a real response, abort a real transfer, expire
a fixture session, revoke a real grant, stop/restart the fixture server. Do not replace
real DTOs with invented successful catalog/menu responses. Deterministic pure tests
are appropriate for timing policy, command projection/geometry and identity logic;
they supplement actual browser/native integration.

## 4. Complete first-release interaction journeys

| ID | Journey | Required real observation |
| --- | --- | --- |
| FLOW01 | Initial Library | Delayed real read produces matching skeleton rows; actual metadata/covers settle without focus/layout jump |
| FLOW02 | Current song | Play, seek 7s, right-click, Pause, reopen, Resume; same active occurrence/source and expected clock |
| FLOW03 | Add next | Context action inserts exact target next, current playback continues, actual queue preview agrees |
| FLOW04 | Add last | Appends distinct occurrence; repeated track does not overwrite current duplicate |
| FLOW05 | Playlist picker | Search/create eligible destination where implemented, add once, actual backend entry/receipt, safe Undo |
| FLOW06 | Rights negative | Metadata-only collaborator cannot play/download or view private art through forced menu invocation |
| FLOW07 | Race | Hold actual picker/menu-related read, navigate/logout/login B, release; no old menu/name/receipt reappears |
| FLOW08 | Focus | More/keyboard/Right/Left/Escape/Tab behavior, disabled reason, focus return including deleted row |
| FLOW09 | Edge and scale | Open near four viewport corners, inside Now Playing dialog and current high scale; actual clicks target correct item |
| FLOW10 | Touch | Long press/cancel-on-scroll/More sheet/Back with safe area; no accidental Play from a completed long press |
| FLOW11 | Refresh/pagination | Same-context refresh retains known rows; tail append preserves scroll/current source |
| FLOW12 | Reduced motion | Static loading and immediate state feedback remain useful; no looping decoration or inaccessible controls |

Later releases add actual drag/selection/session-planner/offline/privacy/share/device/
room flows from ACT and parent G scenarios; do not mark them tested from FLOW01–12.

## 5. Responsive and platform matrix

Web:320/390, tablet portrait/landscape,1440/1728/ultrawide, dark/light/System;
current implemented app scale 80/100/125/150 and browser zoom 200/400% where supported;
text enlargement and long English/Russian labels. Record future scale changes rather
than testing invalid imported values as if they were a supported preference.

Native: actual owned WebKitGTK window, normal tiled Wayland and separate X11 responsive
fixtures, actual viewport/host scale/DPR/PNG sizes, active real native audio engine.
Null-sink correctness and physical output remain distinct evidence classes.
Other desktop OS/mobile require actual toolchains/runtime evidence; touch simulation
does not certify a native Android/iOS media service.

Test menus inside real native HTML Now Playing dialogs/top layers. Review click
geometry after scaling and scrolling, not screenshots alone. RTL and real screen
readers need separately recorded target/version/input evidence.

## 6. Resource and latency checks

Record actual menu-open latency, layout/paint/frame intervals during shimmer/scroll,
DOM/placeholder/layer count, decoded art memory, live timers/listeners/observers and
idle CPU. Compare the same reference host/build/catalog/codec/network state before
and after. Report p50/p95/p99 when sample count supports them, not single-run slogans.

Assertions: no extra stream/body request from opening a menu or a details preview;
no extra source generation from skeleton/theme/scale; no per-audio-tick full-list
rerender; no persistent timer/listener after closing; no unbounded action/picker jobs.
Image request counts should respect existing single-flight bounds and current audio
must not starve under artwork/list skeleton work. Do not claim 144Hz from a CSS effect.

## 7. Review requirements

Before behavior/styling load Better Design UX/UI principles with the existing Vela
design. After real implementation load review rules, check actual visible copy and
inspect captured DOM spacing in named idle/loading/menu-open/selected/focus/error
states. Inspect screenshots at equivalent content/size and fix critical/serious flags.
Record moderate issues and unsupported review tools without inventing approval.

Do not publish private user pages for review. Public preview/external deployment
requires its own authorization. Local isolated fixtures are the default evidence.
No rendered UI is created in this planning task, so no spacing/screenshot pass is claimed.

## 8. Engineering checks and packaging

Use repository-supported fmt, strict workspace/native-qa/WASM Clippy, unit/property/
API tests, current web build, normal native build, browser suites per engine and native
fixture driver. One Cargo process at a time, at most 2 compiler jobs. Re-run the full
relevant integration suite after root command/overlay changes; document targeted
versus full runs accurately. Dependency/license gates apply when dependencies change.

New CSS/JS assets require exact server allowlist and package verification; include
the same source/assets in the executable evidence. Do not validate only a development
page then claim the frozen user service was upgraded. No migration needed for basic
skeletons/menus; broader persisted features follow backup/restore discipline.

## 9. Definition of done and handoff record

For each IX/IN outcome record actual files, source hash/dirty status, API/capability
boundary, tests/exit counts/skips/failures, screenshot/trace paths, resource baseline,
privacy scope, deployed versus source-only state and remaining next tasks. Preserve
intermediate failures. A menu animation, UI mock or successful compile does not
complete the real action behind it.
