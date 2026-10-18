# Приёмка и измерения: доказать улучшение, а не сменить надписи

## 1. Что проверено в этом проходе

Только статический разбор актуальных source/docs, просмотр двух присланных изображений,
чтение текущего unit/process состояния и первичных технических источников. Новые
Cargo, native-window, browser, audio-device, benchmark или mobile проверки **не запускались**.
Проверка ссылок/структуры этого MD-пакета не является проверкой приложения.

Всё ниже — задания будущим исполнителям. Existing evidence в PROGRESS/QA_EVIDENCE
сохраняет свою дату/build/scope; не копировать старые 244/56/22 PASS как новый запуск.

## 2. Изолированный стенд

- Собственный data directory, DB, import staging, private config и ephemeral ports.
  Не заменять production user profile/test DB реальной библиотекой.
- Два аккаунта одного сервера, второй независимый origin с теми же display names и
  намеренно совпадающими local object IDs; третий клиент не имеет permissions.
- Собственные синтетические FLAC/MP3/WAV и разрешённые реальные fixtures с metadata,
  embedded artwork/lyrics там, где применимо. Проверить decode, не просто расширение.
- Короткий track для EOF, длинный/large file для progressive/seek, VBR MP3, FLAC с
  seek metadata, контейнер с metadata at end, damaged/truncated и unsupported file.
- Network fault layer умеет throttle, задерживать headers/body/ACK, рвать connection,
  менять validator и выдавать deny. Счётчики на стороне origin измеряют реальные bytes.
- Native output сначала owned null/loopback sink. Physical listening/route/headset
  отдельно, без неожиданного звука на колонках пользователя.
- Cache cold/warm/pinned/partial/corrupt сценарии разделены. OS page cache не выдаётся
  за Vela cache hit. Один process restart не доказывает cold boot offline.

## 3. Измеряемые точки playback

Для одного selection записывать monotonic timestamps: user command → resolver ready →
request start/headers/first byte → startup buffer ready → decoder first frame → output
first frame → audible loopback observation. Состояния pause/seek/next не смешивать.

Счётчики: full/range/revalidation requests, transferred bytes, disk read/write bytes,
cache hit type, ready-next fraction, canceled jobs, queue plan/source generations,
UI thread stalls, CPU/RSS, pinned/cache/staging quota. Удалить secrets/path/query
из logs; track/session IDs псевдонимизировать внутри fixture evidence.

### Начальные цели для оценки, не результаты

Зафиксировать hardware/codec/output buffer/fixture и условия: idle Linux desktop,
авторизованный loopback server, известный duration, без analysis/import нагрузки.
После baseline пересмотреть цели обоснованно, не подгонять методику под красивое число.

| Метрика | Предварительная цель | Что не следует из успеха |
|---|---|---|
| Warm valid cache Play→first output frame p95 | До300ms на согласованном reference host | Не physical Bluetooth latency и не universal SLO |
| Cached seek→новый output position p95 | До300ms на согласованном fixture | Не sample-accurate seek любого codec |
| Progressive first play | Начать до EOF body; provisional p95 до1s на loopback | Не время старта на любом WAN |
| A→B→A при complete warm cache | 0 повторных полных audio bodies A | Небольшой auth/revalidation request возможен |
| Ready next переход | Без fresh full download; provisional gap до150ms | Не gapless/sample-perfect claim |
| Navigation while playing | 0 source replacements из-за route | UI latency всё равно проверяется отдельно |
| Handoff position error LAN | Начальная цель до500ms после start ACK | Не zero-gap/zero-overlap при partition |
| Speculative work | Не более1 next fetch по initial policy | Shared in-flight current/save не дублирует body |

Оценивать p50/p95/p99 и sample count; не называть p99 надёжным на десяти повторах.
Для exploratory smoke можно30 прогонов с честной пометкой low confidence; для gate
собрать достаточную выборку и raw redacted timings. Отдельно указать warm-up и outliers.

## 4. Layout acceptance

LAYOUT-01–08 определены в [window plan](01_HYPRLAND_AND_LAYOUT.md). Их воспроизведение
обязательно в normal Hyprland tile с реальными app/window IDs и backend. Размеры
в logical px и physical output px фиксируются отдельно. Нельзя принудительно сделать
окно floating и объявить исходную проблему решённой.

Минимальные профили:640×480,1280×840,1920×1080,3440×1440; для shared responsive UI
320×568,390×844,768×1024. Scale1/1.25/1.5/2 по доступным monitors. Выполнять dark/light,
200%text, reduced motion, narrow+short. Недоступная комбинация — NOT_RUN, не PASS.

Screenshots: старт, playlist empty/populated, Home, queue open, modal, playing/paused,
error, last row. Дополнять element bounds assertions: screenshot не ловит offscreen
tab order, player overlap или невидимый inaccessible menu. Focus walkthrough — отдельно.

## 5. Cache / progressive / prefetch

| Тест | Действие и обязательное наблюдение |
|---|---|
| CACHE-01 | Large throttled MP3/FLAC: output advancing до origin EOF; partial integrity честна |
| CACHE-02 | A→B→A: body counter A не удвоен, validated cache hit зарегистрирован |
| CACHE-03 | Library Play того же pinned track: локальный путь, не только Downloads-specific кнопка |
| CACHE-04 | Seek назад в complete cached range: нет повторных bytes этого диапазона |
| CACHE-05 | Два occurrence одного asset: одна byte copy, правильные разные queue items |
| CACHE-06 | Account/origin collision: ни audio, ни artwork/manifest предыдущего scope не выданы |
| CACHE-07 | Revoke online stream grant, затем cache hit: новый доступ запрещён согласно contract |
| CACHE-08 | Disk full/eviction/corruption/cancel: active/pinned healthy copies не повреждены |
| CACHE-09 | ETag/size меняются между ranges: representations не склеены |
| CACHE-10 | Diagnostics соответствуют реальным body/disk counters и не содержат secrets |
| PREF-01 | Во время A запрашивается actual next B по policy, а не random suggestion |
| PREF-02 | EOF переключает на prepared B без второго output и fresh full GET |
| PREF-03 | Play next C меняет planned next, obsolete B не выигрывает callback race |
| PREF-04 | Shuffle preview соответствует Next; возврат и forward deck стабильны |
| PREF-05 | Queue/account/route lifetime отменяют obsolete tasks/leases, stale commit невозможен |
| PREF-06 | Repeat one/all использует cache, manual Next отличается от EOF |
| PREF-07 | Shared flight для duplicate/current/save; отмена одного не ломает другого |
| PREF-08 | При throttling current получает ресурсы раньше speculation |
| PREF-09 | Denied/metadata-only/revoked следующий не получает media bytes |
| PREF-10 | Local/offline/remote EOF/Next/Previous/MPRIS имеют одинаковый command contract |
| PREF-11 | Prefetch/seek/download сами по себе не создают qualified listen |
| PREF-12 | Native output observations и network counts подтверждают UI Ready, не mock state |

Особые codec проверки: MP3 VBR duration/seek, encoder delay/padding, FLAC sample-rate
switch, AAC/MP4 seek metadata и Opus/Vorbis по engine capabilities. Gapless/crossfade
принимаются только waveform/output measurement, не отсутствием visible spinner.

## 6. Синхронизация и handoff

| Тест | Проверка |
|---|---|
| SYNC-01 | Device registry owner-scoped, stable ID, expiry online status |
| SYNC-02 | Queue occurrences/order/modes воспроизводятся на втором клиенте |
| SYNC-03 | Restore paused с правильным track/position, без autoplay при login |
| SYNC-04 | Stale checkpoint не откатывает свежую session epoch |
| SYNC-05 | Concurrent edits обрабатываются CAS/rebase, без last-write whole-queue loss |
| SYNC-06 | Reconnect snapshot+events восстанавливают revision gap |
| SYNC-07 | Duplicate delivery/command ID даёт одно observable изменение |
| SYNC-08 | Offline journal имеет bounded size, explicit conflicts и replay receipts |
| SYNC-09 | Third account/device revoke не видит state и не командует output |
| SYNC-10 | Sync off/history off/presence off независимы и не утрачивают local queue |
| HANDOFF-01 | A plays→B READY→A pause ACK→B actual start ACK→commit |
| HANDOFF-02 | Правильные origin/occurrence/position/modes и принятый target |
| HANDOFF-03 | B denied/unsupported/no network: A не остановлен |
| HANDOFF-04 | B autoplay blocked: gesture до паузы A, понятный target prompt |
| HANDOFF-05 | B fails после A pause: bounded rollback только при актуальном intent |
| HANDOFF-06 | Пользователь выбрал C: obsolete handoff не возвращает A/B |
| HANDOFF-07 | Lost ACK/повтор commands не дают двойной Play/Next |
| HANDOFF-08 | Partition/timeouts показывают uncertain status и безопасную policy |
| HANDOFF-09 | Independent play на B не захватывает A автоматически |
| HANDOFF-10 | Измерены prepare/start latency, gap/overlap/position error для каждого target |

Первая приёмка двумя real browser/native clients полезна, но G19 phone↔desktop
закрывается только с соответствующим реальным phone client.

## 7. Identity, room и mobile checks

### ID-01–10

1. Один issuer+subject связывается с двумя устройствами и разными local memberships.
2. Другой issuer с тем же subject/name/email не получает чужие права.
3. PKCE/state/nonce/issuer/audience/expiry/replayed code negatives.
4. Native system browser cancel/late callback не заменяет активный account.
5. Private profile и no implicit listening/history/cloud media upload после login.
6. Provider outage не удаляет offline и не делает fake command success.
7. Music origin outage отличим от identity outage.
8. Unlink/relink/revoke lost device не удаляет оригинальный local account/library.
9. Portable export без passwords/tokens/stream URLs, recovery policy проверена.
10. Local-only setup/admin/login и streaming работают при отсутствующем Vela ID service.

### ROOM-01–12

Create/invite/join; expired/revoked invite; roles; concurrent queue CAS; distinct
participant visibility; independently authorized sources; missing source; drift/clock
outliers; disconnect/rejoin snapshot; block/kick; host leave/end; сохранение personal
session и отсутствие public history consent. Для каждого из12 случаев отдельный test ID
с expected state и event trace, не один happy-path video.

### MOBILE-01–08 для каждого target

Install/launch; connect/real stream/seek; lock+background; media buttons/headset/BT;
interruptions/route change; offline cold process restart; sync/handoff; secure store
logout/revoke. Android emulator, Android physical, iOS simulator и iOS physical —
разные evidence строки. Подписанный APK/IPA сам по себе не PASS этих проверок.

## 8. Связь с текущим G01–G24 mandate

В этой работе **ни один G-тест не перезапускался**. Столбец «опора» ссылается на
исторический QA ledger, а не присваивает новый VERIFIED статус.

| Golden ID | Опора/ограничение, которое надо сохранить | Новое требование этого пакета |
|---|---|---|
| G01 | Исторические миграции/ownership/hash backup evidence | Origin cache и identity mappings additive, fixture restore |
| G02 | Реальные web upload/artwork/Range journeys | Native artwork/progressive и consistent Library Play |
| G03 | Direct imports/jobs/SSRF tested subset | Не поломать import concurrent с current/preload |
| G04 | Owned signals Home есть | Новый musical Home без fake sections |
| G05 | EXP/PLAY/NP root/browser races | Cache/source state, native parity, route continuity |
| G06 | CAS/playlists/view-only/revoke есть | Новый UX не ослабляет permissions, realtime отдельно |
| G07 | Consent/exactly-once/export/delete scope | Prefetch/sync не коллекционируют listens |
| G08 | Synthetic/DSP/versioned scope | No stream blocking, loudness/tempo claims по extractor evidence |
| G09 | Local explainable radio scope | Radio queue доступна actual resolver, feedback свежий |
| G10 | PARTIAL: нет scheduled full mixes | Sparse library/onboarding и durable refresh при реализации |
| G11 | Two loopback/public TLS debug subset | Public Internet/independent host и protocol scope отдельно |
| G12 | PARTIAL: per-peer search, не full partial UI | Progressive bounded search и origin-aware UI |
| G13 | Cross-origin playlist refs NOT_RUN | Stored refs, disconnect/missing/recover без copying |
| G14 | PARTIAL hostile peer/SSRF/object scope | Signature/replay и новые cloud/WS attacks реально проверить |
| G15 | PARTIAL revocation, нет remote recommender/cache leak proof | Cache/grant isolation и tombstones при новых sources |
| G16 | Android toolchain/device evidence отсутствует | Media3 background/offline, actual APK/device |
| G17 | Apple environment BLOCKED исторически | Xcode/simulator/physical по доступной среде, не Linux PASS |
| G18 | Linux package subset | Hyprland normal tile + каждый native OS separately |
| G19 | Handoff NOT_RUN | SYNC/HANDOFF, затем phone↔desktop |
| G20 | Rooms NOT_RUN | ROOM with authorization/drift/reconnect |
| G21 | Исторические metadata/rotation benchmarks | Cache/TTFA/ready-next + stream under scan/import/DSP |
| G22 | Explicit provenance/publication checks | Правомерность online/offline/room sources не ослаблена |
| G23 | PARTIAL: browser keyboard/theme/320, не full reader/i18n | Actual native layouts, Russian, screen readers/text scales |
| G24 | Historical backup/restart/failure evidence | Cache recovery, account migration и exact installed update |

Не переименовывать старые EXP/PLAY/PLNAV/RES/NET/AUTH/NP/PC/history/DSP tests и не
использовать раннюю G01–G14 нумерацию из старого UX brief для нового OMEGA-mandate.

## 9. Команды будущего code gate

После настоящих изменений, одним координируемым build stream: `cargo fmt --all -- --check`;
`CARGO_BUILD_JOBS=2 cargo clippy --locked --workspace --all-targets -- -D warnings`;
`CARGO_BUILD_JOBS=2 cargo test --locked --workspace -- --test-threads=2`;
`CARGO_BUILD_JOBS=2 cargo clippy --locked -p vela-web --target wasm32-unknown-unknown -- -D warnings`;
валидный build web/desktop и соответствующего server; существующая Python suite,
dependency/license policy и точный package verification.

Browser: использовать существующий `tests/browser` Playwright setup и реальные
fixture servers, Chromium/Firefox; Safari/WebKit scope дополнительно. Native: текущий
`scripts/verify_native_client.py` с актуально собранными driver/server и отдельной
evidence directory, включая opt-in audio/session cases при доступной среде.
Этот script сейчас force-X11; Hyprland Wayland acceptance требует отдельного режима,
а не ложного расширения старого результата. Package verifier `--launch` проверяет
launch, но не автоматически hardware audio или все вышеперечисленные сценарии.

Это список будущих checks; **эти команды в данном docs-only проходе не выполнялись**.

## 10. Release evidence

Для каждой feature: source HEAD, package SHA, runtime binary/assets identity,
OS/backend/scale/codec/device, command, raw redacted log, screenshot/output sample,
expected/actual, PASS/FAIL/NOT_RUN/BLOCKED, failures до исправления и limits.
Не удалять первый failed run после зелёного повторения. Фраза «всё работает» без
этой информации не закрывает исходную жалобу пользователя.
