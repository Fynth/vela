# Public catalog0.11 source integration

Current working-tree source; immutable package acceptance is separate.200 Rust tests, strict native/all-target+WASM Clippy, release native/WASM,
fmt,22 Python, JS syntax/diff and locked dependency policy passed.20 real clean
restarts and4 independent worker faults passed. `target/checks/publication-*` holds
commands/logs; the failed NUL projection run is preserved separately from corrected
whole-workspace evidence. Source Chromium39/39 passed in2.5m, licensed live import; screenshot/evidence paths
are in PROGRESS. No native/public-TLS/ActivityPub compatibility claim.

| Feature | Exact paths | Latest verified source | Evidence command | Result | Next | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| Origin/private publication/independent grants | core/network.rs; media/publications.rs; domain/network.rs; migration0011; server/network.rs | Working tree | cargo test --locked --workspace -- --test-threads=2:200 | PASS | Exact package/regressions | main/media |
| Two independent origins + hostile peer | server/src/network/tests.rs | Working tree | Same Rust run: actual B HTTP preview calls actual A HTTP, Range/HEAD/revoke, slow/oversized/spoofed identity/grants/IDs | PASS for read-only profile | Standards signatures/replay/blocks remain NOT_RUN | main |
| Controlled DNS/redirects and stream capacity | imports/egress.rs; server/network/tests.rs | Working tree | Real two-listener no-dial redirect controls, barrier cancellation retaining4DNS slots,8 public bodies drop/reserved owner capacity | PASS | Public load measurements | main/reviewers |
| Network UI/source guards | web/network.rs; tests/browser/music.spec.mjs | Working tree | Chromium39/39 with live import, PN01/PN02; main inspected Network1440dark/320light, no overflow | PASS for source Chromium | Exact packaged two-engine runs | web/main |
| Same-build persistent user service | scripts/install-user-service.py; tests/test_user_service.py; docs/USER_SERVICE.md | Working tree + frozen902dd47 runtime |22 Python incl13 installer; actual --replace-transient --install and --dry-run recheck | PASS | Preserve pinned upgrade process | devops/main |

Independent readonly QA signed off the implemented permission/projection/peer/DNS/
UI boundaries and checked retained logs. No release-blocking defect found; exact
package/two-engine acceptance still follows. Private playlist sharing remains
metadata-only even though separate public publication now has explicit audio grants.

Latest mandate mapping: G01/G24 additive10→11 legacy-row and backup safeguards;
G11 independent origin/catalog + permitted original public stream subset; G14
bounded hostile/slow/SSRF/metadata subset (forged actors/signature replay NOT_RUN);
G15 removal/stream-revocation subset (no federated recommendation/cache);
G22 declaration/provenance/unknown-rights/private defaults subset. PN01/PN02 are
new named acceptance; preserve all EXP/PLAY/PLNAV/RES/NET/AUTH/NP/PC/history/DSP
regressions. No entire golden scenario becomes green from a partial subset.

# Verified0.10 package/runtime checkpoint

Application/source 902dd47f9dca2d9b6f405cd50c21bf4dab5a73d8. Exact extracted binary passed37Chromium +37Firefox
with live licensed import; static bytes match frozen package before/after runs.
Screenshots8 size/theme combinations per engine; main inspected desktop/phone.
Packaged20clean restarts +4worker DB faults, default/schema10/assets/SIGTERM,
all preserved release checksums and9frozen hashes pass. Fresh stopped0.9backup
restored with every SQL row/media hash; original20tracks/assets and20analysis
rows preserved after9→10, quick_check ok/FK0, no auto consent/exclusions.
Current supervised loopback0.10 PID34675 uses original data,0restarts.
Source gates176Rust/strict native+WASM/fmt/9Python/JS/deny pass. Evidence paths
and exact limits are in PROGRESS; no native/Safari/screen-reader/Docker claim.

# Personal discovery0.10 source evidence

Source/application902dd47; final archive/runtime evidence is recorded above and in PROGRESS.

| Feature | Exact paths | Verified source | Executed evidence | Status | Next / owner |
| --- | --- | --- | --- | --- | --- |
| Private mix/service/feedback | crates/media/src/recommendations.rs; apps/server/src/recommendations.rs; migrations/0010_recommendation_exclusions.sql |902dd47 |176 Rust tests, strict native/all-target+WASM Clippy, real TCP, legacy9upgrade, owner/consent/retention/concurrency | PASS | Preserve gates / main |
| Home/root playback/exclusions | apps/web/src/personal_mixes.rs; tests/browser/music.spec.mjs |902dd47 |Chromium37; G07 rendered actual rotation/erase; PC01 real playback/hide/undo/restore/export/catalog race; PC02 held read cancellation | PASS for source run | Preserve exact package two-engine gates / main |
| SQL performance | media/src/recommendations.rs; examples/benchmark_recommendations.rs | SHA256 in final resource JSON |10k/100k/1m actual release + baseline projection/plan equality; raw samples/hardware/scope in PERFORMANCE | PASS within recorded scope | Tied timestamps/dense history and calibrated quality / main |
| Visual/platform scope | tests/browser screenshots; PLATFORM_MATRIX | Final screenshots pending | Source shots inspected; corrected capture for CSS transition settling; Docker daemon and in-app browser absent | PASS for recorded browser scope | Native/Safari/screen-reader remain unverified / main |

Preserved failed runs remain FAILED, not passing evidence. Original20tracks and20analysis
rows were backed up/restored; frozen0.9 runtime restored separately from0.10 candidate.
Private defaults were unchanged. Full native clients and public federation remain NOT_RUN.

# Current mandate evidence

The latest master mandate has24 golden scenarios. Historical G01–G14 numbers in older docs/tests belong to the earlier UX brief and do not automatically satisfy the new meanings. Preserve named regression IDs EXP/PLAY/PLNAV/RES/NET/AUTH/NP. New history uses G07-private-history, DSP G08-audio-analysis and radio G09-track-radio.

| Feature | Exact paths | Latest verified source | Executed evidence | Status | Next / owner |
| --- | --- | --- | --- | --- | --- |
| Baseline regression | workspace; tests/browser/music.spec.mjs | f6cc845 / app ac31d74 | cargo test --locked --workspace -- --test-threads=2:110; real Chromium33 with licensed live import | PASS | Repeat after integration / main |
| Data safeguard | scripts/backup.py; data; target/upgrade-backups | f6cc845 | New before-listening-20261017T085256Z.tar.gz restored in disposable dir; schema7,1user,20tracks/assets,1artwork,0playlists; quick_check ok, FK0; listening-backup.json | PASS | Preserve backup and frozenruntime / main |
| Private history service/API | core/src/listening.rs; domain/src/listening.rs; migration0008; server/src/listening.rs |bc50564 | Full workspace157; real TCP consent/CAS/dedup/ownership/retention/export/delete; final Chromium35 + Firefox35 including actual playback/RepeatOne/erasure/stale export | PASS | Preserve regressions, add personal ranking / main+core+web |
| Audible clock/outbox/UI | web/src/audible.rs,listening.rs,browser_player.rs,radio.rs |bc50564 | Strict native/WASM Clippy, final real-browser playback/muting/history-to-radio, bounded queue/session checks | PASS | Device registry/offline remain separate / main+web |
| DSP/worker/radio | media/src/analysis.rs,analysis_service.rs,discovery.rs; migration0009; server/src/analysis.rs |bc50564 |10 extractor/9 worker/7 radio tests + real TCP worker+originals+ACL + browser G08/G09;20 benchmark runs, immutable hashes | PASS | Personal recurring mixes and scale/quality evaluation / main+DSP+core |
| Process lifecycle | server/src/main.rs; scripts/verify_shutdown.py |bc50564 |20 real SIGTERM/restarts; four independent worker DB failures stop process | PASS | Preserve regression / main |
| Browser tool availability | Browser skill runtime | This session | getForUrl returned unavailable; documented browsers.list returned[] | BLOCKED in-app | Existing isolated Playwright harness is available / main |
| Native/federation | PLATFORM_MATRIX; FEDERATION_PROTOCOL; NETWORK_RIGHTS | No implementation | Host/tool probes only; no two-instance or native audio test | NOT_RUN | Implement independent milestones; no completion claim / main |

Logs remain in ignored target/checks/listening-*. Do not delete target: it also holds original private backups, release archives and active frozen runtime snapshots. Final commands/counts/source commit/runtime and remaining limitations are recorded in PROGRESS; preserved failed attempts are not passing runs.


Exact local package/runtime checkpoint: source bc50564979a9bcead7ed6667d6ae673df374e265; frozen extracted binary passed another35 Chromium/35 Firefox scenarios, setup/schema9/assets/SIGTERM and all release checksums. Original data was backed up/restored/logically compared, then upgraded in place without reset:1 user,20 tracks/assets,1 artwork,0 playlists; integrity/FK pass. Supervised loopback0.9.0 PID114613, eight served asset hashes match, no auto consent/analysis. Evidence: listening-upgrade-{backup,runtime}.json and listening-upgrade.log. Reinspect runtime on resume.
