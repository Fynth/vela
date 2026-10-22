# Живой реестр пакетов Vela

Источник строк: `../DESIGN_REVIEW/BACKLOG_ROWS.md` (дизайн-аудит Codex
gpt-6.1-sol xhigh, 2026-10-21). Ниже — рабочий стол тимлида: одна строка = один
пакет = один пользовательский исход = один проверенный локальный коммит.

**Статусы:** `PROPOSED` → `IN_PROGRESS` → `DONE` (с коммитом и проверками) или
`BLOCKED` (с причиной). `DONE` ставится только при выполненных командах.

**Порядок (из аудита):** DR-05 → DR-07 → DR-04 → DR-06 → DR-01 → DR-02 →
DR-03 → DR-08. Serious-находки DV-01/02/12 чинятся **до** glass-слоёв.

**Правила:** пакеты технически пересекаются в `apps/web/style.css` — выдаётся
**один пакет за цикл**, параллельных редакторов общего CSS нет. Cargo/lock/
миграции — только у интегратора. Не более 2 cargo-процессов.

## Реестр

| ID | Название | План | Приоритет | Пререквизиты | Ожидаемый исход | Размер | Статус |
|---|---|---|---|---|---|---|---|
| DR-05 | Читаемые состояния и feedback | [05](../DESIGN_REVIEW/05_EMPTY_ERROR_STATES.md) | P1 | нет | Home light читается, recovery/receipt не перекрываются, следующий шаг понятен | средний | DONE 2026-10-21, commit 6dcb3fa |
| DR-07 | Различимое управление с клавиатуры | [07](../DESIGN_REVIEW/07_A11Y_MOTION_AUDIT.md) | P1 | нет | Видимый focus, понятные range controls, предсказуемый выход из overlay | средний | DONE 2026-10-22, commit 9656db3 (ревью: нативный Codex gpt-6.1-sol xhigh ACCEPT; omp-reviewer ПРИНЯТЬ С ПРАВКАМИ — правки раундов 2 и 3 закрыты) |
| DR-04 | Компактное управление с 44px targets | [04](../DESIGN_REVIEW/04_DENSITY_TYPOGRAPHY.md) | P1 | нет | В Next/Queue/navigation проще попасть без разрастания UI | средний | PROPOSED |
| DR-06 | Музыка в первом mobile-экране | [06](../DESIGN_REVIEW/06_MOBILE_320_390.md) | P1 | нет | На 320 первый трек полностью виден, mobile controls и safe-area сохранены | средний | PROPOSED |
| DR-01 | Один словарь glass/motion | [01](../DESIGN_REVIEW/01_TOKEN_LAYER.md) | P2 | нет | Темы и отклик следующих улучшений не расходятся | маленький | PROPOSED |
| DR-02 | Glass player/topbar с безопасным underlap | [02](../DESIGN_REVIEW/02_GLASS_PLAYER_TOPBAR.md) | P2 | DR-01, DR-04 | Контент проходит под читаемыми полосами, последняя строка доступна | большой | PROPOSED |
| DR-03 | Короткий отклик без задержки команды | [03](../DESIGN_REVIEW/03_MOTION_SYSTEM.md) | P2 | DR-01 | Смена cover/очереди/overlay понятна, управление остаётся немедленным | большой | PROPOSED |
| DR-08 | Проверенный Linux presentation parity | [08](../DESIGN_REVIEW/08_NATIVE_PARITY.md) | P2 | DR-01 | Native controls читаются и остаются управляемыми в WebKitGTK | большой | PROPOSED |

## Находки аудита → пакеты

| Находка | Пакет | Суть |
|---|---|---|
| DV-01 | DR-05 | Светлая Home: контраст заголовка 1,114:1, подписи 1,622:1 |
| DV-02 | DR-05 | Recovery и receipt перекрываются на 360/390px |
| DV-03 | DR-06 | На 320px видно 32 из 115px первого трека |
| DV-04/05/08/09/15 | DR-04 | 19 целей < 44px: навигация 41px, transport 28px, utility 32px; Play/seek пересекаются на 44×12px |
| DV-06 | DR-01/02 | Glass в веб-клиенте отсутствует (в оверлеях есть) — но приоритет ниже usability |
| DV-07/14 | DR-01/03 | Вход Now Playing уже есть (180ms/10px); эквалайзер анимирует layout-свойство `height` |
| DV-10/12 | DR-07 | Светлый keyboard focus очереди 1,315:1; подписи/range |
| DV-11/13 | DR-05 | Читаемость/копирайт состояний |
| Native parity | DR-08 | Web PNG не закрывает native-приёмку |

## Пакеты вне дизайн-аудита (продуктовые, из мандата)

Приоритет тимлида — P0 контракт/безопасность/данные, затем P1 видимая
регрессия, затем P2. Дизайн-пакеты DR-01…DR-08 — P1/P2 по usability; они не
отменяют продуктовые срезы.

| ID | Название | Приоритет | Статус |
|---|---|---|---|
| TL-01 | Progressive native playback: старт звука до конца файла (сейчас `client.rs::prepare` тянет весь оригинал, `audio.rs::decode` открывает файл целиком) | P1 | PROPOSED |
| TL-02 | Одноразовый кэш без повторной загрузки A→B→A (в `media_cache.rs` уже есть BLAKE3+sidecar, нужна проверка повторного возврата) | P1 | PROPOSED |
| TL-03 | Single-flight подготовка следующего трека по очереди (спекулятивный next из occurrence-модели) | P2 | PROPOSED |
| TL-04 | Холодный офлайн-старт native (манифесты v2 уже есть в `offline.rs`) | P2 | PROPOSED |
| TL-05 | Перенос очереди/позиции между устройствами с подтверждением (handoff) | P2 | PROPOSED |

## Журнал циклов

| Дата | Пакет | Коммит | Проверки | Итог |
|---|---|---|---|---|
| 2026-10-21 | — | — | — | Реестр создан из аудита Codex (gpt-6.1-sol xhigh), 15 находок, 8 пакетов |
| 2026-10-21 | DR-05 | 6dcb3fa | fmt exit 0; строгий clippy чисто; 380 Rust-тестов; wasm+build-web; Playwright Chromium 61 passed / 0 failed / 1 skipped; новый dr05-states spec passed | Контраст пустых карточек в light/dark ≥ 4.5:1; перекрытие трёх feedback-сигналов 0px, gap 8px, кнопки ≥ 44px на 320/360/390/1284. NOT_RUN: полный Firefox (на хосте нет PulseAudio — те же аудио-ассерты падают и на неизменённом baseline), native GUI, screen reader, safe-area hardware |
| 2026-10-22 | DR-07 | 9656db3 | fmt exit 0; строгий clippy exit 0; 380 Rust-тестов (0 failed, 3 ignored); wasm release + build-web exit 0; Playwright Chromium 62 passed / 1 skipped и Firefox 62 passed / 1 skipped, включая новый постоянный dr07-a11y.spec.mjs | Кольцо фокуса 5.74:1 (light) и 12.54:1 (dark) — было 1.315:1; рельсы 325×44 с видимыми label 7.65:1; aria-valuetext «80%» / «0:00 of 0:01» / «Duration unknown»; фокус возвращается на подключённый триггер после Escape, «Open library», пере-рендера внутри ящика и через Now Playing; таймер сна — role=timer + aria-live=off, одно объявление на переход, live-область всегда в DOM. Ревью: нативный Codex gpt-6.1-sol xhigh — ACCEPT; omp-reviewer — ПРИНЯТЬ С ПРАВКАМИ (3 замечания раунда 2 и 5 раунда 3 закрыты). NOT_RUN: screen reader, forced-colors (режим не эмулировался), zoom 200%, app-scale 150%, native WebKitGTK, физическое аудио, Docker |
