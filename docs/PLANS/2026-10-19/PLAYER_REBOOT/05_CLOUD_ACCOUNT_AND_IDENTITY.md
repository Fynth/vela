# Vela ID: единый аккаунт вне конкретного музыкального сервера

## 1. Что именно пользователь получает

Один личный профиль, вход на разных устройствах, привязанные музыкальные серверы,
настройки, свои playlists/collections с origin refs, queue bookmarks и device registry.
Если домашний music server выключен, аккаунт и уже скачанная разрешённая музыка
не исчезают; private origin tracks становятся temporarily unavailable.

Это **реальная новая capability**, а не переименованный локальный username.
Vela ID не предоставляет коммерческий каталог и не делает владельца участником
всех серверов. Identity, local membership, stream/download rights остаются разными.

## 2. Архитектурное решение

Предусмотреть три совместимых режима:

| Режим | Identity | Music | Sync |
|---|---|---|---|
| Local-only | Existing account своего сервера | Собственный сервер | Local/device или тот же сервер |
| Vela ID connected | Отдельный OIDC identity provider | Любые явно подключённые origin с grants | Выбранный personal workspace service |
| Self-hosted identity/workspace | Собственный совместимый OIDC provider и sync instance | Локальные/удалённые origin | Собственная инфраструктура без Vela-operated cloud |

Hosted Vela ID — опциональный продуктовый сервис; его нельзя создавать/публиковать/
оплачивать в фоне без разрешения. Local-only продолжает жить без него. Общий аккаунт
реализуется стандартным OIDC/OAuth, не собственной схемой паролей/подписей/refresh.
Для первого реального подключения выбрать поддерживаемый provider и проверенную
Rust OIDC client implementation, документировать capabilities и lifecycle.

Personal sync workspace можно сделать отдельной ролью того же Rust binary с SQLite
и bounded log, когда один workspace имеет одну authority. Это не повод требовать
Redis/Kafka/PostgreSQL у каждого личного музыкального сервера.

## 3. Identity binding

Global identity ключ — validated issuer+subject. Email, display name, hostname и
одинаковый username не дают identity equivalence. OIDC pairwise subject semantics
учитываются; публичное social handle — отдельное поле/identifier.

У разных native/web client registrations pairwise subject одного человека может
отличаться. Personal workspace выдаёт свой стабильный person ID после validated
bindings; межклиентное связывание делается через поддержанную provider/sector модель
либо explicit authenticated linking. Нельзя автоматически склеить разные subjects
по email или принять token с неверной audience, чтобы «сделать единый аккаунт».

Local membership binding хранит issuer+subject↔origin local account через явный
authenticated linking flow. Существующий local administrator подтверждает policy
и разрешённые claims/membership. Не auto-link аккаунты по совпадающему email.

Один человек может иметь несколько local memberships, роли и grants различаются.
Смена identity provider/переезд требует verified linking/export, а не тихого merge.
Server administrator не получает чужой cloud password или глобальный refresh token.

## 4. Вход на native/web

1. UI предлагает Vela ID / Свой сервер / Локальная скачанная музыка.
2. Native login открывает system browser для Authorization Code + PKCE; не собирает
   provider password в embedded WebView. Redirect registration/state/nonce проверяются.
3. Web flow использует поддерживаемую secure session модель и CSRF/origin defense.
4. Проверить issuer, audience, signature/keys, expiry, authorized redirect и code replay.
5. Resource access token узко scoped под нужный origin/service; ID token не служит
   универсальным bearer-пропуском к music APIs.
6. Tokens хранятся в platform secure store; refresh rotation/reuse detection по
   выбранному protocol/provider, не самодельному бесконечному token.
7. В профиле видны active devices/sessions, revoke one/all, provider identity и recovery.
8. Недоступность provider не стирает local downloads/valid local memberships;
   initial authentication и offline local unlock явно различаются.

Имена standards — specification pointers, не выполненная security certification.
SDK/version/OS callback registration проверяются до implementation.

## 5. Подключение музыкального сервера

- Войти в Vela ID, выбрать Add server, указать canonical origin/invitation.
- Получить capabilities/name/identity preview; private hostname не публикуется.
- Music server показывает policy: существующий local account link/invite/admin approval.
- На origin получить scoped membership/session, не отправляя ему глобальный password.
- Список connected origins содержит access state/expiry/health и Disconnect.
- Search/browse используют per-origin credentials; queue держит origin refs, не tokens.
- Disconnect отзывает соответствующие grants/sessions и удаляет секреты клиента;
  судьба явно скачанных пользовательских bytes/metadata объясняется отдельно.

Такой flow должен работать для своего VPS, дома, семейного сервера и разрешённых
public catalogs. OpenSubsonic — отдельный client adapter, не OIDC/federation автоматически.

## 6. Что синхронизировать в аккаунте

| Данные | Режим | Обязательные условия |
|---|---|---|
| Theme/density/language/accessibility | Personal settings | Locale/platform overrides, schema version |
| Connected origins | Private metadata | No private directory disclosure; secrets отдельно |
| Favorites/collections | Logical refs | Origin/recording distinction, missing entries retained |
| Cross-origin playlists | Workspace-owned entries | Metadata/stream/download checks независимо |
| Queue/bookmarks | Explicit personal sync | Revision/epoch, restore paused, no auto remote playback |
| Listening history | Отдельное opt-in | Account+device consent, retention/delete/export, не включать login-ом |
| Recommendations preferences | Explicit personal choices | Reversible exclusions, no hidden history cache |
| Pinned device downloads | Device-local manifests | Sync статуса не копирует media/rights на другие devices |

Не синхронизировать пароль music server в обычном JSON, raw media paths, private
provenance и unrestricted audio credentials. Если secrets transfer вообще нужен,
это отдельный reviewed enrollment/secure-store protocol.

## 7. Account recovery и безопасность

Passkeys/WebAuthn, MFA и recovery реализуются возможностями выбранного provider.
Возможность удалить/отвязать аккаунт, скачать portable export, сменить sync authority,
отозвать потерянное устройство обязательна. Recovery ceremony не должна автоматически
раскрывать E2E data без соответствующего recovery key/ясного решения пользователя.

Privacy dashboard разделяет profile discoverability, listening collection, activity
sharing, personalized ranking и device presence. По умолчанию private; никаких
«вошёл в облако — опубликовал библиотеку» и обязательного public friend graph.
Минимизация логов, bounded rate limits/abuse controls, защищённые redirects и SSRF
у OIDC/discovery/network metadata остаются частью реализации.

## 8. Независимость и отказы

Provider offline: cached local session в пределах policy/expiry и offline music
работают; новые cloud commands не притворяются применёнными. Music origin offline:
identity/settings доступны, карточки источника честно unavailable. Workspace offline:
local queues работают независимо и journal сохраняет будущие edits.

У cloud/workspace есть operator, privacy policy, backups, incident/recovery plan,
reproducible deployment и cost budget. Их нельзя заменить словом «децентрализованный».
Hosted service не должен лежать в критическом audio-byte path при обычном playback.

## 9. Миграция без потери локальных аккаунтов

1. Additive mappings; existing user IDs/ownership/sessions/consents остаются.
2. User authenticates current local account, затем отдельно global identity.
3. Preview того, что будет linked/synced, и явное подтверждение.
4. Дубликаты IDs/names не объединяются автоматически; конфликт получает review.
5. Undo unlink оставляет исходную библиотеку/плейлисты; remote grants переавторизуются.
6. Экспорт/перенос workspace имеет version/manifest и origin refs, без server secrets.
7. Offline namespace migration выполняется по origin+membership, не одному UUID.

## 10. Приёмка ID-01–10

Один identity на Linux+Android, два music origins с разными local IDs; одинаковые
usernames у разных identities; issuer spoof/claim mismatch/replayed code; отменённый
login/held old reply; provider outage; origin outage; unlink/relink; lost-device revoke;
private profile defaults; export/delete. Во всех сценариях музыка не становится
авторизованной только из-за существования cloud account.
