# DR-02. Сделайте плеер и топбар читаемыми плавающими слоями

**Статус:** PROPOSED. **Приоритет:** P2. **Размер:** большой. **Пререквизиты:** DR-01 и DR-04 (непересекающиеся transport/seek targets). Один коммит: один main-scroll с проходящим под полосами содержимым и оформлением полос. Связанные геометрические assertions обновляются в этом же коммите.

## Цель

Покажите продолжение библиотеки под плеером и шапкой, сохраняя компактное управление, readable text и доступ к последней строке. Пользователь получает ощущение плавающего transport без стекла в коллекции. DV-06 подтверждён M/S01 и style.css:180–205: сейчас полосы непрозрачны, а контент физически заканчивается перед ними.

## Файлы к правке

| Путь | Селекторы / строки |
| --- | --- |
| apps/web/style.css | .main-content/.topbar:180; .main-scroll/.content-inner:189–205; mobile shell1022–1073; .player-bar1090; mobile surface1252; новая тематическая часть после существующих component rules, до завершающих reduce overrides |
| tests/browser/prototype-transfer.spec.mjs | TR02/TR03/TR09, особенно safe-area geometry:2026+,2522+,2666+ |
| tests/browser/design.spec.mjs | DESIGN01: измерение composited text/focus, scroll-to-end |
| docs/DESIGN_SYSTEM.md, docs/QA_EVIDENCE.md, docs/PROGRESS.md | Только фактические результаты этого пакета; performance scope при измерении также docs/PERFORMANCE.md |

main.rs:2373–2385 остаётся ориентиром существующего DOM. Перестройка root компонентов и новый JS controller не нужны.

## Что конкретно делать

1. Сохраните shell grid, sidebar column, bottom rows и один HTMLMediaElement. Для реального underlap задайте .main-content grid-row:1 / -1 на desktop и mobile, с существующим grid-column2 / mobile1. Sidebar остаётся row1; player остаётся своим row2, mobile-nav row3. Не добавляйте второй fixed player.
2. Топбар остаётся sibling перед .main-scroll, с его текущими height/flex-basis. На .main-scroll задайте отрицательный margin-top, равный calc(-1 * (var(--vela-topbar-height) + var(--vela-safe-top))); padding-top:calc(var(--vela-topbar-height) + var(--vela-safe-top)) для desktop и mobile. Desktop safe-top тоже уже учитывается в style.css:1529–1535. Border-box/negative margin возвращают верх scroller под header; padding сохраняет начальный y первого заголовка. .main-content остаётся overflow:hidden, .main-scroll единственный overflow-y:auto.
3. В .content-inner сохраните текущие top/side insets и задайте padding-bottom:calc(var(--vela-bottom-reserve) + 24px), на mobile +20px. Это **единственный** дополнительный контентный отступ до нижней безопасной границы при span-through layout. Не добавляйте ещё spacer/margin на page. Grid band по-прежнему позиционирует transport, но main теперь занимает все rows.
4. Добавьте .main-scroll scroll-padding-top:calc(высота topbar + 8px), scroll-padding-bottom:calc(var(--vela-bottom-reserve) + 8px). На focus нужный target целиком прокручивается в эффективный viewport между полосами. При initial scroll=0 первые заголовки сохраняют y88 на mobile с нынешним25px inset; при scroll-to-end последняя строка поднимается выше player.
5. Не анимируйте эту геометрию. Сохраните topbar z5, player z40, nav z45. .main-scroll задайте position:relative; z-index:0, без transform/filter/isolation на предках. Проверьте fixed notices DR-05: они не должны попасть в clipped stacking context.

Стекло включайте прогрессивно. Сначала оставьте непрозрачный background каждой полосы. В одном @supports ((backdrop-filter:blur(1px)) or (-webkit-backdrop-filter:blur(1px))) примените к .player-bar и .topbar:

- background:var(--vela-glass-bg); color:var(--vela-glass-text);
- backdrop-filter и -webkit-backdrop-filter:blur(var(--vela-glass-blur)); для topbar12px, для mobile player12px, desktop player16px;
- player hairline1px через существующий ::before из DR-04 с background:var(--vela-glass-border), без физического desktop border-top; mobile может сохранить border1px; inset highlight 0 1px var(--vela-glass-highlight);
- topbar border-bottom той же роли; inset highlight по верхней кромке;
- одна внешняя мягкая тень из --vela-glass-shadow только у player, topbar без второй внешней тени; hairline не считается заменой focus.

Уточните specificity: mobile .app-shell>.player-bar:1252 иначе вернёт opaque background. Размещайте @supports desktop и mobile после этого override, а не перед ним. Не создавайте селектор .mobile-player — такого DOM-класса нет, mobile использует тот же .player-bar.

В glass-ветке переадресуйте .player-time, .player-artist, .player-album, .player-quality-format, .player-quality small и .topbar .breadcrumb к --vela-glass-text. Essential glyphs на glass также должны давать>=3:1; при необходимости используйте этот же semantic foreground. Размеры/weight обеспечивают вторичность подписей; не снижайте opacity текста поверх картинки. Dark timing4.73 из M не гарантирует4.5 на новом фоне.

Оставьте sidebar, nav, .track-row, grid/card/list panels и внутренние панели dialog непрозрачными. Уже имеющиеся ::backdrop остаются третьим фильтруемым слоем: 2 shell bands + 1 modal backdrop =3. Не добавляйте blur ещё и .queue-panel/.np-layout/toasts. На desktop menu оставьте существующий opaque popover. Ни blur целой страницы через filter, ни background canvas.

При reduce сохраняйте статичный glass, но transition:none/animation:none для обеих полос и псевдоэлементов. При forced-colors:active и prefers-reduced-transparency:reduce верните opaque role, backdrop-filter:none/-webkit:none. Для print отключите blur и тень. @supports отсутствие — тот же opaque fallback. Если composited contrast не проходит, верните **всю соответствующую полосу** к opaque fallback; не подбирайте вне tokens новые hex.

Методика: [refactoring-ui §Depth/Color](/root/.codex/skills/hermes/refactoring-ui/SKILL.md), references/theming-dark-mode.md; [top-design §Performance](/root/.codex/skills/hermes/top-design/SKILL.md). Underlap — необходимая часть пользовательского исхода, а не декоративная смена RGB.

## Критерии приёмки

- При scroll реальная существующая обложка/строка проходит под верхней и нижней полосой; glyph/text controls остаются резкими. На первом кадре нет скачка layout.
- Ровно2 shell backdrop-filter users, не больше3 видимых с открытым modal; считайте ::before/::after/::backdrop и vendor-prefixed property. Ноль фильтров на long lists.
- Normal text>=4.5:1, large text>=3:1 по корректной классификации; focus/control glyph>=3:1. Проверяйте composite над светлым cover, тёмным cover, placeholder и при scrolling. M-скрипт с opaque ancestor для этого не подходит.
- Main-scroll один. Effective visible rect: top=scroller.top+topbar-height, bottom=scroller.bottom-bottom-reserve. Last focused44px target полностью между этими границами с8px запасом после прокрутки.
- Нижний reserve: desktop88; mobile120+70+safe-bottom. Safe-top включён ровно один раз. Первое/последнее содержимое не теряется при zoom80/100/150 и browser200%.
- Chromium/Firefox сохраняют source/audio node/queue/cursor/position при scroll, theme и open/close. Blur может выглядеть иначе; fallback остаётся читаем.
- Reduce:0 ненулевых transitions/animations на новых элементах; geometry меняется мгновенно.

## Проверка

Выполните **GATE_WEB из README** (fmt, strict clippy, unit/API, wasm, build-web, Playwright). Сначала TR02/TR03/TR09. Их прежний rect.bottom=player.top описывал неперекрывающиеся siblings: замените **только это** на effective safe rect с двумя точными полосами. Сохраните assertions одного scroll owner, source identity, safe-area once и последнего focus target. Не удаляйте тесты ради зелёного gate.

Снимки Library/Home: 320×700,360×800,390×844,768×1024,1284×856,1440×900,1728×1000; dark/light, scroll0/middle/end. Отдельно1284 и390 с Queue/Now Playing, reduce и opaque fallback. Chromium и Firefox. Для dense long list используйте действительные owned fixtures в изолированном сервере, не mock-popularity.

Снимите короткий performance trace 10s scroll с blur и opaque fallback в одинаковом viewport. Запишите browser/device/частоту дисплея и долю кадров >33ms. Если blur добавляет >5 процентных пунктов таких кадров в сравнении на том же стенде, используйте opaque fallback в этой конфигурации. Это локальный бюджет, не обещание60fps на всех устройствах.

## Границы

Не меняйте playback/source/auth guards, Now Playing или queue ownership, data/API, animation lifecycle, manifests и новые assets. Не трогайте чужой browser_player.rs, media_assets, target checkpoints, runtime. Existing allowlisted CSS переупаковывается build-web; новые static paths запрещены этим пакетом. Публикации нет.

## Риски

Это наиболее рискованный пакет: underlap меняет геометрию scroller и old assertions. Возможны focus под полосой, дублирование safe-area, clip fixed notices и Firefox stacking differences. При невозможности сохранить effective safe rect отклоните пакет целиком; простое translucent background без underlap не считать выполненной целью. Native перенос требует отдельного DR-08.
