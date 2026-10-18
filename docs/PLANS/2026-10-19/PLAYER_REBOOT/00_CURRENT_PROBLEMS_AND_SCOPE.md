# Факты и проблемы: что действительно надо менять

## 1. Точка отсчёта

На старте: чистое рабочее дерево, HEAD `10fe946`, версия workspace 0.12.6.
Присутствуют реальные Dioxus web/Linux, один Rodio/CPAL output, MPRIS, авторизованный
каталог, офлайн-копии, session store, public peers. Android/iOS, handoff, rooms и
независимый hosted account не реализованы. Нельзя начинать с создания нового проекта.

Текущий user-unit ссылается на frozen `target/runtime/vela-0.12.6/vela-server`.
Исходники имеют исправления новее релизного `7fe1ca0`. Каким именно desktop-бинарником
пользователь сделал изображения, **не установлено**. Новое `cargo build` не обновляет
установленный desktop или отдельный frozen server автоматически.

Следующая реализация обязана отдельно идентифицировать: source HEAD, packaged build,
реальный процесс desktop, server build, served web assets, backend Wayland/X11 и DPI.
Не читать чужие credentials/profile/env целиком ради диагностики.

## 2. Что видно на изображениях

### Изображение 1

Видны светлые системные title/menu полосы над тёмным содержимым, узкий фрагмент
навигации и очень широкая композиция строки трека. Изображение обрезано по высоте.
Поэтому нельзя утверждать, что любая пустота на нём — именно GTK headerbar, CSS margin
или настройка Hyprland. Нужно измерить window/content geometry в нормальном tile,
а не исправлять картинку на глаз.

### Изображение 2

Страница плейлистов выглядит служебной формой: крупный заголовок, объяснение
авторизации, две широкие формы создания, отдельные кнопки, большое empty-state поле.
Осталось уведомление о сохранённом offline-треке из другого контекста. Обложек/
содержательного музыкального пространства почти нет. Это конкретные UX-симптомы,
а не основание снять server permission checks.

Личные имена/названия с изображений не копируются в этот пакет, картинки не
дублируются в Git; для будущих regression нужны собственные разрешённые fixtures.

## 3. Подтверждено исходниками

| ID | Наблюдение | Реальные места | Последствие |
|---|---|---|---|
| PR-01 | Полный файл до начала decode | `apps/desktop/src/client.rs:prepare`, `audio.rs:decode`, `lib.rs:PreparedAudio` | Большой файл задерживает Play даже на быстром сервере |
| PR-02 | Обычный переход не ищет reusable cache | `ui.rs:start_source`, `play_catalog`, `select_queue`, `advance` | Новое full GET при большинстве возвратов/выборов |
| PR-03 | `prepared` — lease текущего файла | `ui.rs` root state и source replacement | Pause/resume иногда reuse, но это не кэш прослушанного каталога |
| PR-04 | Нет queue-driven prefetch service | `ui.rs`, `client.rs`, `playback.rs` | Следующий трек начинает готовиться по факту переключения |
| PR-05 | Авторизованный stream уже поддерживает Range/ETag/If-Range/HEAD | `apps/server/src/lib.rs:stream/serve_asset` | Не надо придумывать новый сервер с нуля |
| PR-06 | Default HTTP headers содержат no-store | `server/src/lib.rs:security_headers` | Нельзя чинить native cache слепым shared/browser caching всех API |
| PR-07 | Геометрия fixed-панелей и центрированного content смешана | `apps/desktop/style.css` | На широких/низких tiles возможны лишние поля/overlap |
| PR-08 | Default window/menu не настроены под продукт | `desktop/src/main.rs`, Dioxus desktop Config | Видны дополнительные desktop-полосы; нужна осознанная window policy |
| PR-09 | Native использует нейтральные cover placeholders | `desktop/src/ui.rs`, поля artwork в `client.rs` | Даже реальная коллекция выглядит бедно; нужны authorized image fetch/cache |
| PR-10 | Notice и служебные пояснения занимают primary canvas | `ui.rs:App/playlists_view` | Музыкальная задача уступает формам/диагностике |
| PR-11 | Online/private, remote и offline имеют разный routing | `ui.rs:start_source/start_remote/start_offline_at` | Риск непоследовательных Next/EOF/MPRIS/Repeat при росте функций |
| PR-12 | Нет независимого identity/sync/control-plane | domain/core/server/native | Один login привязан к выбранному серверу, нет готового Vela ID |

Подтверждён control flow, но latency/точные размеры полос этим документом не измерены.

## 4. Уже исправленные в source вещи не переделывать

`3094754` исправляет часть keyring/session lifecycle, `48b2fa4` переводит history
на read snapshots, `e225b63` выносит offline hashing из UI и исправляет ряд quota/
remove/freshness путей; `03b2523` меняет offline EOF/MPRIS routing. PROGRESS указывает
реальные проверки и NOT_RUN. Эти исправления нужно сохранить и выпустить корректным
пакетом; нельзя уверять пользователя, что их содержит его установленный бинарник.

Открыты cold offline launch, origin-scoped cache migration, часть crash/concurrency
проверок, native system/public/offline parity и dependency/security gate.

## 5. Порядок серьёзности

**P0 daily-use blockers:** корректная source/account изоляция; нормальная геометрия
окна; отсутствие UI stalls; повторное использование файла; bounded source preparation;
исправления, которые действительно попали в устанавливаемый пакет.

**P1 experience foundation:** progressive playback, next prefetch, реальные covers,
понятные queue/context/actions, cold offline, обновление search/statistics hot paths.

**P1 connected experience:** независимый ID, permission-bound sync, device handoff,
Android native background/offline. Они идут после contract, а не вместо исправления Play.

**P2 expansion:** rooms, social discovery, сложные mixes/DSP, richer catalog/lyrics,
другие платформы, эффектные визуальные режимы. Обязательные privacy/platform задачи
не переносятся в P2 только потому, что они менее заметны на скриншоте.

## 6. Чего нельзя обещать

«Сервер локальный» не означает отсутствие I/O; «файл уже слушали» не значит, что
его bytes ещё есть в cache; «предзагрузили» не значит, что права всё ещё действуют;
«общий аккаунт» не значит общий доступ ко всем серверам; «общая комната» не лицензия
на rebroadcast; «написали mobile shell» не background audio implementation.
