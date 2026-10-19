# Current state index

Latest source-only interaction repair2026-10-19 (baseline91867f2): row pause/resume,
pending intent, per-entry native playback, route-independent public transport and
frameless Linux behavior are recorded in [review](UX_INTERACTION_REVIEW_2026-10-19.md)
and latest PROGRESS/QA_EVIDENCE. Real Wayland tiling is checked for its stated subset;
separate X11 peers/offline evidence does not imply full Wayland certification.
Debug binary and development web assets are rebuilt. The frozen0.12.6 service and
already running user desktop remain separate processes, not automatically upgraded.

Newer source-only UI increment2026-10-19: original mint high-contrast web/Linux
native design, shell/control fixes and real fixture regression evidence are recorded
in [the design review](DESIGN_REVIEW_2026-10-19.md) and latest PROGRESS/QA_EVIDENCE.
Baseline0170ec5, workspace0.12.6. This did not upgrade the frozen runtime or replace
release archives; the older dated source/runtime observations below keep their own
scope and are not evidence of today's installed desktop appearance.

Updated2026-10-19 (documentation reconciliation, docs-only). This page states **which
state a claim refers to**. It does not re-verify code, tests, browser/native journeys
or the live service; it indexes existing evidence and records the read-only
inspection done while writing it. No code, schema, runtime, test or package was
changed for this page.

## 1. Source behavior — commit `05b4c37`

- Source behavior audited at `05b4c37fc491ef732e75c6decf5f89c0da7f2c71`
  (workspace version0.12.6). The later commit `09c7ad9` adds only the
  `docs/PLANS/2026-10-19/` audit package and README/docs edits; the code tree and
  `Cargo.lock` are unchanged by it.
- Checks re-executed in that source audit (not re-run for this documentation pass):
  `cargo fmt --all -- --check` PASS, strict workspace/all-target Clippy PASS,
  wasm32 web Clippy PASS, `cargo test --locked --workspace -- --test-threads=2`
  **244 passed / 0 failed / 2 ignored**, Python **22 passed**. Logs:
  `target/checks/audit-20261019/`.
- **Not** re-run in that audit: full Chromium/Firefox suites, native WebKitGTK
  journeys, release packages and the two ignored desktop audio/MPRIS tests.
  Historical runs remain valid for their own dates (see PROGRESS/QA_EVIDENCE), not
  as today's verification.
- Source-only changes after the frozen package `7fe1ca0`, present in source but
  **not deployed** to the live runtime:
  - validated peer-preview cursor forwarding (`3a3f463`) and the web preview
    `Load more` (`e3b6168`, `fb43a82`/`1ec2cc8` evidence). The browser suite has no
    successful-preview scenario; source evidence is the server fixture test.
  - 100k artist-grouping/stats planner pins (`1a6bf32`, `05b4c37`). Their numbers
    exist only in the isolated benchmark scope in PERFORMANCE; they are not a live
    runtime measurement.
  - desktop saved-session P0 repair (`3094754`: AUD-01/02/04/11), listening
    snapshot reads (`48b2fa4`: AUD-13), and offline lifecycle/quota repair
    (`e225b63`: AUD-03/07/08/12). Verified at source with fmt/strict Clippy,
    desktop56/3 ignored, core26+7, API31, red/green AUD-13 tests, and the real
    two-process Secret Service/offline native-window QA. Not deployed.

## 2. Deployed runtime — frozen0.12.6, schema12

- `target/runtime/vela-0.12.6` (`BUILD.txt`: version0.12.6, commit `7fe1ca0`,
  `schema_version=12`) serves the original data directory. A new `cargo build` does
  **not** replace or upgrade it; only the explicit installer/upgrade path does.
- Read-only inspection while writing this page (2026-10-19): user unit
  `vela-local-development.service` active, MainPID544183, NRestarts0; `/health`
  returned `{"status":"ok","version":"0.12.6"}` on `127.0.0.1:3210`; SQLx migration
  version12 (`PRAGMA user_version` is unused by these migrations) and `quick_check=ok`;
  counts1user/20tracks/20assets/20analysis/
  0playlists/0folders/0publications. The20 analysis rows retain extractor
  `vela-pcm-spectral-1`; new jobs on this runtime use `vela-pcm-spectral-4`.
- The portable native0.12.6 tar
  (`target/desktop-releases/vela-desktop-0.12.6-linux-x86_64-unknown-linux-gnu.tar.gz`,
  SHA256 `7abe3982…`, BUILD commit `7fe1ca0`) is a separate artifact from the server
  runtime; both derive from the same release commit.
- Consequence: source-verified fixes newer than `7fe1ca0` are absent from the live
  server/web runtime until a future patch release and installer handover. Do not
  call a source-only fix "deployed", and do not call the live runtime "source HEAD".

## 3. Claim classes

| Class | Meaning | Where it lives |
| --- | --- | --- |
| SOURCE-VERIFIED | Executed at `05b4c37` or another stated commit, with logs: fmt/Clippy/Rust/Python, and browser/native only where a dated run is named. | PROGRESS, QA_EVIDENCE, PERFORMANCE |
| DEPLOYED | Read-only live0.12.6 checks plus dated package/installer evidence for that exact artifact. | QA_EVIDENCE package rows, PROGRESS handover entries |
| HISTORICAL | Valid for its own date/version (0.9–0.12.5, earlier commits); not current. | Older PROGRESS/RELEASE_NOTES sections, parity ledger |
| DESIGN CHECKPOINT / PROPOSAL | Designs, UX, visual system, large-hub scale plan and unimplemented scenarios. Not implemented and not requirements. | `docs/PLANS/2026-10-19/`, RESILIENT_SCALE_PLAN, G13/G16/G17/G19 |

Examples of the split:

- Native offline copies, opt-in Secret Service session resume and playlist folders:
  implemented and package-verified for0.12.6 (DEPLOYED for that artifact), with open
  audited lifecycle/timeout defects (AUD-01–AUD-08, AUD-11).
- Peer preview cursor forwarding / `Load more`: SOURCE-VERIFIED only; the live0.12.6
  web assets and server do not include it.
- Redis/PostgreSQL/OpenSearch/NATS,10k RPS and10k listeners: PROPOSAL in
  RESILIENT_SCALE_PLAN. The mandate and the implementation are one SQLite process
  with local files.
- G13/G16/G17/G19, federated recommendation, signed federation: DESIGN/NOT_RUN.
- G14/G15: PARTIAL executed subsets only, exact scope in QA_EVIDENCE.
- Native Windows/macOS/Android/iOS clients: NOT_STARTED/BLOCKED, not inferred from
  Linux.

## 4. Dependency gate (AUD-20 / AUD-21)

- The fresh `cargo-deny check` (advisories) **FAILED** on three INFO Unmaintained
  notices: `zvariant5.15.1` RUSTSEC-2026-0370, `zbus_names4.3.5`
  RUSTSEC-2026-0371, `zvariant_derive5.15.1` RUSTSEC-2026-0372. Chain:
  `vela-desktop → mpris-server0.10.0 → zbus5.19.0`. Bans/licenses/sources passed.
- No ignore/exception was added for these three; they are maintenance notices, not
  published runtime vulnerabilities, and no exploitation is established.
- Therefore the dependency gate is **FAIL, not PASS**. Older "dependency policy
  passes" sentences and logs are historical. `deny.toml` keeps its earlier reasoned
  ignores (paste/GTK/fxhash); none covers these three notices.
- Log: `target/checks/audit-20261019/dependencies.log`. Next work: a compatible
  upstream MPRIS/ZBus6 upgrade or a reviewed minimal patch, then real D-Bus tests.
  Do not silence the gate with an ignore.

## 5. Open items this documentation pass cannot close

AUD-05/06 (cold offline launch, origin-scoped namespace), AUD-09 (search/stats remain
measured costs), AUD-10/14–AUD-20
and AUD-19/20 remain open. This reconciliation fixed stale documentation only
(AUD-10/16 subset, AUD-17 framing, AUD-20 wording); it added no red test and fixed
no code defect. See the [audit](PLANS/2026-10-19/01_AUDIT.md) and
[repair plan](PLANS/2026-10-19/02_REPAIR_PLAN.md).

## 6. Evidence links

- [PROGRESS.md](PROGRESS.md) — chronological executed checkpoints and the
  2026-10-19 planning audit entry.
- [QA_EVIDENCE.md](QA_EVIDENCE.md) — package/runtime/native evidence tables and the
  golden-scenario coverage map (G14/G15 PARTIAL).
- [PERFORMANCE.md](PERFORMANCE.md) — measured benchmarks with hardware/scope caveats.
- [PLATFORM_MATRIX.md](PLATFORM_MATRIX.md) — per-target status;0.12.6 current,
  earlier rows historical.
- [NATIVE_CLIENT_CONTRACTS.md](NATIVE_CLIENT_CONTRACTS.md) — native Linux contract,
  portable package, offline copies, saved session, folders.
- [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) — unsupported/unverified list.
- [RESILIENT_SCALE_PLAN.md](RESILIENT_SCALE_PLAN.md) — opt-in large-hub proposal,
  not current architecture.
- [PLANS/2026-10-19/01_AUDIT.md](PLANS/2026-10-19/01_AUDIT.md) — dated audit record
  (do not edit).

## 7. What this page does not establish

No implementation verification follows from it: no code/test/runtime/browser/native
execution is claimed for this docs pass, no audit defect is marked fixed, no
dependency PASS is claimed, and no design checkpoint is described as implemented.
Source behavior here is pinned to the audited commit; later commits and any
in-progress uncommitted changes must be re-indexed when they land. This docs pass
made no commit.
