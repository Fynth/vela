# DR-01. Введите единый словарь glass и motion

**Статус:** PROPOSED. **Приоритет:** P2. **Размер:** маленький. **Пререквизиты:** нет. Один коммит: канонический словарь и его описание; применение к компонентам — отдельные пакеты.

## Цель

Дайте web-компонентам один набор параметров глубины и отклика, чтобы следующие изменения сохраняли charcoal/pistachio, работали в обеих темах и отключали motion одинаково. Этот инфраструктурный пакет сам не обещает видимого стекла: его пользовательский результат — отсутствие расхождения тем при последующем оформлении полос.

## Файлы к правке

| Путь | Место |
| --- | --- |
| apps/web/style.css | Внутри первого :root:27–136, после --vela-avatar-text:84 и перед layout-ролями:85; light overrides внутри :root[data-theme="light"]:139–179 после avatar:175 |
| docs/DESIGN_SYSTEM.md | Добавьте датированный current-source раздел; старые mint/system-font таблицы сохраните как исторические |
| docs/PLANS/2026-10-21/AUTONOMOUS_TEAM_LEAD/BACKLOG.md | Тимлид фиксирует свой результат; если файла всё ещё нет, используйте BACKLOG_ROWS |

Ориентиры имеют строки исходного HEAD аудита; находите селекторы повторно. Никаких новых CSS-файлов.

## Что конкретно делать

Добавьте dark-роли в канонический :root:

~~~css
--vela-glass-bg: #131513d9;
--vela-glass-blur: 16px;
--vela-glass-blur-mobile: 12px;
--vela-glass-blur-topbar: 12px;
--vela-glass-border: #ffffff14;
--vela-glass-highlight: #ffffff0f;
--vela-glass-shadow: 0 18px 50px #00000059;
--vela-glass-text: var(--vela-text);
--vela-motion-instant: 80ms;
--vela-motion-quick: 120ms;
--vela-motion-overlay: 140ms;
--vela-motion-exit: 100ms;
--vela-motion-panel: 180ms;
--vela-motion-skeleton: 1800ms;
--vela-motion-reorder: 150ms;
--vela-motion-artwork: 140ms;
--vela-ease-enter: ease-out;
--vela-ease-state: ease-in-out;
~~~

В light меняйте только themed glass-роли:

~~~css
--vela-glass-bg: #f4f7f3e6;
--vela-glass-border: #14271c14;
--vela-glass-highlight: #ffffffb3;
--vela-glass-shadow: 0 18px 50px #21372924;
~~~

Параметры blur/time наследуются, повторять их в light не нужно. Fallback каждой полосы — её существующая непрозрачная semantic role (--vela-player-bg / --vela-mobile-player / --vela-bg), а не отдельная палитра. --vela-glass-text — alias существующего foreground для мелких подписей на композитном фоне.

В самом конце style.css добавьте локальный reduced-motion блок для этих восьми duration-ролей: все 0ms. Сохраните существующий глобальный animation:none/transition:none!important в style.css:220 и component overrides. Нулевой токен не заменяет запрет JS/Web Animations; DR-03 обязан проверять media query сам. Не добавляйте скелетон-анимацию только потому, что есть duration token.

В DESIGN_SYSTEM укажите source palette из style.css:42–79 и light139–175 как фактическую; сохраните ADR0023 решения о доступности, одной audio authority, scroll ownership. Отдельно пометьте, что историческая mint-таблица не источник новых hex. Укажите текущий Manrope self-hosted вместо предложения нового font stack.

Методика: [refactoring-ui §Color/Depth](/root/.codex/skills/hermes/refactoring-ui/SKILL.md), references/theming-dark-mode.md; [microinteractions §Feedback](/root/.codex/skills/hermes/microinteractions/SKILL.md), references/feedback-patterns.md (малый общий набор timing). Общие 250–350ms из animation-microinteractions.md сокращены до бюджета DESIGN_DIRECTIVE §3. Ease-out/ease-in-out следуют директиве; запрет этих кривых в top-design здесь не применяется.

## Критерии приёмки

- Определение каждой новой роли одно в dark; light переопределяет только четыре цветовые роли. Нет второй root-палитры внизу файла.
- Alpha dark217/255=0.851, light230/255=0.902; обе >=0.82. Это не доказательство контраста над любой картинкой: проверку делает DR-02.
- Base body14, sidebar216, topbar75, player88 и mobile120+70 не меняются. Текущая source-тема и размеры всех исходных экранов визуально сохранены.
- Все motion duration роли равны0ms при reduce; animated/transition selectors остаются отключены существующими overrides.
- --sidebar остаётся length; --muted остаётся text; --accent action. Cargo/lock/ассеты не меняются.
- DESIGN_SYSTEM отделяет текущую реализацию, исторические решения и PROPOSED DR-пакеты.

## Проверка

Выполните **GATE_WEB из README**: fmt → strict clippy → workspace/unit/API tests → wasm → build-web → isolated Playwright. Статические tokens не требуют зеркальных unit-тестов. Проверьте computed custom properties dark/light/reduce в Chromium и Firefox.

Снимки до/после: Library1284×856 и390×844, обе темы; Home1284×856 light как контроль уже исправленного DR-05. Pixel diff допускает только результаты ранее принятых пакетов, сам DR-01 не меняет layout. Сохраните значения tokens рядом с артефактами QA.

## Границы

Не применяйте backdrop-filter, новые transitions, opacity на controls в этом коммите. Native-токены входят в DR-08. Не меняйте browser_player.rs, playback.rs, source/session guards, manifests, migrations, media_assets, target/, frozen runtime. Нет новых assets — allowlist не расширяется. Любой последующий отдельный asset требует точного server allowlist и packaged verification.

## Риски

Поздние legacy-правила могут переопределять потребителя, поэтому ввод tokens ещё не внедрение. Наследование light имеет значение для aliases. Более прозрачный фон ухудшает contrast quiet10px; DR-02 использует primary text на стекле и проверяет композицию. Native light/system selector отличается и переносится отдельно.
