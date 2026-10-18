# Предзагрузка очереди и предсказуемое native-воспроизведение

## 1. Координатор вместо трёх разных плееров

Сейчас online, remote и offline routing разнесён. Целевое состояние: один playback
coordinator, один queue occurrence model, один native output, несколько source adapters.
Тип источника не меняет смысл Next/Previous/EOF/repeat/volume/MPRIS.

Слои ответственности:

- Queue model: occurrence IDs, порядок, explicit-next priority, shuffle deck/history,
  repeat, context. Существующий pure playback.rs сохраняется/эволюционирует тестами.
- Resolver: доступ, origin, pinned/cache/range/full-prepare source, capabilities.
- Prefetch planner: только подготовка вероятных источников по принятому порядку.
- Audio engine: decode/output/position/EOF/route/errors, не HTTP и не account owner.
- Session coordinator: команды пользователя/устройства/room и scope/generations.
- Dioxus/MPRIS: presentation/typed command routing, не параллельный владелец audio.

## 2. Что предзагружать

Balanced: metadata/artwork ближайших реальных occurrences и media startup window
следующего; более далёкий prefetch только при bandwidth/disk/energy budget.
Для full-prepare-only format допускается вся следующая копия с явным лимитом.
User-added Play next приоритетнее старого speculative context candidate.

Prefetch не должен:

- запускать звук/decoder output или переключать текущий source;
- считать download прослушиванием/популярностью/правом на публикацию;
- обходить media grants, settings/metered/offline policy;
- вызывать настоящий Next для угадывания shuffle и тем самым менять историю;
- заполнять всю очередь из1000треков файлом каждого;
- скрыто включаться у public metadata-only результатов;
- расходовать ресурсы бесконечно после смены аккаунта/очереди/закрытия окна.

## 3. Гарантированное соответствие shuffle

Queue exposes immutable playback-order snapshot с queue revision и real planned
next occurrence. При shuffle выбор deck/order фиксируется в модели, а не создаётся
независимым RNG в prefetch. Просмотр next не потребляет history/deck.

При explicit-next insertion, reorder/remove/current switch, repeat mode или нового
context меняется plan generation. Старый fetch отменяется либо его verified object
сохраняется как обычный scoped cache, но не «назначается следующим» старым callback.
Повтор одного asset допускает один shared byte object и несколько leases.

## 4. Приоритеты и backpressure

1. Current audio callback/decode должен получать готовые bytes вовремя.
2. Seek текущего трека выше продолжения линейной загрузки старого диапазона.
3. Explicit Play/Play next выше фонового speculative fetch.
4. Prefetch next выше не срочного artwork дальних строк.
5. Явный download job соблюдает выбранную user policy и не ломает current playback.
6. Scan/DSP/import на сервере bounded отдельно; prefetch не занимает все stream slots.

Начать с одного speculative media fetch на клиент; reserve current capacity.
Спекулятивный запрос уступает при underrun, высокой IO latency, battery saver,
metered/roaming policy, low disk и слабом сервере. Доступность bandwidth не угадывать
по надписи Wi-Fi: измерять фактический progress/latency, не трассировать content URL.

## 5. Жизненный цикл prefetch

Для каждой задачи хранить scope, queue revision, occurrence, representation key,
plan generation, byte target, deadline, cancellation, quota reservation, lease.
Состояния: eligible→scheduled→fetching→startup-ready/complete→consumed/retired;
denied/unsupported/failed/canceled — раздельные terminal outcomes.

Подготовленный next не обязательно создаёт decoder заранее. Если decoder costly,
pre-probe делается отдельным bounded worker и не открывает второй audible output.
Декодеры/буферы освобождаются при retired task, byte cache можно сохранить по policy.

Commit на переходе: подтвердить actual next occurrence, authorization policy и
representation, передать prepared source lease engine, atomic cursor/source switch.
Поздний callback проигранной задачи не меняет track title/position/current artwork.

## 6. EOF и переходы

Natural EOF, manual Next, previous seek-to-start, replay и room-scheduled switch —
разные команды. Natural EOF использует актуальный next из модели и source adapters,
не всегда private server prepare. Repeat-one не создаёт новый полный download.
Repeat-all возвращается через cache; fair shuffle back/forward остаётся корректным.

Gapless — отдельная проверяемая capability. Отсутствие spinner между треками не
доказывает sample-perfect continuity; MP3 encoder delay/padding и format transition
должны учитываться только проверенным engine. Crossfade отключаем для album mode,
spoken/long-form по preference. Prefetch полезен до gapless и не должен ждать его.

## 7. Клики и быстрые команды

Каждая explicit selection получает intent generation. A→B→C rapid selection:
активным становится только C; A/B requests реально отменяются; их completed bytes
могут попасть в разрешённый cache, но source не перехватывают. Play/Pause не запускает
повторную подготовку при уже paused ready source. Seek во время buffering управляет
intended position; готовый старый range не отменяет новый seek.

Меню/queue row/album/artist/OS controls вызывают один command path. Повторный command
ID после reconnect не применяется дважды. Текст ошибки связан с occurrence и scope.
UI обновляет положение на media events/monotonic observation, не сам «проигрывает»
трек таймером. Native listening events остаются отдельной consent-controlled работой.

## 8. Что должен видеть человек

По умолчанию никакой большой строки «скачиваем очередной оригинал» на каждый Play.
Короткий preparing/buffering state появляется только при реальном ожидании; причина
и Cancel/Retry раскрываются при задержке. В queue можно включить ненавязчивые
Ready/Preparing/Unavailable indicators, без цифр сетевого протокола в основном списке.
Details drawer показывает cache source, bytes, codec/route и безопасную диагностику.

«Следующий готов» означает actual source-ready по decoder requirements, а не получение
title metadata. «Скачано» означает durable pinned copy, не остаток transient buffer.

## 9. Приёмка PREF-01–12

1. После начала A реальный next B готовится до EOF при разрешённых условиях.
2. Natural EOF A→B не ждёт начала full transfer B и не открывает второй output.
3. Вставленный Play next C перехватывает план B; actualnext совпадает с UI.
4. Shuffle preview/preparation совпадают с фактическим deck; предыдущий трек корректен.
5. Cancel/change queue/account stops obsolete fetching/leases, stale commit невозможен.
6. Repeat-one/all не создаёт повторные полные transfers уже cached asset.
7. Cache/prepare для дубликатов дедуплицированы, occurrence история раздельна.
8. Low bandwidth/current stall приостанавливает speculative task, current не голодает.
9. Метаданные-only/denied/revoked next не подготавливаются как разрешённый stream.
10. Offline/remote Next/EOF/MPRIS идут через тот же coordinator contract.
11. No history event от подготовки/скачивания/перехода без фактического clock progress.
12. Real audio fixtures, network byte counters и output continuity, не mocked spinner.
