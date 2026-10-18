# План исправлений: выполнять в этом порядке

Статус всех пунктов: **PLANNED**, не выполнено этим аудитом. Основание/приоритеты —
[01_AUDIT](01_AUDIT.md). Сначала воспроизведение, затем изменение, потом интеграция.

## 0. Зафиксировать безопасную исходную точку

1. Проверить HEAD, dirty diff, branch, процесс/unit, health, schema из SQLx, build manifest.
2. Сохранить baseline244/22 и текущие ignored отдельно; не превращать ignored в pass.
3. При изменении хранения offline сделать копию **device cache**, не только server DB.
4. Перед новой SQL-миграцией stopped/consistent backup; восстановить в новый каталог;
   проверить все owner/playlist/grant rows, foreign keys, bytes/hash, modes.
5. Runtime/target/releases/data не очищать. Работа — на disposable fixture data.
6. Назначить одного интегратора Cargo/lock/migrations; максимум 2 compiler jobs.
7. Никаких настоящих session secrets в repro, снимках экрана или issue body.

## 1. FIX-01: общий контракт freshness для desktop [AUD-01,08]

Файлы: `apps/desktop/src/ui.rs`, впоследствии маленький `request_scope.rs` внутри
того же приложения, `examples/native-ui-qa.*`. Новая crate не требуется.

1. Перечислить каждый async boundary: login, keyring, resume, catalog, favorite,
   playlist, offline save/remove/load/verify, peer fetch/cache, prepare, reconnect.
2. Ввести immutable request scope: account epoch + canonical origin + user UUID +
   operation generation + selected object/occurrence + lifetime + cancel token.
3. `connecting` сохраняется до завершения принятия аккаунта, не только HTTP.
4. После **каждого await** проверять соответствующий scope до Signal write/side effect.
5. `adopt_account` принимает валидированный scope и сам отвергает устаревшее принятие.
6. Для durable локальной записи guard до запуска недостаточен: storage owner проверяет
   ожидаемое поколение/namespace непосредственно перед commit.
7. Смена аккаунта/серверa отменяет работу физически и меняет epoch; закрытие root
   больше не допускает Signal reads из позднего callback.
8. HTTP mutation, уже принятая сервером, не откатывается ложным UI success; новый
   экран перечитывает актуальное состояние независимо.

Приёмка REG-AUD-01/08: delayed save A → Cancel → login B → release A; delayed
failed resume A → login B; same-account logout/login; same UUID different origin;
window close. Старые результаты не принимают аккаунт, не меняют queue/notice/list.

## 2. FIX-02: мгновенная локальная изоляция при logout [AUD-02,11]

1. Первая синхронная операция — invalidate epoch/cancel sources, stop native output,
   clear private UI, queue и leases; не ждать keyring, сети или DBus.
2. Добавить состояние `SignedOut { cleanup: pending/confirmed/failed }` с понятным
   текстом «Вы вышли. Удаление сохранённого входа…» и Retry при ошибке.
3. Keyring обслуживает один bounded worker; операции versioned, без параллельных
   save/clear, способных удалить новый token после старого logout.
4. Subprocess получает stdin, bounded stdout/stderr, deadline, kill+wait на timeout;
   OS prompt/unlocked/denied/unavailable — типизированные случаи.
5. На shutdown дождаться cleanup в ограниченный бюджет; при неуспехе честный статус,
   не обещать удаление reusable token. Не блокировать GTK loop.
6. Remote logout имеет собственный deadline; failure не возвращает старый account.
7. Убрать token-bearing Debug; секрет оборачивается redacted типом, не попадает в URL.
8. Mock keyring допустим для timeout contract, реальный Secret Service — отдельный
   двухпроцессный integration в изолированном service namespace.

Приёмка REG-AUD-02/11: stalled tool/locked keyring/child failure/oversized output,
Cancel/close/relogin. Локальный плеер останавливается в следующий engine tick, UI
не висит; новый вход не стирается старой очисткой. Измерить budget, не выдумывать.

## 3. FIX-03: восстанавливать session по классу ошибки [AUD-04]

1. `client.rs` возвращает typed errors: AuthenticationExpired, PermissionDenied,
   TransientNetwork, Timeout, RateLimited, ServerFailure, ProtocolInvalid, Canceled.
2. UI отображает безопасные сообщения, не сравнивает строку для логики 401.
3. Offline/timeout/5xx сохраняют token и предлагают Retry/Offline library/Change server.
4. Валидный 401 удаляет только ожидаемый saved-session generation этого origin.
5.403 не равен доказательству истечения session; уточнить API и показывать отказ.
6. Malformed keyring envelope изолируется; удаление не затрагивает другое поколение.
7. Дать Forget this device отдельное подтверждённое действие.

Приёмка REG-AUD-04: server down,503,429,invalid JSON,cancel,401 и success; secret
сохраняется/удаляется по контракту; после восстановления сети достаточно Retry.

## 4. FIX-04: scope локальных данных по серверу [AUD-06]

1. `AccountKey = { canonical_origin, origin_server_id_if_verified, local_user_id }`.
   Origin остаётся границей доверия: один заявленный server UUID не доказывает identity.
2. Путь получает hash от versioned length-prefixed encoding AccountKey, не от password,
   token, listening history или отображаемого имени. Track key включает origin+object ID.
3. Manifestv3 содержит origin scope, asset fingerprint, size, content type, download
   authorization provenance, сохранённые metadata и время проверки.
4. Старый cache `<userUUID>` не прикрепляется молча к первому новому серверу. Показать
   «Копии старой версии»; связать с origin явным подтверждением владельца.
5. Миграция: lock→copy/reflink где безопасно→validate manifests/hashes→fsync→atomic
   index switch; старые файлы сохранить до отдельного подтверждения очистки.
6. При неоднозначном origin оставить recoverable legacy collection; не удалять файлы.
7. Namespace применяется к offline, queue bookmarks, settings и будущему sync journal.
8. В этом изменении не менять идентичность server media/playlist IDs.

Приёмка REG-AUD-06: cloned DB A/B с одними UUID; malicious claimed server UUID;
reverse proxy origin rename; interrupted migration; rollback reader; legacy corrupt.

## 5. FIX-05: настоящий холодный офлайн-запуск [AUD-05]

1. Разделить online auth и локальную unlock/selected account state.
2. На launch читать ограниченный каталог device collections без HTTP. Открытие
   локальной коллекции не делает authenticated server calls и не регистрирует consent.
3. Offline доступ подтверждается локальной политикой пользователя/OS session;
   не хранить password и не выдавать локальному состоянию online-права.
4. На sign-in screen — «Открыть скачанную музыку» при реальных проверяемых копиях.
5. Загружать metadata/artwork/queue/position из того же namespace; restore paused.
6. Нет сети: не удалять копии, не считать их revoked; показывать last checked.
7. Есть сеть: explicit reconnect, reauth expired token, reconcile доступ отдельно от
   уже законно сохранённых пользовательских файлов. Не обещать DRM/retract bytes.
8. User-controlled «Удалить локальные копии» отдельно от server logout/erase history.

Приёмка REG-AUD-05: first startup without server after prior download; fresh process,
no network packets, metadata/art displayed, Play/seek/next работают. Повторить после
OS reboot на доступном Linux, затем Android/iOS после native implementation.

## 6. FIX-06: offline integrity вне UI и корректные edits [AUD-03,07,08,12]

1. Move `prepare`/manifest scan/hash в bounded blocking worker (одна подготовка).
2. Ticket создаётся **до** запуска hash; cancel проверяется между 64KiB блоками.
3. Хэшировать и декодировать тот же opened descriptor; сохранить защиту от TOCTOU.
4. UI показывает «Проверяем копию» и Cancel; late result drops descriptor/lease.
5. Удаление текущего occurrence — stop/clear; удаление другого — remap occurrence
   indices, order deck/history и offline list без сброса активного decoder.
6. Одноточечный storage writer/межпроцессный lock, уникальные staging names,
   байтовые резервации до записи, accounted replace delta, cleanup своих staging.
7. Manifest cap до чтения, schema/ID validation, directory enumeration без случайного
   early cut, bounded index. Orphans учитываются в storage usage и предлагаются на review.
8. Atomic publication audio+manifest через immutable generation и маленький index
   pointer; failed replacement сохраняет предыдущую здоровую копию.
9. Quota includes staging/art/manifests; отмена освобождает только свою reservation.
10. Disk-full/power-loss тестирует каждую boundary. Не внедрять auto-eviction downloads.

Приёмка REG-AUD-03/07/08/12:250MiB, slow disk, missing/corrupt file, concurrent two
processes, near-full quota, overwrite, orphan, long manifest; сохранившиеся old bytes
и source generation проверяются явно. Benchmark UI input latency отдельно от hash.

## 7. FIX-07: read-only history и быстрые агрегаты [AUD-09,13]

1. Сохранить privacy semantics: retention cutoff применяется на каждом чтении,
   disabled history не читается ranking; delete+revision остаются atomic writer.
2. Убрать prune из обычных GET; существующий retention worker выполняет ограниченные
   batches. DEFERRED snapshot читает preferences/rows/totals согласованно.
3. Сначала `EXPLAIN QUERY PLAN` текущих stats/search, 100k→1m fixtures с foreign rows.
4. Для stats попробовать covering index; измерить write amplification/DB size. Если
   group counts всё ещё дороги — transactional owner counters с rebuild/reconcile,
   не кэш без invalidation. SQL constraints остаются источником истины.
5. Search: разделить стоимость tracks/albums/artists/playlists; FTS group index с
   owner-scoped filtering; common/rare/miss и Unicode отдельно.
6. Не требовать единый DTO «всё загрузилось» для показа local partial search results.
7. Проверить upload/scan/write latency одновременно с history/search/stats.
8. Прогон baseline→одна переменная→candidate трижды, сохранить raw samples/hardware.

Начальные целевые budgets (не результаты): warm100k stats p95<100ms, local search
p95<150ms, API p99 без секундных провалов; пересмотреть по hardware profile. Не
приравнивать 1-секундный Python loopback report к 10k concurrent listeners.

## 8. FIX-08: документация, UX regressions, рефакторинг [AUD-10,15–19]

1. Начало каждого authoritative doc содержит **Current**, build/source commit и scope.
2. Исторические sections вынести с индексом ссылок, сохранив свидетельства/failed runs.
3. Parity по feature×platform: SOURCE/COMPILED/FUNCTIONAL/HARDWARE/RELEASE различаются.
4. Successful peer preview20+20+last, duplicate/foreign cursor, revoked row, stale
   response/logout — browser journey против двух настоящих fixture servers.
5. Разделить keyring/offline/source coordinator и presentation без изменения wire API;
   shared playback.rs оставить прежним. Один рефакторинг — один regression набор.
6. Cargo local-path versions выровнять поworkspace (desktop devdep сейчас 0.12.0 при
  0.12.6; semver разрешает сборку, но manifests вводят в заблуждение).
7. Проверить notices/OS runtime libs, Docker/ARM64 actual runners и release manifests.
8. Deploy только после source+exact-package gates, fresh backup/restore и migration
   compatibility. До этого original0.12.6 не трогать.

### Дополнительный dependency repair [AUD-20]

1. Зафиксировать failing graph/versions/advisory IDs из сегодняшнего лога.
2. Проверить maintained upstream mpris-server с ZBus6 и совместимость event/property
   APIs; не делать слепой major bump или blanket advisory ignore.
3. Если upstream пока не готов — сравнить минимальный reviewed integration patch
   и явно ограниченную временную maintenance exception с причиной/сроком пересмотра;
   runtime vulnerability policy остаётся deny. Решение оформить ADR, не скрыть FAIL.
4. Повторить private D-Bus integration, properties/seek/volume/shuffle/loop status,
   absent-bus fallback, root logout/old command guards, bounded channel saturation.
5. Обновить lock/notices, native+WASM checks, exact native package и dependency gate.

## 9. Граница завершения repair train

- Все новые REG-AUD сначала красные на baseline, затем зелёные на исправлении.
-244 старых Rust проверок,22 Python, browser EXP/PLAY/RES/NET/AUTH/NP/PC/G07–09
  сохраняются; число тестов растёт только из-за содержательных новых regressions.
- Нет изменения consent/default publications/прав доступа при upgrade.
- Fresh offline launch, смена аккаунта, зависший keyring и pending close проверены
  настоящим native window, аудиовыходом и отдельными storage fixtures.
- Каждый исправленный AUD получает source commit, точную команду и evidence.
- Никакое planned исправление этого документа не помечается DONE без запуска.
