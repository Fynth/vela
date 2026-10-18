# UX: Listening Atlas, по экранам и взаимодействиям

Это спецификация будущего интерфейса. Текущие working APIs/playback не заменяются
макетами. Временные дизайнерские подписи и картинки не попадают в реальные аккаунты.

## 1. Информационная архитектура

Desktop: слева компактные Home/Search/Library/Explore, ниже searchable библиотека
с Playlists/Albums/Artists/Downloads и папками. Центр — контент с history Back/Forward.
Справа opt-in «Сейчас» / Queue / Lyrics, снизу один persistent transport.
Admin и диагностика доступны через account→Administration только администраторам.
Add music остаётся быстрым global action, а не административной операцией.

Phone:4 основные вкладки «Главная / Поиск / Медиатека / Обзор», mini-player над ними;
Downloads/History/Friends внутри соответствующих разделов, не 9 иконок в нижней строке.
Full Player — отдельный полноэкранный route/sheet с сохранённым engine. Tablet имеет
два адаптивных столбца, не растянутый phone и не сжатый desktop.

Новые public/social разделы появляются лишь с доступной capability и реальными данными.
Нет кнопок «скоро» в primary navigation. Server scope всегда понятно переключается.

## 2. Общий interaction contract

Каждый экран описывает 8 состояний: loading, empty, ready, stale/partial, offline,
permission denied, error/retry, mutation pending/conflict. Empty не равно network error.
Скелетон только при ожидаемом запросе, стабильной высоты; spinner не вечный.

- Ссылка открывает сущность, Play играет, add-next только меняет очередь; click target
  для artwork/title не конфликтует с вложенной play button.
- Context menu desktop right-click/кнопка, mobile long-press+кнопка; touch users не
  обязаны знать gesture. Roving focus, Escape, outside click, возврат focus.
- Modal: один top-layer dialog, inert background, focus containment/return, initial
  focus на безопасном действии; destructive confirmation показывает конкретный объект.
- Optimistic update только для обратимого desired-state action, с rollback/receipt;
  разрешения и destructive writes не объявляются успешными до ответа.
- Toast: одно короткое сообщение с Undo/Retry, не закрывает transport; ошибки на
  самой форме сохраняются. Screen reader live-region не зачитывает каждый time tick.
- Смена route не remount audio. Back восстанавливает scroll/query/sort выбранного route.
- Keyboard shortcuts работают вне input/contenteditable/IME; shortcut help доступна.
- Даты/числа/плюрализация locale-aware; unknown value обозначается, не превращается в 0.
- Никаких raw SQL/DSP errors/server paths в listening UI. Диагностика в disclosure.

## 3. Онбординг и sign-in

Пути: «У меня есть сервер», «Создать своё пространство» (existing first-run),
«Открыть скачанную музыку» после FIX-05; public browsing только когда оно разрешено.
Server origin в отдельном поле с canonical validation, clear password visibility
toggle, remember-session default off, понятный Secure Store failure.
Connection pending можно отменить; отменённый вход никогда не возвращается позже.

Первый upload: dropzone+file picker, реальные limits/metadata, destination, progress,
duplicates, per-file error/retry. После первого успеха — один явный Play album/track.
Consent history предлагается отдельно с пользой и кнопкой «Пока нет», без prechecked.
Не заставлять выбирать любимых известных артистов, которых нет в доступном каталоге.

## 4. Home: мало шума, много повода включить

Первый экран: contextual greeting и небольшая строка scope («Ваша библиотека»),
одна редакционная featured полка, быстрый Continue listening,2–3 контрастные секции.
Не 8 одинаковых карточек с длинными алгоритмическими объяснениями.

Приоритет shelves: незавершённая сессия → recent qualified → new additions →
favorite/rotation → discovery → old unexplored. Каждая eligibility-driven;
history disabled скрывает history sections и не читает retained events.
Причина 1 строка «Рядом с вашим избранным», подробности в «Почему».
Play shelf создаёт стабильный snapshot контекста, browse не меняет очередь.

0 треков: персональное приветствие + Upload/Import.5 треков: компактный album/track list,
вместо пустых «Daily» cards.10k+: lazy shelves, bounded cards, «Показать всё» с cursor.
Ошибка одного shelf не закрывает остальные; auth/privacy change убирает private replies.

## 5. Search / Browse

Search field всегда ясно виден; cmd/ctrl-K фокусирует, Escape закрывает suggestions.
Input200ms debounce с cancel; query state отображается, accepted results scoped.
Filters All/Tracks/Albums/Artists/Playlists/Genres и реализованные remote scopes;
не показывать Users/Podcasts до реализации. Enter открывает выбранный result,
первый результат не начинает играть без явного Play.

Local results появляются сразу; origin groups приходят постепенно с independent
Loading/Unavailable/Cached rows и Retry. Cached metadata не показывает active Play.
«Поиск в этом сервере» сохраняет query; clear history отдельно с подтверждением scope.
Long query/no results/short query/unsupported server/error — разные подсказки.

## 6. Library и левый navigator

Filter chips Albums/Artists/Playlists/Downloaded, search within library, sort Recent/
Added/Title/Artist и реализованные options; filters keyboard accessible. Grid/list
переключается без потери position/selection; выбранная плотность сохраняется.
Folders collapsible, move menu+drag accessible alternative, pin with Undo.
Внизу storage/connection warning только если есть действие; никаких vanity counters.

Pagination/virtualization: bounded API + stable row keys; windowed rendering когда
реально отображаются сотни строк. Focus/selection не исчезает при recycling.
Не считать 50 загруженных треков всей библиотекой при Play all; API queue intent должен
объявлять «эта страница» либо поддерживать bounded lazy context cursor.

## 7. Album

Artwork с fallback, title, credited artists, release/edition/date/type при наличии,
Play / Shuffle / Save / More; download только при grant/capability.
Track table по disc/track number, duration, favorite/playing indicator и context action.
Artist click открывает artist, не меняя source. Play row начинает правильный occurrence
в album context. Multi-disc — отдельные заголовки; unknown номера не выдумываются.
Edition selector после модели editions; technical details в раскрытии, не hero.

## 8. Artist

Оригинальная типографическая hero вместо выдуманного портрета. Имя/доступные релизы,
«Популярное в вашей библиотеке» только с соответствующей private evidence; не
monthly listeners. Biography/image только разрешённый provider/provenance.
Top tracks, albums/singles/appearances по модели данных, related artists с reason.
Artist radio доступно при candidates, sparse state объясняет объём. Follow/private
favorite различаются от public follow, не публикуются автоматически.

## 9. Track actions и техническая информация

Основные: Play, next, last, Add to playlist, Favorite, Go to album/artist.
Вторичные: Why recommended, hide, quality/credits, permitted download, share.
Активный трек имеет иконку/текст «Играет», paused не анимируется как playing.
Details: actual codec/container/sample rate/bit depth/channels/size/source/transcoded;
unknown помечается. Lossless-original не обещает lossless Bluetooth route.

## 10. Playlist и folders

Title/description/artwork, owner/scope, collaborative badge, Play/Shuffle/Save/Share.
Описание и cover optional; сгенерированный cover — исходная Vela-графика с attribution
«оформление плейлиста», не borrowed album artwork.
Inline rename — explicit commit/cancel. Delete — имя и последствия, tracks сохраняются.
Reorder drag+buttons, multi-select add/remove, duplicate сохранит entry order.
Conflict: «Плейлист обновился», reload/rebase действий, не молча last-write-wins.
В entries separate labels Metadata only / Stream allowed / Download allowed.

Share sheet: кому/visibility, view/stream/download/edit/manage separately, expiry,
pending invitations, copy link, revoke. Private default. Нельзя checkbox Edit
использовать как условие воспроизведения. Owner transfer — отдельное подтверждение.

## 11. Persistent player, Now Playing, queue

Desktop bottom: cover+title/artist, transport, seek+time, favorite, volume, queue,
devices. Primary play circle визуально сильнее остальных; title открывает entity,
cover/Now Playing — выбранный режим, tooltip/labels точны.
Right panel показывает current artwork, short credits/source, следующий трек;
resize constrained и запоминается per-device. Скрытие панели не меняет audio.

Phone mini-player: artwork/title, Play/Pause, тонкий progress с альтернативным full
seek; весь card открывает full player, button не открывает его попутно.
Full player: большая обложка, title/artist, actualsource/quality disclosure, seek,
transport, favorite/queue/devices, optional lyrics. Safe areas,200%text scrollable.

Queue: Now / User next / Context, deterministic preview, same-track duplicates
различаются occurrences. Undo add/remove, clear next, save queue, missing source row.
Связанный sound output не remount при lyric/tab/theme/route change.
Во время source prepare — real bytes, Cancel; retry не создаёт фантомную history session.

## 12. Lyrics / credits / listening notes

Tabs visible only when actual data/capability exists. LRC highlights current line,
manual scroll удерживается до «К текущей строке». Plain lyrics имеет fallback,
нет подписи Synced если timings отсутствуют. Selection/copy только согласно source
rights. Notes private by default; edit saved separately from original media tags.
Автоматическая прокрутка выключается при reduced-motion/preferences.

## 13. Downloads и offline

Storage bar actual bytes/capacity, Downloading/Ready/Needs attention, per-item size,
grant/source/quality, pause/retry/remove, planned collection count. Ready значит
валидная local copy; stale remote catalog — иной статус.
Airplane-mode launch открывает локальную коллекцию без signin HTTP. Delete server
session не означает delete owned downloads; user выбирает device cleanup отдельно.
Удаление следующей песни не обрывает текущую. Disk-full говорит сколько места
нужно и даёт actions, не удаляет файлы само. Verify/repair не блокирует transport.

## 14. Explore / Connected servers

Public vs connected private явно различаются. Connect URL/invitation → capabilities/
name/license summary → connect/save/request permission. Никакого автоматического
directory registration/local address disclosure.
Origin badge читается, но не доминирует над музыкой. Offline peer не блокирует local
results; cached age и Retry live доступны. Block/report removes discovery and future
fetches, не удаляет owned originals. Rights evidence не публикуется как личный URL.

## 15. Devices, presence, rooms

Devices sheet: This device / Available / Offline; actual current track/progress только
при разрешённой видимости. Transfer confirmation/accepted/error отдельно от обычного
Play. Source продолжает до target acceptance, receipt предотвращает двойной command.

Room: имя/host/participants/queue, permission legend, sync status, invite/leave/end.
User without track access видит название и «Нет доступа» по разрешённой metadata policy,
не получает звук автоматически. Приватная listening activity не включается вступлением.
Social feed выключен по умолчанию; block/mute/revoke доступны без охоты по меню.

## 16. Settings и administration

User settings: Appearance/system theme/density, Audio/capabilities, Downloads/storage,
Privacy/history consent/device grants/export/delete, Notifications, Account/sessions,
Language/accessibility, Connected servers. Форма сохраняет scoped draft/receipt,
ошибка не сбрасывает несохранённый ввод. Consent явно говорит account+device.

Admin отдельно: storage roots/quotas/users/invitations/jobs/backup/health/network policy.
Счётчики/ошибки реальные; private history не видна администратору через app UI.
Backup UI готовит bounded job и проверяемый результат, credentials в diagnostic export
redacted. Update check только опциональный; никаких privileged auto-upgrades.

## 17. Детальные критерии UX-приёмки

1. First-run → один реальный FLAC → play ≤3 основных действия после upload completion.
2. Из Home найти альбом/артиста/плейлист, вернуться Back в тот же scroll и context.
3. Открыть/закрыть right/full player100 раз без второго audio/source restart.
4. Keyboard-only: sign-in/search/result/context menu/playlist/seek/dialog/logout.
5.320px/375px/430px/768px/1024px/1366px/1920px/3440px и 200%text, long Cyrillic/Latin.
6. VoiceOver/NVDA/TalkBack по доступным платформам: names/roles/state/reading order,
   changing track объявляется один раз, таймер не спамит.
7. Touch target44px product minimum (WCAG AA minimum другой); drag имеет альтернативу.
8. Theme/reduced motion/system setting, focus/selection/disabled/error contrast.
9. Slow/timeout/offline/403/401/429/conflict/revoke/diskfull — полезный next action.
10. Ни один cached/metadata-only result не играет только потому, что нарисован Play.
11. User-facing copy проходит ручную en/ru редактуру; не машинный смешанный жаргон.
12. Реальные usability sessions:5–8 людей в первом цикле для обнаружения проблем,
    это не статистическое доказательство превосходства и не гарантия полноты.
