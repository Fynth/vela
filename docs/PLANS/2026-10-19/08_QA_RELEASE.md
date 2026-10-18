# Как проверять исправления и будущий продукт

## 1. Сегодняшние и исторические проверки

Сегодня: source05b4c37; fmt/native strict Clippy/WASM strict Clippy PASS;
workspace244passed/0failed/2ignored; Python22passed. Логи `target/checks/audit-20261019/`,
`baseline.json` содержит команды-сводки/health/aggregate counts. Existing-data чтение
не меняло session/consent/publication. No schema migration или service restart.

Dependency policy свежего аудита **FAILED**: три INFO Unmaintained notices в
MPRIS/ZBus, описанные в AUD-20. Остальные bans/licenses/sources PASS. Это не три
подтверждённых runtime vulnerabilities; исторический PASS не замещает новый FAIL.

Исторически: PROGRESS/QA_EVIDENCE фиксируют packaged Chromium/Firefox41pass+1skip
и реальные native/Linux MPRIS/offline journeys. Их нельзя выдавать за сегодняшний
full rerun. G14 forged/replay и G15 remote recommendation остаются PARTIAL:
проверенный subset не заполняет отсутствующую реализацию.

Design board render — собственный статический HTML в изолированном Playwright,
после недоступности встроенного Browser; это visual artifact, не приложение E2E.

## 2. Матрица новых repair regressions

| ID | Сценарий | Уровень и fixture | Критерий |
|---|---|---|---|
| REG-AUD-01 | Old login/resume after held keyring await | Unit coordinator + actual native window, fake controlled keyring adapter | New account/epoch/source остаются неизменны |
| REG-AUD-02 | Keyring tool hangs/locked/fails | Owned child process/real Secret Service opt-in | Immediate local logout, bounded cleanup, no GTK freeze |
| REG-AUD-03 |250MiB offline hash/slow IO/cancel | Blocking reader fixture+UI heartbeat/null output | UI controls отвечают, late result closes file |
| REG-AUD-04 | Resume offline/503/429/401 | Real TCP fixture server | Only actual expired session clears expected saved generation |
| REG-AUD-05 | Cold offline launch | Save lawful audio→close→stop server→fresh process | No HTTP needed; artwork/seek/next/local playback |
| REG-AUD-06 | Same UUIDs on A/B | Two isolated migrated servers/cache roots | Zero cross-origin copy/queue/manifest exposure |
| REG-AUD-07 | Remove noncurrent/current offline | Real prepared audio/native UI | Noncurrent keeps output; current clear is explicit |
| REG-AUD-08 | Delayed remove/load across relogin | Held actual storage operation | Old receipt cannot overwrite current UI |
| REG-AUD-09 | stats/search scale and contention |100k/1m synthetic metadata + real audio subset | Scoped correctness, raw p50/p95/p99, no streaming regressions |
| REG-AUD-11 | Malformed/huge/slow keyring output | Child process bound/control | Deadline/size enforced before unbounded accumulation, tokens redacted |
| REG-AUD-12 | Quota replace/orphan/two processes/diskfull | Disposable device dirs + fault points | No budget bypass/data loss/corrupt published pair |
| REG-AUD-13 | History GET under active write | Real SQLite WAL transactions | Read snapshot + cutoff, delete/revision semantics intact |
| REG-AUD-15 | Web successful peer page2 and revoke | Two real controlled peers in browser | Forwarded cursor, no duplicate/leaked/actionable stale rows |

Не добавлять flaky wall-clock performance assertions в unit tests. Test clock,
barriers и controlled fixtures для ordering; latency — отдельный benchmark report.

## 3. Golden G01–G24: что нужно на завершение roadmap

| IDs | Gate | Доказательство |
|---|---|---|
| G01/G24 | Data survives migrate/update/restore/fault | Row ownership/grants/revisions equality, file hashes, SIGTERM/crash |
| G02/G03 | Upload/import/metadata/play/range | Real lawful FLAC/MP3/artwork and authorized remote file, cancel/SSRF |
| G04/G05 | Home/queue/Now Playing/navigation | Real data, one source, actual progressing media clock, duplicates/races |
| G06 | Collaboration | Two users/CAS/conflict/idempotency/revoke; metadata no audio grant |
| G07 | Private history | Consent account+device, meaningful clock, epoch/export/delete/late outbox |
| G08/G09 | DSP/radio | Golden signal vectors + rights-cleared real songs, no stream-path decode |
| G10 | Onboarding/mixes | Sparse library; real taste; materialized schedule/coverage; no fake Daily |
| G11/G12/G13 | Multi-origin discovery/search/playlist |3instances, progressive results, IDs/cursors/partial failure/no unauthorized copy |
| G14 | Hostile federation | SSRF, signatures/key rotation/replay/forged actor/huge recursive data |
| G15/G22 | Rights/provenance/recommendations/revoke | Remote accessible candidate, expired rights/tombstone/cache cannot grant audio |
| G16 | Android | APK/emulator AND physical lock/background/headset/focus/airplane evidence |
| G17 | iOS | macOS compile/simulator/device AVAudioSession/remote/offline; blocked if absent |
| G18 | Desktop platforms | Install/run/output/system controls per OS, not server binary proof |
| G19 | Handoff | Two real clients accepted transfer, accurate progress, source stops after accept |
| G20 | Rooms | Membership/commands/queue/clock under latency and revoked grants |
| G21 | Load | Catalog/search/stream under real imports/scan/DSP with hardware/scope |
| G23 | Access and language | Keyboard, actual SR, ru/en,200%type,320px/theme/reduced motion |

Preserve EXP/PLAY/PLNAV/RES/NET/AUTH/NP/PC regression IDs. New tests use REG-AUD/UX/DSP/
FED prefixes; не переиспользовать исторический G номер с новым смыслом.

## 4. Security authorization matrix

Principals: owner, member, collaborator-view, collaborator-edit, invited, anonymous,
remote actor, blocked principal, app administrator. Entities: tracks/assets/artwork,
playlists/entries/folders, history/preferences/devices, analysis/vectors, recommendations,
publications/provenance, remote cache, offline manifests, rooms, exports/jobs.
Actions: list/search/read/aggregate/stream/HEAD/range/download/edit/share/delete/revoke.
Default deny. Same physical bytes/same UUID/same title/metadata permission не grant.

Для каждого URL surface: import/redirect/WebFinger/artwork/avatar/remote catalog/
proxy/feed/object store — closed scheme/port/address policy, DNS pin/rebinding,
private/metadata IP, response/decompression/recursion caps, timeout/cancel/admission.
Trusted local storage connectors не ослабляют публичный importer policy.

## 5. Performance methodology

- Fixtures10k/100k/1m metadata и bounded real codec files;1000queue/large playlists,
 50/500/10000qualified history, equal-timestamp ties, foreign owners, common/rare/miss.
- Hardware/OS/toolchain/SQLite/build SHA/CPU governor/RAM/SSD, background activity,
  cold/warm, concurrency, exact query/network path, codec/bytes, sustained duration.
- API p50/p95/p99/errors/throughput; native/wav audible start отдельно от HTTP TTFA;
  seek stability, cancellation bytes, decoder CPU, RSS idle/load, cache/DSP interference.
- Frontend: startup interactable/route frame/input latency, long list scroll/focus,
  WASM bundle/font/artwork transfer, render counts, theme/dialog overhead.
- Repeat≥3runs при сравнимой нагрузке; raw samples/summary/CI и измеренное изменение.
-10kRPS/10klisteners — не default acceptance personal profile без инфраструктурного
  обоснования. Нельзя подменять isolated SQL μbench реальным audio workload.

## 6. Visual/accessibility matrix

320×568,375×812,430×932,768×1024,1024×768,1366×768,1920×1080,3440×1440;
light/dark/system,100/200%text, keyboard/focus, reduced motion, long mixed-script/RTL,
empty/large/loading/error/offline/denied/conflict и active player. Covers square;
body не имеет horizontal overflow; bottom controls не заслонены safe areas/dialogs.

Automated contrast/axe не равно screen-reader certification. Выполнить ручные
NVDA/VoiceOver/TalkBack по доступным targets, reading order/announcements/menus/
drag alternatives/seek. Изображения вручную просмотреть после capture.

## 7. Commands и разграничение gates

```bash
cargo fmt --all -- --check
CARGO_BUILD_JOBS=2 cargo clippy --locked --workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=2 cargo test --locked --workspace -- --test-threads=2
CARGO_BUILD_JOBS=2 cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings
CARGO_BUILD_JOBS=2 cargo build --locked --release -p vela-server
scripts/build-web.sh
python3 -m unittest discover -s tests -p 'test_*.py'
```

Далее по изменениям: isolated Chromium/Firefox; native driver с owned null sink
и private DBus/keyring namespace; package extraction/hash/assets/ldd; dependency
deny/notices; shutdown/worker faults; backup fixture restore; actual Docker/ARM64.
Не выполнять все hardware tests на машине без hardware и не скрывать skip.
No competing full Cargo builds, не `cargo clean`.

## 8. Релизный checklist

1. Required supported platforms определены, BLOCKED не превращён вpass.
2. P0 defects закрыты red→green и reviewed. P1 known limitations опубликованы честно.
3. Old schema/source/API/offline manifest compatibility, privacy defaults preserved.
4. Immutable source/binary/assets manifest, exact package browser/native checks.
5. Quota/diskfull/interrupted write/recovery/upgrade/restore, exports no secrets.
6. Dependency/security/licenses/font+art provenance и per-platform runtime libs.
7. Runtime upgrade отдельно от compile; fresh private backup and validated restore.
8. Health/readiness/graceful drain/worker supervision/rollback-ready package.
9. Release notes объясняют user value, changed behavior, migration and limitations.
10. Signing/app stores/public deployment/external publication только с явным разрешением.
