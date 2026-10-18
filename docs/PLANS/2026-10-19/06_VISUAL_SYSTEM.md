# Vela Listening Atlas — визуальный язык

Предложенная новая система, **не применена** к production CSS. Существующие Vela
имя/wordmark и музыкальная идентичность сохраняются. Развитие идёт theme-by-theme
и component-by-component через visual regressions, не тотальный rewrite.

## 1. Концепция

Музыкальный атлас и обложки небольших независимых издательств: содержательная
типографика, тонкая сетка, тёплый материал, крупная настоящая обложка, маленькие
каталожные индексы. Приложение ощущается личной коллекцией, а не SaaS-дашбордом.
Асимметрия используется в hero, регулярность — в управлении/таблицах.

Профессиональные ориентиры — ясная сетка Massimo Vignelli, функциональная дисциплина
Dieter Rams, выразительная редакционная иерархия Paula Scher, осмысленность пиктограмм
Susan Kare. Это направления собственного решения, не копирование конкретных работ,
не заявление об их участии/одобрении. Источники и проверяемые reference products —09.

Пять собственных приёмов:

1. **Folio spine:** тонкая вертикальная ось и маленькие номера полок/контекстов;
   нумерация не превращается в технический ID трека.
2. **Listening aperture:** круглая прорезь в геометрической обложке для playlist/mix
   fallback; настоящие album covers не обрезаются автоматически кругом.
3. **Offset title:** serif display слева, маленькая utility-метка справа; serif только
   на редакционных заголовках, не на каждом значении таблицы.
4. **Origin stitch:** короткая маркировка источника рядом с названием там, где это
   важно; доступ/качество сопровождаются текстом, не одной цветной точкой.
5. **Quiet transport:** чёткая линия seek, один сильный Play, прочие controls спокойные.

Не использовать: фиолетовые mesh gradients, стеклянные карточки на каждом экране,
огромные пустые hero, случайные neon outlines, emoji вместо icons, бессмысленные
waveforms, fake popularity/social avatars, одинаковые «AI карточки» с тремя CTA.

## 2. Детерминированность вместо случайного дизайна

Design seed string: `vela:listening-atlas:v1`.
SHA-256: `f50a418f8f7d2c44d1cfff16e1622d35f8f8c679b125c4bd771467d2abb47186`.
Это идентификатор композиционного набора, не секрет, credential или криптозащита.

Сетка, typography, contrast, навигация и порядок действий **не рандомизируются**.
Seed используется только для оригинальных пустых playlist/mix covers:
`SHA256("vela-art-v1" || scoped_public_object_id)` → один из 6 проверенных layouts,
angle∈{-12,-6,0,6,12}, aperture offset∈{0.36,0.50,0.64}, palette pair из allowlist.
Не брать hash из password/history/private taste; не менять рисунок каждый render.
Стабильность воспроизводима в snapshot tests. Unknown album получает честный нейтральный
fallback, а не выдуманную художественную обложку официального релиза.

## 3. Цвет и семантика

| Token | Light | Dark | Назначение |
|---|---|---|---|
| canvas | `#F4F0E6` | `#1D1C19` | Основной фон |
| surface | `#FFFCF5` | `#292722` | Панели/карточки |
| surface-alt | `#E9E2D5` | `#353129` | Selection/disclosure |
| text | `#25231E` | `#F4EFE5` | Основной текст |
| muted | `#635C50` | `#B5AFA2` | Вторичный readable текст |
| accent | `#B74330` | `#F1B887` | Play/focus/selected accents |
| on-accent | `#FFFFFF` | `#231B14` | Текст/иконка primary button |
| link | `#3152A4` | `#ABC4FA` | Ссылки/connected context |
| border | `#C8BEAE` | `#5C564B` | Неключевые separators; controls при необходимости сильнее |
| success | `#356447` | `#A8CBA6` | Только действительный ready/success |
| danger | `#A42E23` | `#FFB4A7` | Ошибка + текст/значок |

Проверять contrast **пар**, включая hover/disabled/selection/placeholders. Цвет outline
тоже должен быть виден на обеих adjacent surfaces. WCAG2.2AA — baseline: нормальный
текст 4.5:1, крупный 3:1, essential UI3:1; продуктовые touch targets44×44CSSpx.
WCAG AA target-size minimum24px с исключениями не переименовывать в 44px стандарт.
Focus минимум 2px ring+2px offset, не ниже 3:1 как внутренний design gate.

Artwork palette может влиять на декоративную поверхность, не на семантические цвета.
Автоматическое выделение доминирующего цвета bounded/local и contrast-clamped.

## 4. Типографика с полноценной кириллицей

- Display: **Source Serif4**, optical sizing, weight500–600; заголовки 32/40/56px,
  line-height1.05–1.16, без искусственного negative tracking на маленьком размере.
- UI/body: **Golos Text**,400/500/600,14–16px, line-height1.4–1.55; phone body16px.
- Utility/time: Golos Text tabular nums; monospace system stack только technical panel.
- Eyebrow11–12px, умеренный tracking≤0.12em, не для длинных важных описаний.
- Контент не truncate без доступного полного названия; hero wrap, table ellipsis+
  details/tooltip; реальные long names проверяются 200%text и несколькими scripts.

Оба семейства имеют OFL/source provenance, Golos содержит Cyrillic/Latin subsets;
metadata/лицензии —09. Font assets в дизайн-борде самостоятельные локальные copies,
не новый обязательный CDN. В приложении self-host WOFF2+license+font-display swap;
subset не вырезает русский/имена коллекции. Font budget измеряется на реальном WASM UI.

## 5. Геометрия

Spacing:4/8/12/16/24/32/48/64. Большие editorial gaps не используются внутри controls.
Radius:4 малые,8controls,12cards,16sheets; pill только filter/tag/transport. Не скруглять
каждый крупный контейнер 32px. Border1px; shadow только floating layer, не весь grid.
Icon24px/1.75stroke, interior optical fit;16px utility имеет 44px hitbox на touch.

Desktop grid: sidebar248px (range224–360), topbar64px, right panel320 (280–420),
player96px; content gap32px, max editorial width1360px, wide viewport не растягивает
строку текста до 2 метров. Under1280 right panel collapses to drawer; under960 sidebar
compact; under720 phone IA, а не скрытые critical actions. Native Linux windowmin640
пока не становится обещанием phone support; shared mobile UI тестируется отдельно.

Track row56px comfortable/44compact pointer; phone≥64px с 44px controls. Album art
desktophero224–320px; phone полноэкранный artwork ограничен свободной высотой,
transport остаётся доступным. Мини-плеер 64px плюс safe areas/nav60px.

## 6. Компоненты и важные варианты

| Компонент | Варианты/состояния | Деталь |
|---|---|---|
| Button | Primary/secondary/quiet/danger, busy/disabled/focus | Busy не меняет ширину; причина disabled рядом |
| MediaCard | Album/playlist/mix, missing art/loading/playing | Главный CTA Play; secondary в context menu |
| TrackRow | Current/paused/unavailable/metadata-only/selected | Иконка+текст, action roles не конфликтуют |
| Hero | Editorial/album/artist/playlist | Реальные metadata, no fake counters |
| SearchField | Idle/suggestions/loading/error/composition | Query status не сдвигает input |
| OriginBadge | Local/live/cached/denied/offline | Text+icon; cached не current grant |
| Player | Full/bottom/mini/prepare/error/offline | Один state/source, реальные progress |
| Dialog/sheet | Confirm/share/edit/conflict | Initial focus/return/inert/top layer |
| Toast | Success/undo/error/retry | Keyboard reachable, не закрывает seek |
| DownloadRow | Plan/active/verifying/ready/corrupt | Actual bytes, cancel/retry и reason |
| Skeleton | Card/track/hero | Реальная загрузка, stable layout |

## 7. Motion и sound UX

Pointer feedback90–120ms, drawer/modal160–200ms opacity/transform, максимум 2 движущихся
layers. Hover scale≤1.015 только artwork CTA, не whole track table. Никаких expensive
background blur или постоянных shadows на 1000 строках. Reduced motion→без slide/scale,
state feedback сохраняется. Equalizer decoration только при actualPlaying, не CSS
«waveform» без связи со звуком. Не генерировать notification sound во время музыки.

## 8. Порядок внедрения дизайна

1. Snapshot текущего поведения/тем; tokens names и palette accessibility test.
2. Local fonts/licenses, typography test на ru/en/long scripts; no runtime CDN.
3. Buttons/inputs/focus/icons/spacing, затем shell; feature flag для rollback.
4. Home/Search/Library; shelf density и actual artwork/fallback.
5. Album/artist/playlist/track/context menu.
6. Bottom player + right panel; не перемещать/создавать media node.
7. Phone tab bar/mini/full player/bottom sheets/safe areas.
8. Downloads/network/social/settings/admin применяют те же primitive components.
9. Compare screenshot+task usability, не «кажется красивее»; profiling rendering.
10. Удалить legacy CSS только после полноценных функциональных/visual gates.

## 9. Правила дизайн-борда

`design/vela-atlas-board.html` показывает **концепт**, а не подключённый player.
Иллюстративные названия/числа явно не реальные пользовательские данные. Все геометрические
covers созданы здесь из SVG/CSS. Иконки собственные; нет заимствованных логотипов,
фото знаменитостей, Spotify artwork или чужого copyrighted интерфейса.
Переключатели представления/темы — управление дизайн-бордом; нарисованные Play
помечены как демонстрация. Screenshot comparison — артефакт review, не UX acceptance.
