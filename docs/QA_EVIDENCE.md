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
