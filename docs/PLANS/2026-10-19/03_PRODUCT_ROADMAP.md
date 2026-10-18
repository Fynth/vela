# Roadmap: продукт, который хочется открывать каждый день

Цель — превзойти эталон по выбранным пользовательским задачам, не объявлять
себя «лучше Spotify» по количеству пунктов. Spotify уже предлагает lossless,
remote control и совместные сессии; такие функции сами по себе не уникальны.
Сравнение опирается на официальные источники в [09_RESEARCH](09_RESEARCH.md).

## 1. Где Vela может выигрывать

| Направление | Предлагаемое преимущество | Как доказывать |
|---|---|---|
| Собственная коллекция | Файлы, версии релизов, provenance и portable playlists под контролем человека | Переезд A→B без потери организации, export round-trip |
| Доверие к рекомендациям | «Почему», регулируемая новизна, никаких выдуманных moods/listeners | Blind relevance task, доступность 100%, coverage/diversity/explanations |
| Спокойное прослушивание | Предсказуемая очередь, album mode, без навязанной смены контекста | Task success/ошибки, поведение duplicates/previous/next |
| Настоящий offline | Холодный launch без сервера, ясные grants и storage control | Airplane-mode fresh-process journeys, восстановление после ошибок |
| Независимая сеть | Опциональные lawful catalogs, ясное происхождение и доступ | Три origin, partial failure, revoke, cache и blocklist negative tests |
| Дизайн | Оригинальная музыкальная типографика, простая mobile IA, доступность | Реальные usability sessions, keyboard/screen reader, perf/contrast |
| Приватность | История off до согласия, delete без хвостов и export без секретов | Consent/revision race tests, проверка derived artifacts/journals |

Необоснованные обещания исключаются: глобальный коммерческий каталог, гарантированная
точность жанров/эмоций, универсальный gapless и поддержка платформ без build/device evidence.

## 2. Пользователи и задачи

- Обычный слушатель: найти/включить/скачать музыку за минимум действий.
- Коллекционер: релиз/диск/кодек/кредиты/правки без повреждения исходников.
- Любознательный: новая музыка с понятной причиной и ручным управлением discovery.
- Семья/друзья: безопасные invitations и совместная очередь без публикации всего.
- Self-hoster:1 процесс+SQLite, backup/restore, понятная диагностика отдельно от музыки.
- Независимый артист/куратор: опубликовать разрешённый релиз с attribution/лицензией.

## 3. Последовательность поставки

Не фиксировать вымышленные даты. Каждый exit gate закрывается рабочим vertical slice;
внутри следующего этапа сначала остаются исправления регрессий предыдущего.

| Этап | Реальный результат | Зависимости | Exit gate |
|---|---|---|---|
| R0: Truth & repair | Исправлены AUD-01–08; current docs; storage/auth contracts | Baseline этого аудита | REG-AUD и сохранённый пакет 244+old browser/native |
| R1: Listening Atlas shell | Единые tokens/components, Home/Library/Search, context menus, right panel и mobile player | R0, утверждённые контракты root playback | Сквозной путь search→album→queue→player;320px/large text/a11y |
| R2: Каталог без трения | Быстрый поиск, Unicode, editions/assets, queue context и organization | R1; замеры R0 |100k p95 budget, page continuity, no lost playlist IDs |
| R3: Offline & playback | Cold offline, resumable permitted downloads, safe credentials, progressive native source | R0; source adapter capability | Реальные Linux offline/reconnect/diskfull/codec matrix |
| R4: Android daily driver | Dioxus+Media3 service, notification/lock controls, focus, offline | Shared contracts R3; SDK/NDK actual | APK install, emulator+physical G16 с пометкой каждого |
| R5: Taste that earns trust | Сбалансированные candidates, scheduled mixes, typed smart rules, quality study | Consent/delete contracts; R2 | G07–10, diversity/coverage/human relevance; нет phantom IDs |
| R6: Seamless devices | Secure registry/commands, explicit accepted handoff, settings/journal sync | R3/R4, WS/auth | G19 two clients, accept-before-stop, повторный command idempotent |
| R7: Lawful network | Versioned interoperable identity, saved authorized servers, progressive search/cache/tombstones | R2/R3; rights model | G11–15 на 3 реальных инстансах, default private и hostile peer |
| R8: People & rooms | Invitations, granular stream/download, opt-in profiles/follows/rooms/shared mixes | R6/R7; block/report model | G06/G20 два аккаунта/latency/revoke, права каждого участника |
| R9: Other native platforms | iOS/AVFoundation, macOS native session, Windows/SMTC, installers | Shared core; Apple/Windows runners | G17/G18 отдельно compile/install/window/audio/background/hardware |
| R10: Delight | Lyrics/LRC, credits/booklet, transitions, taste controls, private monthly stories, lawful imports | R2–R8 по функции | Каждая функция приносит отдельный работающий journey |
| R11: Release candidate | Recovery, security, scale, a11y, reproducible packages/migration/restore | Предыдущие required subset | G01–24 без фиктивных зелёных строк; signed publish только с authorization |

Apple validation блокируется доступом к macOS/Xcode, но проектирование и shared contracts
не должны ждать Apple. Windows/Android нужны настоящие toolchains; не брать их код
«для галочки». Полезные независимые track могут идти параллельно в будущем после
согласования контрактов, но текущим запросом делегирование не выполняется.

## 4. Продуктовые контуры релизов

**Private daily listening:** R0–R4 плюс базовый R5. Пользователь может комфортно жить
в Vela на web/Linux/Android со своей музыкой. Это первое сильное позиционирование.

**Connected collections:** R6–R8. Межсерверный/социальный слой расширяет доступный
каталог с сохранением происхождения, прав и private defaults.

**Cross-platform premium:** R9–R11. Только доказанные платформы и hardware scope
попадают в release support matrix. Не называть незавершённые клиенты production-ready.

## 5. Как выбирать фичи, когда хочется «ещё всего»

Сначала correctness/privacy, потом ежедневная польза, потом выразительность.
Score feature = пользовательская частота × ценность × уверенность / стоимость,
но security/recovery блокеры не конкурируют с анимациями в этой формуле.

Прежде новых сущностей проверить: можно ли реализовать внутри существующего module,
есть ли настоящий datasource, не добавляет ли функция секреты/право-утечку,
можно ли тестировать её без внешнего аккаунта, каков sparse-library fallback.
AI DJ, хаб-кластер, P2P, TV/watch/car — optional later; не блокируют хороший Music core.

## 6. Измерение превосходства

Создать одинаковую доступную локальную библиотеку и сценарии, где сравнение законно.
Измерять time-to-complete find/play/offline/edit/share, количество неверных действий,
восстановление после отказа, субъективную ясность и preference в blind design review.
Для audio — actual start/seek/dropouts/cancellation, а не первый HTTP byte вместо звука.
Для рекомендаций — temporal holdout + blinded human ratings, минимум эффектов
популярности, private/consent-aware data. Зафиксировать протокол до эксперимента.
Улучшение на одном наборе/платформе не равно мировому общему превосходству.
