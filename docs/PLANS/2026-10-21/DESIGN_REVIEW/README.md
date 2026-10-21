# Дизайн-аудит Vela Music: индекс и порядок работ

Дата: 2026-10-21. Адресат: автономный тимлид и его воркер. Это аудит фактического web-рендера и спецификации будущих изменений. Все пакеты имеют статус **PROPOSED**, реализация отсутствует. Native оценён по исходникам; web-скриншоты не являются доказательством native-паритета.

## Прочитайте документы в этом порядке

1. [Аудит и реестр DV](AUDIT.md).
2. [Строки для живого backlog](BACKLOG_ROWS.md).
3. Пакет, выбранный по приоритету и выполненным пререквизитам.

Рекомендуемый порядок исправляет читаемость и управление раньше декоративного слоя:

| Очерёдность | Пакет | Результат |
| --- | --- | --- |
| 1 | [DR-05 — состояния](05_EMPTY_ERROR_STATES.md) | Читаемая светлая Home, понятные состояния и неперекрывающиеся уведомления |
| 2 | [DR-07 — доступность](07_A11Y_MOTION_AUDIT.md) | Контрастный фокус и различимые, доступные с клавиатуры контролы очереди |
| 3 | [DR-04 — плотность](04_DENSITY_TYPOGRAPHY.md) | Реальные цели 44px при компактных иконках и согласованный ритм |
| 4 | [DR-06 — mobile](06_MOBILE_320_390.md) | Первый трек полностью виден на 320px, сохранены безопасные нижние полосы |
| 5 | [DR-01 — токены](01_TOKEN_LAYER.md) | Один словарь glass/motion, без смены палитры |
| 6 | [DR-02 — glass](02_GLASS_PLAYER_TOPBAR.md) | Контент действительно проходит под читаемыми плавающими полосами |
| 7 | [DR-03 — motion](03_MOTION_SYSTEM.md) | Короткий отклик без задержки действий, фокуса и источника аудио |
| 8 | [DR-08 — native](08_NATIVE_PARITY.md) | Отдельно проверенный перенос допустимой части оформления в WebKitGTK |

После DR-02/03 повторите матрицу DR-07 как проверку их приёмки. Это не повторная реализация DR-07 и не дополнительный коммит. Пакеты выполняются по одному. DR-02 зависит от DR-01/04, DR-03 — от DR-01; остальные не требуют внедрения всего набора. Независимость означает отдельный пользовательский исход и возможность принять/откатить один коммит, а не право одновременно редактировать общий style.css.

## Прочитанные источники

Применены полностью прочитанные навыки:

| Навык | Использованное правило | Прочитанные релевантные references |
| --- | --- | --- |
| [refactoring-ui/SKILL.md](/root/.codex/skills/hermes/refactoring-ui/SKILL.md) | Quick Diagnostic, иерархия, ограниченные шкалы, контраст, глубина | advanced-patterns.md: Empty States, Form Design; animation-microinteractions.md: Timing, Accessibility, Performance; accessibility-depth.md: контраст и фокус; theming-dark-mode.md: Dark Mode Color Principles |
| [microinteractions/SKILL.md](/root/.codex/skills/hermes/microinteractions/SKILL.md) | Trigger → Rules → Feedback → Loops & Modes | feedback-patterns.md: пропорциональность, время, перегрузка, доступность; rules-and-state.md: границы, состояния и прерывание |
| [ux-heuristics/SKILL.md](/root/.codex/skills/hermes/ux-heuristics/SKILL.md) | 10 эвристик, severity 0–4, Quick Diagnostic | audit-template.md; nielsen-heuristics.md: Consistency and Standards; wcag-checklist.md: контраст, reflow, клавиатура; heuristic-conflicts.md |
| [top-design/SKILL.md](/root/.codex/skills/hermes/top-design/SKILL.md) | Взвешенная оценка композиции и качества исполнения | animation-patterns.md: Easing Functions; layout-systems.md: Breakpoint Strategy |
| [technical-documentation/SKILL.md](/root/.codex/skills/hermes/technical-documentation/SKILL.md) | Исполнимые процедуры и проверяемые утверждения | audit-checklist.md: R1–R6, S6/S9, P1/P4 |

Также прочитаны autonomous-task-execution/SKILL.md и credential-handling/SKILL.md. Обновления передаются в текущем диалоге. Чужие каналы сообщений и хранилища секретов не изменяются; временные браузерные контексты закрыты, credentials в документы не переносятся.

Проектные основания:

- AGENTS.md; docs/CONTRACTS.md; docs/PROGRESS.md — индекс и релевантные разделы 2026-10-19–21 о shell, TR11, native, пустой Home, browser gate и stall guard.
- docs/DESIGN_SYSTEM.md; docs/DECISIONS/0023-contrast-and-listening-layout.md.
- ../AUTONOMOUS_TEAM_LEAD/DESIGN_DIRECTIVE.md и MANDATE.md.
- docs/CURRENT_STATE.md, docs/KNOWN_LIMITATIONS.md, docs/QA_EVIDENCE.md, docs/PLATFORM_MATRIX.md; релевантные контракты docs/NATIVE_CLIENT_CONTRACTS.md.
- apps/web/style.css, now-playing.css, home-editorial.css, queue-drawer.css; apps/desktop/style.css.
- apps/web/src/main.rs: root shell/player, Art, меню, picker, уведомления; shell.rs, now_playing.rs, queue_drawer.rs, home_editorial.rs.
- apps/web/index.html; scripts/build-web.sh; tests/browser/{playwright.config.mjs,server.sh,design.spec.mjs,prototype-transfer.spec.mjs,design-review-shots.mjs,design-review-metrics.mjs}; интерфейс scripts/verify_native_client.py.

Живой ../AUTONOMOUS_TEAM_LEAD/BACKLOG.md в просмотренном дереве не найден; MANDATE.md задаёт процесс, но не существующую таблицу. BACKLOG_ROWS.md даёт готовые строки с требуемыми столбцами. Чужие файлы директивы/мандата не редактируются.

## Артефакты и границы доказательств

Базовый каталог: /root/.hermes/profiles/vela/cache/scratch/vela-design-stage/.
Использованы metrics.json и все **21** PNG в shots/. Они просмотрены визуально, включая приложенные изображения. Основные ссылки и ошибочные имена перечислены в AUDIT.md.

Исходный HEAD при чтении: e3238ba5ddbb2233586b9c7e9ad144a9c4b33a3f. Это привязка исходников, а не установленной версии пользователя. Рабочее дерево уже содержало чужие apps/web/Cargo.toml и apps/web/src/browser_player.rs, незакоммиченные планы и два скрипта съёмки; во время чтения добавилась чужая правка docs/PROGRESS.md. В ходе аудита другой процесс продвинул HEAD до 0f5042a1f11b8db07ba1e6df798175622fc59ca8, и эти tracked-изменения исчезли из status. Этот аудит не вызывал commit и не менял их содержимое.

Фактическая тема определяется пикселями/DOM, не суффиксом имени. 15-library-1728x1000-dark.png, 16-home-320x700-dark.png и 18-context-menu-1284x856-dark.png показывают **light**. 20-library-1284x856-reduced-motion.png показывает light и открытое меню. 03-queue-drawer-1284x856-dark.png показывает библиотеку с receipt, drawer на нём не открыт; сам drawer виден на 05. Причина theme drift видна в design-review-shots.mjs:20–26, 41–58: тема меняется только при переданном аргументе.

metrics.json — один settled Chromium Library 1284×856, dark. Подсчёт typeScale проходит по всем DOM-узлам, включая SVG/контейнеры; spacing считает gap/rowGap/columnGap отдельно и включает скрытые узлы. Это частоты CSS-значений, не число визуальных ошибок. backdropUsers не обходит псевдоэлементы; пустой массив не означает отсутствия ::backdrop. Контрастный скрипт ищет непрозрачного предка, не композитит стекло/изображения. Для будущего glass он недостаточен.

## Выполненные проверки

Проверки ниже выполнены в этой работе; исторические pass из PROGRESS не присвоены аудиту:

| Команда / действие | Реальный результат |
| --- | --- |
| git status --short; git rev-parse HEAD; rg --files; nl/sed/rg по источникам | Зафиксированы исходники, чужие изменения, места каскада и отсутствие живого BACKLOG |
| view_image по shots + визуальный разбор приложений | Просмотрены 21 PNG; обнаружены неверные названия тем/состояний |
| Чтение metrics.json и обоих design-review-*.mjs | 19 малых целей; ограничения метода измерений отражены в AUDIT |
| node --input-type=module через stdin, Playwright Chromium, порт 3219, Home/Library/menu, 320/390 | Получены LIGHT_HOME, MENU, MOBILE_320/MOBILE_390; точные значения в AUDIT |
| Второй такой Node-проход: TARGETS, HEADINGS, reduced-motion | 19 целей: 12 enabled nav + 2 enabled utility + 5 disabled; h1 34.668/650, h2 21/600; в settled Library 0 активных анимаций и 0 ненулевых transition-duration |
| Проверка фокуса очереди после переключения на клавиатурный ввод | focusVisible=true,outline pistachio2px+offset2 на белом; Escape→0 открытых queue dialog. Обычный programmatic focus после мыши отдельно не доказывает :focus-visible |
| Дополнительный Node-проход PLAYER_RECTS | Play y779..823,seek input811..855:44×12px пересечения; DV-15 |
| Python urllib.request + hashlib: GET четырёх CSS с 3219 и сравнение с apps/web | Все 4 SHA256 source/served равны |
| Python: относительная sRGB-яркость непрозрачных DOM-цветов | Home title 1.114:1; subtitle 1.622:1; исходный focus #d3e9a5 на белом 1.315:1 |
| npx playwright test --help | Exit0; подтверждён --browser=firefox, tests не запускались |
| Python Markdown validator через stdin |11 ожидаемых .md,46 локальных ссылок,0 ошибок: секции пакетов, PROPOSED/gate, закрытые fences, пустые строки вокруг headings, trailing whitespace, отсутствие credentials |
| git diff --check -- docs/PLANS/2026-10-21/DESIGN_REVIEW | Exit0; дополнительно Python проверил whitespace в новых untracked файлах, которые git diff ещё не показывает |
| SHA256 всех tracked файлов вне Markdown до/после подготовки |488→488 файлов,0 изменений содержимого, включая Cargo/lock/browser_player.rs |

Первая дополнительная попытка измерений закончилась locator timeout 30s: скрипт проверил login до окончания WASM boot. Исправлен только однократный stdin-harness: ожидание .auth-card либо .app-shell перед входом. Повторный проход завершился exit 0. Ни один проектный скрипт не изменён.

## Команды приёмки будущего web-пакета

**GATE_WEB** — требование к реализующему тимлиду, здесь **NOT_RUN**. Выполняйте последовательно из /root/vela; предварительно проверьте чужие Cargo-процессы. Не больше двух compiler jobs суммарно. Проверка на порту 3217 запускает собственный изолированный fixture-server; стенд аудита остаётся на 3219.

~~~bash
export CARGO_BUILD_JOBS=2
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace -- --test-threads=2
cargo build --locked -p vela-web --target wasm32-unknown-unknown --release
scripts/build-web.sh
cargo build --locked -p vela-server
cd tests/browser
npx playwright test
npx playwright test --browser=firefox
~~~

Полная workspace-проверка включает API-интеграцию. Если native-зависимости не позволяют clippy/test workspace, фиксируйте точную ошибку и NOT_RUN соответствующей части; отдельный успешный vela-web не превращает общий gate в PASS. Общая browser-конфигурация не содержит named projects: Chromium — штатный запуск, Firefox — --browser=firefox. Этот флаг проверен командой npx playwright test --help (exit0). Не используйте несуществующий --project=firefox.

Точечные повторения выполняйте перед полным browser gate, когда пакет меняет соответствующую область:

~~~bash
cd /root/vela/tests/browser
npx playwright test prototype-transfer.spec.mjs --grep 'TR03|TR09'
npx playwright test design.spec.mjs
~~~

Сохраните EXP/PLAY/RES/NET/AUTH/NP и mapping G01–G24 в QA_EVIDENCE. Для visual gates каждый пакет задаёт свои сцены; общая матрица: 320×700, 360×800, 390×844, 768×1024, 1284×856, 1440×900, 1728×1000; dark/light; normal/reduce; оба browser engine. Имя нового артефакта содержит фактическую тему, viewport и engine; перед съёмкой прочитайте data-theme, media-query и состояние оверлея.

## NOT_RUN

Приложение/CSS/Rust не изменены; cargo fmt/clippy/test, wasm, build-web, полный Playwright и повторная штатная съёмка **не запускались**. Выполнены только описанные точечные DOM-проходы Chromium. Firefox PNG просмотрен, новый Firefox-сеанс не запускался. Не измерены event-to-feedback latency, composited glass contrast, FPS/jank, LCP/CLS, печать и вся keyboard/screen-reader матрица.

Нет новых снимков настоящей нулевой библиотеки, незавершённого импорта, длинной кириллицы, mobile Now Playing/drawer и увеличенного текста. Показанная пустая editorial-карточка относится к отсутствию плейлистов при **двух существующих треках**, а не к нулевой библиотеке. Native GUI/WebKitGTK, Wayland/X11, physical audio, Docker и packaging не запускались. Просмотр исходника не закрывает эти категории.

## Границы передачи

Созданы только Markdown-файлы в этой директории. Нет локальных коммитов, публикации, новых ассетов, изменений зависимостей/миграций. media_assets, исходные данные, target/, frozen runtime и чужая работа остаются вне этой задачи. Строки источников — координаты аудита; воркер на своём HEAD повторно находит селекторы.
