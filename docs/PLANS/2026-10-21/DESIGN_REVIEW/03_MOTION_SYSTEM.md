# DR-03. Объясните смену состояния коротким откликом

**Статус:** PROPOSED. **Приоритет:** P2. **Размер:** большой. **Пререквизит:** DR-01. Один коммит: система presentation feedback; без изменения семантики playback/queue.

## Цель

При открытии очереди, выборе трека и перемещении строки пользователь сразу понимает результат действия, а фокус и аудио не ждут окончания эффекта. Входы становятся согласованными; cover crossfade остаётся визуальным. Основание: DV-07/14; M показывает базовые120/150ms, C now-playing.css:50 уже180ms/10px, state-bound skeletons/eq используют paint/layout animation.

## Файлы к правке

| Путь | Место |
| --- | --- |
| apps/web/style.css | .button/.icon-button/.catalog-card/.playlist-card:180,237; global transition-property:770; skeleton972–1003; eq1118–1129; play face1150–1155; .home-album-card img:1352; .track-menu-panel1436 |
| apps/web/now-playing.css | np-enter:49–50,537–539; transforms/filter:191–244; reduced595–605 |
| apps/web/home-editorial.css | .home-wave-play292–315, reduced404–417 |
| apps/web/queue-drawer.css | dialog[open]:28, .queue-row186, reduced289 |
| apps/web/src/main.rs | Art:860–867; root cover presentation2960+; queue move callbacks284–306; root overlay close/show coordinators |
| apps/web/src/now_playing.rs | .np-artwork:101; только props/presentation для cover |
| apps/web/src/queue_drawer.rs | Существующий keyed row289 и forwarded reorder handlers; только receipt props |
| tests/browser/prototype-transfer.spec.mjs, tests/browser/music.spec.mjs | Реальный open/close/reorder/occurrence/rapid replacement/reduced-motion |
| docs/QA_EVIDENCE.md, docs/PROGRESS.md, docs/DESIGN_SYSTEM.md | Реальные результаты и map G05/G23 |

## Что конкретно делать

Применяйте роли DR-01. Все команды, flags и текстовые state changes исполняются синхронно до animation. Первый видимый pressed/pending feedback начинается в ближайшем кадре, целевой event-to-paint<=100ms; duration эффекта — отдельная величина.

| Событие / селектор | Эффект и время | Завершение/прерывание |
| --- | --- | --- |
| .button/.icon-button:active | transform scale(.97), instant80ms | Release немедленно принимает новый input; без translate всего хитбокса |
| Catalog/playlist/Home album card, только hover:hover + pointer:fine | Внутренний cover translateY(-2px), quick120ms | Фокус и click остаются на неподвижной кнопке; при pointer coarse нет hover-залипания |
| Cover img при hover | scale1→1.03, quick120ms | Не прежние350ms; shadow/background меняются атомарно |
| .track-menu-panel | opacity0→1, scale(.98→1), quick120ms; transform-origin:top right | Desktop anchor и hitbox не двигаются. Mobile sheet opacity+translateY(4px→0), та же длительность |
| .now-playing-dialog[open] | np-enter opacity0→1, translateY(4px→0), overlay140ms | Замените существующие180ms/10px; при track смене entrance не повторять |
| dialog.queue-drawer[open] | opacity0→1, translateX(4px→0), panel180ms | Не прогонять весь370px путь; никакой анимации width/left |
| Root/NP artwork change | Настоящий crossfade двух изображений, artwork140ms, opacity only | Максимум2 слоя, смена только после готовности нового; см. guards |
| Успешный queue move | Короткий settle на уже перемещённой строке: translateY(±4px→0), reorder150ms | Не задерживать swap; следующий edit обрывает старый эффект |

У hover/press оборачивайте содержимое, если transform кнопки уменьшает или смещает реальный hitbox. Не анимируйте focus outline; outline2px появляется сразу.

**Выход оверлеев:** native dialog.close, снятие inertness и возврат фокуса не откладываются. Поэтому dialog/menu exit — **0ms**. Не использовать overlay/display allow-discrete, удерживающий top layer, setTimeout(close,100), копию диалога или DOM snapshot с чужими данными. Это осознанное отклонение от обычного modal fade-out в refactoring-ui references/animation-microinteractions.md: текущий lifecycle и DESIGN_DIRECTIVE важнее эффекта. --vela-motion-exit100ms оставьте для исчезновения чисто декоративного слоя обложки внутри текущей сессии; применять его к dialog не обязательно.

**Cover lifecycle:** используйте presentation wrapper только вокруг root player Art и np-artwork. Art:860 получает необязательный onready callback от img.onload/onerror; остальные Art по умолчанию ведут себя прежним образом. Wrapper хранит лишь current/previous visual identity и максимум один уходящий слой текущего аккаунта. Identity включает session generation + track/artwork identity; одинаковая картинка в другом occurrence не требует fade и не меняет transport key.

Новый onload/onerror принимает результат только при совпадении session/open generation/visual generation/lifetime. При новом выборе/повторном открытии старый callback отбрасывается. До ready сохраняется нейтральный existing fallback либо current cover; после ready crossfade140ms. Cleanup по animationend принимает только актуальный visual generation. Быстрая A→B→C отменяет B, не накапливает слои/таймеры. Unmount/logout немедленно удаляет previous layer и callbacks; уходящая картинка aria-hidden и pointer-events:none. Не создавайте fetch/preload pipeline или новый media URL resolver ради fade. Если нужен deferred browser callback, используйте spawn_local с независимыми guards и явной отменой, без scope-dependent spawn из browser callback.

**Reorder:** сначала существующий root move handler синхронно обновляет PlaybackOrder, queue и cursor. После этого root выпускает один presentation receipt {session generation, edit generation, destination occurrence index, direction}. Drawer подсвечивает/settle только соответствующую уже новую строку. Любой следующий structural edit/session/close уничтожает receipt; track.id не идентифицирует duplicate occurrence. Не вводите второй order model, drag-library, optimistic server reorder или rekey всей очереди. Если focus перестраивается при смене index key, перенесите его на соответствующую кнопку нового destination после текущего DOM batch с guards; аудио и selection не меняйте.

**Loading:** background-position shimmer style.css:972–1003 замените статичным semantic skeleton. При настоящем pending допустим один opacity entrance140ms, не бесконечное мерцание строк. --vela-motion-skeleton1800ms — резервный budget, не требование анимировать. Eq: фиксированная height10px у каждого i, transform-origin:bottom; текущие base heights передайте через scaleY(.4/.7/.5/.9); состояние playing меняет только scaleY(.3→1),800ms state easing. Это существующий state indicator, не фон; пауза/hidden/reduce прекращают цикл. Text/icon state остаётся различимым без него. Никаких новых бесконечных декоративных циклов.

Удалите filter:brightness hover .np-toggle:243 и filter transition:236; цвет/тень меняются мгновенно. Для изменённых selectors задайте transition-property:transform,opacity явно после style.css:770; не анимируйте background/color/box-shadow/height/width/top/left/margin/padding/background-position. Blur статичен.

В каждом из четырёх CSS после новых motion rules добавьте reduce для точных selectors и ::before/::after: animation:none!important; transition:none!important; transform:none для hover/enter/settle, scroll-behavior:auto. JS/presentation branches под reduce меняют artwork сразу, previous не удерживают. После смены reduce во время animation остановите эффекты и cleanup без animationend dependency.

Методика: [microinteractions §Rules/Feedback/Loops](/root/.codex/skills/hermes/microinteractions/SKILL.md), references/rules-and-state.md и feedback-patterns.md; [refactoring-ui animation-microinteractions.md](/root/.codex/skills/hermes/refactoring-ui/references/animation-microinteractions.md) §§Accessibility/Performance. В top-design references/animation-patterns.md используем принцип purpose/performance, но не custom springs/GSAP и длительные staggers.

## Критерии приёмки

- Длительности: press80, menu120, dialog140, drawer180, cover<=140, settle150ms; вход overlay<=4px.
- В trace нет layout/paint-свойств в новых animation/transition. Eq не меняет layout height, skeleton не двигает background.
- State/command/focus не ждут ни одного transition; Escape/close убирают dialog из top layer в текущем цикле, callback не открывает его снова.
- A→B→C/logout/close during fade:<=2 cover layers,0 retained private layers after logout; latest identity only,0 новых audio elements/media-source loads из-за оформления.
- Queue move обновляет порядок сразу; duplicate occurrences, Next/EOF, shuffle/explicit next и current cursor сохраняются.
- Reduce в pending/playing/open/hover/reorder:0 активных animation и0 ненулевых transition-duration у изменённых selectors/псевдоэлементов; все controls работают.

## Проверка

Выполните **GATE_WEB**: fmt/clippy/unit+API/wasm/build-web/full Playwright. Добавляйте browser assertions только на риск: быстрые visual callbacks, close/session race, reorder focus, audio identity; не unit-тестируйте константу140ms.

Снимки/короткие записи:1284×856 и390×844, обе темы; меню, Now Playing и queue closed/open; cover с artwork и fallback, очереди с одинаковым track в разных occurrences; reduce. Затем320×700 и1440×900 для границ, Firefox отдельно. Измерьте event-to-first-paint и animation duration, не делайте вывод о latency из CSS. NP/PLAY/RES/NET/AUTH должны остаться зелёными; результаты только после запуска.

## Границы

Не меняйте browser_player.rs, playback.rs, аудиодекодирование, seek policy, logical listening sessions, auth/privacy, network fetch ownership. Не добавляйте звук/haptics/новые библиотеки/ассеты. Новые статические CSS-файлы не нужны; существующие проходят build-web/packaged paths. Не трогайте data/media_assets/target checkpoints/frozen runtime; без публикации.

## Риски

Crossfade и reorder требуют guards даже при «только визуальном» изменении: stale image/focus может показать старый аккаунт или неверную occurrence. Animationend не приходит при отмене/reduce — cleanup должен иметь синхронный путь. Моментальный exit — сознательный компромисс; сохранение интерактивной копии dialog неприемлемо.
