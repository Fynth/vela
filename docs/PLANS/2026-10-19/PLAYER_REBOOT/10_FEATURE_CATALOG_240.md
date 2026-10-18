# 240 возможностей: продуктовый резерв с проверяемым результатом

Это не список выполненного и не команда выпустить 240 switches одновременно.
Часть основ уже существует; её требуется сохранить, расширить и довести до одинакового
поведения на нужных платформах. Точный статус берётся из source/evidence, не из таблицы.

Волны: **R1** — исправить раздражающее сейчас; **R2** — ежедневный качественный player;
**R3** — устройства, Vela ID, Android; **R4** — совместность и разрешённая сеть;
**R5** — расширение после устойчивого ядра. Внутри волны privacy/security — обязательная
часть каждой функции. R5 не означает, что accessibility/правильные permissions можно
отложить. Нативные возможности включаются только при поддержке и проверке engine.

## A. Воспроизведение — F001–F015

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F001 | Progressive Play: начинать после достаточного проверяемого буфера, не полного файла | Большой fixture начинает воспроизведение до завершения body; integrity state честный | R1 |
| F002 | Быстрый replay прослушанного: resolver использует reusable bytes | A→B→A без второго полного body A при тёплом валидном cache | R1 |
| F003 | Seek без лишнего полного перезапуска: локальные ranges и нужный HTTP Range | Seek вперёд/назад не пересоздаёт выход и не теряет pause state | R1 |
| F004 | Один устойчивый player на всех routes | 20 переходов страниц не меняют source generation/позицию | R1 |
| F005 | Fair Shuffle: стабильный deck и честные Previous/Forward | До исчерпания deck нет случайного повторения, кроме явного выбора | R2 |
| F006 | Repeat off/all/one с отдельным manual Next | EOF и Next проходят разные проверенные transitions | R2 |
| F007 | Resume session: track, queue, position, repeat/shuffle | После restart состояние восстановлено paused, без автозвука | R2 |
| F008 | Sleep timer: время или конец текущего track/альбома | Таймер срабатывает один раз и не переносится на чужую account session | R2 |
| F009 | Normalization по достоверным ReplayGain/loudness данным | Gain/peak protection соответствует измерениям, выключение восстанавливает unity | R5 |
| F010 | Gapless для проверенных codec/engine combinations | Измерен переход fixture без вставленного software gap; matrix явно ограничена | R5 |
| F011 | Crossfade с управляемой длительностью и album-aware исключением | Измерены ramps, отмена и stop; live/continuous album не режется без согласия | R5 |
| F012 | Скорость для spoken audio, без неверной music default | Фактическая скорость меняется, position/history остаются корректны | R5 |
| F013 | Mute/volume memory на данном устройстве | Remote volume не меняет случайно локальный output; UI отражает engine | R2 |
| F014 | Понятное восстановление после network loss | Ограниченный retry продолжает возле подтверждённой позиции, stale source не звучит | R2 |
| F015 | Техническая правда о source: Original/Transcoded/Offline | Badge совпадает с реально выбранной representation и backend | R2 |

## B. Очередь и контекст — F016–F030

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F016 | Предподготовка действительного следующего occurrence | Ready до EOF; новый shuffle не пересчитывается ради preview | R1 |
| F017 | Раздельные Play next / Add to end | Вставка не заменяет очередь и имеет предсказуемую позицию | R2 |
| F018 | Drag reorder и клавиатурный Move up/down | Сохраняется текущий source, курсор следует occurrence | R2 |
| F019 | Очистить upcoming, оставив current | Playing track/position неизменны; Next останавливается по policy | R2 |
| F020 | Сохранить очередь как playlist | Сохраняются порядок и дубликаты, без URLs/credentials | R2 |
| F021 | Отдельные occurrence одинаковой песни | Удаление второго повторения не удаляет первое/asset | R2 |
| F022 | Undo удаления из очереди | Возвращается нужный occurrence на устойчивое место либо виден conflict | R2 |
| F023 | Показать очередь album/playlist context отдельно от ручных вставок | Next priority соответствует видимой структуре | R2 |
| F024 | Очередь на заданное время: «до 40 минут» | Сумма duration близка к лимиту, unknown durations явно отмечены | R5 |
| F025 | Альбомы как цельные группы в очереди | Перестановка группы сохраняет disc/track order внутри | R5 |
| F026 | Несколько именованных listening sessions | Переключение не теряет предыдущую очередь и восстанавливает paused | R3 |
| F027 | Autoplay с объяснимым продолжением после context | Добавляются только доступные кандидаты; можно отключить/просмотреть | R5 |
| F028 | Политика unavailable: skip / ask / stop | Нет бесконечного loop на полностью недоступном наборе | R2 |
| F029 | Queue history текущей сессии без выдачи за listening history | Previous работает, даже если приватный сбор истории выключен | R2 |
| F030 | Очередь из разных origin с явными метками | Local UUID collision не подменяет другой remote track | R4 |

## C. Кэш и офлайн — F031–F045

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F031 | Ограниченный autocache недавно полученного | Повторный Play экономит body, quota не растёт бесконечно | R1 |
| F032 | Отдельные pinned downloads, не подлежащие обычному eviction | Очистка автокэша оставляет сохранённый альбом | R2 |
| F033 | Cold offline launch без рабочего сервера | Restart в airplane mode открывает разрешённый local catalog | R2 |
| F034 | Download album/playlist с поэлементным статусом | Частично failed collection не выдаётся за полностью готовую | R3 |
| F035 | Resume transfer по сильному validator | Изменившийся ETag начинает новую representation, не склеивает версии | R2 |
| F036 | Одна загрузка на несколько consumers | Prefetch + Play + Save не получают три одинаковых body | R1 |
| F037 | Wi-Fi/metered/roaming policy | Speculative traffic прекращается на запрещённой сети, current управляется отдельно | R3 |
| F038 | Автоматически поддерживать выбранные playlists offline | Revision diff скачивает новые и предлагает судьбу удалённых элементов | R5 |
| F039 | Storage inspector: pinned/cache/staging/other | Суммы сопоставимы с disk usage; активный lease учитывается | R2 |
| F040 | Smart eviction только неприкреплённых данных | Игровой current и pinned не удаляются ради speculative next | R1 |
| F041 | Offline artwork/lyrics/credits вместе с музыкой | Карточка и lyrics открываются без network, missing данные честно пусты | R3 |
| F042 | Проверка integrity и восстановление повреждённой копии | Corrupt fixture не декодируется как healthy; repair не уничтожает backup | R2 |
| F043 | Low-storage flow с точным планом освобождения | Есть нужные bytes/action; originals на сервере не удаляются | R2 |
| F044 | Export portable download manifest без secrets | Импорт сопоставляет origin/assets и повторно проверяет права | R5 |
| F045 | Device-bound private storage и account isolation | Другой пользователь не видит covers/queue/bytes предыдущего | R1 |

## D. Поиск и навигация — F046–F060

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F046 | Debounced Search с cancellation и reply ordering | Медленный ответ старого query не заменяет новый | R2 |
| F047 | Typed search tracks/albums/artists/playlists | Каждая группа открывает настоящий объект с правильным ID | R2 |
| F048 | Keyboard suggestions и command palette | Полный путь Search→Play доступен без мыши | R2 |
| F049 | Одновременный поиск connected servers с bounded fanout | Offline peer не задерживает local results до своего timeout | R4 |
| F050 | Явный scope All / This server / Downloaded | Scope фильтрует доступность, а не только подпись | R3 |
| F051 | Очистка opt-in search history | После Clear подсказки не возвращаются из скрытой второй копии | R2 |
| F052 | Поиск без диакритики с сохранением оригинального написания | Fixture café/cafe находится; display title не переписывается | R5 |
| F053 | Осторожная typo tolerance | Опечатка даёт ранжированный suggestion, не подменяет точное совпадение | R5 |
| F054 | Поиск по credits/композитору/album artist | Ответ содержит только реально заполненные и разрешённые поля | R5 |
| F055 | Advanced filters без пользовательского SQL | Codec/year/size/favorite rules параметризованы и bounded | R2 |
| F056 | Jump to letter с многоязычным collation | Кириллица/латиница/числа имеют предсказуемую группировку | R5 |
| F057 | Сохранённые search views | Восстанавливаются typed filters и sort, а не private result dump | R5 |
| F058 | Browser-like Back с восстановлением scroll | Возврат к album wall не прыгает наверх и не останавливает audio | R2 |
| F059 | Local/remote provenance прямо в выдаче | Два одноимённых объекта различимы без открытия diagnostics | R4 |
| F060 | Быстрые действия из результатов | Add next/favorite открывают нужный origin и не теряют input | R2 |

## E. Библиотека и организация — F061–F075

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F061 | Настоящие covers в Linux native, включая lazy loading | Artwork owner checks действуют, placeholders остаются лишь при отсутствии | R1 |
| F062 | Album wall и плотный list mode | Оба режима показывают один paginated catalog без загрузки всех rows | R2 |
| F063 | Library pins и недавние коллекции | Pin переживает restart, не означает public sharing | R2 |
| F064 | Личные folders для playlists/collections | Удаление folder не удаляет media/дочерние playlists | R2 |
| F065 | Multi-library membership и быстрый scope | Private library не просачивается через counts/artwork | R4 |
| F066 | Editions/masters и несколько encodes одной recording | Выбор FLAC не склеивает разные live/remix recordings | R5 |
| F067 | Корректные multi-disc и compilations | Album Play соблюдает disc+track order и track artist | R2 |
| F068 | Favorites tracks/albums/artists разного смысла | Unfavorite artist не удаляет track likes автоматически | R5 |
| F069 | Коллекции «послушать позже» для albums | Открываются реальные объекты, отметка listened не выдумана | R5 |
| F070 | Read-only watched folder с UI progress | Rescan сохраняет ручные edits и playlist item identities | R2 |
| F071 | Bulk selection с preview действия | Выбраны только видимые/явно заявленные objects; per-item failures видны | R2 |
| F072 | Logical duplicate review без автоматического удаления | Разные encodes группируются как предложение, originals целы | R5 |
| F073 | Missing file repair / relink | Смена пути не ломает stable track ID и playlists | R5 |
| F074 | Пользовательские tags и быстрые tag views | Private tags не становятся публичными genre metadata | R5 |
| F075 | Library health как отдельная спокойная utility page | Ошибки видны владельцу без захвата Home технической панелью | R2 |

## F. Плейлисты и совместное редактирование — F076–F090

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F076 | Create в небольшом modal вместо постоянных форм | Пустая страница остаётся музыкальной, ввод/error/focus сохранены | R1 |
| F077 | Description и собственная cover/генерация из разрешённых covers | Нет внешней картинки без прав; revoke artwork отражён | R2 |
| F078 | Duplicate playlist с сохранением occurrence order | Создан новый ID/owner, чужие permissions не копируются молча | R2 |
| F079 | Search within playlist | Playing context остаётся исходным, filtered row не меняет скрыто порядок | R2 |
| F080 | Collaborative reorder через CAS/idempotent commands | Concurrent add/move не теряют чужие изменения | R4 |
| F081 | Near-real-time changes и activity receipt | После reconnect повторный event не применяет add второй раз | R4 |
| F082 | Named collaborators с granular editor rights | Metadata view не даёт media stream/edit автоматически | R4 |
| F083 | Revocable links с expiry и ограниченным scope | Старый link перестаёт открывать новые ресурсы после revoke | R4 |
| F084 | Smart playlist visual rule builder | Typed preview/count соответствует evaluation, нет raw SQL | R5 |
| F085 | M3U/M3U8 import/export с unresolved entries | Не найденные треки не исчезают бесследно | R5 |
| F086 | Queue→playlist и playlist→queue с контекстом | Не дублируется команда при double tap/retry | R2 |
| F087 | Follow shared playlist отдельно от local duplicate | Update origin виден, пользовательские копии не перезаписываются | R4 |
| F088 | Доступный drag/drop между sidebar и content | Есть клавиатурная альтернатива и rollback при отказе | R2 |
| F089 | Owner transfer с подтверждением обеих сторон | Нельзя передать незнакомцу через подставленный UUID | R5 |
| F090 | Version/activity view для восстановления ошибок редакторов | Recovery создаёт новую revision, не стирает audit | R5 |

## G. Рекомендации и личное открытие — F091–F105

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F091 | Home из настоящих доступных sections | При пяти tracks нет пяти фиктивных «ежедневных mixes» | R2 |
| F092 | Heavy Rotation из consented qualified listening | При выключенном consent сохранённая история не участвует даже в suppression | R2 |
| F093 | Forgotten Favorites с понятной причиной | Есть реальные favorites и достаточно разрешённых данных для давности | R5 |
| F094 | Track Radio по signal/tag fallback | Кандидаты owner-authorized, причина similarity объяснима | R2 |
| F095 | Artist/Album/Playlist Radio | Seeds ограничены, исключения и diversity проверены | R5 |
| F096 | Weekly Discovery snapshot с durable refresh | Неделя/версия/причина набора сохранены, это не переименованный on-demand | R5 |
| F097 | Несколько taste-cluster mixes | Наборы отличаются по данным; при слабой базе предлагается один | R5 |
| F098 | New Additions отдельно от достоверных New Releases | Import timestamp не выдаётся за release date | R2 |
| F099 | Why this / Less like this / Undo | Receipt устойчив к refresh; скрытое исчезает до candidate cap | R2 |
| F100 | Novelty/diversity control для radio | Изменяется ranking с измеряемым coverage, не только UI caption | R5 |
| F101 | Album-first discovery для цельного прослушивания | Не обрывает continuous album случайными singles без выбора | R5 |
| F102 | Настроение как пользовательский tag/context | Нет ложного утверждения, что RMS распознал happy/sad | R5 |
| F103 | Time-limited journey с понятной последовательностью | Доступные tracks укладываются в duration budget и объясняют подбор | R5 |
| F104 | Mutual mix с явным согласием участников | Вывод не раскрывает raw private history и учитывает playable intersection | R4 |
| F105 | Локальный spoken host как опциональный поздний режим | Без доступного licensed/local TTS обычный mix работает; narration можно отключить | R5 |

## H. DSP и качество — F106–F120

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F106 | Explicit background audio analysis с progress/cancel | Не декодирует в stream path/UI, очередь bounded и durable | R2 |
| F107 | Версии extractors и immutable descriptor cache | Новый алгоритм не переинтерпретирует старые vectors молча | R2 |
| F108 | Waveform overview с multiresolution buckets | Zoom/seek корректны на длинном fixture без огромного DOM | R5 |
| F109 | Loudness/R128 analysis отдельным проверенным extractor | Измерения сопоставлены с эталоном; RMS не подписан LUFS | R5 |
| F110 | True-peak/clip risk с корректным методом | UI различает sample peak и проверенный intersample estimate | R5 |
| F111 | ReplayGain review и non-destructive playback gain | Original tags/files не переписываются без разрешения | R5 |
| F112 | Tempo estimate с confidence и «не определено» | Click/silence/live fixtures проверены; нет обещания точности на всей музыке | R5 |
| F113 | Chroma/key estimate как явно heuristic tool | Неуверенный результат не используется как достоверный genre/mood | R5 |
| F114 | Similar-sounding radio без выдачи similarity за recording identity | Похожие tracks остаются разными masters/performances | R5 |
| F115 | Audio format inspector с original/actual path | Lossy не маркируется lossless из-за контейнера/расширения | R2 |
| F116 | Capability-negotiated transcode только при необходимости | Поддерживаемый FLAC direct-play не транскодируется автоматически | R5 |
| F117 | Контролируемые FFmpeg presets и cancellation | Subprocess без shell URL, ограничен concurrency, завершён после cancel | R5 |
| F118 | Equalizer с безопасным preamp на поддержанном backend | Реальный сигнал меняется; unsupported engine не показывает рабочий toggle | R5 |
| F119 | Per-device output preference и unplug recovery | Headphone removal не создаёт второй output/неожиданную громкость | R3 |
| F120 | Read-only integrity/quality comparison encodes | Пользователь видит measurements и выбирает asset; ничего не удаляется | R5 |

## I. Metadata, artwork, lyrics — F121–F135

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F121 | Embedded artwork pipeline с authorised thumbnails | Size/decode caps и owner checks действуют и для cached images | R2 |
| F122 | Album-art-derived accent без потери contrast | Светлая/тёмная/одноцветная cover не прячет text/focus | R2 |
| F123 | Ручные catalog edits с provenance | Rescan не затирает подтверждённый user edit | R2 |
| F124 | Review неоднозначного MusicBrainz matching | Слабое совпадение не применяется без согласия | R5 |
| F125 | Album artist / featured artists / composers как разные credits | Artist links не склеиваются по display string | R5 |
| F126 | Editions и release-date precision | Year-only не выдумывает 1 января как точную дату релиза | R5 |
| F127 | Embedded plain lyrics без обязательного provider | Lyrics показываются offline и только разрешённому owner | R5 |
| F128 | LRC synced lyrics с manual scroll/follow | Highlight следует реальному position; seek корректно перематывает строки | R5 |
| F129 | Личный lyrics correction/offset | Изменение локального offset не публикует защищённый текст | R5 |
| F130 | Digital booklet/liner notes с безопасным viewer | Только авторизованные attachment MIME/size, без arbitrary active content | R5 |
| F131 | Label/license/source fields там, где известны | Unknown явно остаётся unknown и не даёт download rights | R4 |
| F132 | Разрешённые artist biographies и photo attribution | Есть provider/provenance, небезопасный HTML не исполняется | R5 |
| F133 | Batch metadata edit с preview и undo policy | Применяются выбранные fields, ошибки по tracks не скрыты | R5 |
| F134 | Сравнение embedded / enriched / user values | Видно происхождение, user choice не меняет original file автоматически | R5 |
| F135 | Explicit tag write-back с backup и dry preview | Требует отдельного разрешения, read-only sources защищены | R5 |

## J. Устройства, sync, handoff — F136–F150

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F136 | Named device registry и expiry presence | Offline device не показывается онлайн бесконечно | R3 |
| F137 | Синхронизация сохранённой queue между устройствами | Stable occurrences, revisions и explicit conflict handling | R3 |
| F138 | Position checkpoints с bookmark, restore paused | Позиция не прыгает назад из-за позднего старого checkpoint | R3 |
| F139 | Explicit handoff с READY/ACK/rollback | A играет до подготовки B, commit не происходит по одному spinner | R3 |
| F140 | Remote play/pause/next с authorization | Другой account не может контролировать device по UUID | R3 |
| F141 | Remote volume только capability-aware | Неподдерживаемое действие не притворяется выполненным | R3 |
| F142 | Независимое одновременное listening | Новый Play на B не останавливает A без explicit transfer/policy | R3 |
| F143 | Переключение control target с заметным именем | Человек понимает, какой output изменяет до нажатия Play | R3 |
| F144 | Reconnect command dedup и sequence repair | Повторный delivery не делает двойной Next | R3 |
| F145 | Revocation device/session | Новые команды/streams отозванной сессии отвергаются | R3 |
| F146 | Offline operation journal для favorites/queue edits | Reconnect не перезаписывает чужую актуальную очередь целиком | R5 |
| F147 | Browser autoplay handoff confirmation | Target требует user gesture, source не останавливается преждевременно | R3 |
| F148 | История личных session bookmarks без public presence | Список не означает listening consent и чистится отдельно | R3 |
| F149 | Cross-server source resolution при handoff | Origin grants проверены target; токен A не пересылается произвольному B | R4 |
| F150 | Device capabilities и честное degraded mode | Unsupported seek/output/offline не обещаны в Devices UI | R3 |

## K. Независимый Vela ID — F151–F165

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F151 | Account вне конкретного music server | Пользователь существует как issuer+subject без локальной media library | R3 |
| F152 | Hosted sign-in как отдельный будущий сервис | Нет обязательной зависимости для существующего local install | R3 |
| F153 | Self-hosted совместимый identity/workspace вариант | Можно выбрать provider/coordinator без фирменного центра | R3 |
| F154 | System browser OAuth Code+PKCE для native | Reject wrong state/issuer/nonce; secrets не в WebView storage | R3 |
| F155 | Явное linking local membership к Vela ID | Одинаковый email не связывает чужие accounts автоматически | R3 |
| F156 | Список подключённых серверов после sign-in | Показываются connections, не обещается доступ без server membership | R3 |
| F157 | Cloud workspace для preferences и session pointers | По умолчанию нет media bytes/raw listening history | R3 |
| F158 | Per-server scoped authorization и disconnect | Token одного origin не уходит другому origin | R3 |
| F159 | Passkeys/MFA через проверенный provider | Реальная recovery strategy; не собственный crypto protocol | R5 |
| F160 | Session/security dashboard и revoke | Видно активные sessions с приватным минимумом данных | R3 |
| F161 | Recovery без потери независимого local admin | Отказ IdP не блокирует разрешённый emergency local access | R3 |
| F162 | Portable account/workspace export | Нет credentials в export, документированы limits переезда | R5 |
| F163 | Удаление cloud profile отдельно от media ownership | Не удаляются оригиналы/server accounts без отдельного действия | R3 |
| F164 | Opt-in encrypted sync payload после отдельного security design | Проверены key recovery/device revoke; label E2EE появляется только после review | R5 |
| F165 | Cloud outage mode с локальным listening | Уже разрешённые cached/local functions не ждут IdP network каждый Play | R3 |

## L. Комнаты и социальность — F166–F180

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F166 | Private listening room с expiring invitation | Link даёт room admission по policy, не неограниченный media token | R4 |
| F167 | Shared timeline и bounded drift correction | Измерены offset, reconnect и clock error, нет ложного sample-perfect обещания | R4 |
| F168 | Roles host/editor/listener и participant permissions | Listener не меняет queue простым поддельным WS event | R4 |
| F169 | Suggest/vote tracks как отдельная очередь предложений | Votes idempotent; host policy определяет принятие | R4 |
| F170 | Ready/access state участников | Missing grant показан без выдачи чужого private metadata | R4 |
| F171 | Персональный queue сохраняется при входе в room | Leave предлагает restore paused / continue, без потери старой сессии | R4 |
| F172 | Follow requests и opt-in profiles | Private пользователь не появляется в search/directory без согласия | R4 |
| F173 | Mute/block с локальной и server policy | Заблокированный не обходит запрет через повторную invitation | R4 |
| F174 | Playlist invitation inbox | Accept/reject/revoke имеют durable состояние и не спамят | R4 |
| F175 | Presence «слушает сейчас» только по отдельному согласию | Device heartbeat не включает публичную activity | R4 |
| F176 | Private monthly/yearly listening story | Минуты из corroborated media time, export/share только по явному действию | R5 |
| F177 | Music notes для друзей/playlist items | Текст безопасен, visibility наследуется явно, доступен edit/delete | R5 |
| F178 | Licensed artist-led room/event | Scheduling не даёт отсутствующих streaming rights | R5 |
| F179 | Общий mix без раскрытия private raw history | Участник видит согласованный aggregate и может отозвать участие | R4 |
| F180 | Notification preferences / quiet hours | Нет push/listening disclosure при выключенном consent | R5 |

## M. Законная музыкальная сеть — F181–F195

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F181 | Manual connect server с identity/capability preview | Private addresses не публикуются автоматически | R4 |
| F182 | Public collections только explicit publish | Enabling federation не открывает owner library | R4 |
| F183 | Отдельные metadata/stream/download grants | View title не позволяет получить оригинал без grant | R4 |
| F184 | Rights/provenance review для публикации | Unknown/недостаточные основания не выдаются за CC/public domain | R4 |
| F185 | Open-license catalog adapters с соблюдением source rules | У конкретного asset разрешён download, license/source показаны | R4 |
| F186 | Artist direct-support links из достоверных metadata | Нет скрытого affiliate/payment и ложной продажи чужих rights | R5 |
| F187 | ActivityPub/WebFinger profiles при доказанной совместимости | Два независимых implementation обмениваются проверенными типами | R5 |
| F188 | Origin-authenticated update/removal events | Forgery/replay/чужой update отвергаются | R4 |
| F189 | Scoped remote metadata cache | Cached object не становится locally owned media | R4 |
| F190 | Per-peer deadlines/circuit breaker | Медленный peer не занимает весь search/stream capacity | R4 |
| F191 | Server/user blocklists и complaint flow | Abuse action влияет на admission/delivery/fetch, audit сохранён | R4 |
| F192 | OpenSubsonic catalog adapter | Проверен реальный compatible server; не назван федерацией | R5 |
| F193 | Cross-server playlist references без autocopy music | Resolve использует разрешённый source либо missing state | R4 |
| F194 | Отзыв публикации с tombstone/expiry | Новые fetch запрещены; limits уже выданных bytes честно объяснены | R4 |
| F195 | Independently hosted opt-in directories | Private server работает без directory, registration добровольна | R5 |

## N. Mobile и OS-интеграции — F196–F210

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F196 | Android Media3 service, не WebView audio workaround | Физический device играет с погашенным экраном и app background | R3 |
| F197 | Android media notification / lockscreen | State и buttons отражают один настоящий engine | R3 |
| F198 | Android focus/headset/Bluetooth lifecycle | Звонок, unplug и reconnect не дают overlap/утечку playback | R3 |
| F199 | Android offline data source | Airplane mode после process restart воспроизводит pinned file | R3 |
| F200 | iOS AVFoundation/AVAudioSession engine | На реальном устройстве interruption/route changes проходят тест | R5 |
| F201 | iOS Control Center/background audio | Подписанный app с capability проверен на physical device | R5 |
| F202 | iOS background downloads с OS constraints | Suspension/resume не теряет manifest/целостность | R5 |
| F203 | Linux Hyprland нормальное tiling/scale | Без forced floating и глобальных WM overrides нет лишней полосы | R1 |
| F204 | Linux MPRIS parity для local/offline/remote | Hardware Next/Pause соответствуют тем же transitions | R2 |
| F205 | Windows SMTC и real installer | Real packaged app управляется системными media buttons | R5 |
| F206 | macOS media commands/audio session integration | Sleep/wake, headset и close semantics проверены на macOS | R5 |
| F207 | Secure credential stores отдельно по OS | Нет plaintext password/token в logs/manifest/WebView storage | R3 |
| F208 | PWA offline shell при поддержке browser policy | Не назван native background client; offline limits видны | R5 |
| F209 | Tablet/foldable responsive layouts | Rotation/resizing сохраняют playing source и focus | R3 |
| F210 | Notification actions для реальных задач | Import completion/open/room invite работают без публичной history | R5 |

## O. Характер интерфейса и интерактивность — F211–F225

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F211 | Continuous canvas без коробки на каждую сущность | Home/Library/Player имеют разные композиции и единую навигацию | R2 |
| F212 | Dark/light/system с artwork accent | Contrast/focus проходят проверку на случайных разрешённых covers | R2 |
| F213 | Hover/focus Play на cover без layout jump | Keyboard и touch имеют тот же action, text не перекрыт | R2 |
| F214 | Живой playing marker с текстовой альтернативой | Не зависит только от цвета/animation, reduced motion работает | R2 |
| F215 | Resizable desktop panels с разумными limits | Узкий tile всегда оставляет доступные transport/navigation | R1 |
| F216 | Настоящий phone mini-player/full-screen player | Safe area, gestures, focus и controls проверены на телефоне | R3 |
| F217 | Context menus/long press с доступными командами | Правильный origin/item, закрытие/positioning/focus возвращаются | R2 |
| F218 | Inline rename и ограниченный modal Create | Нет постоянно открытых огромных форм на playlist landing | R1 |
| F219 | Optimistic feedback с rollback/undo | Failed favorite не остаётся ложным success после refresh | R2 |
| F220 | Skeletons по форме реального content | Нет fake rows/music; нагрузка и retry различимы | R2 |
| F221 | Typography-first artist/playlist headers | Длинная кириллица и OS text scale не ломают controls | R2 |
| F222 | Album booklet / immersive listening mode | Выход и transport доступны, музыка не remount при смене режима | R5 |
| F223 | Настраиваемая плотность и compact queue | Text остаётся читаемым, touch targets не становятся 12px | R2 |
| F224 | Подробные keyboard shortcuts и remapping конфликтов | Shortcut не захватывает text input/OS reserved keys | R5 |
| F225 | Original generated artwork для пустых обложек | Deterministic art не меняет layout и не кодирует private listening data | R2 |

## P. Удобство владения, приватность, переносимость — F226–F240

| ID | Возможность и конкретное поведение | Проверяемый результат | Волна |
|---|---|---|---|
| F226 | Add Music → Upload / URL / Scan в приложении | Новый разрешённый track попадает в Library без SSH | R2 |
| F227 | Batch/resumable uploads с staging/quota | Restart/cancel не публикуют partial asset и не теряют healthy duplicate | R5 |
| F228 | Import preview с source/files/license/progress | Выбранное соответствует downloaded asset, SSRF tests проходят | R2 |
| F229 | Safe ZIP album import | Path traversal/symlink/zip-bomb/expansion limits отвергаются | R5 |
| F230 | Trusted WebDAV/S3 connectors отдельной policy | Internal connector не ослабляет public URL SSRF protections | R5 |
| F231 | Private listening account+device consent | Upgrade не opts in; monotonic media-time пороги проверены | R2 |
| F232 | History export/delete с privacy epochs | Late outbox/held export после delete не возвращает удалённое | R2 |
| F233 | Отдельные sync/presence/history/privacy настройки | Выключение presence не отключает нужную очередь и наоборот | R3 |
| F234 | Portable metadata/favorites/playlists export | Нет account secrets, unresolved origins документированы | R5 |
| F235 | Backup и restore на fixture copy через понятный admin flow | Восстанавливаются DB/media/config без уничтожения оригинальных данных | R2 |
| F236 | Health/job center с reauthorized actions | Aggregate не раскрывает raw URLs/diagnostics, cancel/retry проверяет owner | R2 |
| F237 | Diagnostic bundle с preview/redaction | По умолчанию нет track paths, cookies, emails, private history | R2 |
| F238 | Small-server profiles и bounded workers | Import/analysis/transcode не лишают current stream ресурсов | R2 |
| F239 | Честные обновления с build identity и rollback plan | Installed build/assets/source различимы; migrations совместимы с restore | R1 |
| F240 | Accessibility/i18n как release criterion | Keyboard, screen reader, 320px, Cyrillic и reduced motion проверены | R2 |

## Как этим пользоваться

1. Не заводить 240 параллельных PR. Сначала выполнить R1 из
   [dependency plan](11_EXECUTION_DEPENDENCIES.md).
2. Перед реализацией каждого F-ID найти существующий код/тест: PRESERVE / EXTEND /
   NEW / BLOCKED. Этот каталог не заменяет actual parity ledger.
3. Перенести F-ID в bounded ticket с file ownership, contract, security cases,
   acceptance и exact evidence. Результат «скомпилировано» не закрывает native/audio UX.
4. Новая функция не появляется в primary UI до backend/engine capability. Unsupported
   control объясняет ограничение либо вообще не засоряет повседневный экран.
5. Оценивать не число toggles, а отсутствие раздражения: Play, повторный Play, Next,
   найти album, собрать playlist, передать телефон, слушать offline, вернуть свои данные.
