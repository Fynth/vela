# DR-07. Сделайте управление очередью различимым с клавиатуры

**Статус:** PROPOSED. **Приоритет:** P1. **Размер:** средний. **Пререквизиты:** нет. Один коммит: focus и именованные controls очереди, с проверкой общего accessibility контракта. Повторная матрица после DR-02/03 входит в их приёмку.

## Цель

Пользователь клавиатуры видит, где находится, различает громкость и позицию, выходит из dialog/Menu и возвращается к вызвавшей кнопке. Человек с reduced-motion получает тот же результат без движения. Это проверка конкретных свойств, не WCAG/Orca-сертификация.

## Файлы к правке

| Путь | Место |
| --- | --- |
| apps/web/queue-drawer.css | focus295–298; range139–148 и новый .queue-seek; .queue-track202–212 |
| apps/web/src/queue_drawer.rs | labels210–244, current/remove note331–340, sleep timer371–391 |
| apps/web/style.css | Только выявленные focus/control contrast ошибки; global focus180, range1426, coarse1547/1623 |
| apps/web/now-playing.css | Focus532, controls191–244; только воспроизведённые contrast regressions |
| tests/browser/prototype-transfer.spec.mjs, design.spec.mjs | TR07/TR09 keyboard, contrast и real target checks |
| docs/QA_EVIDENCE.md, docs/KNOWN_LIMITATIONS.md, docs/PROGRESS.md | Исполненные engine/input проверки, отдельный screen-reader статус |

## Что конкретно делать

1. В dialog.queue-drawer :focus-visible замените hardcoded #d3e9a5 на var(--accent), offset3px, thickness2px. L подтвердил keyboard focusVisible=true, outline rgb(211,233,165) solid2px на белом panel, offset2px:1.315:1. Light semantic #08754c обеспечивает другую пару; измерьте её после правки. В forced-colors используйте системный CanvasText/Highlight, без outline:none.
2. Для .queue-track min-height44px; текущий38px cover остаётся. .queue-panel .icon-button уже44×44, увеличивать заново не нужно. Range .queue-volume input/.queue-seek input:display:block;width100%;min-width0;min-height/height44px;padding0;margin0. Родители min-width0. Не рисовать второй input и не менять root oninput.
3. Добавьте видимые label for=queue-volume «Volume» и for=queue-position «Playback position» размером12px/500,var(--muted),margin0 0 4px. Свяжите labels существующими id; текущие aria-label можно сохранить, если совпадает смысл. В range дайте aria-valuetext: volume округлённый0–100 percent; seek m:ss of m:ss. У неизвестной duration announce «Duration unknown», не выдумывать длину1s из технического max=1.
4. Keyboard Arrow/Home/End у range оставьте native. Глобальные Space/arrows не перехватывают ввод в input/select/button/menu/dialog. Соберите regression через реальные keypress, не dispatchEvent с недостоверными keyboard flags.
5. Сохраните browser-owned focus containment у настоящих dialog. Для контекстного div[role=menu] сохраните текущий ring из main.rs:2510–2518; не добавляйте второй trap и не объявляйте div нативным dialog. Escape возвращает фокус на исходный More, а picker получает фокус при переходе menu→picker. Close queue/NP идёт до unmount; нет animation timeout.
6. Статус не зависит только от цвета: действующий Play/Pause glyph, text state, selected/aria-pressed и mode dot сохраняются. В collapsed player уже есть sr-only player-state-text; не озвучивайте эквалайзер/каждый clock tick. У decorative Art внутри именованной кнопки alt="" допустим и не заменяется повтором title.
7. Timer остаётся видимым countdown, но не aria-live каждую секунду. Старт, cancel и expiry получают одно существующее meaningful announcement; pause берётся из root, не создаётся новый timer/collection. Progress импортов с actual total использует native progress; unknown total не получает ложный aria-valuenow.

Составьте checklist без объявления всех экранов прошедшими:

| Область | Измеримый gate |
| --- | --- |
| Text contrast | Обычный текст>=4.5:1; large>=3:1 (>=24 CSS px либо>=18.67 CSS px bold);10–14px captions всегда normal |
| Focus/control contrast |>=3:1 относительно прилегающей поверхности, outline2px+offset3, не clipping |
| Targets | Primary/coarse>=44×44px реальные rect; 15–22px glyph не равен размеру кнопки |
| Keyboard | Tab/Shift+Tab, Enter/Space, range keys; escape/return и отсутствие shortcut theft |
| Reduced motion | Ни transitions/animations у changed selectors/псевдоэлементов; данные/controls не пропали |
| Reflow |320px, app80–150%, browser200%, text spacing override без потери действия |
| Announcements | Одно meaningful message, без дублирования из inert/hidden surfaces и clock spam |
| Print/forced colors | Читаемая fallback поверхность без blur/тени, без потери focus в forced colors |

В reduced-motion проверяйте уже существующие6+3 blocks, а не предлагайте внедрить поддержку с нуля. Если DR-03 ещё не принят, существующие press transforms могут меняться мгновенно даже при transition:none; отдельно записывайте static transform, не называя его активной анимацией. После DR-03 ожидается также transform:none на его hover/settle.

Методика: [ux-heuristics §Recognition/Help/Severity](/root/.codex/skills/hermes/ux-heuristics/SKILL.md), references/wcag-checklist.md; [refactoring-ui accessibility-depth.md](/root/.codex/skills/hermes/refactoring-ui/references/accessibility-depth.md); [microinteractions feedback-patterns.md](/root/.codex/skills/hermes/microinteractions/references/feedback-patterns.md) «Feedback Accessibility».

44px здесь проектный target, а не доказательство, что любая41px строка нарушает AA. Сохраняйте desktop menu36px для fine pointer как явное dense-исключение; mobile/coarse меню должно иметь>=44 (нынешнее48 сохраняется). Не переносите рекомендации auto-focus traps из React-примеров скилла поверх native dialog.

## Критерии приёмки

- Queue light/dark: focused close/move/remove/track/range ring>=3:1,2px,offset3, виден с клавиатуры; нет hardcoded pistachio focus в light.
- Queue range width равна доступной колонке±1px и height44; label виден при focus/input, current value проговаривается осмысленно. Нет путаницы между volume и seek.
- Background inert при queue/NP/picker; menu сохраняет существующий ring; Escape закрывает только верхнюю поверхность и возвращает фокус к подключённому trigger.
- У disabled remove-current сохраняется объяснение через aria-describedby; отсутствие доступа не исправляется enabled-кнопкой. Metadata-only не выдаёт Play.
- После смены темы/reduce/session или закрытия ни отложенный feedback, ни focus не возвращают старую поверхность.
- Проверка не объявляет dark timing4.73 и quality5.41 «непроходящим контрастом». Glass composite после DR-02 проверяется отдельным методом.

## Проверка

Выполните **GATE_WEB**: fmt → strict clippy → unit/API → wasm → build-web → Playwright. Сначала TR07c/TR09 keyboard/reduced и DESIGN01. В сценариях используйте current long fixture, duplicates, paused/recovery, пустую очередь. Для измерения :focus-visible сначала keyboard Tab, затем фокус: programmatic focus после мыши может законно не включить кольцо.

Снимки1284×856 и390×844 обе темы: queue закрыта/открыта, каждый тип focused control, menu/picker/NP, normal/reduce.320×700,768×1024,1440×900 для reflow. Firefox отдельный запуск, штатные range controls. В reduced запрашивайте getAnimations и computed duration у обычных элементов и псевдоэлементов во время pending/playing/open — idle0/0 из аудита недостаточен.

Text spacing: line-height1.5, letter-spacing.12em, word-spacing.16em, paragraph margin-bottom2em; восстановите CSS после измерения. Browser200% и app150% тестируйте отдельно, затем одну совместную сцену без clipping; не трактуйте множители как новый scale setting.

Orca/screen reader: вручную проверьте имена, порядок, range values, один status announcement и возврат из dialogs на реальном Linux GUI. При отсутствии среды оставьте **NOT_RUN: screen reader**, запишите выполненный DOM/keyboard subset. Print preview/forced-colors тоже отдельные результаты; screenshot не сертифицирует их.

## Границы

Не менять audio/client ownership, shared PlaybackOrder, credentials/storage/CSRF, consent и history. Browser_player.rs, manifests, migrations, media_assets, target/, frozen runtime вне scope. Не добавлять assets/allowlist routes или third-party a11y framework. Существующие static paths перепаковываются build-web. Публикация не требуется.

## Риски

Фокус при снятии DOM batch и animation cancellation легко уходит в body; сохраняйте generation/lifetime guards. Native input styles отличаются между Firefox/Chromium/WebKitGTK. DOM contrast calculation не композитит картинку; screen-reader evidence не выводится из aria-атрибутов. Полный G23 остаётся PARTIAL до действительно выполненных проверок.
