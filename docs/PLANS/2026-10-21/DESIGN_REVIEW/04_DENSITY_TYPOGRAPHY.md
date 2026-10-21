# DR-04. Сохраните плотность, увеличив реальные цели управления

**Статус:** PROPOSED. **Приоритет:** P1 для transport, P2 для ритма. **Размер:** средний. **Пререквизиты:** нет. Один коммит: плотное, согласованное управление desktop. Mobile pre-list и bands принадлежат DR-06; queue labels/focus — DR-07.

## Цель

Пользователь уверенно попадает в Play/Next/Queue и читает иерархию, а интерфейс сохраняет небольшие glyphs и плотные строки. DV-04/05/08/09 основаны на M:19 целей, отступы13/3px, радиусы7/9px; это не основание увеличивать весь интерфейс или переименовывать каждый SVG font-size в заголовок.

## Файлы к правке

| Путь | Селекторы / строки |
| --- | --- |
| apps/web/style.css | :root27–136 — semantic rhythm roles; nav784–794; .track-header/.track-row830–839; .player-buttons1134–1153; .player-right1211–1223; responsive1216–1228,1543–1563 |
| apps/web/home-editorial.css | Только radius10:22,186 →12; размеры/empty flow не менять |
| docs/DESIGN_SYSTEM.md | Карта размеров, roles и сознательных исключений |
| tests/browser/prototype-transfer.spec.mjs | TR03/REVIEW04: действительные rect и labels; размерыglyph отдельно от кнопки |
| docs/QA_EVIDENCE.md, docs/PROGRESS.md | Измерения и фактическая матрица |

## Что конкретно делать

Не меняйте Manrope, base14, row title12/550 и secondary12. Подписи плеера10/11 допустимы по DESIGN_DIRECTIVE, но не уменьшайте их. Timing10px с4.73:1 и quality12px с5.41:1 проходят обычный порог; при glass контраст проверяется заново DR-02.

Добавьте роли ритма в существующий root перед compatibility aliases, а не отдельный design-system файл:

~~~css
--vela-space-1: 4px;
--vela-space-2: 8px;
--vela-space-3: 12px;
--vela-space-4: 16px;
--vela-space-6: 24px;
--vela-space-8: 32px;
--vela-radius-control: 8px;
--vela-radius-cover: 6px;
--vela-radius-panel: 12px;
--vela-target-size: 44px;
--vela-heading-section: 20px;
~~~

Это имена значений существующей системы, не вторая палитра. Не заменяйте все числа regex. Порядок точечных правок:

1. .sidebar .nav-item:height/min-height44px; margin:0; gap12 вместо13; radius8 вместо7. Group gap24 остаётся. У .sidebar-playlist min-height44 вместо40. Не ужимайте текст/scrollbar; на коротком окне account достигается одним sidebar scroll.
2. .player-buttons .icon-button width/min-width/height44px, padding0; glyph15px сохранить, Play face32/hitbox44 сохранить. .player-buttons gap4px вместо17/12/10;5 controls занимают236px. В width761–900 ограничение центра300 остаётся, если labels/side zones проходят fit.
3. .player-right .icon-button44×44 во всех desktop overrides, glyph17. gap4, volume65px; на<=1100 volume44 и существующее скрытие quality/expand сохранить. Не возвращайте правило30px из1222. Main player height88, seek target44 и visually thin rail3px сохраняются. У current .player-favorite тоже44×44, glyph17: в idle M19 его нет.
4. При min-width761 и max-width1100 задайте .player-bar columns minmax(0,1fr) auto minmax(0,1fr), center max-width:none, чтобы236px не отнимали всю ширину у title/utility. Сохраните существующие доступные paths через Now Playing/Queue. При очень узком zoom150 используйте уже принятую адаптивную композицию, не overlapping hit rectangles.
5. Общие input/select padding10px 13px из minified180 меняйте только по горизонтали на12, вертикальный total target>=44 сохраняйте; gap13 у legacy track grid переадресуйте12 только там, где он ещё применяется. Рамки1px, rail3px, glyph stroke и badge2/5px — осмысленные оптические значения, не whitespace scale bug.
6. .section-toolbar h2:20px/600 вместо21; общий h2 остаётся24/600. h1 .page-heading clamp28..38/650, Home title clamp30..45 и Now Playing clamp24..32 сохраняются как существующие fluid роли; не вводите новую шкалу «только круглые px». Верхняя Home45 — документированное source-исключение относительно директивы19–42, не повод незаметно менять заголовки в transport-пакете.
7. .track-row radius7→8; control/input/button9→8; feature/wave10→12. Cover6, pill50% и modal24 сохраняйте как исключения. Не округляйте hairline, border/rail thickness и44px target к шкале whitespace.

M typeScale19/400,20/400,21/400,24/400 получен по всем nodes, включая SVG и container inheritance. Сначала соберите DOM только text-bearing leaves/h1–h3 с selector, visible rect, font-size/weight; исключите icon span/SVG и скрытые узлы. L: реальные Library h1=34.668/650, h2=21/600. Нельзя массово менять font-size19 у .nav-icon/.icon-button как «заголовки».

Дополнительно устраните **DV-15**: L измерил12px перекрытия Play и seek. Только на>=761px уберите физический border-top у .player-bar; нарисуйте hairline1px через .player-bar::before (position:absolute;top:0;left:0;right:0;height:1px;background:var(--vela-player-border);pointer-events:none). Центр занимает всю высоту88: .player-center height:var(--vela-player-height);margin-block:-10px;gap:0; .player-buttons и .seek-row flex-basis/height/min-height44; .seek-control height44; range top0. Внешний bar сохраняет padding10px по вертикали для боковых зон, но центр отрицательными margins использует всю полосу. Верхний transport row y768..812, seek row812..856 в baseline1284. Если active CSS добавляет другой vertical inset, перенесите центр в полный88px внутренний grid без изменения root DOM; gate — точные rect без пересечения. На mobile оставить120px схему DR-06.

Методика: [refactoring-ui §Spacing/Typography/Hierarchy](/root/.codex/skills/hermes/refactoring-ui/SKILL.md), references/advanced-patterns.md «Border Radius System»; [ux heuristic-conflicts.md](/root/.codex/skills/hermes/ux-heuristics/references/heuristic-conflicts.md): эффективность и discoverability сохраняются через progressive disclosure. Отступление от generically spacious UI осознанное: glyph остаётся маленьким, hitbox увеличивается без роста page cards.

## Критерии приёмки

- В исходной Library1284 среди тех же19 targets остаётся **0** rect с какой-либо стороной<44px. Disabled фиксируются отдельно; после загрузки current track проверяются те же controls enabled.
- Compact sidebar step44px;12nav занимают528px плюс реальные межгрупповые24px, а не добавленные margins поверх каждого пункта. Account/Settings достигаются scrolling.
- В desktop player44px rect не пересекаются, включая transport с range; intersection area0. Glyph15/17 и Play face32 сохранены. Высота player88±1px при100% app scale; hairline не отнимает вертикальное место.
- h220/600, h1 fluid сохранён; в scope gap/padding используются4/8/12/16/24/32, исключения перечислены. Frequency histogram не объявляется количеством дефектов.
- В320/390 нет регрессии существующих44px transport/navigation. Полную mobile-композицию принимает DR-06.
- При zoom80/100/150, viewport768/1284/1440/1728:0 горизонтального overflow и0 перекрытий controls. Ни source, ни queue/cursor не сбрасываются.

## Проверка

Выполните **GATE_WEB из README**: fmt, strict clippy, unit/API, wasm, build-web, Playwright. Перед полным запуском TR03, DESIGN01 и REVIEW04. Измерьте hitbox через getBoundingClientRect, intersection соседей и реальные pointer/focus действия; псевдоэлемент больше кнопки не считается44px.

Снимки Library idle/current track, Home:1284×856,1440×900,1728×1000 обе темы. Regression shots320×700,390×844,768×1024; pointer fine/coarse, reduce. Firefox отдельно проверяет range/native control geometry. Сохраняйте таблицу text-leaves и targets до/после; не требуйте unit-тестов каждого CSS значения.

## Границы

Никакой смены font/палитры, overlay lifecycle, mini-player height, mobile pre-list, stream/session, cookies/CSRF и очереди. Не менять browser_player.rs, manifests, migrations, media_assets, target или runtime. Существующие CSS переупаковать; новых static assets/allowlist paths нет. Без коммита до приёмки тимлида и без публикации.

## Риски

Увеличенные utility targets съедают боковую ширину: не компенсируйте уменьшением текста. Nav может потребовать чуть больше scroll, что допустимо. Поздние responsive overrides способны вернуть28/30px; измерение должно охватить весь каскад. Замена всего13→12 сломает optical dimensions и запрещена.
