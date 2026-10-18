# Совместное прослушивание, друзья и межсерверная сеть

## 1. Режимы совместного опыта

- **Room:** синхронизированная queue/timeline, каждый играет через свой authorized source.
- **One speaker party:** один явно выбранный output, участники предлагают/меняют queue
  по роли; membership не предоставляет доступ к unrestricted server control.
- **Collaborative playlist:** постоянная коллекция с CAS/permissions, не live timeline.
- **Shared mix:** согласованные taste summaries участников, не выгрузка raw histories.
- **Listening presence:** opt-in видимость текущего состояния, не автоматический feed.

Эти режимы не сливаются в один переключатель «сделать публичным».

## 2. Room lifecycle

1. Create private room: имя, allowed controls, invited users, host output policy,
   media availability requirements, expiration/max participants.
2. Invitation: random hashed secret, TTL, role, intended scope; revoke/list invitations.
3. Join preview: host/участники согласно privacy, current media origin, доступность
   и что будет видно другим. Join не включает consent listening history.
4. Join подтверждает participant identity/room rights и отдельно media authorization.
5. Snapshot содержит room revision, queue occurrence IDs, timeline epoch/anchor,
   host/permissions и resume cursor; credentials/source URLs не рассылаются всем.
6. User controls проходят server authorization; принятые events распространяются по WS.
7. Reconnect: resume по event cursor либо fresh bounded snapshot; duplicate commands
   не добавляют повторные записи. Local buffered old event не возвращает revoked user.
8. Leave/end/host transfer имеют явный effect; оставшаяся личная queue не теряется.

## 3. Роли и команды

| Роль | Default действия |
|---|---|
| Host | Manage room/participants/permissions, control transport/queue/end |
| Co-host | Разрешённые moderation/transport/queue, без owner transfer по умолчанию |
| Collaborator | Add/suggest tracks; reorder/remove только согласно настройке |
| Listener | Join/leave, own volume/output, видеть разрешённую room state |

Право host в room не делает его owner медиа участника. Локальная громкость/выход
каждого независимы. Глобальный pause/skip возможен только при разрешённой роли;
личный mute/leave доступны всегда. Votes не обходят role/resource limits.

## 4. Clock и синхронизация

Измерять round-trip и clock offset с monotonic sampling; server timeline anchor
задаёт intended position, playback rate и scheduled start. Client сообщает buffering/
ready/stalled, а не только локальную кнопку Play. Малые drift исправлять мягко
в пределах supported engine capabilities; большой drift требует guarded seek.
Не обещать sample-perfect sync через Интернет или Bluetooth без измерений.

Настройки: host continues if one buffers / wait for participants с deadline, late
join catch-up, manual resync. Нельзя бесконечно стопорить всех одним offline участником.
Output latency разных устройств и browser autoplay restrictions учитываются отдельно.
Expired command/timeline epoch не применяются после reconnect/new track.

## 5. Права на аудио

Каждый listener должен иметь права на underlying source. Возможные источники:
owned local original, explicit scoped shared grant, разрешённая public publication,
подтверждённая local offline copy. Room сама не создаёт права на любое найденное title.

При недоступном треке: authorized metadata policy определяет, что видно; можно
показать «Нет доступа к источнику», Request access или доступную разрешённую копию
той же recording **после** explicit source resolution. Не заменять silently live/remix/
remaster на другую запись с похожим названием. Комната не audio proxy для запрещённого
rebroadcast, не обход DRM и не источник коммерческого каталога.

Revoke/expiry/block закрывают новые requests/control/membership; уже доставленные
bytes невозможно надёжно «забрать обратно». Это честно отражается в policy/UI.

## 6. Друзья и социальные действия

Public profile и public handle отдельно от login/почты; username не globally unique.
Find user по разрешённому identity lookup/manual invite. Follow requests для закрытого
профиля; accept/reject/unfollow/block/mute. Avatar/bio/links проходят text/image bounds
и SSRF policy, profile privacy granular.

Activity opt-in: опубликованный playlist/collection, invitation accepted, room invite,
разрешённый новый release от followed artist. Нет автоматической публикации listening
history/favorites/«в сети». Feed limited/durable, без бесконечной engagement-ленты,
spam notifications и ложных global trending counters.

## 7. Shared mixes без раскрытия всего

Каждый участник выбирает доступный catalog scope и consent на вклад. Используются
summary affinities/explicit favorites, объяснения не называют чужие private listens.
Candidates сначала фильтруются по rights каждого, затем rank+diversify с балансом
участников. Sparse catalogs дают короткий честный mix, не synthetic songs.
Можно видеть свой вклад, выйти/отозвать consent, пересчитать без оставленного cache.

## 8. Сеть и модерация

Public network opt-in: discovery/metadata/stream/download/republish отдельны. Сохранить
current catalog profile и эволюционировать через versioned interoperability contract.
Actors/follows/activity — изученные ActivityPub/WebFinger механизмы где подходят;
signatures/key rotation/replay window/tombstones/blocklist нужны до публичного inbox.

Обязательные limits: peer fanout/deadline/object size/redirect/DNS pin/recursion,
delivery retries/backoff/dead letters, malformed object isolation, per-actor/server
rate limits. Cached metadata не active grant. Reports/takedown/moderation receipts
не раскрывают private provenance/history и не обещают юридическую проверку прав.

## 9. Room UX

Постоянная тихая room ribbon показывает host/participants/sync, не гигантскую карточку
поверх музыки. Participant list раскрывается; current track крупный с реальной cover,
queue открыта и interactive по роли. Pending suggestion видна как pending, не уже
применённый skip. Leave/End разные controls, accidental tap не заканчивает всем session.

На телефоне room controls и queue — соседние листы full player; на desktop optional
right pane. Приватный room не попадает в Explore. Invite copy/share scopes понятны.
Disabled controls объясняют role/availability, не выглядят неисправными.

## 10. Приёмка ROOM-01–12

Два/три real clients, private room, duplicate invite, host/cohost permissions,
concurrent queue edits, latency/jitter/reconnect, one participant buffering, host leave,
revoked invitation/user, inaccessible media, clock jump, expired timeline event.
Каждый сценарий проверяет state и actual engine position/sound on owned outputs.
Mocked WS state alone недостаточно для заявления «совместное прослушивание работает».
