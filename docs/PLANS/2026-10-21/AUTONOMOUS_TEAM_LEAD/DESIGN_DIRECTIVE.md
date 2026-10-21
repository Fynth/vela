# Директива UX/UI: полупрозрачные слои, motion и плотная типографика

Дата: 2026-10-21. Область: web (`apps/web`) и Linux native (`apps/desktop`).
Статус: **директива для автономного тимлида**, обязательная к соблюдению при
изменении UI. Она не отменяет `docs/DESIGN_SYSTEM.md` и ADR0023, а конкретизирует
их в части полупрозрачности и анимаций.

Источник истины — код: `apps/web/style.css` (1640 строк), `now-playing.css`,
`home-editorial.css`, `queue-drawer.css`, `apps/desktop/style.css`.

## 0. Что уже есть (не переделывать)

Проверено в исходниках на 2026-10-21:

- **Канонический слой ролей** `apps/web/style.css:27-136` — `--vela-*` роли плюс
  legacy-алиасы; светлая тема — отдельная адаптация `:root[data-theme="light"]`
  (`style.css:139-179`). Новые цвета добавляются **туда**, а не в отдельную палитру.
- **Полупрозрачность есть только у оверлеев**: `style.css` `.modal-backdrop`
  (`blur(8px)`), `now-playing.css:46` `.now-playing-dialog::backdrop` (`blur(14px)`),
  `queue-drawer.css:24` `.queue-drawer::backdrop` (`blur(8px)`). Панели, сайдбар,
  топбар и плеер — непрозрачные.
- **Motion-бюджет мал и корректен**: `transition` в `.button`, `.icon-button`,
  `.track-row`, `.playlist-card`; `@keyframes`: `spin`, `vela-pulse`, `vela-skeleton`,
  `vela-track-loading`, `vela-player-eq`, `np-enter`.
- **`prefers-reduced-motion` уже присутствует** в 6 блоках `style.css` плюс в
  `now-playing.css`, `queue-drawer.css`, `home-editorial.css`. Он не может быть
  удалён ни одним новым правилом.
- **Шрифты**: Manrope self-hosted, latin + cyrillic subset, `font-display:swap`
  (`style.css:9-26`, `index.html:10-11`). CDN не допускается.
- **Контекстное меню реализовано** (`apps/web/src/main.rs:1142+`, `.track-menu-panel`
  `style.css:1436-1467`): right-click, Shift+F10, ContextMenu, touch-sheet.
- **Drawer очереди** — `apps/web/src/queue_drawer.rs`, нативный `<dialog>`.
- **Now Playing** — `apps/web/src/now_playing.rs`, нативный `<dialog>`.

## 1. Главный принцип: стеклянные слои — только плавающие

Spotify-подобная полупрозрачность — это размытие фона **плавающих** слоёв над
содержимым, а не «стеклянные» длинные списки. Правило:

| Слой | Прозрачность | Почему |
|---|---|---|
| Нижний плеер (web, десктоп) | glass, 12–18px blur, `backdrop-filter` | ощущение Spotify, видно контент под собой |
| Мини-плеер (web, mobile ≤760px) | glass, 10–14px blur | совпадает с desktop-правилом |
| Топбар | частичная (полупрозрачный фон) | контент уходит под шапку |
| Сайдбар | непрозрачный | длинный список, читаемость важнее эффекта |
| Оверлеи: очередь, Now Playing, контекст-меню, тосты | glass + затемнение | уже есть `::backdrop`, доводится |
| Длинные списки/сетки/панели | **непрозрачные** | blur на сотнях строк = jank и потеря читаемости |

**Запрещено:** blur на `.track-row`, `.catalog-grid`, `.playlist-grid`,
`.listening-card`, `.home-album-card`, длинных панелях, а также «вечная»
(неизменяемая) анимация фона. `backdrop-filter` применять только к элементам,
которые действительно перекрывают контент.

**Правило для прозрачности:** `backdrop-filter` всегда с `@supports`, плюс
непрозрачный fallback-цвет под ним, чтобы слой оставался читаемым без blur
(Firefox/Safari рендерят иначе — это уже зафиксировано в
`docs/DESIGN_SYSTEM.md:99`).

**Производительность:** `backdrop-filter` допустим не более чем на **трёх**
одновременно видимых плавающих слоях. `will-change`/`filter: blur()` по контенту
запрещены. Если страница с glass-слоями теряет кадры — blur убирается первым,
эффекты не трогают.

## 2. Токены полупрозрачности (добавить в канонический слой)

Определения — рядом с `--vela-*` ролями, чтобы наследовались темой:

```css
:root {
  --vela-glass-bg: #131513d9;        /* фон плавающего слоя, dark */
  --vela-glass-blur: 16px;
  --vela-glass-border: #ffffff14;
  --vela-glass-shadow: 0 18px 50px #00000059;
  --vela-glass-highlight: #ffffff0f; /* верхняя грань стекла */
}
:root[data-theme="light"] {
  --vela-glass-bg: #f4f7f3e6;
  --vela-glass-blur: 16px;
  --vela-glass-border: #14271c14;
  --vela-glass-shadow: 0 18px 50px #21372924;
  --vela-glass-highlight: #ffffffb3;
}
```

Правила: фон — `color-mix`/hex с alpha **не ниже 0.82 по непрозрачности** для
текста ≥12px; 1px hairline-грань; внутренний highlight только на верхней кромке;
тень — одна, мягкая, не «glow».

## 3. Motion-токены (время — проектное правило, не украшение)

| Токен | Время | Где применяется |
|---|---|---|
| `--vela-motion-instant` | 0–80ms | pressed/active, отклик на ввод |
| `--vela-motion-quick` | 100–140ms | hover, смена фона, раскрытие меню |
| `--vela-motion-overlay` | 120–160ms | вход оверлея, opacity + ≤4px сдвиг |
| `--vela-motion-exit` | 80–120ms | закрытие; фокус и блокировка снимаются **сразу**, не после анимации |
| `--vela-motion-panel` | 160–220ms | drawer/peek, если без layout-jank |
| `--vela-motion-skeleton` | 1600–2200ms | shimmer, только во время реального pending |
| `--vela-motion-reorder` | 120–180ms | settle после реального reorder |

- Кривая: `ease-out` на вход, `ease-in-out` на статусные переходы. Одна
  общая система, без двадцати независимых spring-поведений.
- Анимируются **только** `transform` и `opacity` (и `filter` у backdrop-слоёв).
  Никогда не анимируются `height`, `width`, `top`, `left` — только `transform`.
- `prefers-reduced-motion: reduce` → мгновенный переход, `animation:none`,
  `transition:none`, `scroll-behavior:auto`. Это правило **уже есть**; новое
  правило обязано иметь собственный reduced-motion override, иначе оно нарушит
  существующий контракт.
- Никакой анимации, имитирующей прогресс: реальные байты/проценты — только из
  сервера (`/api/jobs`), как в `read_import_activity`.

## 4. Что анимировать (конкретно, по текущему коду)

1. **Плеер** (`.player-bar`, `style.css:1083+`): glass-фон; при смене трека —
   crossfade обложки ≤140ms (presentation-only, **не** crossfade звука);
   `.player-eq` (`style.css:1118-1127`) уже анимируется при `data-state="playing"`.
2. **Мини-плеер** (`.mobile-player`, mobile-композиция ≤760px, `style.css:1237+`):
   тот же glass, что и десктоп, иначе web↔native/web расходятся.
3. **Топбар** (`.topbar`): полупрозрачный фон, чтобы контент уходил под шапку;
   `:hover`/`:focus-within` — только фон/тень, без сдвига.
4. **Drawer очереди** (`queue-drawer.css`): вход `np-enter`-подобным opacity+4px;
   выход — мгновенный focus-возврат.
5. **Контекст-меню** (`.track-menu-panel`, `style.css:1436`): вход 100–140ms
   opacity+scale(0.98→1) **внутри** фиксированного хитбокса; на touch-ширине —
   существующий bottom-sheet, без изменений геометрии.
6. **Карточки каталога**: `:hover` — `translateY(-2px)` + рост тени, как у
   `.playlist-card` (`style.css`, `transition:background-color .15s,transform .15s`);
   картинка — `transform: scale(1.03)` 350ms, как `.home-album-card .catalog-cover img`
   (`style.css:1352`).
7. **Скелетоны**: `vela-skeleton`/`vela-track-loading` уже есть и обязаны
   отражать **реальный** pending, а не висеть вечно.

## 5. Типографика и плотность (как в Spotify, но свои)

- Основной текст 14px, трек 12px, вторичные подписи 10–11px, eyebrow 8–10px
  uppercase с трекингом, заголовки 19–42px. Уже задано в `style.css` и
  `docs/DESIGN_SYSTEM.md` — **не переопределять**.
- Плотные ряды вместо «карточной сетки на каждую мелочь»: строки, обложки,
  отрицательное пространство, типографика (`08_VISUAL_LANGUAGE_NO_CARD_GRID.md`).
- Основные цели — 44px (`.button` уже `min-height:44px`).
- Радиусы: 6/8/12/16/20px; hairline-границы 1px; тени — одна, мягкая.
- Цвет: `charcoal #131513` / `warm white #f3f3ed` / `pistachio #d3e9a5` (primary
  **непрозрачный**), `forest #0e100f`. Никаких бирюзовых Spotify-гринов, неона,
  синих CTA, emoji-иконок, градиентных заголовков.

## 6. Адаптив (обязательный набор проверок)

320, 360, 390, 768, 1284×856, 1440, 1728. На ≤760px — **одна** мобильная
композиция: мини-плеер 70/120px (`--vela-mobile-nav-height`, `--vela-mobile-player-height`)
над навигацией, safe-area insets, ни одна строка не перекрыта. Sidebar на web —
один скролл-регион, не схлопывается в «десять пунктов меню».

## 7. Доступность (неотделимые от дизайна)

- Фокус: `outline:2px solid var(--accent)` + offset — виден всегда.
- Диалог: `::backdrop` затемняет и размывает фон, фокус внутри, возврат фокуса
  на триггер. Оверлей не может быть «вторым» фокус-ловушкой.
- Ошибка/статус никогда не только цветом: текст + иконка.
- `prefers-reduced-motion` — см. §3.
- Длинный кириллический текст, пустая библиотека, отсутствие обложек — это
  обязательные состояния рендера, а не edge-case.

## 8. Ограничения, которые действуют всегда

- Никаких фейковых обложек: реальные — только через авторизованный
  `/api/artwork/{id}`; иначе оригинальный нейтральный плейсхолдер.
- Никаких декоративных контролов без действия.
- Никакого «пиксель-перфект», WCAG-сертификации и скринридер-сертификации без
  соответствующих проверок.
- Новый статический ассет → точный allowlist сервера
  (`apps/server/src/web_assets.rs`, `apps/server/src/lib.rs`) + копирование в
  `scripts/build-web.sh` + тест упаковки.
- Native (`apps/desktop/style.css`) — свои токены `--vela-*`; glass и motion
  выглядят так же, но через WebKitGTK: если `backdrop-filter` не поддержан,
  обязателен непрозрачный fallback.

## 9. Приёмка UI-пакета

1. `cargo fmt --check` → строгий clippy → `cargo test -p vela-web` → wasm-сборка
   → `scripts/build-web.sh`.
2. Playwright (`tests/browser`, chromium+firefox) — реальный сервер на 127.0.0.1:3217.
3. Скриншоты: 1284×856, 390×844, 360×800, 320, 1440, 1728; тёмная и светлая
   темы; `prefers-reduced-motion: reduce`; длинный кириллический текст; пустая
   библиотека; трек без обложки.
4. Отдельная проверка: blur действительно виден в Chromium; в Firefox читаемость
   сохраняется без blur; при reduced-motion нет ни одного transition/animation.
5. Фокус-обход по всем новым интерактивным элементам; 44px-цели измерены.
6. Итог пишется в `docs/QA_EVIDENCE.md` с путями артефактов.
