# UX по экранам: что видит человек и что делает каждое действие

Статус: требования к будущей реализации. Ни один новый экран этим файлом не поставлен.
Визуальная основа — [непрерывное музыкальное пространство](08_VISUAL_LANGUAGE_NO_CARD_GRID.md),
поведение аудио — [единый координатор](03_PREFETCH_QUEUE_AUDIO_ENGINE.md).

## 1. Общая оболочка

### Desktop

1. Слева: знак Vela, Home, Search, компактная Library; ниже закреплённые коллекции,
   папки, плейлисты, последнее выбранное. Add Music и Create доступны рядом с Library.
2. В центре: один прокручиваемый canvas. Back/Forward относятся к навигации, а не
   Previous/Next песни. Заголовок страницы при прокрутке становится компактным.
3. Справа по запросу: Now Playing, Queue либо Devices. Не три панели одновременно.
   При узком tile панель становится sheet; трек продолжает играть.
4. Снизу: непрерывный transport с текущим artwork, названием, исполнителем, Favorite,
   Previous/Play/Next, timeline, Queue, Devices, громкостью. Дополнительные действия
   уходят в меню. Ошибки не увеличивают постоянную высоту player на целый экран.
5. Ширины панелей сохраняются отдельно от playback. Ограничения не допускают
   отрицательной ширины центра, спрятанного Play или горизонтального page scroll.
6. Закрытие правой панели возвращает фокус на открывшую кнопку. Переключение маршрута
   не заменяет output, source, decoder, queue occurrence и позицию.

### Phone

Нижние вкладки Home, Search, Library; четвёртая доступна только для реально готовой
крупной функции. Mini-player находится над tabs, системная safe area — под ними.
Tap открывает Now Playing, горизонтальный жест переключения имеет кнопочный аналог.
Скрытие mini-player свайпом не должно незаметно остановить музыку. Действия строки
открывают bottom sheet, длинное нажатие никогда не является единственным способом.

### Общие правила интеракций

- Один click/tap по названию открывает объект; отдельный Play играет; double-click
  строки desktop может играть, но не обязателен для доступа к функции.
- Context menu появляется рядом с точкой вызова, не за границей окна; Escape закрывает
  верхний слой. Нельзя одновременно держать context menu и modal с двумя focus traps.
- Space управляет playback только вне input/text editing и незахваченного modal.
  Ctrl/Cmd+K открывает поиск команд; `/` фокусирует Search вне текстового поля.
- Drag всегда имеет Move up/down и доступное клавиатурное управление. Drop показывает
  точную вставку; автоскролл ограничен; отмена возвращает исходное положение.
- Optimistic favorite/rename показывает pending и откатывается при отказе сервера.
  Добавление в очередь, плейлист и перенос между устройствами различаются.
- Toast на 4–8 секунд — начальная гипотеза, без фокуса и без потери сообщения для
  screen reader. Error с необходимым действием остаётся у своего объекта.
- Undo применяется только к действительно обратимому действию; серверная новая
  revision и смена permissions могут потребовать показать конфликт вместо «успеха».

## 2. Первый запуск и вход

Отделить consumer sign-in от admin setup. Пользователь существующего сервера видит
сервер, Continue with Vela ID при наличии интеграции, local login, Add another server.
Пароль не попадает в URL, clipboard, diagnostics или сохранённый bookmark.

Первый администратор проходит welcome → account → разрешённое storage → Add Music →
privacy review → Library. Не спрашивать десяток технических решений до первой песни.
External access и federation отдельные opt-in шаги с возможностью пропустить.

При входе по Vela ID сначала открыть системный browser, затем показать проверяемое
возвращение в приложение и конкретный server membership. Cancel не сбрасывает
старый local account. Cloud account без сервера получает честный экран подключения,
а не фиктивную пустую «глобальную музыкальную библиотеку».

Ошибки: неверный login, недоступный сервер, TLS, недоступный identity provider,
отозванная membership — разные состояния. Не удалять сохранённую сессию из-за 5xx.

## 3. Home

### Пустая библиотека

Короткое приветствие, выразительная авторская композиция без чужих album covers,
две основные возможности Upload и Import URL. Scan появляется у пользователя с
разрешением. Под объяснением «Ваша музыка, ваш сервер» — privacy, без marketing chart.

### Библиотека из 1–10 треков

Показать реально доступные New Additions, Continue Listening при наличии session,
короткую дорожку альбомов. Не размножать один трек на шесть «персональных» секций.
Предложить добавить музыку или сохранить playlist. Нехватка истории не ошибка.

### Достаточная библиотека

- Hero — реальный недавний альбом или продолжение сессии, а не случайная огромная плитка.
- Jump Back In — компактные пары cover/title из доступного catalog/session state.
- Heavy Rotation — только при разрешённой qualified history; без истории секция скрыта.
- Rediscover — объяснение на строке: давно не выбирали / сохранено / похожие теги.
- Discovery — одна выразительная редакционная полоса и несколько музыкальных объектов,
  а не одинаковые dashboard cards с разными заголовками.
- New Additions не называется New Releases: дата импорта не является датой релиза.

У каждой рекомендации: Play, Why this, Less like this, Add next. Скрытие обратимо в
Settings. Refresh не заменяет текущую очередь. Все действия повторно проверяют origin,
account, privacy revision и lifetime; pending feedback не затирается обычным refresh.

## 4. Search

Ввод расположен в устойчивом месте; Escape сначала закрывает suggestions, затем
снимает фокус. Debounce начать с 180–250 ms и проверить на реальном вводе; при смене
query физически отменять прошлый запрос и дополнительно проверять generation.

Состояния: пустой query → недавние запросы при opt-in; короткий → локальные suggestions;
поиск → skeleton подходящей формы; результат → best match + tracks + albums/artists/
playlists; empty → исправить запрос/снять фильтры; partial remote → локальное видно
сразу, рядом статус конкретного сервера. Не блокировать всю страницу медленным peer.

Табы показывают только реализованные типы. Фильтры origin, codec, year и favorites
скрываются в расширенный ряд; выбранные фильтры видны и сбрасываются одной кнопкой.
Up/Down перемещают suggestion, Enter открывает или играет согласно явной подписи,
Tab не заперт в бесконечный цикл. Не принимать stale result после смены account.

История запросов очищается отдельно от listening history. Кнопка Clear не удаляет
треков, а серверные search logs не становятся скрытой персонализацией.

## 5. Library

Header: Library, search within, sort, view density, Create. Фильтры Albums, Artists,
Playlists, Favorites, Downloaded имеют ясную текущую область. Нельзя обещать filter
Downloaded для cloud metadata без реального manifest на данном устройстве.

Основное представление выбирается по содержимому: album wall с разным размером
обложек, плотные track rows, вертикальные playlist covers, artist typography.
Grid — допустимый инструмент для коллекции обложек, не универсальная оболочка всего UI.

Sidebar содержит pins и folders. Rename inline только после явного действия; click
по папке не включает случайный playback. Большая коллекция использует cursors и
виртуализацию со стабильными ключами origin+object ID; не скачивает весь catalog.

Bulk mode открывается из явного Select, показывает count и действия к выбранному,
проверяет права каждого элемента. При смене фильтра не применять удаление к невидимым
объектам без ясного подтверждения выбранного набора.

## 6. Album

Cover занимает заметную часть header; рядом title, artist links, release metadata,
edition при достоверных данных, duration. Primary Play, Shuffle, Save, меню. Download
виден как разрешённое действие или disabled с конкретной причиной.

Track table: number/playing indicator, title/credits, favorite, duration, row menu.
Disc разделён подзаголовком, не новым набором dashboard boxes. Компиляции показывают
album artist и track artist отдельно. Недоступный track остаётся читаемым и объясняет
причину; Play album строит контекст с явной политикой пропуска недоступного.

Play с трека №4 начинает с правильного occurrence; дальнейшая навигация не пересоздаёт
контекст. Смена edition не подменяет файл уже играющего track без выбора пользователя.
Credits/technical details — drawer с указанием происхождения/неполноты данных.

## 7. Artist

Настоящие имя/фото с правом использования или выразительный типографический hero.
Follow означает сохранение интереса, а не разрешение на всю удалённую музыку.
Top tracks означает «в вашей библиотеке» или иной точно обозначенный scope.

Discography разбивается на albums/singles/appearances, только если модель различает
эти типы. Artist Radio создаёт объяснимый набор доступных tracks; для одного трека
предлагает replay или расширение выбора, не выдумывает related catalog.
Biography не генерируется из предположений и не подтягивается из неизвестного scraping.

## 8. Playlists: конкретная замена страницы со скриншота

### Список

1. Убрать постоянно открытые New folder / New playlist forms и технический абзац.
2. Header: «Плейлисты», компактное Create с выбором Playlist / Folder / Smart playlist
   только при готовности последнего; Refresh — secondary, а не главное действие.
3. Слева folders, в центре последние и закреплённые коллекции: cover montage,
   title, owner/collaborative indicator, count. В плотном режиме — музыкальные строки.
4. Create playlist открывает небольшой modal: name обязательно, description опционально;
   Enter submit, Escape cancel, focus/error рядом с name, pending защищает от double submit.
5. После успеха открыть новый playlist, вернуть доступный heading focus, предложить
   Add tracks или drop из Library. Не закрывать modal при ошибке, сохраняя введённое.
6. Create folder — маленькое inline поле в sidebar либо modal. После создания фокус
   на folder, не на случайном первом track. Названия экранируются при render.
7. Empty state без огромного пустого прямоугольника: небольшая иллюстрация/типография,
   Create playlist и, если есть музыка, предложение выбрать несколько tracks.

### Detail

Header: cover, title, description, owner, privacy, collaborators, duration, Play,
Shuffle, Share, меню. Privacy chip открывает понятный список доступа; authorization
разъясняется в Share, а не основным слоганом страницы.

Drag handle активен при editor rights; move up/down — доступная альтернатива.
Pending reorder виден, CAS conflict перечитывает authoritative revision и предлагает
повторить намерение, не перезаписывает чужой весь playlist. Duplicate track разрешён
как отдельный occurrence; remove убирает выбранное playlist item, не исходный asset.

Share: named user / link / server members / public только по реальной policy.
Отдельно View metadata, Edit; streaming/download отображают самостоятельные grants.
Expiration, revoke, pending invites и effective rights видны без похода в admin.
Copy link не кладёт в clipboard account token. Delete с точным названием и объяснением,
что media останутся; owner transfer — отдельный подтверждаемый flow.

## 9. Now Playing

Desktop panel и phone full-screen читают один playback state. Large artwork, readable
title/artist, source label, Favorite, transport. Timeline имеет elapsed/remaining mode,
клавиатурный seek, точный preview. Drag preview не посылает сотни сетевых seek; commit
на отпускании, отмена сохраняет исходное. Если duration неизвестна, не рисовать fake %.

Показать реальные состояния: Preparing, Buffering, Playing, Paused, Unavailable.
Не писать Downloading, если читается уже готовый локальный asset. Подготовка может
показывать transferred/expected bytes только при известном размере; это не seek bar.

Quality details: original/transcoded, codec, sample rate, channels и bitrate из
фактического source. Crossfade/normalization/output selection скрыть или корректно
disable при неподдерживаемом engine. Тумблер не должен лишь менять надпись.

Lyrics — соседний режим с timed highlight при LRC; обычный текст без fake karaoke.
Follow current line отменяется ручным scroll и возобновляется отдельной кнопкой.
Редактирование локального текста отделено от source file write-back.

## 10. Queue

Current отделён от Next, затем виден исходный album/playlist context. Рядом с первым
следующим — ненавязчивый Ready/Preparing по настоящему состоянию resolver.
Сеть/кэш не показывать десятком лишних колонок; details открываются по запросу.

Add next и Add to end имеют разные подписи. Повторный track в очереди — другой
occurrence. Reorder current не пересоздаёт source. Clear upcoming сохраняет current.
Save as playlist сохраняет список, не credentials/stream URLs или историю shuffle.

Shuffle отображает реальный предстоящий порядок; show original order — отдельный
режим просмотра, не случайная перемешка ещё раз. Repeat one не ломает explicit Next.
Previous сначала возвращает к началу после согласованного порога, затем предыдущий
occurrence; выбранный порог записать и протестировать на всех engines.

При online→offline unavailable элементы объясняются и могут пропускаться по явной
policy. Если все недоступны — stop с действием, без бесконечного skip/download loop.

## 11. Devices и передача

Кнопка Devices показывает This device, online controllable, recently offline.
Иконка не является единственным признаком: name, platform, last seen, capabilities.
Переименование и revoke доступны владельцу. Рядом с выбранным remote device явно
написано «Управление устройством …», чтобы изменение volume не было неожиданным.

Transfer here / Transfer to … → Preparing target → Waiting for confirmation →
Playing there. Пока target готовится, source играет. При браузерном autoplay block
target показывает Tap to start; source не останавливается до готовности. Timeout
возвращает управление с Retry, не бесконечным spinner.

Play independently — отдельное действие. Online/offline presence не равна согласованному
публичному listening presence. Не показывать friends private device names.

## 12. Downloads / Storage

Верхняя строка: ready bytes, quota, active operations. Tabs Saved / In progress /
Needs attention. Group album/playlist разворачивается в tracks. Есть pause/cancel
transfer при поддержке, retry конкретного failed item, clear errors.

«Сохранено для офлайна» и «В автокэше» различаются. Autocache можно очистить, pinned
downloads сохраняются. Delete group сообщает, сколько bytes освободит, и не удаляет
оригиналы на сервере. Quota подсчитывает staging и активные leases.

Offline launch открывает сохранённый catalog без успешного server login; это отдельный
privacy-aware local profile, а не обход online authorization для новых assets.
Reconnect показывает sync state, не стирает сохранённое из-за временного timeout.

## 13. Vela ID / Connected servers

Account area: independent identity, local accounts/memberships, devices, sessions,
export/delete, security. Server list: name, verified canonical origin, health, active
membership, catalog capability. UUID/issuer доступны в details, не вместо имени.

Connect server: URL → identity/capabilities preview → server authorization → confirm
scope → catalog. Не передавать cloud token произвольному вставленному URL.
Disconnect предупреждает о remote-only queue items; не удаляет музыку, old local
account и pinned files автоматически. Unresolved references остаются видны.

## 14. Rooms / Friends / Network

Room — одна сцена с artwork, большим текущим track и живой queue ribbon. Participants
не выглядят как admin user table. Ready/access state виден для конкретного участника
без показа его secret/grant. Host actions отделены от suggest/vote.

Join preview показывает host, privacy, allowed controls и media access requirements.
Room link не запускает аудио и не включает публичную activity автоматически.
Leave возвращает в прежний personal context paused или предлагает Continue here;
никогда не теряет прежнюю очередь без предупреждения.

Friends: explicit follow/request, mute/block, visibility settings, invitation inbox.
Network: только opted-in profiles/collections с origin и provenance. Отсутствие
контента с законным stream grant — нормальное честное состояние, не повод качать
защищённый commercial catalog или выдавать metadata за playable asset.

## 15. Settings и Administration

Consumer settings: Appearance, Playback, Downloads, Privacy, Devices, Accounts,
Connections, Accessibility. Поиск settings ведёт к реальному control. Experimental
functions по capability, без длинного списка декоративных switches.

Admin отдельно: users/invites, storage roots, imports/jobs, backups, external access,
federation policy, health. Неадминистратор не видит raw logs, filesystem paths,
source URLs из агрегированного task center и секреты конфигурации.

Каждый пункт объясняет эффект до изменения. Например, history account consent,
device consent, sync session и публичное presence — разные toggles. «Очистить
историю» сообщает про отключение сбора и инвалидирует late outboxes/exports.

## 16. Обязательная таблица состояний для каждой реализации

Перед merge каждого экрана будущий исполнитель заполняет: first load, refresh,
empty, partial, offline, retry, 401, 403, 404, conflict, cancellation, account change,
route exit, keyboard-only, screen reader, reduced motion, dark/light, narrow tile.
Для неприменимого состояния пишет причину. «Выводится toast» недостаточно: указать
где focus, сохранился ли ввод, что случилось с аудио и может ли late reply изменить UI.
