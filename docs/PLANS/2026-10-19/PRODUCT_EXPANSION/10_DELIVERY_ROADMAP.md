# Delivery roadmap and future work assignments

This is one proposed delivery backlog. EX entries are acceptance outcomes within
packages, not320 concurrent projects. Accept packages in dependency order and update
the existing authoritative ROADMAP/PROGRESS after real implementation. No deadlines
or staffing capacity are invented. Complexity estimates are relative, not promises.

## Ordered packages

| Package | Prerequisites | Complete user-visible increment | Primary ownership / risk |
| --- | --- | --- | --- |
| EP00 | None | Verified current checkout/runtime/data inventory, real UX baseline and accepted feature ledger | Integrator/QA; preserve dirty tree and original data |
| EP01 | EP00 | Typed device preferences, persistence/receipts, System/Dark/Light and safe recovery | Shared validation + roots; small/medium |
| EP02 | EP01 | Application scale on desktop/web with reset, persistence, HiDPI and audio continuity | Desktop window adapter/web tokens; medium; SCALE01–14 |
| EP03 | EP01/EP02 | Real native/web Settings navigation, local search, scopes, import/reset and initial accessibility | UI service/components; medium/large |
| EP04 | EP01/EP03 | Administrator policy/effective configuration, safe quota/jobs/security/settings actions | Core/server + admin UI; large; privileged boundaries |
| EP05 | EP00 | Stable queue occurrence model, native paused restore, saved sessions, reorder/undo and actual-next preview | Playback/integrator; large regression risk |
| EP06 | EP01/EP03 | Measured cache/download storage UI, safe configurable budgets/cleanup/integrity | Native cache/offline; medium/large |
| EP07 | EP05/EP06 | Single-flight actual-next preparation, measured recovery; progressive source only after integrity gate | Media/client/root; large; no security regression |
| EP08 | EP05/EP07 | Negotiated audio quality/output plus individually validated DSP/transition slices | Audio/platform; large; requires measured signal tests |
| EP09 | EP06/EP07 | Album/playlist offline manager, resumable transfers, network policies and offline metadata | Download/client/native; large |
| EP10 | EP00 | Additive rich catalog/editions/assets/contributors/multiple libraries and usable browsing | Data/domain/core/UI; large migration risk |
| EP11 | EP10 | Multilingual ranked search, indexed filters, saved views and keyboard suggestions | Search/data/UI; large; benchmark gate |
| EP12 | EP10 | Authorized native artwork, provenance editing, conflicts, lyrics and repair workflows | Media/metadata/UI; split into small independent slices |
| EP13 | EP05/EP10/EP11 | Stable collaborative entry provenance, smart rules, cover/description and portable playlist formats | Playlist/core/UI; large CAS/rights risk |
| EP14 | EP05/EP10/EP11 | Actual materialized mixes, radio variants, feedback/coverage/explanations | Recommendation/media; large; privacy gate |
| EP15 | EP01 | Private native listening, retained-history policy, aggregates/export/delete and accurate stats | Listening/privacy; large; consent gate |
| EP16 | EP05/EP09 | Device presence/commands, incremental queue/bookmark sync and acknowledged handoff | Backend/playback; large; two-real-client gate |
| EP17 | EP13 | Local profiles/invitations/blocking/activity/notifications with separate media permissions | Social/core/UI; large |
| EP18 | EP14/EP16/EP17 | Authorized listening rooms/shared mixes with reconnect/roles/clock handling | Realtime/playback; large; participant rights gate |
| EP19 | EP10/EP11/EP16/EP17 | Multi-server adapters, optional portable identity and standards-based federation increments | Network/security; very large; independent protocol gates |
| EP20 | EP03/EP05/EP09/EP12 | Linux daily-client completion: Add Music, file/deep links, output/MPRIS/window behavior, mini-player | Native/Linux; large; physical output validation |
| EP21 | EP01/EP05/EP09/EP16 | Android Dioxus client with Media3 service, downloads/focus/system controls | Android/platform; very large; SDK/device required |
| EP22 | EP01/EP05/EP09/EP16 | iOS client with AVFoundation/session/background/offline/remote commands | Apple/platform; very large; Xcode/device required |
| EP23 | EP20 | Windows/macOS runtime/media-key/install packages | Platform/release; large; actual runners required |
| EP24 | Continuous after EP01 | Full accessibility/localization/context menus/notifications/polish tied to working features | UX/QA; ongoing, not final cosmetic cleanup |
| EP25 | Continuous after EP00 | Backup/restore/update/observability/performance/license/package release gates | DevOps/security/QA; original-data handover explicit |
| EP26 | Appropriate foundations | Optional lawful import adapters, podcasts/radio, casting/automation/speech/extensions | Integration specialists; each adapter independently accepted |

## Dependency nuances

EP12's first authorized artwork slice does not need every edition model in EP10:
it may start once current track/artwork contracts are frozen. EP14 content-only
mixes can work without EP15 history collection; history-based mixes require EP15's
consent/aggregate evidence. EP16's first two-client web/Linux handoff can precede
mobile; phone-to-desktop acceptance waits for actual native clients.

EP19 is several gates, not one enormous federation merge: saved native Vela adapters,
unified search, source resolution, optional OIDC, local social export, then versioned
federation discovery/signed delivery. Do not block useful single-server releases
on public-network completeness. EP26 optional integrations never hold the core app
hostage to unavailable provider credentials or rights.

## Release slices

### Release slice A — settings that work

EP00–EP03; add safe minimal EP04 status projection if available. Ship native Settings,
scale/theme/text/density where implemented, local search/reset/persistence and truthful
capabilities. Do not expose DSP/rooms/provider settings before their actual engines.
Exit: SET01–SET16, SCALE01–14 and preserved browser/native regression suites.

### Release slice B — daily listening without friction

EP05/EP06, first artwork/metadata slice, initial EP20 actions, selected EP24 fixes.
Exit: queue restore, consistent duplicate actions, safe storage control, real art,
Add Music path and keyboard parity. No repeated unnecessary body transfer on validated
cache hit. Do not claim progressive playback before its distinct gate.

### Release slice C — dependable offline and smarter playback

EP07/EP09 and individually verified EP08 components. Exit: actual-next prefetch,
resumable downloads, local cold startup, measured transport/seek/cancel/restart.
Audio quality/transition claims limited to the tested engine/codec matrix.

### Release slice D — rich catalog and meaningful discovery

EP10–EP15 with feedback/statistics/privacy. Exit: useful album/artist/playlist pages,
multilingual search, stable edits, real scheduled mixes and accurate private totals.
Five-track cold-start and100k-entry workloads are both required.

### Release slice E — connected clients and social listening

EP16/EP17/EP18 plus native-platform work. Exit: real handoff between two clients,
revocable invitations and room authorization/drift/reconnect tests. Platform claims
wait for their own hardware evidence; an emulator is explicitly labeled.

### Release slice F — optional open network and wider platforms

EP19/EP21–EP23, continuing EP25 and selected EP26 adapters. Exit: two independent
servers with actual signed/authorized interoperability, useful native platforms,
safe release/restore and rights provenance. No central account required for basic use.

## Package work-ticket template

For every future task record:

1. User outcome and EX IDs, current implementation evidence and what changes.
2. Owner, allowed files, forbidden shared files and integration contract.
3. Required DTO/state/schema/API changes; zero schema change when unnecessary.
4. Exact UI states/actions, permissions, empty/loading/error/unsupported behavior.
5. Cancellation/session/generation/privacy/identity invariants.
6. Persistence/migration/backfill/rollback strategy and fixture backup prerequisites.
7. Meaningful tests, real runtime/browser/native scenarios and target evidence paths.
8. Bounds/resource/latency measurements; targets distinguished from results.
9. Documentation/platform/feature-ledger updates and unresolved external gates.
10. Concrete completion receipt and one coherent local integration commit.

## First agent task, already specified

Take EP00–EP02 as a bounded first vertical slice. Preserve the existing three dirty
desktop/QA files. Freeze a scale/presentation contract, implement device preferences,
build native/web App size controls with exact pinned APIs, run the real scale matrix
and preserve playback. EP03 Settings category/search work can share that integration
boundary after the preference service is stable. Do not start with social icons or
a static new Home while scale has no persistence/recovery.

## Future parallel assignment boundaries

Delegation is for a future authorized implementation run, not this documentation pass.
Use the real available concurrency limit, not an imaginary11-agent organization.
The integrator owns workspace files, migrations, root app wiring and shared contracts.
Assign other contributors isolated component/service files or worktrees:

- Preferences engineer: pure validation/store adapters with agreed signatures.
- UI engineer: Settings presentation components/styles only; root integration stays lead-owned.
- Playback engineer: occurrence/journal logic in agreed isolated files; no simultaneous root edits.
- QA/security reviewer: tests/read-only review with test runner schedule controlled by lead.

At most two Cargo compiler jobs on this host; one coordinated Cargo build/test process.
Do not let several agents edit main.rs/ui.rs/Cargo.lock or allocate the same migration.
Main reviews contributions and executes combined integration acceptance. A subagent's
claim is not proof without actual diff and evidence.

## Product comparison gate

To claim Vela is preferable on a journey, compare task success, steps, recoverability,
measured latency, privacy/ownership and accessibility on the same lawful catalog and
documented devices. Catalog size/licensing, paid services and provider recommendations
are separate variables. Feature quantity alone cannot establish “better than Spotify”.
