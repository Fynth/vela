# DR-05. Сделайте состояния читаемыми и неперекрывающимися

**Статус:** PROPOSED. **Приоритет:** P1. **Размер:** средний. **Пререквизиты:** нет. Один коммит: понятный ответ на «что сейчас происходит и что сделать дальше» на Home/Library/player. Исправляет DV-01/02/11/13.

## Цель

В светлой теме человек читает пустую Home-карточку, на телефоне видит причину остановки и Retry, а после команды получает подтверждение без перекрытия другого сообщения. Состояния показывают только подтверждённые данные аккаунта. S08/S17 и L дают contrast1.114/1.622; S09/S10 показывают overlapping feedback.

## Файлы к правке

| Путь | Место |
| --- | --- |
| apps/web/home-editorial.css | .home-feature-empty168–179, .home-wave-empty318–325; error/note styles |
| apps/web/src/home_editorial.rs | EN системные constants17–21; fallback88–105; loading/error/empty345–355 |
| apps/web/src/experience.rs | HomeEmptyState359–389, Home loading/error/render417–555; не менять логику запросов |
| apps/web/src/main.rs | stats grammar2437–2438; notices/Undo2386–2422; playback-status2953; idle2980–2982 |
| apps/web/src/queue_drawer.rs | Только UI-тексты143,258,309,349–357; root dispatch не менять |
| apps/web/style.css | status607–610, alerts801–805, mobile1072/1233; .home-empty1372–1397 |
| apps/web/now-playing.css | .np-connection500–529 |
| tests/browser/design.spec.mjs, prototype-transfer.spec.mjs | Контраст empty-card и совместное recovery/receipt; реальные empty/pending/error |
| docs/QA_EVIDENCE.md, docs/PROGRESS.md, docs/KNOWN_LIMITATIONS.md | Выполненные проверки и честные NOT_RUN |

## Что конкретно делать

Для **обоих** .home-feature-empty и .home-wave-empty после их текущих базовых rules задайте title/eyebrow:var(--vela-text), subtitle/note:var(--vela-muted-text), error:var(--vela-danger). .home-wave-note p также наследует semantic foreground в empty-ветке. Обычные image-backed feature/wave сохраняют light-on-image text и shading; не переопределяйте весь .home-feature-title глобально.

Light empty title станет #14271c/#fff, расчёт15.710:1; subtitle #40594a/#fff,7.648:1. Это ожидаемые расчётные значения на непрозрачном фоне, live после реализации нужно измерить заново. Размеры пустой mobile карточки принадлежат DR-06.

Сохраните EN как язык системного UI: shell и index.html lang=en уже такие. Это ограниченный выбор без нового localization framework. Переведите только UI literal строки затронутых editorial/queue состояний; реальные playlist/title/artist не переводите. Используйте согласованный словарь:

| Состояние | Заголовок/сообщение | Реальное действие |
| --- | --- | --- |
| Нет owned playlist, музыка есть | Choose music for this space / Create a private playlist to choose what appears here. | Open playlists → существующий actions.navigate |
| Нет Wave candidates, запрос завершён | No suitable tracks yet / Add music or explore your library. | Open library; Play остаётся disabled |
| Wave pending без candidates | Loading your mix… | Существующий pending; не имитировать проценты |
| Подтверждённая пустая Home | Сохранить Make room for your first listen / Add music you own. Your library stays private to your account. | Один существующий Add music |
| Library filter/search empty | Сохранить No matching tracks + причина | Существующий Reset applied filters / очистка query |
| Partial Home failure | Some sections could not load. Refresh to try again. | Существующий Refresh; подтверждённые строки остаются |
| Idle mini-player | Nothing playing / Choose a track | Никакого fake Play source |
| Queue system copy | NOW PLAYING, Saved queue, No tracks queued. Add tracks from your library. | Existing Open library |
| Browser gesture denial | Существующая безопасная причина, Retry playback | Только root engine.resume через реальный жест |

В EN stats показывайте track/tracks, album/albums, artist/artists по count==1. Не вычисляйте counts из числа текущих двух строк и не выдумывайте zero при неизвестном stats.

**Feedback layout:** вынесите render существующих notice, playlist_undo и playback_status в один root .feedback-stack непосредственно рядом с persistent player, вне .main-scroll. Их исходные signals и обработчики остаются единственными владельцами. Ошибки форм и mutation остаются inline рядом с формой. Это проекция, не новый task/notification store.

Stack:position:fixed; z-index60; bottom:calc(var(--vela-bottom-reserve) + 16px); right:24px; width:min(600px,calc(100% - 32px)); display:flex; flex-direction:column; gap8px. На<=760 left/right16px,width:auto. .playback-status и бывшие floating receipts внутри получают position:static; inset:auto; width100%; margin0. Удалите старые fixed bottom/left правила этих **перемещённых** экземпляров, включая specificity .content-inner>.alert.success.

Порядок сверху вниз: recovery, Undo receipt, ordinary receipt. Не теряйте Undo при появлении общего notice. Длинный stack ограничьте высотой calc(100dvh / var(--app-zoom,1) - var(--vela-topbar-height) - var(--vela-safe-top) - var(--vela-bottom-reserve) - 32px) и overflow-y:auto; это локальный overflow слоя feedback, не второй каталог/main-scroll. Фокус на Retry/Undo должен прокручивать сам слой и не перекрывать player. Не добавляйте auto-dismiss timer и новые причины успеха.

При открытом Now Playing/queue root recovery визуально не дублируется: оба dialog уже имеют свой status/Retry. Root state сохраняется; показ восстанавливается после close. Receipts не размещаются поверх browser top layer и не крадут фокус. Не меняйте per-open/session/occurrence guards.

Для .np-connection используйте flex-wrap:wrap, align-items:center, gap8px, padding12px; p:flex:1 1 240px;margin0; button:min-height44px;margin-top0; на mobile button может перейти на следующую строку. Это экономит высоту S04 без уменьшения artwork/transport. Конкретная normal short-title сцена1284×856 должна показывать Retry целиком; для длинных titles и увеличенного текста сохраните существующую прокрутку dialog вместо сжатия targets.

Loading/success/error правила берите из [microinteractions §Feedback](/root/.codex/skills/hermes/microinteractions/SKILL.md) и references/feedback-patterns.md, rules-and-state.md. Empty semantics — [refactoring-ui references/advanced-patterns.md](/root/.codex/skills/hermes/refactoring-ui/references/advanced-patterns.md). Цвет исправляется через semantic theme согласно references/theming-dark-mode.md.

Отступление от советов скиллов: не добавлять illustration/fake-progress к каждому empty и не показывать optimistic mutation success. Existing Home unknown/pending/settled-empty logic и jobs projection остаются как есть; метаданные плейлиста не предоставляют audio access. Реальный total>0 допускает progress; unknown total — только status/байты, без0% или выдуманного ETA.

## Критерии приёмки

- Light/dark empty editorial/Wave: весь обычный текст>=4.5:1, title>=3:1; область действия и ошибка читаются. Image-backed card не потеряла shade.
- В recovery+receipt на320/360/390 пересечение их rect равно0; gap>=8px. Retry, dismiss и Undo>=44px и не закрывают transport/nav.
- S04-сцене соответствует целиком видимый .np-connection Retry при1284×856,100%scale,коротком title. При720px высоты controls достигаются без горизонтального overflow.
- Pending не превращается в zero/empty до текущего подтверждения; old-owner data никогда не показывается. Ошибка не стирает last confirmed same-owner rows.
- В системных labels затронутых блоков один EN-словарь, count1 использует singular. Кириллические пользовательские названия сохраняются.
- Нет новых серверных job state, private analytics, fake images, цифр популярности, credits/rights/качества.

## Проверка

Выполните **GATE_WEB**: fmt, strict clippy, workspace unit/API, wasm, build-web, Playwright. Дополните существующие browser проверки контрастом пустой **editorial** карточки при2 треках, а не только страницы с нулём треков. Обязательны две разные реальные сцены.

Снимки Home/Library320×700,360×800,390×844,1284×856,1440×900; dark/light. Сцены:0 tracks;2 tracks и0 playlists; настоящий playlist; missing cover; loading; failed read с сохранёнными данными; recovery+Add-to-queue receipt; Undo+recovery; Now Playing720/856px высотой. Chromium/Firefox и reduce.

Ошибки создавайте abort/held request существующим harness над настоящим fixture server; не подменяйте successful recommendations/jobs фиктивными ответами. Для pending import используйте разрешённый локальный fixture путь тестов, не открывайте production egress. Исходный screenshots набор этих состояний не закрывает: до новой проверки они NOT_RUN.

## Границы

Не менять algorithms/consent/collection guards, network/API, playback/session/order, browser_player.rs, manifests и migrations. Новых CSS/asset paths нет: существующие allowlist пути остаются, packaging проверяется build-web. Не трогать media_assets, target/, frozen runtime, чужие изменения. Публичной публикации нет.

## Риски

Перемещение feedback может затронуть focus order и существующие selectors тестов; root callbacks и Undo identity сохраняйте. aria-live не должен объявлять одно сообщение из трёх поверхностей одновременно. Слишком длинный stack требует собственного bounded overflow. EN-выбор не означает завершённую локализацию всего Vela.
