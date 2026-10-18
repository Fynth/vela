# Сохранность данных, cache rights и приватность новых возможностей

Это требования к будущим изменениям. В этой документационной работе DB/media/
manifest/session store/consents/runtime не изменяются, migrations не применяются.

## 1. Неприкосновенные данные

Существующие local user/track/asset/playlist IDs, owner relations, playlist item order,
user metadata overrides, favorites, existing explicit downloads и privacy choices
сохраняются. Расширение Vela ID не заменяет local user новым cloud user автоматически.
Не объединять recordings/releases/assets по похожему имени или BLAKE3 разных encodes.

`target/` содержит private backups, releases и active frozen runtimes. Его нельзя
чистить целиком; build output и данные восстановления не одноразовый мусор.
Нельзя запускать cargo clean для «исправления интерфейса».

## 2. Минимальная migration ceremony

1. Установить реальный текущий source/package/runtime/data, владельцев files и
   trusted storage roots. Не выбирать случайную fixture DB вместо пользовательской.
2. Сделать согласованный snapshot существующим проверенным backup mechanism.
   Непроверенный cp открытой SQLite с WAL не считается сохранением данных.
3. Восстановить snapshot в isolated directory. Сверить migrations, FK, row identities,
   owner/playlist relations, media hashes, preferences/consents и playable fixtures.
4. Применить additive migration только на copy, запустить старые/new code journeys.
   До реального upgrade зафиксировать downtime/rollback compatibility strategy.
5. Добавления сначала nullable/default-safe; большие backfills bounded/resumable.
   Для нового origin namespace использовать mapping, не массовый rename неизвестных файлов.
6. При реальном разрешённом upgrade остановить/переключить только нужный process
   согласованным installer, сохранить frozen старую сборку/backup/asset hashes.
7. Проверить counts/IDs/ownership/media/playlists/consents и read/write flows после
   запуска. Silent opt-in/history reset/publication — release blocker.
8. Rollback не запускает старый binary на несовместимой новой DB. Восстановление
   snapshot выполняется отдельно с ясной судьбой новых записей после upgrade.

Нельзя объявлять backup проверенным, пока не выполнен restore на copy. Наличие архива
и успешный exit backup script ещё не доказывают пригодность восстановления.

## 3. Origin/account cache namespace

Scope содержит canonical verified origin/server identity, local membership/account,
asset ID, representation/encoding, validator и format/schema version. Внешний display
name и один UUID недостаточны. Changed public base URL/redirect не даёт новый trusted
origin автоматически; server identity continuity требует проверки.

Каждая cached entry различает bytes/ranges, integrity, pinned state, current grants,
expiry/policy, active leases и last access. Ошибка hash не превращается в «просто попробуем
декодировать». Byte completeness и full verification — самостоятельные состояния.

Origin-less старые downloads не удалять. Сопоставить доказуемые данные по существующему
profile; ambiguous поместить в local review и сохранить, не отдавая другому account.
Copy/rename/publication только atomic/recoverable, с fixture crash injection на границах.

## 4. Локальное хранение

- Private directories0700 и files0600 где OS поддерживает; secure per-app data location.
- Staging уникален, без predictable public filename; atomic publication на том же
  filesystem либо явный безопасный fallback. Не давать web routes читать staging.
- Незавершённые ranges не выглядят как completed pinned manifest. Manifest size,
  entry count, path components и relative storage keys валидируются до allocations.
- Текущий decoder получает lease/проверенный descriptor; pathname replacement не
  подменяет уже проверенные bytes. Не открывать arbitrary paths из remote DTO.
- Disk quota учитывает reservations/staging/orphans/metadata/artwork и replacement
  delta; process locks и cross-process race tests для shared directories обязательны.
- Linux anonymous/unlinked active-source защита не должна исчезнуть случайно при
  cache refactor: durable cache и private playback lease имеют разные lifetimes.
- Native credential protection выбирается отдельно по OS; blanket «шифруем всё» без
  key storage/recovery модели не является выполненной защитой.

## 5. HTTP/cache authorization

Сейчас stream имеет no-store. Разрешение нового private native/browser caching —
явное изменение reviewed server contract, не client игнорирование directive.
Authenticated sessions/private JSON по умолчанию остаются no-store.

Для media storage рассмотреть selective private+revalidate policy: auth и owner/grant
check выполняются перед304/HEAD/range, validator не является access token. Не выдавать
shared CDN/browser cache всех account assets. `Vary`/cache partition/credentials
рассматриваются по конкретному client path, без надежды на случайную HTTP библиотеку.

При новом online playback cached bytes не отменяют проверку актуального stream grant.
Автокэш не создаёт download permission. Явная pinned offline copy выдаётся только при
допустимом download policy; private owner music и remote licensed grants не смешиваются.

**Отзыв не может физически отозвать уже отданные пользователю bytes.** Offline client
не узнаёт немедленно о remote revoke без сети. Нельзя обещать такой DRM. Contract
объясняет: новые server requests запрещаются сразу по актуальному состоянию; online
client перестаёт рекомендовать/запрашивать отозванное; offline entitlement/expiry
подчиняется явно принятой policy. Временная network failure сама по себе не удаляет
законно сохранённую личную музыку. Caching revoked remote content не становится
новым правом на публикацию/redistribution.

## 6. Listening privacy при prefetch/sync

Private history по-прежнему требует актуального account AND device consent и privacy
revision. Login/install/upgrade/Vela ID connection не включает collection автоматически.
Prefetch, download, artwork decode, queue preview, seek distance и synchronized
checkpoint не равны фактически прослушанному времени.

Один logical listen переживает pause/recovery; explicit replay после EOF — другой.
Считается advancing media time, подтверждённый monotonic clock, с bounded intervals
и idempotent outbox. Clock sync handoff не дописывает «прослушанные» минуты задним числом.
Transfer одного logical listening session требует отдельной dedup identity policy,
а независимые sessions не склеиваются только из-за одного account.

Delete истории атомарно отключает collection, обновляет privacy epoch/revision и
инвалидирует held exports/late outboxes. Native и cloud sync должны проверять тот же
инвариант; не воскрешать удалённую историю из offline journal. Account history disabled
означает, что retained history не читается и для recommendation suppression.

Queue/session sync раскрывает музыкальные refs выбранной sync authority, поэтому
имеет собственное объяснение и consent. Reversible hidden tracks/preferences не
удаляются командой erase history, если пользователь не запросил это отдельно.

## 7. Vela ID и sync security boundary

OIDC issuer+subject — identity key; local membership — право конкретного сервера;
stream/download grants — отдельные capabilities. Email не используется как auto-link.
Identity token не bearer для всех music origins. Native OAuth в system browser,
PKCE/state/nonce/redirect/issuer validation, secrets вне WebView persistence/logs.

Sync command содержит target, session/authority epoch, sequence/revision, idempotency ID,
expiry и минимальный typed payload. Authorization до application; replay receipt
scoped account/device/session. Bounds у history/outbox/pending handoffs и WS messages.
Disconnect/revoke отзывает secret и новые commands, но не удаляет unrelated local data.

По умолчанию workspace хранит нужные preferences/origin refs/bookmarks, не raw history
и не media. E2EE объявлять только после threat model, device enrollment, verified key
exchange, lost-device revoke и recovery review. TLS/storage encryption и E2EE — разные
свойства; server operator visibility объясняется честно.

## 8. Rooms/network/content

Room admission и media authorization независимы. Invite token случайный, expiring,
revocable, узко scoped; нельзя класть unrestricted media token в публичный QR/link.
Participants не получают чужие account IDs/secrets/private queue/history от одного join.

Metadata/stream/download/reshare scopes раздельны. Public publication требует ясной
пользовательской policy/provenance; автоматическое «анализатор узнал песню» не
подтверждает copyright/license. Private server не публикуется от включения federation.

Remote metadata untrusted: bounded sizes/depth, escaping, no active HTML/scripts,
safe artwork formats/decoding и controlled egress. DNS/redirect/SSRF defenses не
обходятся из-за «доверенного cloud account». Trusted internal connectors отдельны.

Tombstones/blocks/expiry применяются к reads/search/recommendations/cache pointers;
retained audit не раскрывает private metadata. Нельзя гарантировать удаление из
backup/чужого уже экспортированного файла — это явное ограничение сети.

## 9. UI, diagnostics и поддержка

Диагностический bundle формируется после preview/redaction. По умолчанию исключить:
cookies, CSRF, passwords, token URLs, private filesystem paths, personal profile,
listening history, full remote source URL, содержимое keyring. Не включать screenshots
пользователя в Git/tests без отдельного основания; собственные разрешённые fixtures.

Повседневный UI объясняет user action, не выводит raw SQL/decoder/path diagnostic.
Error details доступны владельцу с safe request ID. Task center остаётся projection
durable workers; Retry/Cancel всегда повторно авторизуются независимо от action flags.

## 10. Негативные gate cases

Account A→B при pending Play/artwork/prefetch/download/export; совпадающие UUID у
origins; namespace migration ambiguity; symlink/path replacement; corrupted manifest;
stale304 после revoke; held WS command после device revoke; OAuth callback другой
сессии; erased history с late offline outbox; room без stream grant; full disk во
время publication; crash между bytes/manifest rename; старая сборка на новой schema.
Каждый имеет ожидаемый безопасный исход и сохранённое пользовательское состояние.
