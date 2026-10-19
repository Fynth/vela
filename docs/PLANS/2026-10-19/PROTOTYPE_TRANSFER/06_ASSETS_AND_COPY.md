# Original reference assets, fonts, icons and copy

These are inspected reference files and approved wording, not an application asset
installation or proof of music rights. No new images were generated for this task.
The PNGs were downloaded and visually opened; WOFF2 magic/bytes were checked.

## 1. Exact public image manifest

| File | Original public URL | Dimensions | Bytes | SHA256 |
| --- | --- | --- | --- | --- |
| coast.png | [original](https://prototyp-opal.vercel.app/images/coast.png) | 1376×768, RGB | 1,516,616 | 3e9fe7014f03512f64e43601a16f749d0ec3e683023c782579f34623be4732c0 |
| your-wave.png | [original](https://prototyp-opal.vercel.app/images/your-wave.png) | 1024×1024, RGB | 1,230,494 | ff27654ab61791306dca4bae75c2fdf8aba38adc607da7ce90c2fce0d2453db8 |
| album-atlas.png | [original](https://prototyp-opal.vercel.app/images/album-atlas.png) | 1264×848, RGB | 1,403,864 | 9f6aa2ca3593e606ca02522e94a362da5ff74bea1bac8caae6cf99b48506dd16 |

The URLs are actual accessible originals observed on 2026-10-19. Verify bytes/hash on
retrieval. If deployment contents change, record the new revision and review rather
than calling it the same reference. A filename alone is not an integrity check.

coast.png: dark Icelandic shoreline, sea left, black beach/green cliffs right, overcast
sky. Crop 60% 53% in the editorial card with the two approved dark shading layers.
The image alone does not supply text; keep copy as real accessible text above it.

your-wave.png: matte translucent ribbon, lilac/powder/peach layers, deep plum background.
Desktop cropcenter 32%; mobile banner uses the inspectedcenter 39% refinement. Purple
is confined to this module; do not recolor the entire app around it.

album-atlas.png: three columns×two rows of concept images. It is not a sheet of
official covers or the user's actual recordings. Use only as a clearly isolated
reference/visual fixture if needed; production catalog art comes from authorized DTOs.

| Cell | Reference concept | CSS background position |
| --- | --- | --- |
| Top left | Red-orange sunset over dark sea | 0% 0% |
| Top middle | Pale sculpture in light-blue water | 50% 0% |
| Top right | Lone figure in blue sea | 100% 0% |
| Bottom left | Burgundy petals on near-black | 0% 100% |
| Bottom middle | Conifers in pale green mist | 50% 100% |
| Bottom right | Warm orange curved architecture | 100% 100% |

Reference atlas treatment: background-size 300% 200%, no repeat, reserved square cell.
Do not display the entire atlas six times.1264/3 is fractional and the image ratio
is not mathematically exact 3:2; do not claim a hand-cut pixel-perfect tile set without
checking seams/crops. Keep original proportions or explicitly reviewed fixture crops.
The atlas is never a fallback for missing actual album art or an invented artist portrait.

## 2. Production asset handling

Use local app-owned assets for decorative coast/wave art; no hotlink in the listening
runtime. Store provenance and any required rights/credits before redistribution.
Public HTTP availability alone does not establish a production asset license. This
is a reference inspection, not a claim that all artwork has a verified release license.
Resolve that release gate without blocking independent shell/interaction work.

Real covers remain private/origin-aware. Do not route brand assets through an
authenticated music download worker or route private covers through a public brand
directory. Follow exact allowlist/build/package/native-resolver requirements in 04.

If an original cannot be obtained, record IMAGE_MATCH_BLOCKED for that asset. Continue
with composition/geometry and a clearly temporary neutral treatment in development.
Do not generate a replacement, remove the condition from the report, or claim identical
visuals. Never substitute random copyrighted catalog art to make a screenshot full.

## 3. Manrope files and license

Published CSS uses variable Manrope 200–800 and explicit Unicode subset ranges.
The initial page preloads the following two original WOFF2 files. These files were
retrieved and checked for the WOFF2 signature during this planning audit.

| Subset | Original URL | Bytes | SHA256 |
| --- | --- | --- | --- |
| Latin | [a343f882a40d2cc9…woff2](https://prototyp-opal.vercel.app/_next/static/immutable/media/a343f882a40d2cc9-s.p.3259ncl47-hqj.woff2) | 24,576 | e310b55a7fd9677f5e3555e6c6c4d064fa1f1d24393f0ddbe217cea12a8c432f |
| Cyrillic | [d3fe2f289711ac3f…woff2](https://prototyp-opal.vercel.app/_next/static/immutable/media/d3fe2f289711ac3f-s.p.2g9li78_0_it5.woff2) | 14,544 | 95a493061fe0a8d0d027c2892747134ba747141112d3be4d5022e70ab7d3a1a6 |

The publication also references Cyrillic-ext, Greek, Vietnamese and Latin-ext files.
Only the two table entries were downloaded/verified here; do not imply all language
coverage has been tested. Add permitted subsets based on actual supported locales
and test fallback on missing glyphs, not because a long CSS list looks comprehensive.

The [Google Fonts Manrope OFL file](https://raw.githubusercontent.com/google/fonts/main/ofl/manrope/OFL.txt)
was inspected: Manrope Project Authors copyright, SIL Open Font License 1.1. Its
retrieved SHA256 is 58172e0c0fac2cda8a37b348164bb55e44b0e69051e557e92b1d3f6910141f7b.
Keep the actual font distribution's notice/license and provenance; the upstream
license reference is not a new license for the PNG artwork.

Use local WOFF2 and correctly scopedunicode-range/weight declarations. Preserve
fallback metrics until font readiness and test actual кириллица, Latin, diacritics,
numerals, arrows and punctuation. No runtime Google Fonts request, no fake “Manrope”
family that actually renders Inter, and no separate font service dependency.

## 4. Icons and owned logo

The prototype bundles Lucide-style outlines; Vela already has its own coherent SVG
family in apps/web/src/icons.rs, reused by native. Extend this family where necessary:
House, Search, Radio, Library, Album, Artist, Heart, Plus, History, Shield, Settings, Code,
Chevron/Back/Forward, Upload, Queue, AudioLines, Shuffle, Previous, Play, Pause, Next, Repeat,
RepeatOne, Volume, Mute, Expand, Close, Clock and external/open arrows.

Existing apps/web/vela.svg is an owned V-shaped mark, not the prototype's sail drawing.
The user explicitly permits reusing an existing logo. Keep that asset family with
approved role colors and the `vela.` wordmark, or record the deliberate authorized sail
asset replacement if supplied later. Do not falsely report the existing mark as an
identical two-sail logo. No emoji, brand theft, remote icon loader or React-only package.

## 5. Literal Russian reference copy: shell and Home

Keep reference Russian strings as provided when shipping Russian locale. English
engineering instructions do not mean translating the app into English only. Use a
central string mechanism compatible with the current code, not scattered accidental
literal copies. Existing locale behavior must not be silently reset on upgrade.

| Surface | Approved reference text | Production condition |
| --- | --- | --- |
| Brand | vela. | Owned brand action/Home |
| Primary nav | Главная; Поиск; Радио | Real destinations, Radio seed chooser when needed |
| Library group | МОЯ МЕДИАТЕКА | Actual account scope |
| Library nav | Все треки; Альбомы; Исполнители; Любимое | Real counts/roles |
| Playlist group | ПЛЕЙЛИСТЫ | Create plus must work |
| History nav | Недавно прослушано | Actual qualified history, not transient queue traversal |
| Ownership block | Ваша музыка. Ваши правила. | Separate lines as in reference |
| Ownership body | Без рекламы. Без алгоритмов, которые решают за вас. | Do not expand to a false claim that Vela has no algorithms |
| Ownership link | Это Vela ↗ | Real About/project action |
| Settings | Настройки | Functional screen |
| Topbar search placeholder | Найти то, что звучит внутри | Accessible search name separate |
| Home eyebrow | МУЗЫКА. ТОЛЬКО ВАША. | Real own-library scope |
| Home h1 | В своём ритме | Decorative primary dot |
| Privacy caption | Личное пространство | Do not mislabel a public/shared scope |
| Filters | Для вас; Альбомы; Плейлисты | One real selected state |
| Filter caption | Знакомое. Любимое. Неожиданное. | Hide secondary caption on mobile |
| First editorial tag/title | ВРЕМЯ ДЛЯ СЕБЯ / Тихие горизонты | Title refers to an actual configured target or is explicitly editorial, not a fake saved playlist |
| First editorial body | Музыка, чтобы замедлиться. И услышать что-то большее. | Two reference lines, legible over shaded image |
| Second tag/title | ОСТАВАЙТЕСЬ В ПОТОКЕ / Вне суеты | Actual configured target |
| Second body | Знакомые мелодии. Новые ощущения. Останьтесь в моменте. | Reference line breaks can adapt without orphaning phrases |
| Third tag/title | ДРУГАЯ СТОРОНА ДНЯ / Ближе к ночи | Actual configured target |
| Third body | Город засыпает, музыка остаётся. Ваша история после полуночи. | Not an automatic listening-history/publication claim |
| Editorial action | Слушать | Actual accessible target; otherwise useful choose/add fallback |
| Wave eyebrow/title | ПЕРСОНАЛЬНЫЙ МИКС / Моя волна | True eligible mix capability |
| Wave body | Ваша музыка. Новые сочетания. | Do not promise a commercial/global catalog |
| Wave tag | Настроена на вас | Only actual personal candidate/ranking provenance |
| Album section | Снова в наушниках | Actual qualified replay basis; otherwise Ваши альбомы |
| Album subtitle | Хорошую музыку хочется слушать снова. | Quiet reference supporting copy |
| Album link | Все альбомы → | Complete real paginated destination |
| Albums filter subtitle | Маленькие миры, к которым хочется возвращаться. | Reference composition |
| Playlist filter heading | Для каждого настроения | Curated lists, not an inferred mood classifier |
| Playlist create | Создать | Actual create dialog |
| Additions heading | Недавно в коллекции | Actual added_at |
| Additions subtitle | С каждым треком — чуть больше вашего мира. | Supporting copy, not an import success message |
| Library link | Вся медиатека → | Real destination |
| Footer body/link | Ваша музыка. Без лишнего шума. / О проекте ↗ | Inside the central scroller |

Do not hard-code the reference artist/album/playlist names as account data. The named
Bonobo/Tycho/ODESZA/RÜFÜS DU SOL/Nils Frahm/Jon Hopkins records establish spacing
examples only. No original commercial recordings were inspected or downloaded.

## 6. Route/dialog copy and truthful variants

| Surface | Reference text | Required adaptation |
| --- | --- | --- |
| Common eyebrow | ВАШЕ МУЗЫКАЛЬНОЕ ПРОСТРАНСТВО | Only one title/introduction |
| Main titles | Медиатека; Поиск; Альбомы; Исполнители; Любимые треки; Плейлисты; История; Радио | Match actual route |
| Header action | Добавить музыку / Создать плейлист | Capability/role aware |
| Library chips | Все треки; Электроника; Эмбиент | The last two appear only if real facets exist |
| Library sort | По добавлению; По названию; По исполнителю | Last option needs genuine server sort |
| Library play | Слушать всё | Only true complete query scope; otherwise label loaded scope |
| Search placeholder | Трек, альбом или исполнитель | Match actual categories, keep playlist results if already supported |
| Favorite subtitle | То, что отзывается. Всегда рядом. | Not a promise that streaming works offline |
| Artist caption | Исполнитель ↗ | Real artist page; representative art not called a portrait |
| Playlist visibility | Личный плейлист / Только для вас | Only when actual permissions justify it |
| Create tile | Новая история / Создать плейлист | One real action, not decorative form controls |
| Radio introduction | Следуйте за любимым звуком | Actual seed/results/actions |
| Search empty | Пока ничего не нашлось / Попробуйте другое название или имя исполнителя. | Only after completed nonempty query, no hidden request still pending |
| History empty | Здесь начнётся ваша история | For genuinely enabled empty history; disabled consent needs a different explanation |
| Empty playlist | Добавим немного музыки? | Actual picker/Library action |
| Favorite empty | Место для любимой музыки | Actual Library/search action |
| Queue title | Очередь воспроизведения | Real schedule, not a separate player |
| Queue subtitle | Всё готово к следующему моменту. | Do not use as a false cache/download-readiness statement; omit or adapt while resolving |
| Queue section | СЕЙЧАС ИГРАЕТ / Дальше | Actual state/current+next count |
| Queue footer | Выбрать ещё музыку | Close/navigate without stopping current source |
| Queue empty | Здесь пока тихо. Добавьте треки из медиатеки. | Useful Library action |
| Expanded title/body | Сейчас играет / Останьтесь наедине с музыкой. | Real current state and one root engine |
| Upload title/body | Больше вашей музыки / Откройте аудиофайлы со своего устройства. | Server upload capability, not session-only local import |
| Upload dropzone title | У вашей коллекции есть продолжение | Decorative support, not a success claim |
| Upload dropzone instructions | Перетащите файлы сюда или нажмите, чтобы выбрать | Real drop/picker behavior and keyboard alternative |
| Upload action | Выбрать файлы | Formats/limits from actual pipeline |
| Create title/body | Новая музыкальная история / Дайте настроение вашему новому плейлисту. | Real owner-private creation |
| Create label/placeholder | Название плейлиста / Например, утро без спешки | Whitespace/length validation |
| Create privacy body | Личный плейлист. Только ваша музыка и ваше настроение. | Actual private default; shared changes update the projection |
| Create submit | Создать плейлист | Disabled on empty/whitespace, busy/failure truthful |
| Account surface | Ваше пространство / Моя библиотека | Actual account, not fixedV identity |
| About title/slogan | Музыка без лишнего шума / Ваша музыка. Ваши правила. | Real project link https://github.com/Fynth/vela |
| Mobile tabs | Главная; Поиск; Медиатека; Плейлисты; Ещё | More opens actual Settings with remaining routes |

## 7. New production-state wording

These are proposed truthful extensions, not observations from the prototype. Review
actual localized copy with real states and screen-reader announcements before shipping.

- Upload explanation: “Файлы будут добавлены на ваш сервер Vela.” Add actual library/
  destination and privacy detail according to server policy, not a blanket promise
  that nothing leaves the device.
- Disabled history: “История прослушивания выключена.” Action: “Настройки приватности”.
  Visiting this screen must not enable account/device consent.
- Preparing source: “Подготавливаем аудио…” with real Pause/Cancel intent; error/retry
  remains accessible. No fake percent or equalizer before actual playing state.
- No configured editorial collection: “Выберите музыку для этой подборки.” Action:
  “Выбрать плейлист”. Do not show a Listen button that plays a random queue.
- Limited mix: “В медиатеке пока мало подходящих треков.” Show actual candidates and
  a useful change-seed/Library action without inventing a larger catalog.
- Unsaved preference: “Настройка применена временно. Не удалось сохранить.” Only if
  the adapter actually applied it and persistence failed; offer the current real retry.
- Unknown write outcome: “Проверяем, добавлен ли трек…” Reconcile before another POST.
- Denied media: “Нет доступа к воспроизведению этого трека.” A visible metadata item
  does not imply an access-request service exists; show only an implemented recovery.

Use correct Russian plural forms for counts, time and missing items. Keep actual
quantity/source/visibility separate from poetic supporting copy. A phrase approved
for a reference card is not universal consent, storage or rights language.
