# Пошаговое исполнение: что исправлять, в каких файлах и чем принимать

**Сейчас исполняется только подготовка Markdown.** Задания ниже пригодны для будущей
работы команды; агенты по ним не запускались. Новые contracts/migrations сначала
согласует будущий интегратор. Один владелец workspace Cargo, migrations и cross-module
contracts; на этом host не более двух Cargo compiler jobs, без конкурирующих full builds.

## 1. Десять самых болезненных разрывов

| Порядок | Разрыв | Точка изменения | Условие закрытия |
|---|---|---|---|
| P0-1 | Непонятно, какая сборка реально запущена | package/install scripts, desktop version/About, CURRENT_STATE | Source/package/process/assets связаны manifest/hash, старый runtime сохранён |
| P0-2 | Полосы/поля и теснота Hyprland | `apps/desktop/src/main.rs`, `style.css`, root `ui.rs` | LAYOUT-01–08 на реальном tile, scale и фактическом backend |
| P0-3 | Повторный полный download A→B→A | `desktop/src/{client,lib,ui,offline}.rs`, server stream policy | CACHE-01–10, origin/account изоляция и реальные body counters |
| P1-1 | Весь файл до первого звука | `desktop/src/{client,audio,lib}.rs`, stream tests | Audible start до EOF body; cancellation/seek/integrity проверены |
| P1-2 | Следующий track готовится слишком поздно | queue/coordinator в desktop, pure playback model | PREF-01–12, speculated next совпадает с реальным Next |
| P1-3 | Covers отсутствуют, forms/notice захватывают страницу | desktop native HTTP images, ui/style; web shared visual tokens | Настоящий artwork, компактный Create, scoped feedback, UX screenshots |
| P1-4 | Разные local/remote/offline пути ломают parity | desktop source selection/MPRIS, web adapter boundary | Один command contract; EOF/Next/Previous/repeat на всех sources |
| P1-5 | Нет session sync и явного handoff | domain/core/server + clients | Два real clients, ACK/rollback/reconnect/cross-account negative cases |
| P1-6 | Нет независимого аккаунта и зрелого Android | OIDC/workspace + Android bridge/service | ID и Android device acceptance, local-only regression |
| P2-1 | Rooms/social/discovery/прочие платформы не доведены | отдельные permissions/protocol/platform slices | ROOM/matrix, права каждого участника, реальные platform evidence |

## 2. Нулевая работа: воспроизводимость и данные

### W00 — зафиксировать действительное исходное состояние

1. Прочитать актуальные AGENTS, PROGRESS, CONTRACTS, CURRENT_STATE, SECURITY,
   NATIVE_CLIENT_CONTRACTS и текущий mandate. Не опираться на этот план как на snapshot
   будущего HEAD: `10fe946` — только исходная точка данной документации.
2. Снять git branch/HEAD/status, список затрагиваемых uncommitted changes. Не reset,
   не rebase чужой работы. Установить, есть ли уже исправление выбранного дефекта.
3. Определить desktop executable через реальный процесс; server executable, frozen
   manifest, asset hashes и оригинальный data root. Не выводить `/proc/.../environ`
   целиком, session store содержимое или весь DB пользователя.
4. Снять read-only health/version и unit состояние без restart; отделить данные этого
   runtime от fixture. Цель — диагностировать, а не обновить его «заодно».
5. Выполнить согласованный backup DB/media/config с существующим механизмом. Online
   SQLite copy только согласованным backup API/механизмом; обычный cp WAL DB не backup.
6. На отдельной fixture copy восстановить и проверить ownership, track/playlist IDs,
   media hashes, migrations, consent и FK. Оригинал не подменять восстановленной копией.
7. Сохранить baseline screenshot/window geometry и измерения Play/A→B→A/Next.
   Только после этого начинать source/cache/schema изменения.

Выход: воспроизводимый дефект либо честное NOT_REPRODUCED с недостающими условиями;
известная безопасная точка возврата, реальные цифры, точный исполняемый build.

## 3. W01 — нормальное окно и transport (R1)

Файлы: `apps/desktop/src/main.rs`, root presentation в `ui.rs`, `style.css`, существующий
native QA harness. Не менять source/auth/audio semantics в этом задании.

1. Разделить native decoration/menu и DOM layout, собрать backend/scale/window evidence.
2. Заменить двойной sidebar offset + centered canvas одной геометрией shell.
3. Выделить transport в собственную строку, задать один main scroll owner, убрать
   постоянную вторую status row при нормальном состоянии. Не скрывать важную ошибку.
4. Настроить осмысленную native menu policy, сохранив keyboard/OS actions и window buttons.
5. Проверить wide/short/640px window, normal tile, fullscreen, drag resize, scale1–2,
   светлую и тёмную темы. В QA больше не принимать forced X11 как Wayland evidence.
6. Собрать exact package, открыть его в обычном Hyprland tile и повторить проверку.

Выход: LAYOUT pass с геометрией и screenshots, ни одного сдвига audio source от resize.

## 4. W02 — media cache contract и тёплый replay (R1)

Зависит от W00, может не зависеть от визуальной W01. Владелец contracts сначала
описывает capability/private HTTP policy; затем реализация в desktop/server.

1. Зафиксировать cache key origin+account/membership+asset+representation+validator,
   separate current lease/pinned/autocache, integrity/grant states и quotas.
2. Определить новую selective private media HTTP cache policy. Сохранить `no-store`
   для sessions/account/private DTO; не начать тайно кешировать endpoint с no-store.
3. Добавить reusable complete-file cache сначала как безопасный первый increment:
   он уже устраняет A→B→A downloads, даже до progressive decoder.
4. Resolver проверяет pinned/cache/validator/grants прежде полного GET; Cache hit
   всё равно не grants bypass для нового online access.
5. Добавить leases, bounded single-flight, cancellation consumers, atomic publication,
   disk accounting staging/orphans и cache inspector. Не затирать existing offline v2.
6. Проверить same UUID across origins, logout/account switch, revoked grants,
   size/hash mismatch, 304/206/416, disk full и cancellation.

Выход: CACHE suite; network/disk counters подтверждают отсутствие повторного full body.
Нельзя остановиться на изменении текста «Downloading» на «Loading».

## 5. W03 — progressive source и реальный быстрый старт (R1/R2)

Файлы: `desktop/src/{client,lib,audio,ui}.rs`; server Range tests при необходимости.
Только одно место владеет output/decoder. Нельзя дать UI синхронный сетевой Reader.

Текущие AGENTS/NATIVE_CLIENT_CONTRACTS описывают full-prepare, whole-file BLAKE3 и
250MiB cap как действующий контракт. До code integration оформить ADR и согласованно
обновить эти правила: новый progressive trust model, явные bounds и strict verified
fallback. Не отменять существующую integrity гарантию молча ради latency.

1. Выбрать проверяемый adapter: bounded progressive spool/range source с blocking
   decoder worker и async fetch boundary. Оценить seek requirements реального codec.
2. Явно решить integrity: partial bytes не называются complete verified BLAKE3 asset;
   strict full-verification mode остаётся отдельной policy/fallback.
3. Запускать звук после достаточного буфера; seek запрашивает нужный диапазон и
   физически отменяет ненужные запросы, не всю правильную source session.
4. Файлы без подходящего validator/seek metadata получают bounded fallback с ясным UI.
5. Проверить быстрые A→B→C switches, held old source result, EOS/seek near end,
   unknown duration, truncation, server restart и logout.
6. Измерить TTFA, buffered bytes, CPU/RSS, current-source cancellation и time to audible.

Выход: звук до whole-body EOF на поддержанных codecs; остальные честно отмечены.
Не добавлять LUFS/gapless/crossfade в этот PR.

## 6. W04 — next preparation и queue parity (R1/R2)

Зависит от W02; W03 улучшает старт, но complete next-cache уже полезен отдельно.

1. Общий план actual-next с explicit-next priority, fair shuffle/repeat/occurrences.
2. Read-only peek не расходует deck/history. Каждая structural edit синхронно
   обновляет модель и prefetch plan revision.
3. Один speculative job с меньшим приоритетом, single-flight с current/save, bounded
   bytes и отказом при low disk/metered/battery policy.
4. Ready state отображается из resolver; decoder ready и bytes ready различаются.
5. EOF/Next/Previous/MPRIS проходят одинаковый routing для local/offline/remote.
6. Prefetch не создаёт вторую playing session, listen event, autoplay и decoder output.

Выход: PREF + source parity tests; natural transition не ждёт полного fresh download.

## 7. W05 — музыкальный UI поверх реального состояния (R2)

Файлы: desktop UI/style/client artwork path; web shell/icons/now_playing/main и
соответствующие CSS allowlist/package entries — при переносе согласованного design.
Не переписывать browser_player ради нового header.

Порядок: реальные thumbnails → shell typography/colors → player/queue → playlist
landing/Create modal → Home/Library/Search → album/artist → loading/error/focus.
Каждый экран принимается отдельно через [UX states](09_SCREEN_BY_SCREEN_UX.md).

Сохранить shared API и root playback. Artwork requests авторизованы, bounded и
cancellable, не гоняют full-resolution image в каждой строке. Проектировать по
настоящим 0/5/1000 tracks, длинной кириллице и incomplete metadata. После modal/drag
проверять focus и ongoing playback. Screenshots — реальный app, не screenshot HTML-концепта.

Выход: core journeys visually reviewed на desktop/narrow/mobile widths; active
controls работают, source generation при навигации неизменна.

## 8. W06 — offline как самостоятельный ежедневный режим (R2)

Сохранить уже сделанные e225b63/03b2523. Устранить cold-launch зависимость от успешного
online login. Мигрировать namespace в origin+account без автоматического удаления
старых файлов; предложить review ambiguous entries. Проверить startup manifest sweep,
corruption, saved queue, two-process concurrent save, crash/disk-full и removable storage.

Выход: app закрыт → server выключен → app открыт → album Play/Next/Seek работают;
другой local profile не видит bytes/metadata. Explicit downloads не удалены eviction.

## 9. W07 — origin refs, device/session sync contract (R3)

Владелец domain/core/server определяет typed identifiers, device capabilities,
queue revisions, session/checkpoint/control envelopes и failure policy из
[device plan](04_DEVICE_SYNC_AND_HANDOFF.md). Только затем additive migrations.

Первая поставка: два real web/native клиента одного local server. Register device,
save/restore paused session, explicit sync selection, bounded WS snapshots/reconnect,
CAS edits. Без cloud ID и без remote auto-play уже можно доказать sync correctness.

Выход: stale checkpoints, replay/double delivery, conflicting edits и account isolation
проходят tests. Добавленные tables имеют полезный endpoint/UI, не пустой scaffold.

## 10. W08 — независимый Vela ID и personal workspace (R3)

Зависит от identity design, не от готового Apple клиента. Реализовать standards-based
OIDC client/provider integration, local membership binding и versioned sync workspace;
не собственный password grant. Сначала локальный isolated provider/fixture, system
browser native PKCE и реальные state/issuer/replay negatives.

Local-only regression обязателен. Hosted operation, домен, recovery/security policy,
mail/paid infrastructure/public deployment — отдельные внешние решения, не фоновая
активность будущего агента. До них функция проверяется локально и честно маркируется.

Выход: один identity на двух clients, разные local memberships двух origins,
disconnect/unlink/export/provider outage без потери existing user data.

## 11. W09 — Android native daily client (R3)

После shared command/source/session contracts выбрать exact current toolchain и
завести реальный build. Делать вертикально: launch/connect → Media3 stream →
background/lockscreen/focus → offline → device sync → accessibility/install.
Одного Dioxus window недостаточно; UI bridge и service lifetime проверяются раздельно.

Выход: G16 на device/emulator с маркировкой hardware; local server адрес доступен
телефону явно, не автоматическим public exposure и не `127.0.0.1` desktop на телефоне.

## 12. W10 — настоящий handoff (R3)

Зависит от W07 и двух поддержанных clients; принять Linux↔browser сначала, затем
Android. Implement prepare/READY/pause-ACK/start-ACK/commit/rollback transaction.
Capabilities/grants/source readiness проверяются target независимо. При partition
описать conservative stop/rollback policy; zero-gap+zero-overlap не обещать.

Выход: HANDOFF suite с latency/lost ACK, browser autoplay block, source disappear,
revoked membership, target app killed и stale rollback after user's new selection.

## 13. W11 — комнаты, друзья и permitted network (R4)

Не объединять room queue с personal device sync. Сначала admission/roles/media
authorization → authoritative timeline/queue → two real clients → drift/reconnect →
invitations/block/mute → origin-aware missing state. После этого UI room stage.

Сохранить existing profile1 public publications и отдельные grants. Signed social
federation, ActivityPub compatibility и remote recommendation — новые acceptance,
а не переименование two-loopback public catalog tests в full federation.

Выход: ROOM + network hostile peer cases; metadata не раскрывает private content,
каждый участник действительно имеет разрешённый source.

## 14. W12 — Apple/Windows, затем расширение качества (R5)

iOS/macOS требуют реальной Apple среды; Windows — своего runner/launch/audio.
Без неё реализуемый shared contract готовится, platform evidence остаётся BLOCKED.
Не отмечать phase complete ради отсутствующего hardware.

Дальше выбирать небольшие цельные группы F-ID: lyrics/booklet; reliable loudness;
scheduled mixes; smart playlists; public artist collections; yearly stories;
verified gapless/crossfade. Только на измеренном baseline, без mandatory cloud/GPU.

## 15. W13 — release, установленная сборка, обратная связь

1. Fmt/strict Clippy/unit/API/WASM и relevant browser/native regression.
2. Dependency/license gate закрыт без blanket ignore; текущие AUD-20/21 проверить
   по актуальным advisory и lockfile, не наследовать старый PASS.
3. Exact packaged assets/hash, normal installed launch и реальные flows.
4. Backup/restore/migration rehearsal, unchanged user ownership/consents.
5. Update явно целевого local runtime после проверки и в разрешённой области;
   не принимать restart другого активного server как случайную часть build.
6. PROGRESS: source hash, package hash, runtime hash, commands, failures, NOT_RUN,
   screenshots и measured latency. Только затем feature VERIFIED.
7. Список remaining irritations сокращается фактическими действиями пользователя,
   а не закрытием документации. Следующий slice берётся по наибольшему remaining impact.

## 16. Шаблон будущего задания для независимого исполнителя

Указать: W/F-ID, цель человека, исходный HEAD/runtime, разрешённые files, запрещённые
areas/data, входной contract, dependencies, exact expected behavior, failure cases,
privacy/account boundary, relevant tests, visual/native evidence, migration/rollback,
resource budget, результат review и remaining NOT_RUN. Один ticket — один проверяемый
vertical slice. Изменения общего API/schema до merge принимает интегратор.

Возможный безопасный параллелизм после contract: window/UI presentation, isolated
source/cache service и QA fixtures в непересекающихся файлах. Одновременное редактирование
root `ui.rs` двумя людьми не считать параллелизмом. Сейчас делегирование не выполнялось.
