# DR-08. Перенесите допустимое оформление в Linux WebKitGTK

**Статус:** PROPOSED. **Приоритет:** P2. **Размер:** большой. **Пререквизит:** согласованный словарь DR-01. Web-реализация DR-02/03 не является доказательством native. Один коммит: ограниченный native presentation parity с отдельными window/input/audio evidence.

## Цель

Пользователь Linux получает те же цвета плавающих полос, короткий отклик и читаемый фокус, сохраняя родные оконные controls и один AudioEngine. Пакет не обещает native Home/Wave/Now Playing, которых web-скриншоты не доказывают. Native source baseline .native-player/.native-topbar непрозрачны; фактический рендер нового оформления **NOT_RUN**.

## Файлы к правке

| Путь | Место |
| --- | --- |
| apps/desktop/style.css | .native-app roles15–76; light80–115; layout133–179; .native-player281–340; compact383–442; system/light484–519; reduced125,600,609 |
| apps/desktop/src/ui.rs | Только при необходимости presentation labels/classes/aria; без HTTP/audio/queue изменений |
| apps/desktop/examples/native-ui-qa.js | DOM geometry/focus/contrast/blur counters в реальном WebKitGTK |
| apps/desktop/examples/native-ui-qa.rs | Только дополнительные viewport/theme checks при нехватке driver, не второй engine |
| docs/NATIVE_CLIENT_CONTRACTS.md, docs/PLATFORM_MATRIX.md | Отдельные source/window/input/audio/background/packaging evidence |
| docs/DESIGN_SYSTEM.md, docs/QA_EVIDENCE.md, docs/PROGRESS.md, docs/KNOWN_LIMITATIONS.md | Фактический scope и отсутствующий паритет |

## Что применимо и что не переносить

| Web-пакет | Native действие |
| --- | --- |
| DR-01 tokens | Перенести **значения** glass/motion в .native-app; light и system/light — разные selectors |
| DR-02 player/topbar glass | Применить только к .native-player/.native-topbar, после проверки настоящего overlap и fallback |
| DR-03 feedback | Только transform/opacity на существующих native buttons, без web timers/dialog code; artwork fade лишь при уже авторизованном готовом изображении |
| DR-04 density | Existing native transport уже44px (style314), не уменьшать до web baseline28; проверить range22px:337 и сделать реальный44px target |
| DR-05 states | Semantic colors и краткие реальные ошибки; не добавлять browser autoplay copy в Rodio client |
| DR-06 mobile | Не копировать HTML mobile-nav и320px web composition в Linux desktop. Проверять native compact640×700 и его собственную навигацию |
| DR-07 a11y | Focus/labels/keyboard в WebKitGTK проверять заново. DOM/AT bridge и оконный drag — отдельные взаимодействия |

В .native-app после current color roles и перед layout49 перенесите DR-01 glass-роли и durations. Light80 и system+prefers-color-scheme:light484 переопределяют четыре glass цвета так же, как web. Нельзя изменить только explicit light и оставить System с dark glass. --bg/--raised/--hover/--accent остаются aliases, а не новым набором hex.

**Underlap:** родной shell тоже разрезан на main row и player row. Для normal fixed112px player после исправления range targets примените проверяемую схему DR-02 к существующему DOM: .native-main grid-row:1/-1; .native-scroll margin-top:calc(-1 * var(--vela-topbar-height)), padding-top:var(--vela-topbar-height), scroll-padding-top:calc(var(--vela-topbar-height) + 8px), scroll-padding-bottom:calc(var(--vela-player-height) + 8px). .native-scroll-inner padding-bottom:calc(var(--vela-player-height) + 32px); сохраните боковой gutter и текущий top inset. Player остаётся row2, sidebar row1; native-topbar position:relative;z-index5, native-player position:relative;z-index40, scroller position:relative;z-index0. Никаких transforms на native window/drag ancestors.

**Динамическая высота:** style.css:380–405 увеличивает player при error/preparation/compact. Положительное условие glass/underlap задайте строго: viewport>=1101px, app scale100% (data-app-scale="100" либо атрибут отсутствует), нет .player-error и .preparation-progress. Все остальные сцены используют исходную неперекрывающуюся схему: .native-main grid-row:1, .native-scroll margin-top0;padding-top0; .native-scroll-inner исходный padding-bottom32/compact24; player/topbar opaque. Так не нужен новый UI polling высоты или второй reserve authority. Glass/underlap selectors помещаются только внутрь положительного условия и @supports; compact fallback — намеренный поддержанный результат.

В @supports стандартного или prefixed backdrop-filter включите dark/light glass фон, desktop blur16px player и12px topbar, hairline1px, верхний inset highlight, одну мягкую player shadow. Scope и контраст те же, что DR-02; essential small text uses --vela-glass-text. До @supports — existing opaque roles. Backdrop count<=3, в normal shell ожидается2. Не размывайте .native-track/.native-scroll/.native-queue-row и sidebar.

При unsupported filter оставляйте opaque fallback и исходную geometry без underlap; при локальном jank отключайте ветку glass/underlap целиком в проверяемой конфигурации. Reduce:все durations0ms, transitions/animations:none!important, static hover transform:none. Forced-colors/print/reduced-transparency — opaque. Внешняя прозрачность GTK-окна не нужна: CSS backdrop работает только с содержимым данного WebView; не обещайте размытие рабочего стола.

Native range сейчас height22px:style337; поднимите реальный input до44px, rail3/thumb8 остаются. .player-controls rows44px 44px вместо44px22px, padding:0 и gap:0; исходный normal player88px не вмещает их с20px вертикального padding и1px border. Поэтому не допускайте overlap: используйте normal player height112px (44+44+20+1px border и3px свободного места), обновите **единственный** --vela-player-height в native canonical layer53 и все его потребители/normal underlap reserve. Это целевое native-исключение от web88, проверяемое при реализации; его нужно описать в DESIGN_SYSTEM. Compact auto rows сохраняют свои исходные fallback правила и минимум44 для ranges. Самостоятельного второго volume owner нет.

Отдельный decision внутри этого пакета фиксирован: полный44px native range важнее буквального88px web band. Не возвращать22px target ради визуального совпадения. В normal underlap все формулы используют token112, не захардкоженный88. Перед этим normal-проверка branch должна определять фактически фиксированную высоту, а не старое числовое88.

Сохраните native window44px close/minimize/maximize и drag region из style445–450; их hit/focus не смещается motion. Не добавляйте web Now Playing/dialog, если native такого действия не имеет. Cover fade включается только для уже реализованного безопасного native artwork presentation, без fetch/cache работы в UI thread; при отсутствии необходимой готовности этот подпункт честно NOT_IMPLEMENTED, остальные controls не имитируют его.

Методика: [refactoring-ui §Hierarchy/Color/Depth](/root/.codex/skills/hermes/refactoring-ui/SKILL.md), references/theming-dark-mode.md; [top-design §Performance](/root/.codex/skills/hermes/top-design/SKILL.md); [microinteractions rules-and-state.md](/root/.codex/skills/hermes/microinteractions/references/rules-and-state.md). Web animation libraries, custom cursor и platform-independent «60fps гарантия» не применяются.

## Критерии приёмки

- Explicit dark/light и System switch используют один theme vocabulary; нет dark glass над light content. Встроенные Manrope data URIs не изменены.
- Normal fixed native band112px и44px ranges не пересекаются; transport и window buttons44. Compact640×700: все controls достигаются,0 горизонтального overflow.
- Overlap виден только в заявленной normal сцене; при auto/error/compact — opaque fallback и последний focus target выше player. Sidebar offset и content scroll owner учтены один раз.
- Text>=4.5:1 normal/3:1 large; focus/glyph>=3:1 над real composite.<=3 backdrop слоёв с псевдоэлементами.
- Один AudioEngine/output thread, bounded channel,0 HTML audio elements; theme/scroll/resize не меняют source/position/queue.
- Credentials/CSRF не попадают в WebView storage, screenshots/logs/файлы. Native tests используют disposable fixture data и owned null sink.
- Отдельно записаны COMPILES, WINDOW_TESTED, INPUT_TESTED, AUDIO_NULL_SINK_TESTED, BACKGROUND_NOT_RUN, PACKAGING_NOT_RUN; ни один статус не подразумевает другой.

## Проверка

Сначала выполните **GATE_WEB из README** для общей регрессии и пересоберите native с максимум2 compiler jobs суммарно. Native-команды известны из действующего driver:

~~~bash
export CARGO_BUILD_JOBS=2
cargo fmt --all -- --check
cargo clippy --locked -p vela-desktop --all-targets --features native-qa -- -D warnings
cargo test --locked -p vela-desktop
cargo build --locked -p vela-desktop --example native-ui-qa --features native-qa
cargo build --locked -p vela-server
python3 scripts/verify_native_client.py --backend x11 --evidence /tmp/vela-dr08-x11
~~~

При реальном Wayland-сеансе отдельно:

~~~bash
python3 scripts/verify_native_client.py --backend wayland --window-only --evidence /tmp/vela-dr08-wayland
~~~

Не запускать Wayland полный peer/offline сценарий: wrapper разрешает только window-only. При отсутствии GTK/pkg-config/display/dbus/audio среды запишите точную первую ошибку и **NOT_RUN**, не подменяйте native браузерным WebKit. Не запускать/перенастраивать пользовательский desktop или default audio output.

Настоящие WebKitGTK shots:1280×840,1440×900,1728×900 и640×700; normal/error/compact, обе темы,System и scale80/100/150, reduce. Проверить реальные rect окна, а не cropped viewport. Firefox/Chromium результаты из DR-02 отдельно от native. Проверка packaging существующим scripts/verify_desktop_package.py после создания артефакта — отдельный будущий gate; этот пакет не публикует и не заменяет frozen snapshot.

Локально сравните scroll trace glass/fallback одинаковой сцены; бюджет DR-02 (>5п.п. кадров>33ms) применим только к этому устройству/engine. Исторические72fps из PROGRESS не гарантия для нынешнего хоста.

## Границы

Не менять native audio.rs/client.rs/MPRIS, storage/credentials, source temporary file contract, shared playback.rs, domain/server API, workspace Cargo/lock/миграции. Не переносить web HTMLMediaElement в native. Embedded CSS не создаёт static route; если всё же появился новый packaged asset, остановите расширение scope и соблюдите точный server allowlist/build/package contract. data/media_assets/target checkpoints и живые процессы не трогать. Не публиковать.

## Риски

WebKitGTK support/performance, GTK theme/system choice, масштабирование окна и frameless drag — самостоятельные риски. Прозрачность рабочего стола отсутствует и не требуется.112px band уменьшает content height; compact/error fallback проверять на640×700. Native screen-reader, физический звук, background и packaging остаются отдельными evidence classes даже после успешного GUI smoke.
