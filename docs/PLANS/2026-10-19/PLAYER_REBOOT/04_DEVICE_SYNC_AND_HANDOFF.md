# Очередь и позиция между устройствами: точный продуктовый контракт

## 1. Четыре разных механизма

| Механизм | Что делает | Чего не делает |
|---|---|---|
| Library sync | Favorites, playlists, settings, availability | Не запускает музыку на другом устройстве |
| Session/bookmark sync | Queue/context/current occurrence/position/modes | Не обязан каждый момент управлять чужим output |
| Remote control | Явные команды выбранному устройству | Не меняет ownership всей библиотеки |
| Handoff | Передаёт конкретную playing session с prepare/accept/commit | Не означает автоматическое «играть может только один» |

Комнаты — пятый отдельный механизм. Нельзя назвать периодическую запись position
готовым handoff или сделать все queue state одного аккаунта одним mutable global blob.

## 2. Устройства и источники истины

Device: случайный stable ID, owner binding, friendly name, platform/version,
capabilities, last-seen/presence expiry, revoked flag, authenticated session binding.
Никаких hardware fingerprints или одного device ID на весь browser account.

Каждое устройство владеет реальным output и своей подтверждённой playback state.
Personal session workspace владеет queue revisions/bookmarks/control log для
конкретной сессии. В простом self-host режиме это существующий сервер; при подключении
независимого аккаунта — явно выбранная sync authority, а не незаметная двойная запись.

Queue snapshot: stable occurrence IDs, origin-scoped media references, current cursor,
context, repeat/shuffle state, revision. Не сохранять в snapshot bearer tokens или
готовые signed URLs; target сам разрешает свой authorized source.

## 3. «Продолжить там, где остановился»

1. A отправляет bounded checkpoint на meaningful changes и примерно раз в5секунд
   при играющем audio; pause/seek/track switch flush раньше, с coalescing.
2. В checkpoint есть session/occurrence/source epoch, position, paused/playing,
   sample monotonic stamp и receiver/server timing для оценки возраста.
3. B показывает «Продолжить с [A]» с actual track/position, не начинает autoplay.
4. Выбор человека восстанавливает очередь/modes и готовит source B; metadata-only
   недоступные items остаются помечены, не silently подменяются похожей песней.
5. Если A продолжает играть, UI предлагает Transfer или Start independent session.
6. Играющая локальная B-сессия не затирается фоновым checkpoint A.

Не использовать last-write-wins wall-clock всех устройств для выбора правильной
позиции: clocks могут расходиться, seek назад легитимен, старый batch может прийти позже.
Sequence/source epoch/revision определяют порядок, время — freshness/информация.

## 4. Queue sync и edits

- Mutations typed: append/insert-after/remove/move/select/replace-context, occurrence
  ID и expected revision; idempotency command ID привязан к actor/session.
- Один session authority сериализует принятые edits и возвращает revision+receipt.
- По WS приходят accepted events; reconnect получает snapshot+bounded tail по cursor.
- Offline edits записываются в local journal с base revision. После reconnect
  безопасные adds/removes rebased по stable IDs; конфликт replace/reorder требует
  явного выбора/branch, не уничтожает другую новую очередь.
- UI может optimistic отобразить безопасное действие и rollback по receipt; реальный
  current source не меняется от отклонённого network edit.
- Граница current/played/next одинакова на всех клиентах; duplicate song ID не заменяет
  occurrence identity. Fair shuffle state совместим по версии либо restore paused.
- User может отключить queue sync, сохранив library sync и consent history отдельно.

## 5. Remote command envelope

Для каждой команды: command ID, owner/session, issuer device, exact target device,
expected session/source epoch/revision, monotonic per-target sequence, issued/expiry,
capability и payload. Target проверяет актуальность и permissions независимо.

Delivery: at-least-once transport, deduplicated application+ack. Нельзя обещать
exactly-once delivery при сетевом разрыве; observable command применится идемпотентно.
Bounded inbox/outbox, TTL, compacted receipts. Command с expired session не оживает
после reconnect. Изменение громкости идёт только поддерживающему её target.

Commands: play/pause/toggle, next/previous, absolute seek и bounded relative seek,
volume/mute, selected queue edits, prepare/accept handoff. Unsupported capability
выключает control с причиной, а не успешным no-op.

## 6. Handoff: безопасный normal path

Workflow имеет transaction ID и ownership/session epochs, но не притворяется
распределённой SQL-транзакцией.

1. Source A подтверждает, что передаётся именно текущая session/revision.
2. Coordinator создаёт transfer intent с TTL, snapshot queue/position/modes.
3. Target B получает media refs, проверяет свои права/codec/output/autoplay policy.
4. B готовит source **paused**, из cache/prefetch/range, и возвращает READY/REJECT.
5. Пока B не READY, A продолжает играть. Denied/buffering/cancel сохраняют A.
6. После READY вычисляется fresh position из A, B уточняет seek-ready.
7. A получает pause-for-transfer и подтверждает stopped/paused на нужном epoch.
8. Только после этого B получает start; он подтверждает actual started position/output.
9. Coordinator фиксирует transferred ownership; A показывает «Играет на B» и остаётся
   контроллером, а не самовольно запускается снова.
10. Если B не стартовал после pause A — ограниченный rollback/resume A при совпавшем
    исходном intent; новый пользовательский Play/selection имеет приоритет.

Такой normal path предпочитает контролируемую короткую паузу неконтролируемому двойному
звуку. Нулевая пауза и нулевое перекрытие при любой network partition одновременно
не обещаются. Scheduled переход можно добавлять после проверки clocks/leases.

## 7. Отказы и спорные случаи

| Случай | Ожидаемое поведение |
|---|---|
| B offline/reject/no rights | A играет, виден понятный отказ, Retry/другой target |
| B buffer slow | Подготовка с Cancel; никаких forced pause A |
| A сменил track во время подготовки | Intent stale; пересоздать/отменить, не проигрывать старый snapshot |
| A недоступен, невозможно подтвердить pause | Не объявлять успешный seamless transfer; явный independent play/timeout policy |
| ACK потерян | Повтор с тем же ID не удваивает pause/start/queue mutations |
| У B autoplay blocked | READY недостаточно; запросить gesture до остановки A |
| Device revoked | Новые commands/WS отклоняются, presence/target исчезает |
| Есть только local offline asset A | B показывает unavailable; не копирует файл без download/share permission |
| Оба человека нажали Play | Independent sessions остаются возможны; управление конкретным target |
| Clock drift/long suspension | Reconnect snapshot и source epoch, не догонять ошибочную wall-clock позицию |

## 8. UI и приватность

Devices sheet различает «На этом устройстве», «Доступно», «Недавно было в сети».
Названия редактируемы, одинаковые имена имеют platform/device hint без fingerprint.
Current device controls видны по owner policy. Другой аккаунт не видит presence,
имена устройств и queue. Shared household controls требуют отдельного явного grant.

Queue/bookmark sync сам по себе раскрывает sync authority музыкальные metadata.
При включении объяснить это отдельно от private listening history. E2E-encrypted
sync — отдельная reviewed возможность с device enrollment/recovery keys, не
самодельная криптография и не ложное «облако ничего не знает».

## 9. Приёмка SYNC-01–10 / HANDOFF-01–10

- Два настоящих клиента одного аккаунта и отрицательный третий другого аккаунта.
- Queue reorder/duplicates/play-next/modes/position сохраняются без source reset.
- Fresh reconnect/downstream event order/duplicate command/expired TTL/stale revision.
- Offline journal merge не уничтожает новую remote queue.
- A играет→B READY→A pause ACK→B actual sound и правильный occurrence/position.
- B reject/no rights/autoplay/diskfull/codec fail сохраняют или восстанавливают A.
- Concurrent user selection побеждает obsolete transfer.
- Network partitions честно показывают uncertain state, не фиктивный success.
- Measurements: transfer prepare/start latency, position error, gap/overlap отдельно
  по LAN/WAN/codec/device. Числа устанавливаются после baseline, не по обещанию.
- Нет credentials в logs/queue/export, нет history event от sync/preparation.
