# Аудит существующего Vela

Основание: HEAD05b4c37, файлы на диске, повторные Rust/Python-проверки,
read-only health/SQLite, сохранённые измерения и изображения fixture-окружений.
Это предметный аудит ключевых путей, не формальная проверка всех строк или независимый пентест.
Новые дефекты ниже пока не исправлены. Никакого сброса библиотеки/миграции не было.

## 1. Карта настоящей реализации

| Область | Реальные файлы | Состояние |
|---|---|---|
| Восемь workspace members | `Cargo.toml`, `Cargo.lock` | Rust2024, сервер default-member, Dioxus web/desktop feature split |
| Auth/config/database | `crates/core/src/{auth,config,database}.rs` | Argon2id, opaque hashed sessions, WAL, ограничения, миграции |
| HTTP | `apps/server/src/{lib,network,listening,analysis,recommendations}.rs` | Auth/CSRF/ownership, Range, существующие read/write APIs |
| Каталог/FTS/ingest | `crates/media/src/{lib,catalog,filters,search,metadata,validation}.rs` | Владельческие страницы, реальные файлы, artwork, теги |
| Плейлисты/папки | `crates/core/src/playlists.rs`, `migrations/0012_playlist_folders.sql` | CAS, idempotency, локальные collaborator grants, folders |
| История | `core/src/listening.rs`, `web/src/{audible,listening}.rs` | Account+device consent, export/delete, meaningful events |
| DSP/рекомендации | `media/src/{analysis,analysis_service,discovery,recommendations}.rs` | spectral-4, durable worker, описательные tempo/chroma/key, локальные подборки |
| Импорт/сканирование | `crates/{imports,scanner}/src/` | Контролируемый egress, durable jobs, безопасные read-only snapshots |
| Web | `apps/web/src/{main,shell,experience,browser_player,playback,session,now_playing}.rs` | Один HTMLMediaElement, Home/Search/queue/recovery/Now Playing |
| Desktop | `apps/desktop/src/{ui,client,audio,mpris}.rs` | Настоящий Linux Dioxus, Rodio/CPAL, MPRIS, не HTML audio |
| Native offline/credentials | `desktop/src/{offline,session_store}.rs` | Явные копии с BLAKE3, Secret Service opt-in; важные дефекты ниже |
| Публичная сеть | `media/src/publications.rs`, `server/src/network*`, `desktop/src/{remote,peers}.rs` | Объявленные права, отдельные grants, peer preview/search/cache; не полная социальная федерация |
| Выпуск | `scripts/{backup,install-user-service,verify_*,package-*}.*`, `Dockerfile`, `compose.yaml` | Реальные локальные пакеты/backup; все платформы не проверены |

## 2. Десять наиболее важных проблем

P0 означает блокер следующего публичного клиентского выпуска; P1 — ближайшая
обязательная работа; P2 — развитие после надёжной основы. Это оценка влияния,
а не CVSS или утверждение о подтверждённой эксплуатации.

| ID | Приоритет / тип | Доказательство | Последствие | Обязательный новый тест |
|---|---|---|---|---|
| AUD-01 | P0 / дефект lifecycle | `desktop/src/ui.rs:login`, `resume_saved`: проверки ticket до HTTP, затем `spawn_blocking(session_store::save/clear).await` без повторной проверки перед `adopt_account`/`expired` | Позднее завершение keyring может принять отменённый вход или повлиять на уже новый аккаунт | REG-AUD-01: удержать save/clear, отменить/сменить аккаунт, отпустить старую операцию |
| AUD-02 | P0 / блокирующий logout | `ui.rs:sign_out` вызывает синхронный `session_store::clear()` **до** `clear_account`; в `session_store.rs` нет deadline subprocess | Зависший Secret Service блокирует UI и откладывает остановку музыки/локальный logout | REG-AUD-02: зависший fixture secret-tool; звук/доступ прекращаются сразу, UI отвечает |
| AUD-03 | P1 / блокировка UI | `ui.rs:start_offline_at` прямо вызывает `offline::prepare`; тот читает manifest и хэширует весь файл до 250MiB | Долгий freeze при Play/Next offline; отмена и управление запаздывают | REG-AUD-03: медленный reader/большой файл, UI heartbeat/отмена и смена аккаунта |
| AUD-04 | P0 / ошибочная потеря session | `ui.rs:resume_saved` очищает Secret Service при **любом** `Err`, включая timeout/offline/5xx | Временная потеря сети уничтожает валидный сохранённый вход | REG-AUD-04: offline/503 не удаляют; настоящий 401 обрабатывается отдельно |
| AUD-05 | P1 / отсутствующий основной workflow | Offline-каталог доступен после `adopt_account`, восстановление требует `/api/me` | Офлайн работает после входа и отключения сервера, но холодный запуск без сервера не завершён | REG-AUD-05: закрыть приложение, отключить сервер, запустить и проиграть сохранённое |
| AUD-06 | P0 перед multi-account/multi-origin / дефект идентичности | `offline.rs:data_root(account_id)` и Record хранят account UUID без origin/server identity | Два разных origin с одинаковым UUID (например, fork DB) попадают в один offline namespace | REG-AUD-06: A/B с одинаковыми user/track IDs получают разные локальные пространства |
| AUD-07 | P1 / ошибка очереди | `ui.rs:remove_offline` останавливает всю очередь при любом `offline_active`, не проверяя current track | Удаление другого скачанного трека обрывает текущую музыку | REG-AUD-07: удалить следующий/предыдущий, текущий source/position сохраняются |
| AUD-08 | P1 / freshness после записи | `ui.rs:remove_offline`: после первого await проверяется только user.id, после следующего `load.await` полного guard нет | Ответ старого удаления может заменить новый список/состояние после повторного входа тем же UUID | REG-AUD-08: задержать delete/reload, logout/login и сменить origin |
| AUD-09 | P1 / измеренная производительность | `media/src/lib.rs:stats`, `search.rs`, `target/checks/phase0/http-bench-100k-stats.json` | Исторический 100k p50 stats≈490ms/search≈156ms; лишняя работа на Home и поиск, конкурентная нагрузка не доказана | PERF-AUD-09: sparse/common/missing query и stats, cold/warm, p95/p99, owned/foreign |
| AUD-10 | P1 / документация и проверяемость | `KNOWN_LIMITATIONS`, `SPOTIFY_PARITY_MATRIX`, `PLATFORM_MATRIX`, `NATIVE_CLIENT_CONTRACTS` одновременно содержат устаревшие утверждения | Можно повторно сделать уже готовую функцию или ошибочно принять отсутствующую за готовую | DOC-AUD-10: current index + ссылки на source/commit/test/platform, отдельный historical archive |

AUD-01–AUD-08 подтверждены разбором реального control flow/ключей хранения.
Их новые воспроизводящие E2E **не исполнялись в этом аудите**: сначала добавить red test.
Для AUD-06 подтверждён коллидирующий namespace, а не удалённая эксплуатация:
нужны совпадающие ID/выбор соответствующего origin; случайные UUID обычно различны.

## 3. Дополнительные дефекты, риски и ограничения

### AUD-11 — keyring subprocess и секреты

`session_store.rs` использует `Command::output/status/wait` без deadline и ограничения
stdout до чтения (MAX_SECRET проверяется после output). `SavedSession` выводим через
`Debug` вместе с token. Действующего места логирования token не найдено: это риск
случайной утечки, а не установленная утечка. Нужны typed error, redacted Debug,
bounded output, kill/wait и сериализация операций хранилища; клиент не должен
считать физическую отмену ожидания отменой уже запущенного внешнего процесса.

### AUD-12 — квоты и crash consistency offline

`offline.rs:save_to` учитывает только читаемые manifests, проверяет бюджет только
для нового track ID, пишет фиксированные `.id.*.tmp`, не блокирует два процесса.
Неучтённые orphan/corrupt файлы могут расходовать диск; замена и параллельные save
нуждаются в отдельных тестах. `load_from` читает manifest целиком без byte cap,
рассматривает только первые 200 unordered directory entries. Это подтверждённые
свойства кода; выход за квоту/потерю пары файлов ещё надо воспроизвести на fixture.
Нельзя исправлять автоматическим удалением пользовательских оригиналов.

### AUD-13 — чтение истории берёт writer reservation

`core/src/listening.rs:history/export` выполняют `BEGIN IMMEDIATE` и prune на GET.
Это сериализует их с записью SQLite даже когда удалять нечего. Privacy/cutoff уже
проверяются; вынести физическое удаление в существующий retention worker, читать
пользовательский snapshot с cutoff. Сначала конкурентный regression, затем замер;
сам факт reservation не доказывает сегодняшнюю жалобу на latency.

### AUD-14 — рекомендации ограничены последними 500

Это намеренное ограничение `recommendations.rs`/`discovery.rs`, не ошибка реализации.
Старый редкий альбом может не попасть в exploration; Refresh не создаёт новый
суточный микс. spectral-4 tempo/chroma/key не участвуют в нынешнем similarity score.
Нет ground-truth evaluation на лицензированной музыкальной выборке. Нужны измеримые
candidate coverage, exploration/diversity, calibration; нельзя просто добавить
случайные веса BPM и назвать алгоритм лучше Spotify.

### AUD-15 — test coverage границы

`network/tests.rs` проверяет передачу cursor, но полный successful preview + page2
web-journey не входит в PN02 (там rejection paths). Rust244 и зелёный browser-suite
не доказывают этот путь. Native tests часто проверяют сервер, выключенный после
входа, а не offline cold launch. Hardware/media interruption, macOS/Windows/Android/
iOS/Safari/screen reader должны оставаться отдельными статусами.

### AUD-16 — расхождение source / installed / docs

Исходный HEAD05b4c37 включает SQL/per-peer cursor исправления после пакета 7fe1ca0.
Запущенный frozen runtime0.12.6 не получает их от нового `cargo build`. Это безопасная
модель развёртывания, но UI/документации нужен явный source/build/schema/asset manifest.
В PLATFORM_MATRIX ещё есть 0.12.2 browser row и spectral-3 рядом с spectral-4;
KNOWN_LIMITATIONS отрицает уже реализованные native offline/Secret Service/folders.
В QA_EVIDENCE G14 нельзя считать целиком EXECUTED, пока forged actor/replay отсутствуют;
фактически проверены SSRF/oversized/metadata subset. G15 тоже частичный.

### AUD-17 — конфликт масштабирующих документов

`RESILIENT_SCALE_PLAN.md` предписывает Redis/PostgreSQL/OpenSearch/NATS,10k RPS и
10k listeners без соответствующих нагрузочных доказательств. Это предложение,
не текущее устройство и не обязательная цель для личного сервера. Основной мандат
требует простой SQLite-monolith. Развести personal profile и opt-in large public hub;
переход хранилища/БД — ADR после измерений и тестируемой миграции, без blind dual-write.

### AUD-18 — большие root-компоненты

`web/src/main.rs`≈2260 строк, `desktop/src/ui.rs`≈3577, native client≈2322,
`server/src/lib.rs`≈1359. Это повышает риск пропустить guards, но число строк
не само по себе дефект. Выделять сервисы session/offline/source и presentation по
одному контракту с regression. Не переписывать проверенный playback model.

### AUD-19 — лицензии/дистрибуция

Документы фиксируют 15 metadata-only dependency notices. Это не доказательство
нарушения, но внешний выпуск требует проверить полные применимые grants и
provenance. Docker/ARM64/native non-Linux/remote CI не подтверждены локальным tar.
Не игнорировать runtime vulnerability advisories ради зелёного отчёта.

### AUD-20 — сегодняшний dependency gate FAILED

Выполненный `cargo-deny check` отклонил три **INFO Unmaintained** advisory:
`zvariant5.15.1` RUSTSEC-2026-0370, `zbus_names4.3.5`0371,
`zvariant_derive5.15.1`0372. Цепочка: `vela-desktop → mpris-server0.10.0 → zbus5.19.0`.
По текущим [RustSec notices](https://rustsec.org/advisories/RUSTSEC-2026-0370.html),
standalone crates перенесены в ZBus6; это maintenance notice, не опубликованная
runtime vulnerability и не доказательство эксплуатации. Bans/licenses/sources PASS,
но общий dependency gate **FAIL**. Лог: `target/checks/audit-20261019/dependencies.log`.
Нужно оценить совместимый upstream MPRIS/ZBus6 upgrade или reviewed минимальный
patch, затем real D-Bus tests. Автоматические ignores в этом аудите не добавлялись.
Тексты старых документов «dependency policy passes» не считать свежим результатом.

## 4. Визуальный аудит текущего направления

Просмотрены **сохранённые** fixture screenshots `preview-cursor-firefox/
firefox-run3-head-results/personal-{1440-dark,320-light}.png`, а не текущая приватная
библиотека. Встроенный Browser вернул no available browser/пустой список.

Хорошая основа: стабильный нижний плеер, тёмная/светлая палитра, компактный sidebar,
реальные причины рекомендаций, readable desktop. Недоработки: одинаковые крупные
fallback covers, длинный технический текст в каждой карточке, одинаковая визуальная
важность Play и множества служебных действий, мало иерархии и музыкального характера.
На 320px длинная вертикальная лента карточек занимает почти весь путь к следующей
полке; прогресс/mini-player требуют более аккуратной плотности и target sizes.
Нужны редакторская композиция, реальное artwork, короткие причины с disclosure,
контекстные действия, библиотечная навигация, самостоятельный mobile layout.
Это дизайн-оценка; task usability и screen-reader ещё не измерены.

## 5. Что сегодня НЕ признано неисправным

Не найдена новая подтверждённая утечка server-private audio через playlist metadata.
Нет оснований удалять SQLite, заменять Dioxus или ослаблять importer SSRF.
Существующие auth/CAS/Range/consent/worker/backup regressions сохраняются.
Зелёный baseline:244 Rust,22 Python, fmt/native+WASM Clippy. Browser/native opt-in/
Docker/hardware не приписываются сегодняшнему запуску.
