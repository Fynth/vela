# Аудит фактического UI Vela Music

Для тимлида и воркера: реестр ниже отделяет дефекты использования от отличий от целевого оформления. Дата 2026-10-21. Реализация по этому аудиту **не выполнена**.

## Основание и метод

Скриншоты находятся в /root/.hermes/profiles/vela/cache/scratch/vela-design-stage/shots/, метрики — в соседнем metrics.json. Дополнительные Chromium DOM-пробы выполнены на 127.0.0.1:3219. Четыре CSS, отданные стендом, побайтно совпадают с apps/web: проверка SHA256. Связь с frozen runtime и точный WASM build identity отдельно **NOT_RUN**.

Обозначения доказательств:

- **S01** — 01-library-1284x856-dark.png; **S04** — 04-now-playing-1284x856-dark.png; **S05** — 05-queue-drawer-1284x856-dark.png.
- **S06/S08** — 06-home-1284x856-dark.png / 08-home-1284x856-light.png.
- **S09/S09b/S10/S11** — 09-library-390x844-dark.png / 09b-library-390x844-dark.png / 10-library-360x800-dark.png / 11-library-320x700-dark.png.
- **S12/S13/S14** — 12-home-390x844-dark.png / 13-library-320x700-light.png / 14-library-1284x856-light.png.
- **S15/S16/S17/S18** — 15-library-1728x1000-dark.png / 16-home-320x700-dark.png / 17-home-1440x900-light.png / 18-context-menu-1284x856-dark.png. Все четыре фактически light.
- **S20** — 20-library-1284x856-reduced-motion.png: light, меню открыто. **S21** — 21-library-1284x856-firefox.png: dark.
- **M** — исходный metrics.json; **L** — дополнительные измерения этой работы, результаты ниже; **C** — исходник с координатой.

Ещё просмотрены 02-playing-1284x856-dark.png, 03-queue-drawer-1284x856-dark.png и 07-home-1440x900-dark.png. На 03 нет открытого drawer; есть два уведомления. На снимках с текстом browser gesture нельзя утверждать, что аудио реально играет, по одному имени playing или эквалайзеру.

Оценка severity следует ux-heuristics/SKILL.md и references/audit-template.md: 4=critical — задача заблокирована; 3=serious — существенная потеря доступности/работоспособности; 2=moderate — задержка/ошибка/раздражение; 1=minor — оформление. Частота, ущерб и устойчивость оцениваются отдельно. Критических дефектов по этому набору не доказано.

## Оценки навыков

### Refactoring UI: 6/10, подтверждено 5 из 8 строк

Применена формула round(5/8 × 10). Это оценка данного набора экранов, не всего продукта. Неизмеренное свойство не получает pass.

| Quick Diagnostic | Вердикт | Основание / что исправлять |
| --- | --- | --- |
| Иерархия читается при прищуре | Да, визуальная экспертная оценка | S01: заголовок, поиск, строки и метаданные различимы; числовой blur-тест NOT_RUN |
| Работает в grayscale | NOT_RUN, балл не начислен | Не подменять проверку снижением насыщенности на словах |
| Достаточно отрицательного пространства | Да для desktop | S01/S15: две реальные строки, пустое продолжение не требует fake-контента; mobile отдельно DV-03 |
| Labels вторичны относительно значений | Да | S04: подписи metadata меньше значений; S01: TITLE/TIME вторичны |
| Отступы следуют шкале | Нет | M: 13px ×36, 3px ×25; C style.css:784, 835; DV-08 |
| Ширина текста ограничена | Да | C home-editorial.css:75,83 — 15ch/34ch; S06 |
| Контраст достаточен | Нет | L+S08/S17: title 1.114, subtitle 1.622; DV-01 |
| Глубина соответствует роли | Да | S18: плавающее меню с тенью; S05: dimmed backdrop. Наличие стекла не обязательное условие usability |

Иерархия 7/10, отступы 5/10, цвет/читаемость 5/10, глубина 7/10 — экспертные подоценки, не отдельные результаты теста. До повторного высокого балла нужны DV-01/03/08 и настоящий grayscale/reflow проход.

### Microinteractions: 5/10, подтверждено 4 из 8 строк

Формула round(4/8 × 10). Оценивается видимый набор Play/queue/error/menu, не все асинхронные сценарии.

| Quick Diagnostic | Вердикт | Основание / действие |
| --- | --- | --- |
| Понятный видимый trigger | Да | S01/S18: Play, More; C main.rs:2503 |
| Trigger показывает состояние | Да | S05: Paused/attention; C main.rs:2997–3006: pressed/mode dot |
| Правила просты и предсказуемы | Да по читаемому контракту | C queue_drawer.rs:91–109: saved order, state; функциональный rerun NOT_RUN |
| Feedback начинается менее чем за 100ms | NOT_RUN | CSS duration 120/150ms из M не является latency события |
| Feedback соразмерен событию | Нет | S09/S10: два floating message перекрываются; DV-02 |
| Взаимодействие развивается со временем | NOT_RUN | Не добавлять обучающую телеметрию ради балла |
| Нет скрытых лишних режимов | Да | S05: shuffle и сохранённый порядок объясняются; C queue_drawer.rs:254–259 |
| Понятно с первого использования | Нет для двух range очереди | S05: без видимых подписей; DV-10 |

Trigger → root command → actual result → one feedback — целевая цепочка. Не менять обработчики ради анимации. Для высокого балла нужны DV-02/07/10 и измерение event-to-paint; progressive reduction здесь не обязательна и не оправдывает новую коллекцию данных.

### UX heuristics: 6/10, ограниченная экспертная оценка

Quick Diagnostic: site/page, main action, navigation, search, cancel/back и работа без hover подтверждены видимым UI/кодом; задержки и полный keyboard run не проверены. Status имеет serious-дефект DV-02 (−2); labels — moderate DV-10 (−1); «нужно задуматься» — moderate DV-11 (−1). Итого 10−4=6. Светлая нечитаемость DV-01 усиливает уже учтённую проблему ориентации, повторно балл не вычитается. Это не пользовательский тест.

Таблица охватывает все 10 эвристик:

| № / эвристика | Результат | Severity / доказательство |
| --- | --- | --- |
| 1. Видимость состояния | Частично | 3: перекрытие feedback, S09/S10, DV-02; status/role уже существуют C main.rs:2386+, 2960+ |
| 2. Язык и модель пользователя | Частично | 2: RU editorial и EN shell, S06; «1 artists», S01; DV-11 |
| 3. Контроль и выход | Видимые выходы есть | 0 по видимому набору: S04/S05 close; Escape menu измерен L. Undo всех действий NOT_RUN |
| 4. Последовательность | Частично | 2: C queue-drawer.css:295–298 hardcoded focus вне semantic theme; DV-12 |
| 5. Предотвращение ошибок | Есть проверяемые ограничения | 0 в просмотренном коде: нельзя удалить current occurrence, C queue_drawer.rs:331–340. Полный mutation QA NOT_RUN |
| 6. Узнавание вместо памяти | Частично | 2: range без видимых названий, DV-10; у иконок есть aria-label, C main.rs:3025–3030 |
| 7. Эффективность | Частично | 2: 28px/32px desktop controls, DV-04; keyboard Shift+F10 предусмотрен C main.rs:1141–1144 |
| 8. Минимализм и композиция | Частично | 2: S11/S12 — большой pre-list/empty-card блок; DV-03. Пустота после двух треков не баг |
| 9. Восстановление после ошибки | Путь есть, подача мешает | 3: реальный Retry виден, но перекрывается receipt, S09; DV-02. Не объявлять autoplay failure неисправностью движка |
| 10. Контекстная помощь | Частично | 2: технические aria-label без видимых range-label; DV-10; полная помощь/Orca NOT_RUN |

### Top Design: 5.9/10

Взвешенная экспертная оценка: typography 6×0.25 + composition 6×0.25 + motion 4×0.20 + color 7×0.15 + craft 7×0.15 = 5.9. S01/S06 показывают узнаваемую Manrope/pistachio-композицию; S08 теряет текст; M фиксирует базовые 120/150ms transitions, а не продуманную систему. Целевые улучшения — читаемость, ритм, ограниченная глубина, ориентация при смене состояния.

По Quick Diagnostic подтверждены 5/12 строк: визуальная иерархия, асимметрия Home, собственные цвета, продуманные focus/hover в исходниках, функциональная причина np-enter. Полный focus результат и LCP/CLS NOT_RUN; signature/«хочется скриншотить» не измерены. Шкала относит этот диапазон к 5–6. Балл не повышается обещанием Awwwards.

Отступление от top-design осознанное: нет 10:1 display/body, scroll hijack/Lenis/GSAP, магнитных кнопок, нового шрифта, драматических page-load staggers и custom cursor. Приложению для ежедневного прослушивания нужны плотность, быстрый ввод и стабильный playback. Готовая light-палитра с #fff сохраняется вопреки общему запрету pure white в скилле. Проектная директива ease-out/ease-in-out приоритетнее запрета generic easing в top-design.

## Реестр находок

Измерение L выполнялось без изменения приложения; px округлены только в пояснениях. Значения в спецификациях пакетов являются **целевыми**, не результатами уже внедрённого исправления.

| ID / severity | Экран и селектор | Что видно / измерено | Источник | Рекомендация и правило |
| --- | --- | --- | --- | --- |
| DV-01 **serious (3)** | Home light, .home-feature-empty .home-feature-title / subtitle / eyebrow | S08/S17/S16: почти белый текст на белой пустой карточке. L: title #f3f3ed, 30.816px/550, фон #fff → **1.114:1**; subtitle #c0cfc2,11px → **1.622:1**; eyebrow9px →1.114 | apps/web/home-editorial.css:49,68–83,168–179 | DR-05: empty-ветки используют semantic text/muted/danger. [refactoring-ui §Color](/root/.codex/skills/hermes/refactoring-ui/SKILL.md), references/theming-dark-mode.md; ux references/wcag-checklist.md |
| DV-02 **serious (3)** | Mobile .playback-status + .content-inner>.alert.success | S09: receipt закрывает часть текста/Retry; S10 аналогично. Обе привязаны bottom=190+16px; status z45, receipt z60. На desktop S06 обе читаются, но закрывают covers | apps/web/style.css:607–610,801–805,1072,1233–1235; main.rs:2386+,2960+ | DR-05: один presentation stack, gap8px, error/recovery выше receipt; без дублирования root state. [microinteractions §Feedback](/root/.codex/skills/hermes/microinteractions/SKILL.md), references/feedback-patterns.md «Preventing Feedback Overload» |
| DV-03 **moderate (2)** | Library 320, pre-list; Home390 .home-feature-empty | S11/S13: первый row начинается y≈478, player y510 — visible лишь32px из115. L: header50.5px, filters72.688px, toolbar111px. S12: пустая editorial-card **288px**, playable Wave ниже fold. Это потеря полезной высоты, не доказанный unreachable row | apps/web/style.css:799,409–412,427–432,873–885; home-editorial.css:348–360 | DR-06: компактный pre-list и empty-card по содержимому; не уменьшать44px. [refactoring-ui §Spacing/Layout](/root/.codex/skills/hermes/refactoring-ui/SKILL.md); top-design references/layout-systems.md |
| DV-04 **moderate (2)** | Desktop .player-buttons / .player-right | M+L: 4×28×28,3×32×32. В idle 5 disabled, Queue/Mute enabled; при current track transport становится рабочим. Основной Play уже44px, face32px | apps/web/style.css:1134–1153,1211–1223; main.rs:2990–3030 | DR-04: увеличить реальный hitbox, сохранить15–17px glyph; gap уменьшить. [ux-heuristics «Tiny tap targets»](/root/.codex/skills/hermes/ux-heuristics/SKILL.md), references/heuristic-conflicts.md |
| DV-05 **minor (1)** | Desktop .sidebar .nav-item | M+L: **12×181×41px**, видимый интервал около44px за счёт margin3; маленький дефицит высоты, мышью менее серьёзно. Coarse override уже min44 | apps/web/style.css:180,784,1623–1631 | DR-04:44px реальные nav rect, убрать margin3 вместо увеличения общего шага. Не объявлять12 самостоятельных serious. refactoring-ui §Spacing; ux references/wcag-checklist.md (44px здесь проектная цель) |
| DV-06 **minor (1), целевой gap** | Player/topbar glass | M: player rgb(16,19,15), topbar rgb(19,21,19), backdropFilter none. S01: сплошные полосы. Scroller заканчивается у player; blur сам по себе не покажет содержимое | apps/web/style.css:180–205,1022–1073,1090–1096,1252 | DR-01/02: два плавающих слоя с настоящим underlap и fallback; сохранить scroll/focus reserve. refactoring-ui §Depth; DESIGN_DIRECTIVE §1 |
| DV-07 **moderate (2), системный gap** | Overlay/card/cover/reorder motion | M:24 узла с150ms,2 с120ms, animations0 только idle. C: Now Playing уже180ms и10px, drawer entrance нет; cover/reorder transition не найден в audited presentation | now-playing.css:49–50,537–539; queue-drawer.css:28; style.css:1153,1352; now_playing.rs:101; queue_drawer.rs:289 | DR-01/03:80/120/140/100/180/1800/150ms, <=4px overlay, artwork-only fade. [microinteractions §Rules/Feedback](/root/.codex/skills/hermes/microinteractions/SKILL.md); refactoring-ui references/animation-microinteractions.md |
| DV-08 **moderate (2)** | Ритм Library и роли текста | M:13px×36,3px×25; «19/400×18,20/400×15,21/400×6,24/400×9» включает не текст. L реальных заголовков: h1 **34.668/650**, h2 **21/600**. Значения13/3 и h221 подтверждены, обвинение всех19/400 в заголовках неверно | apps/web/style.css:180,784,776–777,830–835; metrics script typeScale/spacing loops | DR-04: точечная нормализация gap13→12,3→4 либо0 в nav; h2 21→20; Manrope и утверждённые dense labels сохранять. refactoring-ui §Spacing/Typography |
| DV-09 **minor (1)** | Радиусы control/menu/card | M:8px×19,7px×14,9px×7; C feature radius10, menu8, modal24. Это небольшой drift от целевых6/8/12/16/20, не функциональная поломка | apps/web/style.css:180,784,1436; home-editorial.css:22; now-playing.css:34 | DR-04:7→8,9→8,10→12 в scoped control/card; modal24 оставить документированным исключением. refactoring-ui references/advanced-patterns.md «Border Radius System» |
| DV-10 **moderate (2)** | Drawer .queue-volume / .queue-seek | S05: два native range без видимых названий, seek визуально короткий. C: volume width100%, seek без sizing rule, общая min-height20px. aria-label присутствует: это не отсутствие accessible name | apps/web/src/queue_drawer.rs:210–244; apps/web/queue-drawer.css:139–148; apps/web/style.css:180 | DR-07: видимые labels, range width100%,44px rect, formatted aria-valuetext; не рисовать второй seek. ux §Recognition/Help; refactoring-ui references/advanced-patterns.md «Form Design» |
| DV-11 **moderate (2)** | Home, drawer, Library copy | S06/S05: RU «Моя волна/СЕЙЧАС ИГРАЕТ» рядом с EN shell/«Saved queue». S01: **1 artists**. Документ lang=en | apps/web/src/home_editorial.rs:17–21,95–101; queue_drawer.rs:143,249–258,349–357; main.rs:Library toolbar; apps/web/index.html:2 | DR-05: для текущего EN shell единые EN системные состояния и EN fallback, русские пользовательские названия сохранить. Полная локализация вне пакета. ux references/nielsen-heuristics.md §Consistency |
| DV-12 **serious (3)** | Light drawer keyboard focus | L: focused=true, focusVisible=true после Tab; outline rgb(211,233,165) solid2px, offset2 на #fff → **1.315:1**, semantic accent не используется. Escape закрывает dialog (open count0) | apps/web/queue-drawer.css:295–298; style.css:144–145 | DR-07:var(--accent), offset3; контроль>=3:1 для кольца. Не заводить второй focus trap. refactoring-ui references/accessibility-depth.md; ux references/wcag-checklist.md |
| DV-13 **moderate (2)** | Now Playing 1284×856, .np-connection | S04: recovery начинается y≈779, нижняя часть/Retry ниже края dialog y824. Artwork276px из height clamp; основной transport и volume видны. Прокрутка есть: полная недоступность не доказана | apps/web/now-playing.css:30,114–116,500–529; apps/web/src/now_playing.rs:117 | DR-05: summary+Retry в один компактный wrap-row, gap8/padding12, action44; при normal names и height>=720 transport и retry видимы. Сложные строки допускают прокрутку. micro references/feedback-patterns.md «Progressive Disclosure» |
| DV-14 **moderate (2)** | Реальные pending/playing animations | C: skeletons background-position1.3s infinite, eq меняет **height**3→10px/800ms. Они state-bound, не «вечный декор», но layout/paint animation расходится с transform/opacity-only целью | apps/web/style.css:972–1003,1124–1129 | DR-03: state-bound static skeleton без shimmer, eq fixed-height scaleY, reduced none. refactoring-ui references/animation-microinteractions.md §Performance; top-design §6 |
| DV-15 **moderate (2)** | Desktop transport/seek hitboxes | L1284: Play x620..664,y779..823; seek input x448..836,y811..855. Rect пересекаются на **44×12px**,27.3% высоты Play. Размер44 сам по себе не гарантирует независимого попадания; фактический ошибочный клик в enabled-состоянии NOT_RUN | apps/web/style.css:1134,1158–1175; input z-index1 | DR-04: два непересекающихся44px ряда внутри88px band, без вертикального padding центра и layout border; hairline отдельным pointer-events:none псевдоэлементом. ux §Error prevention; refactoring-ui §Spacing |

Всего15 находок:0 critical,3 serious,9 moderate,3 minor. DV-01/02/12 требуют раннего исправления. DV-06 — запрос на визуальную эволюцию, он не равен дефекту безопасности или доступности.

## 19 целей меньше 44px: что действительно исправлять

L повторил M на idle Library1284: 12 enabled nav,4 disabled transport28px,3 utility32px (Queue/Mute enabled, Expand disabled). Итого19; **14 enabled,5 disabled**. После выбора current-track пять disabled могут стать доступными; не вычёркивайте их из будущей приёмки.

| Группа | Риск | Решение без разрежения композиции |
| --- | --- | --- |
| 12 nav181×41 | Низкий для мыши, потенциальный для большого touch desktop | height/min-height44, margin0, gap между группами24; glyph20. Ритм около44px сохраняется |
| 4 transport28×28 | Выше: частые мелкие команды | hitbox44, glyph15, face32 только у Play. gap17→4; центр5×44+4×4=236px помещается в470 |
| 3 utility32×32 | Средний: часто Queue/Mute; Expand отдельный путь | hitbox44, glyph17, gap4; rail65 сохранить; font quality не уменьшать ради fit |
| Mobile | Не переносить desktop-дефект автоматически | C style.css:1263–1267 уже44px; нижняя nav>=44. S09b не доказывает bad targets. Проверить фактические rect после всех пакетов |

44px — контракт/дизайн-цель Vela, не заявление, что каждое отклонение само по себе нарушает WCAG AA. Не использовать невидимые перекрывающиеся ::before hit halos. Измерять enabled, disabled, coarse, fine отдельно; disabled не считать активной ошибкой попадания. Menu36px desktop из C1437 — отдельный known dense case, не входит в M19; coarse/mobile уже48px C1468. DR-07 доводит keyboard focus и coarse, не требует раздуть все desktop menu rows.

## Mobile-композиция, пустота и перекрытия

L на320×700: topbar63px; main-scroll y63..510, height447; player y510..630,height120; nav y630..700,height70. Ширина document320. На390×844: main-scroll y63..654,height591; player654..774; nav774..844; document390. Геометрия нижних полос корректна при нулевом safe-area. Safe-area hardware/zoom здесь **NOT_RUN**.

На320px pre-list занимает415px от main-scroll.top до первой строки. Из115px строки видно32px; это результат суммарных отступов и раскрытого в две строки page heading, не overlap player поверх scroller. На390px первый row y430..545 полностью виден; второй y545..660 подрезан границей scroll viewport на6px, и может быть прокручен. Оставить второй ряд track actions: он удерживает44px targets. Не сжимать controls до28px ради ещё одной строки.

В mini-player на320px три44px controls с gap8 занимают148px; левая зона132px в idle. S11 показывает ellipsis короткого приветствия; заменять длинный CTA строкой «Ничего не играет»/EN «Nothing playing» и не рассчитывать, что место покроет полную кириллицу. Полный title доступен в expanded view и accessible name. Существующее conceal album/heart на узких ширинах не означает потерю функции: C style.css:1260,1280.

S12 показывает288px пустой editorial card на390: понятная CTA, но большая пустая нижняя часть отодвигает рабочую Wave. S16 фактически light и подтверждает также DV-01, не dark contrast bug. На desktop S01 после2 строк закономерно остаётся свободная область; не заполнять её рекомендациями, количеством слушателей или ненастоящими covers. S15 при1728 — light evidence, dark ultrawide screenshot для этого baseline **NOT_RUN**.

## Полупрозрачность и motion

Существующее оформление уже содержит modal backdrops: .modal-backdrop blur8, Now Playing ::backdrop14, queue ::backdrop8; также metadata-dialog::backdrop6 в style.css:362. Это всё класс оверлеев. Не считать backdropUsers=[] доказательством их отсутствия. Плеер/топбар/sidebar/panels непрозрачны; это подтверждено M и C.

Относительно DESIGN_DIRECTIVE отсутствуют канонические glass/motion роли и стекло полос. У topbar/player пока нет проходящего под ними содержимого: только rgba+blur даст смену оттенка, не эффект плавающего слоя. DR-02 задаёт ограниченный underlap одного scroller, visual padding и scroll-padding для фокуса. Sidebar и длинные коллекции остаются непрозрачны.

Now Playing уже имеет вход180ms/10px; reduced-motion уже есть в6 блоках style.css и дополнительных CSS. Это сохраняемые механизмы. Новая система сокращает entrance до140ms/4px, drawer180ms/4px, press80ms, cover140ms. Motion cover — только картинка, без crossfade аудио. Поведение reorder обновляется синхронно, settle150ms следует после него.

У интерактивного native dialog визуальный exit не может удерживать top layer/inertness ради анимации: close/restore focus незамедлительны. DR-03 выбирает мгновенный exit для dialog/menu и объясняет исключение, вместо delayed close или копии диалога. Exit100ms допустим только для неинтерактивного status decoration, если не сохраняет данные старой сессии.

Запрещены blur на .track-row/.catalog-grid/.playlist-grid/.listening-card/.home-album-card и больших панелях; полноэкранное стекло самого контента; filter:blur() над страницей; постоянный will-change; анимация layout/background-position, постоянный декоративный shimmer; задержка команды/focus ради motion; более3 одновременно видимых backdrop-filter слоёв с учётом псевдоэлементов. Статический backdrop blur тоже не анимировать. При jank сначала отключается blur, не essential feedback.

## Уточнения и NOT_RUN

Контекстное меню **не native dialog** в этом baseline: main.rs:2510 div role=menu; L: tag DIV, role menu, aria-modal null, height228px. Native dialog — queue-drawer, Now Playing и playlist picker. Это исправление входного предположения, а не повод переписать рабочий menu lifecycle.

Данные docs/DESIGN_SYSTEM.md включают датированные pre-transfer mint/system-font утверждения и раздел «proposed». C style.css:31–79 и S01 уже Manrope/charcoal/pistachio. DR-01 предлагает согласовать текущий раздел документа, сохранив датированную историю и принятые accessibility/audio решения ADR0023. Не откатывать source к старым hex.

Контраст dark timing10px **4.73:1** и quality12px **5.41:1** из M проходит обычный порог4.5:1; размер/разборчивость — отдельный вопрос, это не новый contrast failure. Font CDN, отсутствие reduced-motion и «нет native dialogs вообще» не являются находками.

Полный keyboard обход, live Firefox, screen reader/Orca, true empty library, slow/error fixtures, artwork404, длинная кириллица, увеличенный текст, реальные safe-area, FPS и native остаются NOT_RUN. Снимок reduced-motion — статическое доказательство кадра; дополнительная settled DOM-проба0/0 относится только к Library без pending/playing.
