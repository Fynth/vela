# Repository facts and actionable UX gaps

## Inspection and preservation record

Inspection anchor: main `6fa2346`, workspace version 0.12.6, edition 2024.
There are eight real workspace members, not a greenfield scaffold. Workspace pins
include Dioxus 0.7.10; Cargo.lock resolves dioxus-desktop 0.7.10 and Wry 0.53.5.
The pinned local dependency source was inspected for zoom behavior.

At entry, these files already contained uncommitted changes:

- `apps/desktop/src/main.rs`: Wayland/DMABUF configuration work.
- `apps/desktop/examples/native-ui-qa.rs`: matching QA launch configuration.
- `apps/desktop/examples/native-ui-qa.js`: an FPS measurement branch.
- Untracked unrelated `prompt.txt`, not read or changed.

Do not overwrite, stage or characterize those changes as delivered by this plan.
Non-Markdown tracked files were fingerprinted before writing; compare them after
documentation work. No subagents, Cargo builds, browser runs, migrations, releases,
source changes or runtime upgrades are authorized by this documentation task.

Final preservation observation: those pre-existing native changes were independently
committed as7113fd5 during this task. All443 initially fingerprinted non-Markdown
working-tree files remain byte-identical. This is concurrent repository activity,
not a source edit or performance verification performed by this plan's author.

Read-only runtime observation: `vela-local-development.service` active,
MainPID 1178, NRestarts 0; loopback `/health` reports 0.12.6. This does not prove the
service runs source HEAD. Preserve its frozen runtime and original data directory.
No database/media content was changed or copied by this planning pass.

## Actual architecture to retain

| Boundary | Existing locations | Preserve |
| --- | --- | --- |
| Presentation/root playback | apps/web/src/main.rs, browser_player.rs, playback.rs, experience.rs, shell.rs | One persistent HTMLMediaElement; session/query/generation/lifetime guards; occurrence queue |
| Native presentation | apps/desktop/src/ui.rs, window.rs, main.rs; style.css | One root coordinator, integrated window actions; native output stays outside WebView |
| Native media | apps/desktop/src/audio.rs, client.rs, media_cache.rs, offline.rs | One Rodio/CPAL authority; bounded preparation, verified bytes, isolated cache and pinned copies |
| Native account/peers | session_store.rs, peers.rs, remote.rs, mpris.rs | Reviewed Secret Service path, canonical origin identity, MPRIS observer, remote authorization |
| Application services | crates/core/src/config.rs, auth.rs, listening.rs, playlists.rs, network.rs, jobs.rs | Owner scoping, CSRF, revision/CAS, durable existing workers |
| Catalog/intelligence | crates/media/src/catalog.rs, search.rs, filters.rs, metadata.rs, analysis_service.rs, recommendations.rs | Indexed bounded queries, immutable originals, explicit/versioned analysis, private ranking |
| Transport | apps/server/src/lib.rs, listening.rs, analysis.rs, recommendations.rs, network.rs | Typed/authenticated endpoints; no business policy duplicated in handlers |
| Persistence | migrations/0001–0012; crates/core/src/database.rs | SQLite/WAL/FTS and forward migrations; no reset or destructive replacement |
| Quality | tests/browser/*.spec.mjs; apps/server/tests/api.rs; scripts/verify_native_client.py; crate tests | Preserve named regressions and distinguish browser/native/hardware evidence |

## Settings findings from current source

Web Settings in `apps/web/src/main.rs` currently combines account/version/storage
information, a theme action, private listening controls, Network navigation,
administrator account creation and scanning panels. This is not a complete preference
system. Native desktop navigation currently has no dedicated Settings destination;
theme/sign-out are toolbar actions. Desktop theme starts from an in-memory dark
signal, while web theme uses `vela.theme` local storage. Application scale is absent.

`crates/core/src/config.rs` reads bind/data/web directories, cookie security,
file/storage limits and scan roots from environment variables. These are not all
safe runtime user preferences. The application needs an effective configuration
projection and policy-aware controls, not a form that rewrites arbitrary environment
variables or shell commands.

Native reusable cache exists: current module limits are 200 entries/2 GiB per
account+origin namespace; offline copies have separate 100-track/2 GiB limits.
Those constants are source observations, not recommended universal budgets. Cache
hits still authorize and validate a complete representation. Localhost transport,
hash verification, decoding and UI scheduling can all contribute to preparation:
“local server” does not mean zero work. Prefetch/progressive preparation remains
distinct from this complete-file cache.

## Prior executed evidence, not rerun here

The latest progress records 293 Rust passes/3 ignored, Chromium and Firefox each
43 passes/1 live-import skip, 89 native X11 checks, and a separate 5-check normally
tiled native-Wayland subset. These are historical results at their stated source
boundary. Current uncommitted launch/FPS work has not inherited that certification.
Review [interaction evidence](../../../UX_INTERACTION_REVIEW_2026-10-19.md) and
[platform matrix](../../../PLATFORM_MATRIX.md) before execution.

## Priority gaps

These are observed missing/partial paths or design risks, not newly reproduced bugs.

| ID | Gap and affected source | Priority | Concrete closure |
| --- | --- | --- | --- |
| GAP01 | No device scale setting; desktop/window.rs, ui.rs; web root/styles | P0 | EX261; scale preset/numeric/step/reset/recovery, no audio restart, HiDPI matrix |
| GAP02 | No native Settings route; web Settings mixes administration/listening | P0 | Dedicated searchable categories and admin boundary; keyboard/back/scroll behavior |
| GAP03 | No typed scope/default/effective/locked/restart preference model | P0 | Registry, persistence adapters, capability snapshot and truthful save state |
| GAP04 | Appearance persistence differs across clients | P0 | Device-scoped System/Dark/Light migration; no white flash or account data leakage |
| GAP05 | Cache/offline quotas and transfer policy are not ordinary UI controls | P0 | Actual usage, bounded editable quotas, clear-cache preview excluding downloads |
| GAP06 | Native queue/context/position lack web-equivalent durable restore | P0 | Account+origin snapshot, stable occurrence IDs, paused cold restore |
| GAP07 | Duplicate playlist context uses projections needing stronger provenance | P0 | Stable playlist-entry and queue-occurrence identity through concurrent reorder |
| GAP08 | Native artwork/album/artist presentation remains sparse | P1 | Authorized bounded artwork adapter; rich metadata, no fabricated covers/biographies |
| GAP09 | Native full-original prepare still delays unsupported cache paths | P1 | Measured source pipeline, progressive integrity decision and compatible fallback |
| GAP10 | Next-track preparation does not follow an immutable actual-next plan | P1 | Side-effect-free order peek, one speculative worker, mutation cancellation |
| GAP11 | Context menus, batch actions, undo and library navigation are incomplete | P1 | Shared action capability model and keyboard/long-press equivalents |
| GAP12 | Search lacks full multilingual ranking/suggestions/saved scopes | P1 | Unicode-aware normalized index, exact-first ranking, bounded federated fanout later |
| GAP13 | Playlists lack smart rules/cover workflow/portable round-trip | P1 | Typed AST, revisioned edits, authorized images, versioned exports |
| GAP14 | Catalog does not yet model editions/assets/credits comprehensively | P1 | Non-destructive additive model preserving existing track references |
| GAP15 | Native Add Music requires web administration for ordinary ingestion | P1 | Shared authorized client flows with transfer queue and honest recovery |
| GAP16 | Discovery is bounded on-demand, not scheduled daily/weekly mixes | P1 | Materialized private sets, feedback/explanation, sparse-catalog fallbacks |
| GAP17 | Device position/queue sync and acknowledged handoff absent | P1 | Separate device state/coordinator with epochs, acknowledgements and expiry |
| GAP18 | Android/iOS playback/offline clients not implemented | P1 program | Native services/bridges and real hardware acceptance, not a WASM claim |
| GAP19 | Public-catalog profile is not full federation/social/rights interoperability | P2 | Versioned standards profile, scopes, signed delivery, moderation/two-node tests |
| GAP20 | Account/recovery/integration/settings portability incomplete | P1/P2 | Session inventory, secure credential references, explicit export/import preview |
| GAP21 | General accessibility beyond fixture geometry not certified | P0 continuous | Text enlargement/reflow, keyboard, screen reader, reduced motion, RTL/locales |
| GAP22 | Operational controls mostly require install/bootstrap knowledge | P1 | Safe in-app status/policy/quota/jobs/backup workflow with privilege boundaries |
| GAP23 | Some declared quality/audio modes are platform-dependent | P0 continuous | Requested/effective/capability presentation; no pretend gapless/bit-perfect/normalization |
| GAP24 | Known dependency/license gate remains open in dated evidence | P0 release | Reassess advisories/notice gaps without suppressing failures to pass a UI release |

## Review limits

Source and existing screenshot/evidence review informed this plan. No new user study,
browser/native interaction, FPS benchmark, audio latency or accessibility execution
was performed. Inspect all screens against current runtime in package EP00 before
implementation. A working feature can remain incomplete without being a new bug.
