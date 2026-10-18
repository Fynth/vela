# Реализации функций: польза, данные, алгоритм, проверка

Все новые функции этого документа PLANNED. Существующие primitives переиспользуются;
название компонента не является поводом создавать новую crate. Privacy defaults,
owner checks, source generations и медиа-права применяются ко всем рецептам.

## 1. Общий client core без второй реализации продукта

После FIX-train выделить внутри workspace библиотеку только при реальном совместном
потребителе web/native: `AccountScope`, `OriginObjectKey`, `CatalogAdapter`,
`PlaybackCommand/Event/Capabilities`, download/journal policies. Начать с modules,
не 30 пустых crates. Server domain DTO остаются framework-free; Dioxus — presentation.

`OriginObjectKey = { origin, object_type, object_id }`; recording ID и physical asset
hash отделены. Один claimed server UUID не заменяет verified origin. Queue occurrence
имеет собственный ID; две одинаковые песни — два occurrence. Browser/native adapters
владеют реальными output, coordinator — только намерениями/порядком/состоянием.

Каждый async result несёт account epoch, query/selection generation и lifetime.
Сторонний catalog adapter возвращает capabilities: stream/download/offline/lyrics/
artwork/server search, чтобы UI не показывал недействующие controls.

## 2. «Понятная очередь»

Польза: предсказать следующий трек, не теряя album/playlist context.

- Три секции: «Сейчас», «Вы добавили», «Далее из [контекста]»; shuffle preview строит
  **реальный** deck, а не случайный список рядом с другим алгоритмом.
- Play next имеет явный приоритет; add last не прерывает output; remove/reorder
  remap model synchronously; undo использует occurrence ID и revision.
- Save queue создаёт обычный playlist через existing CAS API; missing rows остаются
  с причиной, владелец выбирает skip/remove. Не включать чужой metadata-only track.
- Queue journal локально сохраняет scope/cursor/modes/context, restore paused.
  Credentials и stream URLs не пишутся. Нужен versioned migration reader.
- Keyboard move-up/down + drag handle, touch reorder с доступной альтернативой.

Тест: дубликаты, shuffle back/forward, EOF vs Next, clear upcoming, undo after
concurrent queue change, unavailable origin, reload/logout, максимальная очередь.

## 3. «Быстрый поиск, который понимает имена»

Существующий FTS оставить базой. Ввести отдельный normalized search document для
tracks/albums/artists/playlists; owner/origin scope сохраняется **до** выдачи.
Unicode normalization/casefold, диакритика и optional transliteration — поисковые
ключи, исходные имена никогда не перезаписываются. Префиксный поиск от 2 символов;
короткий запрос показывает явные shortcuts/history, не full catalog scan.

Ranking: exact title > prefix title > artist/album credit > token match; typo correction
только ограниченным candidate set после точного этапа. Не внедрять fuzzy full scan
на 1m. Category cursors/versioned filters, строка происхождения, local results first.
Physical cancel + debounce~200ms (измерить), generation checks, Keyboard Arrow/Enter/
Escape и cmd/ctrl-K. История запросов локальная opt-in/clear и scoped.

Fanout connected catalogs bounded, per-peer deadlines/circuit status; incremental
result batches не заменяют уже видимое другим query. Каждый origin имеет собственный
cursor/availability; глобальное stable merge вводить только с contract/test.

Тест: кириллица/латиница/акценты/emoji в тегах/RTL, exact/rare/common/missing,
50→nextpage, чужой owner, held old query,8slow peers, отменённый query не обновляет UI.

## 4. «Миксы, которым доверяешь»

Пайплайн: eligible candidates → candidate sources → exclusions/authorization → rank
→ diversity → reason → immutable mix snapshot → just-in-time source authorization.
Историю читать только при актуальном consent; не хранить скрытый taste cache после erase.

Candidate sources: recent additions, unexplored old catalog, saved/favorite anchors,
qualified history, editorial/user tags, version-compatible audio descriptors; разрешённые
public peers добавлять отдельно без отправки им private history/vector.
Вместо последних 500 сделать bounded stratified retrieval: recent/old/random reservoir/
artist/genre/neighbours с индексами. Затем re-rank несколько сотен, не full1m load.

«Новизна» и «Широта» — понятные настройки session/mix; не fake mood classifier.
MMR/artist-repeat penalty прозрачны и протестированы, веса версионированы.
«Почему» — короткий reason code + реальный anchor, в раскрытии — источник данных.
Hide song/artist, less of this, undo, exclude a playlist from taste — отдельные prefs.

Daily/weekly snapshots: existing durable SQLite worker, уникальный(owner,period,algo,
privacy_revision), lease/retry/idempotency, bounded refresh budget. Timezone задаётся
явно. При erase/delete/disable derived history-dependent snapshots инвалидируются
в той же транзакции/epoch; retained disabled history не участвует даже в suppression.
При 5 треках честный короткий mix; «нечего нового» не переименовывает повтор в discovery.

Evaluation: temporal holdout, Recall/NDCG@K, unique artist/catalog coverage, repeats,
actual playable fraction; tiny libraries отдельно. Human paired ratings на разрешённой
выборке. Клик — слабый сигнал, skip в первые секунды не сильная ненависть.

## 5. «Маршрут через свою музыку» — отличительная функция

Человек выбирает начальный/конечный трек и длительность. Candidate graph содержит
только разрешённые доступные tracks; edge cost — совместимость реальных descriptors,
жанровых tags, длительность и разнообразие. Bounded beam search строит несколько
маршрутов с reason на каждом переходе. Без калиброванного tempo/key эти поля
не используются как музыкальная истина. Можно вручную переставить/запретить узел.

Показывать queue preview, суммарную длительность и честный «Недостаточно треков для
плавного маршрута». Не требуются LLM/GPU. Тестировать deterministic fallback, циклы,
блокировки, sparse catalog, временное исчезновение origin и real listening study.

## 6. DSP, который действительно помогает

Текущий spectral-4 не объявлять готовой моделью настроения. Последовательность:

1. Rights-cleared music corpus с genre разнообразием, reference BPM/key от человека;
   происхождение, dataset license, train/eval separation и hashes фиксируются.
2. Decoder correctness: sample rates/channels, silence, clipping, anti-phase, truncated,
   VBR, long tracks. CPU/RSS/bytes/time budget; crash recovery и audio contention.
3. LUFS/true peak по подходящему опубликованному стандарту через проверенную Rust
   реализацию и golden vectors; RMS не переименовывать в loudness normalization.
4. Tempo: octave errors60/120/240 отдельно, beat F-score/abstention/calibration;
   key: weighted accuracy/ambiguity на real corpus, не только sine/chord fixtures.
5. Timbre/MFCC/robust spectral summaries; optional versioned local embeddings только
   если модель/weights лицензированы и есть измеренный выигрыш.
6. Candidate index: сначала SQL buckets/compact vectors и bounded retrieval;
   ANN при доказанном масштабе, отдельный rebuildable index без media ownership.
7. Version extractor/model/schema independently, immutable old descriptors, cancellation,
   Job center projection, admin minimal/balanced policies. Stream path не декодирует.

## 7. «Релиз как вещь», а не набор случайных файлов

Recording, release group, edition, disc/track, contributor roles, asset encoding —
разные IDs. Исходные tag strings сохраняются; external IDs имеют provenance/confidence.
Существующие group IDs мигрировать с alias mapping, playlist_items не теряются.

Фичи: оригинал/ремастер/live переключатель; credits/booklet когда доступны права;
«Лучшая доступная версия» по codec capability и preference, без ложного lossless;
duplicate review exact/recording/edition с объяснением; no auto delete. Artwork
thumbnail pipeline с decoder/bounds/cache и отдельными permission checks.

Metadata studio: batch preview/diff/confidence, undo catalog-only edits, CAS conflict,
manual match review; MusicBrainz/CAA/AcoustID — optional rate-limited providers.
Не перезаписывать точный user edit remote guesses; write-back отдельно с authorization.

## 8. «Умные полки»

Typed rule AST: And/Or/Not + typed field/op/value; whitelist fields/operators, maxdepth/
clauses, validated ranges; parameterized SQL, никогда unrestricted SQL.
Примеры: «не слушал 6 месяцев», «неизвестный альбом», «избранные 90-х», «lossless если
codec проверен», «короткий сет 25 минут». Preview count ограничен и cancellation-aware.
Rules requiring history показывают disabled reason без consent. Snapshot/export
и dynamic source semantics различаются; maxruntime/query plan acceptance обязательны.

## 9. Качественный offline manager

После FIX-04–06: track/album/playlist download plans, per-entry grants, estimated
bytes, selected quality, disk reservation, Wi-Fi-only preference, pause/cancel/retry.
HTTP resume использует validator/If-Range и final hash, не слепой append.
Manifest содержит origin, rights receipt (не DRM), original hash, variant identity,
artwork, last verification и sync status.100/2GiB текущие лимиты становятся настройками
с bounded safe maxima; интерфейс заранее объясняет объём.

Никакой eviction явных downloads из-за network failure. Explicit cleanup preview,
статусы Healthy/Verifying/Corrupt/Missing/Unverified и возможность repair без потери
остальных файлов. Offline queue/likes/playlist edits — отдельный idempotent journal,
конфликт никогда не стирает новый server revision. Cached credentials в platform store.

## 10. Progressive native playback и качество

Текущий полный download-before-play надёжен, но 250MiB может быть медленным стартом.
Новый adapter: bounded range-aware spool, capability decoder seeking, cancellation,
backpressure, incremental buffering. Whole-file ETag/hash нельзя выдать за verified
до конца загрузки; status distinguishes transport-validated и whole-original-verified.
Путь mature direct media streaming/decoder capability выбирать после прототипа/bench.

Codec matrix каждого engine/browser; source resolver выбирает original, optional FFmpeg
bounded subprocess при несовместимости, аргументы без shell, quotas/cancellation.
Bit depth/source quality/output route/transcode status показывать фактически.
ReplayGain/normalization только с measured metadata; limiter/clip guards.
Gapless/crossfade/A-B repeat/speed/EQ по platform capabilities, sample tests/latency,
не универсальный checkbox. Album mode уважает порядок и не меняет его Autoplay.

## 11. Device handoff без борьбы плееров

Device registry: random stable device ID, name, platform, capabilities, last-seen expiry,
owner session, revoke. WS command envelope: command_id, target_device, owner session,
sequence, expected revision/expiry, payload. Сервер reauthorizes, target deduplicates.

Explicit transfer: prepare target → acknowledge playable+position/queue → commit
transfer → source pauses. Target reject/offline не останавливает source. Queues use
origin object keys + occurrence IDs; current progress с monotonic sample/clock estimate.
Volume и unsupported output controls disabled с причиной. Simultaneous independent
listening не запрещается автоматически; public network не предоставляет remote control.

## 12. Федерация, в которой понятны права

Сохранить read-only catalog profile как совместимый baseline. Добавить protocol ADR:
ActivityPub/WebFinger для actor/follow/activity где подходит; RFC-compatible signatures,
canonical object identity, key rotation, bounded verification/replay window и inbox.
Никакой новый «универсальный протокол» без сравнительного interoperability fixture.

Manual connect/invite, public origin card, block/mute/report, capabilities/version;
library metadata/stream/download/re-share — разные grants. Public artist/curator
profiles opt-in; private fields/history не индексируются. Persist remote public metadata
с TTL/revision/tombstone, no current playback grants из stale cache.
Проверка источника при Play, conditional updates/revoke, bounded traversal/redirect/DNS.
Origin offline — catalog остаётся явно cached, owned local music продолжает играть.

Независимый optional directory только opt-in signed registrations, abuse controls;
никакой обязательный Vela cloud. Private deployment never announces local address.

## 13. Совместные плейлисты и комнаты

Invitations: random hashed token, expiry, intended principal, permissions, acceptance,
revocation, audit; metadata ≠ stream ≠ download. Публичная ссылка не master audio token.
Collaborative edit server-authoritative revision + idempotent command, SSE revision
feed для passive updates; stale order не overwrite. Offline editing journal conflicts
решаются отдельным flow, CRDT только при реальной необходимости.

Room WS: host/cohost/guest roles, participants/presence expiry, queue revision,
playback epoch and scheduled start estimate. Каждый участник reauthorizes source;
нет доступа — видит unavailable, не получает audio через room membership.
Clock sampling/drift correction bounded; reconnect snapshot+missed sequence; duplicate
commands ignored; vote/suggestion отдельные allowed actions. Invitation revoked →
немедленный control disconnect и новые requests denied, delivered bytes не отзываются.

## 14. Мелочи, из которых складывается хороший продукт

| ID / функция | Реализация / условие | Приёмка |
|---|---|---|
| DEL-01 Undo queue/playlist action | Occurrence/mutation ID, bounded undo stack, CAS | Конкурентный edit не откатывается чужим Undo |
| DEL-02 Pin and folders | Existing folders + owner pin order revision | Move/rename без потери ссылок |
| DEL-03 «Послушать позже» | Private album collection, не fake favorite | Album удалён/remote offline — понятное состояние |
| DEL-04 Liner notes | User private notes по edition, portable export | Нет публикации note вместе с public track |
| DEL-05 Last position on long works | Per-device progress, explicit resume | Не считает seek прослушиванием |
| DEL-06 Sleep at end of track | Coordinator EOF hook, scope/device local | Repeat/Next/cancel взаимодействуют предсказуемо |
| DEL-07 Keyboard command palette | Typed actual actions, capability-aware | Focus trap/Escape/text inputs/IME |
| DEL-08 Album-first shuffle | Shuffle albums, сохранять discs/track order | Multi-disc/compilation/dedup fixtures |
| DEL-09 Queue duration | Sum known durations; unknown отдельно | Не показывает точный fake total |
| DEL-10 Bring your playlist | M3U/M3U8/CSV, sandbox local path resolver | Missing paths не дают filesystem traversal |
| DEL-11 «Почему здесь?» | Structured reason + editable preference | Reason match actual ranking evidence |
| DEL-12 Year/month listening story | Private qualified aggregates, opt-in share card | Minutes accurate; erase invalidates artifacts |
| DEL-13 LRC/embedded lyrics | Licensed embedded/local source, parser/time bound | Seek/repeat/unsynced fallback, reduced motion |
| DEL-14 Smart offline pack | User-initiated plan with quota/rights | Ни одного auto-download без согласия |
| DEL-15 Duplicate workbench | Hash exact vs recording/edition grouping | Никакого auto-delete/source overwrite |
| DEL-16 Credits graph | Verified contributor role links | Unknown credits скрыты, не выдуманы |
| DEL-17 Accessible density | Comfortable/compact, independent text scaling |200% text/keyboard/44px touch |
| DEL-18 Region/language | en/ru bundles, locale dates/plurals/search | Mixed-script long names и plural rules |
| DEL-19 Connection health | Per-origin age/latency/error class | Не показывает tokens/raw worker diagnostics |
| DEL-20 Friendly import receipts | Real per-file results, duplicate link/retry | Restart/cancel сохраняют фактический статус |
| DEL-21 Focus listening view | Artwork/transport/notes, opt-in quiet mode | No new engine/changed active source |
| DEL-22 Personal listening goal | Local opt-in timer/album intention | Никаких streak pressure/public rankings |
| DEL-23 Album artwork palette | Local bounded image sampling, contrast-safe tokens | Нет external image service/нечитаемых controls |
| DEL-24 OpenSubsonic adapter | Capability mapping, scoped tokens, actual client requests | Не считается federation/не обещает все extensions |
| DEL-25 Podcast mode | Separate episode/resume/download semantics | Music queue/history не смешиваются молча |
| DEL-26 Internet radio | Separate live/nonseekable source/capabilities | Seek disabled; URL policy и reconnect |
| DEL-27 Artist support links | Explicit verified link/provenance, external browser | Никакого fake sponsorship/payment |
| DEL-28 Private shared taste mix | Opt-in participants + aggregate summaries only | Не раскрывает raw private history |

## 15. Что не делать ради «крутости»

Не добавлять LLM API как обязательную часть radio, не генерировать nonexistent tracks,
не использовать DSP как доказательство прав, не публиковать историю/векторы, не вводить
P2P для private files, не смешивать music/podcast/live semantics, не продавать similarity
как предсказание эмоций. Автоматизация должна быть объяснимой и отменяемой человеком.
