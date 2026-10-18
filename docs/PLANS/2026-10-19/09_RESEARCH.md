# Источники и пределы исследования

Проверено 2026-10-19. Основное доказательство состояния Vela — собственный source
и выполненные тесты, не старый transcript и не сайт конкурента.
Рекомендации в планах — инженерные предложения, не копирование чужих реализаций.

## Продуктовые ориентиры

- [Spotify Connect](https://support.spotify.com/us/article/spotify-connect/):
  референс понятного выбора playback device и удалённого управления. Vela нужен
  собственный authorized handoff contract; товарное название не переносится в UI.
- [Spotify Jam](https://support.spotify.com/us/article/jam/): референс invitations,
  participants и совместной queue. Vela синхронизирует команды, а право каждого
  участника на media проверяет отдельно.
- [Spotify offline](https://support.spotify.com/us/article/listen-offline/): baseline
  ожидания downloaded music. Собственное Vela-преимущество надо доказывать холодным
  offline launch/recovery и пользовательским контролем, не наличием кнопки Download.
- [Spotify Lossless Audio Quality](https://support.spotify.com/us/article/lossless-audio-quality/):
  lossless уже существует у эталона; некорректно строить сравнение на устаревшем
  «Spotify не умеет lossless». Output route/capability и source quality различаются.
- [Plexamp](https://www.plex.tv/plexamp/): ориентир музыкального personal-library
  experience. У Vela отдельный scope DSP/relevance testing; наличие FFT само по себе
  не доказывает равную perceptual recommendation quality.

Не использовались чужие логотипы/музыкальные фото/album covers/внутренние APIs.
Нет заявления об измеренном превосходстве или доступе к коммерческому каталогу.

## Технические primary references

- [Android Media3: background playback](https://developer.android.com/media/media3/session/background-playback):
  native session/service lifecycle — основание для Android work package, не WebView timer.
- [Apple: configuring media playback](https://developer.apple.com/documentation/avfoundation/configuring-your-app-for-media-playback):
  native audio session/background configuration; проверки требуют Apple environment.
- [WCAG2.2](https://www.w3.org/TR/WCAG22/): normative accessibility baseline.
  Порог 44px — внутреннее product touch правило, AA minimum target-size24px имеет
  исключения. Contrast ratios проверяются по actual adjacent colors.
- [ActivityPub](https://www.w3.org/TR/activitypub/),
  [WebFinger RFC7033](https://www.rfc-editor.org/rfc/rfc7033),
  [HTTP Message Signatures RFC9421](https://www.rfc-editor.org/rfc/rfc9421):
  нормативные ссылки для последующего protocol ADR; interoperability ещё надо
  исследовать/реализовать/проверить, их упоминание не делает Vela совместимым.

Последние 3 ссылки — нормативный список для WP30, их полный interoperability analysis
не выполнен этим аудитом. Версии mobile SDK/native dependencies надо сверять
непосредственно перед реализацией и фиксировать в lock/CI, не угадывать.

## Typography и авторские материалы

- [Golos Text metadata](https://github.com/google/fonts/blob/main/ofl/golostext/METADATA.pb),
  [OFL](https://github.com/google/fonts/blob/main/ofl/golostext/OFL.txt):
  Cyrillic/Latin UI font, авторство/лицензия сохраняются рядом с используемым файлом.
- [Source Serif4 metadata](https://github.com/google/fonts/blob/main/ofl/sourceserif4/METADATA.pb),
  [Adobe Source Serif license](https://github.com/adobe-fonts/source-serif/blob/release/LICENSE.md):
  editorial heading candidate; local asset bytes/hash/license документируются.
- Hash-композиции, схемы и cover геометрия в design board созданы для этого плана.
  Они не являются настоящим artwork загруженных альбомов.

## Новые dependency notices

Проверены первичные RustSec notices
[RUSTSEC-2026-0370](https://rustsec.org/advisories/RUSTSEC-2026-0370.html),
[0371](https://rustsec.org/advisories/RUSTSEC-2026-0371.html),
[0372](https://rustsec.org/advisories/RUSTSEC-2026-0372.html).
Это INFO Unmaintained при переносе standalone wire/name/derive crates в ZBus6,
а не объявление найденной атаки. Actual cargo-deny на текущем lock FAILED;
возможная миграция и MPRIS regression описаны в AUD-20/FIX-08.

## Ограничения аудита

Не выполнены полноценный пентест, всеобщий formal verification, blind usability
исследование, real music DSP calibration, публичная Internet federation deployment,
новые mobile/Apple/Windows builds, hardware звук/наушники/фон, новый Docker runtime.
Current private credentials не читались; браузерный signed-in profile/storage не
использовался. Данные runtime только агрегатно и read-only. Сохранённые screenshots
предыдущего fixture-прогона обозначены историческими. Findings по control flow
не объявляются воспроизведённой атакой без соответствующего теста.
