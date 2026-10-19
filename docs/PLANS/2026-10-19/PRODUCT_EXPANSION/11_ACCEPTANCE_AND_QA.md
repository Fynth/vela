# Acceptance, regression and measurement plan

All tests below are future requirements, not executed results. Preserve existing
EXP/PLAY/RES/NET/AUTH/NP/UX01/DESIGN01 tests and existing server/privacy/network tests.
Run application tests only in an authorized implementation task with isolated data.

## Settings acceptance suite

| ID | Scenario | Required observation |
| --- | --- | --- |
| SET01 | Native/web/signed-out Settings entry | Correct available categories; no hidden admin data or forced server contact |
| SET02 | Search aliases and keyboard suggestions | scale/zoom/масштаб find one actual control; Escape/Back restore focus |
| SET03 | Device versus account scope | Two devices keep independent scale; two accounts never share private preference refs |
| SET04 | Local immediate apply | Live effect without audio/route remount; session-only feedback on persistence failure |
| SET05 | Safe account update | Server receipt/revision controls Saved state; another user's data inaccessible |
| SET06 | Stale/held account preference reply | New session/origin/query/lifetime never accepts old result |
| SET07 | Concurrent updates/conflict | CAS rejects or merges explicit independent fields without silently losing intent |
| SET08 | Unsupported/policy-locked setting | Requested/effective/reason accurate; forced activation cannot bypass server authorization |
| SET09 | Draft Apply/Discard/Stay | Failed validation preserves draft; no unrelated section commits accidentally |
| SET10 | Next-track/output/restart setting | Active versus pending state accurate; no surprise playback/output interruption |
| SET11 | Preference export/import | Bound types/version/size; secret, privacy and identity fields excluded/skipped visibly |
| SET12 | Row/category/reset-all presentation | Exact preview and exclusion behavior; downloads/media/history unchanged |
| SET13 | Legacy/corrupt/future-version store | Valid theme migrated; corrupt/future documents fail safely without dropping healthy data |
| SET14 | Disabled collection/profile import | No history enrollment, public visibility or remote integration activation |
| SET15 | Admin mutation/CSRF/Origin/redaction | Ordinary member denied; hard ceilings enforced; logs contain no secrets |
| SET16 | Keyboard/screen-reader/high-scale/light/dark | Primary actions reachable, names/value announcements correct; record actual AT/browser/platform |

Also execute every SCALE01–14 case in the [scale specification](04_APPLICATION_SCALE.md).
For native work execute ART01–ART14 in the
[native master plan](14_NATIVE_CROSS_PLATFORM_MASTER_PLAN.md): real authorized art,
private/stale denial, bounded decode/cache, correct surfaces/offline and system media.

## Major subsystem acceptance

| Test family | Required integration cases |
| --- | --- |
| QUEUE | Duplicate occurrence reorder, current toggle, explicit context start, journal restore paused, edit/EOF races, fair shuffle peek without mutation, Undo and account isolation |
| CACHE | Authorized warm hit with no body, revoked HEAD denial, validator mismatch, corrupt sidecar/file, clear while active, lower quota, staging/orphan accounting, concurrent consumers/processes |
| PREFETCH | Actual shuffle-next, manual-next priority, repeat-one, last item, queue mutation, network policy, cancellation, single-flight ownership and zero second output |
| DOWNLOAD | Resume206/If-Range, changed ETag/length,416 recovery, disk full, restart mid-write, Partial collection, cold offline launch, local art/lyrics, permission/retention semantics |
| CATALOG | Additive migration preserves IDs, multiple library rights, editions/masters, contributor roles, missing tags, Unicode ordering,100k indexed filtering/paging |
| PLAYLIST | Stable duplicate entry ID, CAS/idempotency, concurrent moves, metadata-only collaborator, revoke/expiry, smart AST bounds/SQL safety, M3U unknown refs and export round-trip |
| DISCOVERY | Deterministic fixture ranking, novelty/diversity, sparse catalog, hidden-before-limit, current-consent checks, scheduled snapshot invalidation, feedback race and actual playable IDs |
| HISTORY | Monotonic qualified time, seek/retry/pause/EOF distinctions, device revocation, export race, erase/prune atomics, aggregate rebuild and timezone boundaries |
| DEVICE | Two real clients, separate sessions, command replay/reorder/reconnect, pending transfer cancel, destination denied/offline, source continues until acceptance |
| ROOM | Actual participant grants, latency/jitter/reconnect, host departure, concurrent queue commands, stale invite, blocked participant, unauthorized track does not become playable |
| NETWORK | Two independent origins, colliding IDs, slow peer partial search, invalid signatures/replays/SSRF/huge metadata, cache tombstone and revoked stream denial |
| NATIVE | Lifecycle/system controls/output route/focus/offline on real target; compile/run/hardware/package evidence separate |
| OPS | Restore fixture copy, forward migration, shutdown during work, quota recovery, exact package asset verification, redacted support bundle, dependency/license gate |

## Realistic fixtures and catalog sizes

Use committed lawful FLAC/MP3 fixtures and generated signals/PCM for transport tests.
Add licensed samples for actual music analysis and an explicit license manifest.
Include long titles, Cyrillic/Latin/diacritics, emoji in metadata as user text, unknown
artists, compilations, multi-disc editions, duplicates and remote UUID collisions.
Do not seed the real user library or replace backend responses with demo rows.

Test a five-track empty-history account, a realistic mixed small library,10k and100k
metadata owners, and1M synthetic metadata only as a separately labeled scale test.
Synthetic performance data never appears as a real recommendation or user statistic.

## Browser, native and responsive matrix

- Web: real current Chromium and Firefox suites in separate invocations with separate
  isolated server data. Safari/WebKit requires its own actual browser evidence.
- Native Linux: normal tiled Wayland in Hyprland, floating/maximized and separate X11;
  record actual backend/PID, host scale, DOM viewport, DPR and PNG dimensions.
- Sizes:320/390, tablet portrait/landscape,1440,1728 and ultrawide; dark/light/System,
  app75/100/125/150/200, text100/200 and browser zoom200/400%. Use a risk-based pairwise
  sweep plus all scale extremes; do not imply every Cartesian combination was run.
- Screen readers: actual Orca/NVDA/VoiceOver/TalkBack on supported targets, with exact
  version/device/action sequence. DOM labels alone are not screen-reader acceptance.
- Android/iOS: emulator/simulator versus physical device, background/locked/call/
  Bluetooth/airplane-mode conditions recorded individually.
- Windows/macOS: compile, install, launch, real decode/output/media controls and
  signing/notarization/entitlement status separately reported.

No new external public deployment is required for this documentation plan. For later
public visual previews follow explicit authorization and repository policy; isolated
local browser/native screenshots remain the default product QA evidence.

## Performance measurement design

Measure cold versus warm source starts separately. Record authorization/HEAD, bytes
transferred, integrity time, decoder time, engine-ready time, first media clock advance
and, only with suitable instrumentation, first audible sample. UI clock advance is
not proof of physical sound. Use actual packet counts to demonstrate prefetch/cache.

For each run record hardware/OS/backend/build/source hash, dataset ownership/distribution,
cache state, codec/size, concurrency, repetitions, raw samples and p50/p95/p99. Include
RSS/CPU/disk, library query plans, dropped frames and energy where measurable. Report
noise/outliers and compare identical workloads; do not publish unsupported user counts.

Proposed targets for baseline discussion, not promises: local preference input reacts
within100ms; scale/theme causes zero new media-source resolution; remote search shows
local partial results without waiting for a slow peer; current playback is prioritized
over speculative preparation; idle Settings does not start a continuous animation loop.
Set handoff/room drift budgets after actual clock/transport measurements, not by claiming
sample precision from command timestamps.

## Commands the receiving implementation agent should adapt and execute

Use current repository scripts/features and one Cargo process at a time, max2 jobs.
Run fmt, strict workspace/all-target Clippy, strict wasm32 web Clippy, explicit
native-qa/all-target Clippy, workspace tests, pinned web build, normal native main
and QA example builds. Run Python tooling tests and browser suites per engine.
Use `scripts/verify_native_client.py` with fresh evidence directories for its supported
X11/Wayland scenarios; expand the harness only for real new acceptance requirements.

For source/dependency changes run the established dependency/license audit and fix
failures rather than suppressing them. Docker/ARM64/native release/runtime claims
need actual builds and execution. Restore/migration checks use isolated copies.
No test commands in this section were run by the author of this documentation package.

## Current mandate G01–G24 mapping for future acceptance

| Mandate scenarios | Relevant packages and required scope |
| --- | --- |
| G01/G24 preservation, upgrades/restores | EP00/EP25; actual old-data/fixture restore, same-artifact runtime validation |
| G02/G03 upload/import | EP09/EP12/EP20/EP26; real media and controlled egress |
| G04/G09/G10 Home/radio/cold taste/mixes | EP14/EP15; content eligibility and truthful personalization |
| G05 queue/player/recovery | EP02/EP05–EP09; retain current named playback regressions |
| G06 collaborative playlist | EP13/EP17; CAS and independent streaming authorization |
| G07 private listening | EP15; all consent/erase/export race cases |
| G08 DSP | EP08/EP14; synthetic and licensed real material, honest descriptor limits |
| G11–G15/G22 public network/rights | EP19; two-instance/provenance/hostile-peer/stream-revocation evidence |
| G16 Android | EP21; real APK and documented hardware/background/offline scope |
| G17 iOS | EP22; Apple toolchain and actual simulator/device evidence |
| G18 desktop platforms | EP20/EP23/EP25; per-OS install/run/audio/session proof |
| G19 handoff | EP16 plus available native targets; actual independent clients |
| G20 rooms | EP18; participants/media/latency/reconnect/role tests |
| G21 workload/performance | EP07/EP11/EP14/EP25; reproducible contention and search metrics |
| G23 accessibility/localization/theme | EP02/EP03/EP24; actual keyboard/AT/locales/narrow views |

## Evidence receipt

For each accepted EX/EP record exact source commit and dirty-tree status; schema;
artifact hashes; command exit codes; passed/failed/skipped/ignored counts; test IDs;
screenshots/traces; real server/backend/device details; retained intermediate failures;
privacy/rights scope; known limitations; deployed versus source-only state; next task.
Never turn a successful plan validation into an application feature verification.
