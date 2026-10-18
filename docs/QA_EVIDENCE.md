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
