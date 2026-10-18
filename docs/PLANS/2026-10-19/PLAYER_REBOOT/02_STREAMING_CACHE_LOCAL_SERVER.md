# Playback без повторного полного скачивания

## 1. Ответ на «сервер локальный, почему вообще загрузка?»

Сервер и desktop — разные процессы. Сервер хранит/авторизует файл, клиент должен
получить байты для декодера. При `127.0.0.1` они идут по loopback внутри компьютера,
не через Интернет. Но клиент всё равно сейчас создаёт отдельную полную копию,
проверяет её и лишь затем запускает decoder. Это ограничение реализации, а не
неизбежное свойство локального сервера или Rust/Dioxus.

Нормальное поведение: первый Play использует небольшой стартовый буфер; уже
полученные разрешённые байты переиспользуются; следующий трек готовится заранее.
Короткая HEAD/permission revalidation допустима и не является повторным download
всей песни. Отдельно надо сократить дисковые копирования и интегрировать offline.

## 2. Три независимых слоя

| Слой | Жизнь | Что хранит | Может ли сам обещать offline |
|---|---|---|---|
| Active buffer | Текущий source generation | Небольшие byte/PCM windows | Нет |
| Automatic playback cache | Ограниченный LRU/TTL, account+origin scoped | Уже полученные immutable encoded ranges/files | Нет, если нет явного durable download/grant |
| Pinned downloads | До явного удаления пользователем | Проверенные bytes+manifest+artwork+metadata | Да, по отдельной local entitlement/unlock policy |

Server OS page cache, HTTP cache и application cache тоже разные. Тёплый page cache
ускоряет повторное чтение на сервере, но не отменяет нынешний full GET/копию клиента.
Byte cache не является вторым каталогом собственности или источником media permission.

## 3. Source resolver: что проверяется по нажатию Play

Порядок решения:

1. Зафиксировать account/origin/queue occurrence/source generation и intended position.
2. Проверить действительную локальную pinned copy в соответствующем namespace.
3. Проверить already-open active/prepared lease того же immutable asset.
4. Найти verified playback cache с тем же representation identity и допустимым grant.
5. Если нужна свежая online authorization — revalidate маленьким HEAD/conditional
   запросом к origin, не читать тело заново при совпадающем validator.
6. При частичном cache получить недостающие ranges; старые корректные ranges сохранить.
7. Если локальных bytes нет — открыть progressive source, подготовить startup window.
8. Если codec/сервер/decoder не поддерживает progressive — честно перейти в bounded
   full-prepare fallback только для этой комбинации, с прогрессом и отменой.
9. Любой результат перед commit повторно проверяет scope/generation/lifetime.
10. Cached byte hit не включает историю прослушиваний; только реальный audio clock.

Одинаковое поведение у Library, Search, album, playlist, queue, MPRIS, remote и offline.
Не заставлять человека выбирать особую кнопку Play offline, чтобы избежать full GET
для уже сохранённой и разрешённой local copy.

## 4. Идентичность кэша

| Поле | Для чего |
|---|---|
| Canonical origin / trust namespace | Не смешивать независимые серверы и перенаправления |
| Local account ID / linked identity binding | Изоляция аккаунтов, включая одинаковые UUID на разных origin |
| Track/asset origin IDs | Разрешённый объект и фактический источник |
| Strong validator/content hash | Immutable bytes; BLAKE3 только когда он действительно известен/проверен |
| Representation/codec/quality version | Original и transcoded вариант не один файл |
| Exact byte length/content type | Bounds и корректное Range обращение |
| Available ranges + integrity level | Transport-validated части не притворяются whole-hash-verified файлом |
| Verification/integrity status | Transport-valid, partial, whole-hash-verified, corrupt |
| Authorization provenance / checked-at / grant expiry | Permission и bytes не смешиваются |
| Pin status/access time/lease count | Offline защита, LRU и безопасная eviction |

Не использовать один title/artist, пользовательский filename, access token или
голый UUID как global cache key. Tokens/stream URLs с credential не сериализуются.

## 5. Повторное прослушивание

Сценарий A→B→A в одном аккаунте:

- A cache остаётся в пределах бюджета после switch, а не исчезает с текущим Arc.
- Выбор A использует нужный queue occurrence и новую selection generation, но не
  новый полный transfer. Явный replay может создать новую logical listening session
  по consent policy; ради cache hit не добавляется лишняя запись в queue.
- Resolver при необходимости проверяет grant/ETag; совпадение использует local bytes.
- Seek/replay не перехэширует весь250MiB на UI thread. Integrity worker и verified
  immutable descriptor/generation позволяют избежать ненужной повторной работы.
- При invalidation/per-byte corruption copy не используется; repair ограничен
  соответствующими bytes, остальная коллекция не сбрасывается.
- Logout закрывает account leases и UI; cache lifetime/erase policy отдельна и ясна.
- Повторный вход другим аккаунтом не получает old metadata/audio через cache hit.

Считать audio body bytes по сети и server body requests; «показываем меньше spinner»
не доказывает отсутствие повторной передачи.

## 6. HTTP-политика: не ломать privacy ради cache

Текущий server no-store нельзя игнорировать обычным HTTP/shared cache. Для reusable
автокэша нужна явно спроектированная политика только у подходящих media responses.

1. Сохранить no-store у login/session/history/provenance/private JSON и diagnostics.
2. Для owner-authorized immutable audio определить private storage/revalidation
   contract; например private+no-cache допускает хранение, но требует проверки
   перед повторным использованием, когда действует HTTP-cache семантика.
3. Сначала auth/owner/grant, затем If-None-Match/HEAD/Range.304 не обходит авторизацию.
4. Приватный endpoint не попадает в shared CDN/proxy cache; ключ включает identity
   boundary. Cookie/Authorization handling и Vary проверяются реальными запросами.
5. Ответы401/403/revoke не заменяются stale cached success. Наличие bytes не даёт online
   stream permission. Pinned user downloads имеют отдельную заранее оговорённую policy.
6. Новое правило override не должно случайно убрать security headers у остальных routes.
7. Public license/grants могут позволять другой lifetime, но не inferred private grants.
8. Browser service worker, если появится, не кэширует «все fetch подряд».

Отдельный явный offline/download action создаёт пользовательскую разрешённую копию;
он не равен тайному игнорированию no-store protocol cache. Contract должен точно
различать эти режимы и ограничения их последующего использования.

## 7. Progressive source для native

Сохранить один output thread. Transport/spool работают отдельно, decoder получает
bounded read/seek source. На audio callback не выполняются HTTP, DNS, allocation-heavy
file fetch или async ожидание.

Требования:

- Startup не ждёт end-of-file, когда контейнер допускает раннее decode.
- Bounded RAM ring и disk spool; backpressure и cancellation из selection/seek/logout.
- Sparse/range-aware доступ; seek требует нужный диапазон, не повтор всего оригинала.
- Один in-flight range на source либо контролируемая малая concurrency с dedup.
- Сервер без Range: progressive sequential read возможен, distant seek имеет честное
  состояние; fallback ограничен, не скрытая бесконечная загрузка.
-206/200/416/If-Range/изменившийся validator обрабатываются раздельно; нельзя append
  новое тело к частям старого asset. Завершённый whole hash сверяется отдельно.
- Short read, disconnect, duplicate range, VBR/MP4 metadata at end, FLAC seek table,
  unknown duration и unsupported format имеют реальные tests/capabilities.
- Buffering state означает недостаток media bytes, Preparing/Verifying — другие причины.
- Поддержка original остаётся первой; FFmpeg optional и bounded, не обязательный
  round-trip даже для MP3 с локального сервера.

Полная BLAKE3-проверка всего файла **до первого звука** и progressive start без
предварительно полученного доверенного whole file одновременно невозможны.
Показывать честное состояние whole-file verification; при необходимости усиленной
проверки внедрять versioned per-chunk integrity contract с независимой оценкой,
а не подписывать частичный файл как полностью проверенный.

## 8. Управление ресурсами

Начальный balanced client profile как гипотеза: около16–32MiB active byte buffering,
ограниченный disk playback cache порядка1–2GiB, один speculative next fetch. Эти
числа сверяются с codec bitrate, memory pressure и реальным hardware; не обещание RSS.
Downloads pinned имеют отдельный бюджет. Minimal отключает дорогое speculative work,
но сохраняет reuse уже полученных bytes. Performance увеличивает лишь измеренно полезное.

Quota учитывает staging/orphans/metadata/artwork, reservations и replacement delta.
Eviction не удаляет pinned downloads или файл с live decoder/prefetch lease.
Disk-full не останавливает уже подготовленный звук без необходимости: очистить только
evictable automatic cache, прекратить speculative writes, показать actionable notice.
Один shared download на одинаковый scope+asset; второй потребитель подписывается на
lease/progress. Отмена одного не обрывает другого; после последнего отменяется transport.

## 9. Локальный быстрый путь

Сначала исправить progressive/reuse: loopback достаточно для хорошего personal player.
Оптимизировать direct access лишь если профиль показывает оставшуюся цену копирования.

Опциональный Linux colocated adapter:

- Explicit pairing desktop↔server на том же host с обычным account authorization.
- Private Unix socket, проверка peer credentials и capability-bound opened read-only
  descriptor; не публичный API «дай путь к любому файлу».
- Server выдаёт доступ только к выбранному authorized immutable asset; scoped lease,
  expiry/revocation policy, regular-file/no-follow validation, никаких arbitrary paths.
- Descriptor transport выбирается через изученный OS IPC механизм; Linux-only fallback
  не становится обязательным для web/Android/iOS/VPS.
- Для Docker разные mount/user namespaces учитываются; одинаковое localhost само по
  себе не доказательство same filesystem или доверенного процесса.
- При отсутствии trusted local adapter незаметный и безопасный HTTP fallback.

Нельзя убирать auth, отдавать media directory целиком, открывать пути из DTO или
включать network filesystem как решение пользовательского интерфейса.

## 10. Приёмка CACHE-01–10

1. Первый MP3/FLAC start до получения всего body там, где формат поддержан.
2. A→B→A: нет второго полного body transfer при доступном валидном кэше.
3. Library Play на pinned copy использует local copy, не требует специального обхода.
4. Seek назад по cached ranges не скачивает их повторно.
5. Duplicate queue entries используют один object cache и разные occurrences.
6. Different account/origin не получают cache/metadata друг друга.
7. Revoked online grant не превращается в cached разрешение.
8. Disk-full/eviction/corruption/cancel не повреждают pinned files/active output.
9. Source/validator changed: старая и новая representation не склеиваются.
10. Cache hit/miss/range bytes видны в redacted diagnostics и измерениях, а не только UI.
