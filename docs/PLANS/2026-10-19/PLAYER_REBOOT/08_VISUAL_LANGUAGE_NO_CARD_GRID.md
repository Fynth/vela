# Визуальная концепция: живая музыкальная коллекция

## 1. Что меняется по сравнению со скриншотами и прежним макетом

Сейчас интерфейс показывает главным образом формы, пустые прямоугольники,
нейтральные иконки вместо artwork и длинные технические пояснения. Ранее предложенный
Atlas тоже нельзя механически превратить в ещё одну сетку одинаковых карточек.
Новая цель — **continuous music canvas**: музыка, люди и связи образуют композицию,
а формы/диагностика раскрываются в нужном контексте.

Не требуется избавиться от всех прямоугольников: настоящая обложка альбома обычно
квадратная. Требуется перестать заключать каждый заголовок, объяснение и кнопку в
отдельный одинаковый bordered card, раздувающий пустоту.

## 2. Собственный характер Vela

Три визуальных слоя:

1. **Музыка:** большие настоящие обложки, artist imagery с разрешением, typography
   релизов, playlist collage, горизонтальные listening strips, credits/liner notes.
2. **Навигация:** спокойная непрерывная поверхность, тонкие разделители, ясные заголовки,
   удобная плотность, постоянный плеер и разумный optional context.
3. **Действия:** один сильный primary Play на текущем объекте, остальное по состоянию
   и в context menu. Не6равнозначных кнопок на каждом треке.

Влияние профессиональных систем — редакционная сетка, ясность иерархии, музыка как
визуальный материал, meaningful icons — не копирование чужой brand surface.
Никаких Spotify логотипов/заимствованных album photographs/имитации известных дизайнеров.

## 3. Цвет: богато, но управляемо

База: светлый тёплый нейтральный и тёмный near-neutral; не одна зелёная плоскость
для всех экранов. Vela accent остаётся стабильным semantic знаком, а музыкальный
контекст даёт дополнительные bounded цвета.

- Album/Now Playing берёт локально вычисленный dominant/secondary tone разрешённого
  artwork и использует его в ограниченной backdrop/полосе/прогрессе.
- Recommender shelf может иметь одну art-led color field без рамки вокруг каждого item.
- Origin/rights/error/focus имеют постоянную семантику, не переокрашиваются от картинки.
- При плохом artwork palette extraction fallback выбирает contrast-tested pair.
- Background не состоит из бесконечных неоновых gradients, glass panels и blur blobs.
- Light theme равноценна: читаемые controls, не просто инверсия чёрного.
- Artwork занимает внимание, не маскирует title/seek текст; overlay contrast проверяется.
- User может выключить dynamic color/animation без потери навигации.

## 4. Композиции вместо одной карточной сетки

| Поверхность | Основная композиция | Когда уместны карточки |
|---|---|---|
| Home | Listening strip + выделенный album/editorial feature + открытые shelves разной плотности | Playlist/mix preview с artwork, без рамок и длинных описаний |
| Search | Ranked result list, компактные album/artist rows и scoped tabs | Browse topics как осмысленные poster tiles |
| Library | Адаптивный album wall или dense track list с headers | Только выбираемый режим grid, не единственный layout |
| Album | Artwork+title area и непрерывный tracklist по дискам | Edition chooser/popover |
| Artist | Hero, горизонтальные release strips, popular owned tracks | Relational/release summaries |
| Playlist | Выразительная cover/title, toolbar и открытый track table | Grid коллекции плейлистов при реальном наличии |
| Queue | Группированный список Now/User next/Context с линиями порядка | Не отдельная «карточка» на каждую строку |
| Now Playing | Artwork-led full view, typography, transport, lyrics | Только menu/sheets |
| Room | Музыкальная сцена, participant ribbon и shared queue | Participant popover/permission sheet |
| Downloads | Storage summary и item rows со статусом/bytes | Error details/onboarding |
| Settings | Section navigation и аккуратные строки controls | Destructive confirmation |

Одинаковую hero-конструкцию не копировать на каждую страницу с новым h1.

## 5. Типографика и плотность

Сохранить выбранные self-hosted Cyrillic/Latin fonts при подходящей лицензии;
Golos Text — UI, Source Serif4 — редкие editorial/liner notes. Album/artist headings
можно выразить весом/размером body family, serif не обязательна всюду.
Desktop основной текст14–16px, mobile16; utility12–13,9px не несёт важного смысла.
Track title/artist не исчезают в крошечной подписи. Large text200% проверяется.

H1 не обязан быть огромным приветствием. Header библиотеки компактен, больше места
музыке. Отступы4/8/12/16/24/32/48, осмысленные горизонтальные gutters. Compact mode
меняет row height/padding, но не делает текст/targets недоступными.

## 6. Artwork — обязательная часть музыкальной среды

Native сейчас рисует neutral placeholders даже при artwork ID. Добавить authenticated
bounded image service/cache: origin+account+art ID, variant sizing, decode outside UI,
lazy loading, request cancellation/freshness, fallback при forbidden/missing/corrupt.
Image route не обходит приватность; collaborator metadata не даёт cover permission.

Оригинальные user-upload/embedded covers показываются без произвольной дорисовки.
Для playlists — editable collage из доступных own artwork или собственная композиция.
Для missing album — узнаваемая Vela-placeholder с title initials/материалом, не fake
официальная картинка. Artwork status не задерживает Play или keyboard focus.

## 7. Motion и интерактивность

- Hover: появляется contextual Play/action, item не прыгает и не сдвигает соседей.
- Current track: маленький state indicator, понятный в paused и reduced-motion.
- Navigate album/artist: короткий controlled cover/title transition, один source не трогается.
- Queue drag: реальный insertion marker, live order preview, keyboard alternative.
- Mobile player expand/collapse сохраняет визуальную continuity обложки и focus.
- Like/save: короткая state confirmation, bounded Undo, no confetti spam.
- Tabs/filter: smooth state change без layout thrash и long loading flash.
- Lyrics: time highlighting, user scroll lock, no forced constant animations.
- Progress: работает от native clock; waveform только из реальных derived data.
- Reduced motion и battery saver отключают несущественное движение.

Motion budgets сначала короткие100–200ms, opacity/transform; измерить frame/input
latency на реальном Dioxus/WebKit/device. Красота не оправдывает пропуски звука/фризы.

## 8. Детерминированное оформление

Хэш допустим для собственных fallback artwork/playlist compositions: stable public
object identity + versioned art recipe, ограниченное семейство проверенных layouts.
Не рандомизировать расположение buttons, type sizes, семантические цвета и navigation.
Не хэшировать password/history/private taste в публичный рисунок.
Случайность — материал иллюстрации, не замена дизайнерского решения.

## 9. Что убрать из primary flows

Полную форму New folder всегда сверху; форму New playlist на всю ширину; строку
Revision как главный subtitle; постоянный «Owner-authorized metadata…» paragraph;
URL сервера под именем на каждой музыкальной странице; большой success banner после
каждого действия; обязательные refresh controls как normal mode; production labels
NATIVE/decoder/DSP bytes; пустую рамку на полэкрана.

Вместо этого: компактный Create menu→dialog/sheet, inline rename, contextual sharing
legend, origin indicator при необходимости, scoped ephemeral toast, актуальные data
updates и полезный музыкальный empty state. Diagnostics остаётся доступной в details.

## 10. Критерий «не выглядит как сгенерированный dashboard»

1. При выключенных borders композиция и hierarchy остаются понятными.
2. У Home/Artist/Album/Playlist разные роли и разные пропорции, не один template.
3. Первый взгляд видит музыку/контекст/Play, не статистику/администрирование.
4. Без artwork UI остаётся красивым и честным, не пустым набором giant grey squares.
5. Длинные объяснения не повторяются на каждой карточке; один короткий reason и disclosure.
6. Цвет связан с музыкой/состоянием, не случайный градиент для «премиальности».
7. Все visible controls выполняют action/имеют disabled reason; фальшивых celebrities нет.
8. Phone/tablet/desktop — самостоятельные проверенные композиции, не scale одного png.
9. Actual usability: find/play/queue/share/offline заметно проще; одного screenshot review мало.
10. Performance и accessibility проверяются до удаления прежней working theme.

## 11. Конкретная стартовая палитра для следующего design review

Характер: **типографика музыкального журнала + ясность хорошего проигрывателя**.
Рабочее название направления — «Vela / Side A». Это обозначение дизайн-плана,
не необходимость менять название продукта или рисовать кассеты на каждом экране.
Акцент тёплый, окружающая поверхность чернильная/бумажная; обложки дают разнообразие.
Не использовать сплошной зелёный фон нынешнего native как единственную идентичность.

| Роль | Dark | Light | Применение |
|---|---|---|---|
| Основной canvas | `#11141B` | `#F6F3EB` | Непрерывная музыкальная поверхность |
| Navigation/transport | `#181C25` | `#EDE8DE` | Спокойный постоянный каркас |
| Raised menu/dialog | `#242A35` | `#FFFCF5` | Только действительно отделяемый слой |
| Main text | `#F6F1E7` | `#20242C` | Названия и основные controls |
| Secondary text | `#B9BEC9` | `#565D69` | Credits, durations, supporting labels |
| Brand accent | `#FF8A64` | `#A83A20` | Выбранные actions, небольшие акценты |
| Secondary musical accent | `#A9BCFF` | `#3655A5` | Дополнительный art-led контекст |
| Primary Play surface | `#F6F1E7` | `#20242C` | Контрастная заметная кнопка |
| Primary Play ink | `#171B24` | `#FFF9ED` | Иконка/label primary Play |
| Focus | `#B8C8FF` | `#284999` | Стабильный outline, не dominant artwork color |
| Success | `#A0DEBC` | `#23623E` | Редкие semantic confirmations |
| Warning | `#F3CA84` | `#79500D` | Storage/buffering attention с текстом |
| Error | `#FFA7A7` | `#A32332` | Отказ/нужное действие, не permanent red banner |

Эти значения — кандидаты, не закодированные tokens и не измеренный contrast PASS.
Проверить реальные composited surfaces, disabled/hover/focus, artwork overlay и
цветовую слепоту по [reference criteria](14_REFERENCE_READING.md). Неконтрастный
candidate исправляется, а не защищается ссылкой на «авторский стиль».

Dynamic palette ограничить двумя контекстными оттенками на странице. Тёмный art
backdrop не должен превращать все controls в тёмно-зелёные; text не окрашивать
случайным dominant hue. Для светлой темы использовать спокойную tint область,
а не тот же насыщенный dark gradient поверх paper background.

## 12. Размеры, формы и глубина

| Элемент | Стартовое правило |
|---|---|
| Main text | 15px desktop /16px phone, line-height около1.45 |
| Track title | 15–16px medium, artist13–14px с читаемым contrast |
| Page heading | 28–36px; большой album/artist title до56–72 при доступной ширине |
| Utility | 12–13px; права/ошибки не прятать в9px |
| Desktop row | 56–64px normal,44–48 compact с отдельными target constraints |
| Touch row/actions | Рекомендуемые44–48px targets; spacing проверять по accessibility |
| Cover thumbnail | 40–48px row; album artwork160–240 в коллекции, адаптивно |
| Album/Now Playing hero | 240–360px desktop, fluid phone с сохранением controls |
| Radius | 4–6px у cover,8–12 у dialog/menu; round только настоящие круговые controls |
| Focus outline | Видимый2–3px с offset, не сливается с artwork |
| Разделение | Пробел/выравнивание/тонкая линия; border не вокруг каждой строки |
| Motion | Feedback100–140ms, panel180–220ms; reduced motion immediate/fade |

Значения не должны побеждать OS font scale/320px reflow. После text scaling дать
перенос/изменение layout, не просто обрезать текст до одной буквы. Track controls
не исчезают полностью до hover; selected/focused состояние равноценно.

Глубина нужна для menu/dialog/drag preview. Постоянные огромные shadows на каждой
обложке, glass blur и круглая карточка вокруг любого subtitle запрещены как default.
Пустая страница не растягивает декоративную рамку до низа экрана.

## 13. Четыре композиции для будущих макетов

### Desktop Home,1440×900

Navigation около248px. TopBar52px, transport88px. Canvas свободно занимает оставшееся,
внутренний gutter28–36px. Верх: короткое приветствие и тонкая горизонтальная Jump Back
In strip. Ниже слева крупный настоящий album cover, справа title и небольшая подпись
«Продолжить»; не помещать весь hero в raised card. Следующая зона — компактная
Heavy Rotation track list рядом с несколькими обложками Rediscover. Только eligible
sections. Right panel320px появляется по запросу и меняет композицию, не накрывает Play.

### Playlist landing,1280×840

Header компактный: title слева, Create справа. Folders в sidebar, под title тонкие
filters. Несколько настоящих playlist covers с открытой подписью; ниже list mode
recent collections. При0playlists одна выразительная строка и Create; никакой
растянутой формы, пустой dotted box или непрерывного успех-баннера из Downloads.

### Album listening,1440×900

Artwork слева, title и реальные credits справа на мягком album-derived color field,
переходящем в canvas без жёсткой рамки. Play/Save/menu одним рядом. Ниже open tracklist
с disc markers и line rhythm. Playing row отмечена icon+text state. Правый panel
может раскрыть liner notes/queue; quality не конкурирует с названием песни.

### Phone player,390×844

Один art-led фон, safe top close и context title, artwork примерно280–310px при
достаточной высоте. Ниже title/artist, timeline, заметный Play/Next, нижний ряд
Queue/Devices/lyrics. На коротком экране artwork уменьшается первым, кнопки и seek
сохраняются. Queue/Devices sheet не пересоздаёт engine. Никаких desktop sidebar forms.

## 14. Как принимать будущие визуальные материалы

Подготовить real app screenshots с licensed fixture artwork: Home/Playlist/Album/
Player/Queue/Room, dark+light и минимум narrow/wide. Для каждого пояснить focus,
scroll owner, current audio state и активные controls. Нужен reviewer walkthrough,
а не просто красиво вырезанный hero без краёв окна.

В текущей работе новые PNG/HTML/CSS не создаются по требованию «только MD».
Эти композиции достаточно конкретны для следующего design/implementation задания,
но пользовательский screenshot останется неисправленным, пока код не будет изменён,
проверен и действительно установлен.
