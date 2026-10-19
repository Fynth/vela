# Implementation-agent handoff

This file is a copyable future implementation mandate. The current author is only
writing documentation. Receiving this document as a file does not override a user's
current “do not edit code” instruction. Start implementation only when assigned.

For the latest native-focused request, use the dedicated
[native master plan and copyable prompt](14_NATIVE_CROSS_PLATFORM_MASTER_PLAN.md)
first, together with the requirements below.

## Copyable master prompt

> Work as the principal engineer on the EXISTING Vela Music Rust/Dioxus repository.
> Read AGENTS.md and current docs/PROGRESS.md, CONTRACTS.md, CURRENT_STATE.md,
> ARCHITECTURE.md, SECURITY.md, KNOWN_LIMITATIONS.md, PLATFORM_MATRIX.md and the
> current product mandate. Then read every file in
> docs/PLANS/2026-10-19/PRODUCT_EXPANSION/ in its README order. These are proposals;
> verify actual source/runtime before declaring anything implemented.
>
> Preserve dirty changes and original user data. The planning anchor was6fa2346,
> with three existing dirty desktop/QA files and unrelated prompt.txt; that is a
> historical clue, not permission to reset a later checkout. Preserve target/
> runtime snapshots/backups/releases. Never cargo clean, recreate the project,
> rewrite history, remove original media or weaken authorization for convenience.
>
> Deliver complete vertical slices from10_DELIVERY_ROADMAP.md. The current urgent
> native outcome is authorized artwork in real Library/player/album/playlist/queue
> surfaces, followed alongside by searchable native/web Settings and App size.
> Read14_NATIVE_CROSS_PLATFORM_MASTER_PLAN.md and execute its ART/SCALE acceptance.
> Existing native complete-file cache already exists; extend it safely rather than
> pretending it is absent. Prefetch and progressive playback are separate gates.
>
> Establish typed preference scope/default/effective/policy/restart contracts.
> Implement device-only presentation persistence first. Use exact pinned Dioxus/Wry
> APIs with Result-aware adapters, preserve browser zoom and OS DPI, retain root
> audio across scale/theme/navigation and provide safe100% recovery. Do not expose
> settings for unsupported engines or store tokens in preferences/renderer storage.
>
> Apply the320-feature catalog in accepted dependency-ordered packages. Every active
> control must call an actual authorized service. Add mature playback/queues/cache/
> downloads/library/search/metadata/playlists/discovery/private statistics and native
> clients. Devices/rooms/multi-server/federation follow actual rights and lifecycle
> contracts. Optional identity remains optional; no paid/cloud/AI dependency for core.
>
> Keep the existing modular monolith, SQLite/WAL, private libraries, stable media
> identity, typed SQL/API contracts, root playback owners and explicit native engines.
> Maintain cancellation plus independent session/origin/query/generation/lifetime
> guards. No private-history collection without current account/device consent;
> no public popularity from private self-reported history. No playlist metadata
> grant becomes media permission. No remote cache becomes locally owned audio.
>
> Audit confirmed defects versus missing/unsupported/stale-build behavior. Reproduce
> bugs with real fixtures, fix root causes, retain failures and add meaningful tests.
> Prioritize known P0 defects, artwork/scale, native paused restore/origin-safe offline,
> measured body reuse/actual-next prefetch and daily UX before advanced experiments.
>
> Use real specialist subagents only when the active task authorizes them and the
> environment supports them. Freeze contracts/file ownership first. Main owns root
> integration, workspace/lock/migrations and build scheduling. Avoid simultaneous
> ui.rs/main.rs/Cargo edits. At most2 Cargo compiler jobs on this host. Review and
> integrate contributions; never invent delegation or accept unsupported test claims.
>
> Before schema/storage changes verify backup and isolated restore. Prefer additive
> migrations and preserve old refs/snapshots. Actual sensitive/destructive operations,
> original-data restore, privileged/public deployment, paid services, account creation
> and external release require appropriate specific authority. Routine reversible
> engineering choices are yours; do not ask the user to pick crates or continue.
>
> Verify with the current repo's fmt, strict workspace/native-qa/wasm Clippy, unit/API/
> property tests, real isolated browser/native servers/audio and exact fixture artifacts.
> Run11_ACCEPTANCE_AND_QA.md, SCALE01–14, SET01–16 and ART01–14. Preserve existing
> named regressions and report skips/ignored/failures. Use actual Android/iOS/desktop
> platforms for background/system-control/hardware claims; record blockers precisely.
>
> Follow Better Design on the existing original Vela system: principles before
> behavior/styling, actual rendered screenshots/DOM and review after implementation.
> No second framework, copied Spotify branding, demo songs, decorative device buttons
> or false gapless/bit-perfect/public-catalog guarantees. No malicious source access,
> DRM bypass, arbitrary FFmpeg shell or weakened import SSRF policy.
>
> Profile before optimization, capture cold/warm/prefetched/seek costs and actual
> transfer counts; distinguish engine time from audible sound. Fix blocking work,
> unbounded tasks, image memory/request storms, repeated copies and query plans
> before inventing infrastructure. Maintain a deployable working increment.
>
> Update authoritative progress/contracts/architecture/roadmap/security/performance/
> limitations/platform/parity evidence with exact source and artifact identities.
> Commit only coherent owned changes after relevant gates. Continue accepted
> packages while the active session permits; do not claim background work outside
> execution. Before stopping leave a precise resumable checkpoint and prioritized
> next unblocked task, not a fictional “all done”.

## Mandatory task receipt

For each package hand back: changed paths, useful real user outcome, EX/EP/ART/SCALE/
SET test mappings, exact commands and exit codes, counts/skips, source/runtime/artifact
identity, screenshots/traces/bench methodology, permission/privacy scope, migrations/
restore evidence where applicable, unresolved gates and next task. “Build passes”
alone is not a product feature receipt.

## Immediate acceptance checklist

The first native contribution must demonstrate a real uploaded cover in the Library
and current player, denied foreign art, stale image suppression after sign-out,
bounded rendition/cache, no credential in renderer URL and source continuity.
The first settings contribution must demonstrate actual percent change/reset,
persistence/recovery, no media remount, keyboard/HiDPI/reflow and truthful save state.
If either cannot be demonstrated, keep it PARTIAL and fix it before expanding visuals.
