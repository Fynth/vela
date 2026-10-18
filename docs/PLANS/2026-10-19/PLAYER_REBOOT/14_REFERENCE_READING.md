# Первичные источники и границы применимости

Дата проверки ссылок/доступных материалов: 2026-10-19. Это целевая техническая
проверка выбранных решений, не полный аудит всех RFC и не native certification.
Продуктовые решения Vela и предположения о производительности принадлежат этому
плану; стандарты сами по себе не доказывают правильность будущей реализации.

## 1. HTTP, повторное использование данных и Range

- [RFC9110: HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html) — HEAD,
  validators, conditional requests, Range/If-Range,206/416. Применять к существующему
  server stream, а не заменять протокол произвольным «chunk API» без необходимости.
- [RFC9111: HTTP Caching](https://www.rfc-editor.org/rfc/rfc9111.html) — различие
  no-store/no-cache/private и revalidation. Новый media cache требует согласованной
  endpoint policy; нельзя игнорировать текущий no-store из удобства.

Связанные планы: [cache/local](02_STREAMING_CACHE_LOCAL_SERVER.md),
[data/privacy](13_DATA_MIGRATION_PRIVACY.md). Server authorization остаётся
дополнительной обязательной проверкой, даже при корректной HTTP семантике.

## 2. Независимый аккаунт

- [OpenID Connect Core1.0, errata2](https://openid.net/specs/openid-connect-core-1_0.html) —
  issuer/subject, claims и authentication; email не устойчивый ключ привязки.
- [RFC9700: OAuth2 Security Best Current Practice](https://www.rfc-editor.org/rfc/rfc9700.html) —
  актуальные security рекомендации; выбранные flows/token handling требуют review.
- [RFC8252: OAuth2 for Native Apps](https://www.rfc-editor.org/rfc/rfc8252.html) —
  external user-agent/system browser, native redirect и PKCE pattern.

[Vela ID design](05_CLOUD_ACCOUNT_AND_IDENTITY.md) — наш integration/workspace план,
а не указание, что OIDC уже включает library sync, media grants или готовый облачный
сервис. Версии provider/SDK, security updates, callback registrations и operational
requirements проверяются заново непосредственно перед реализацией.

## 3. Linux, Hyprland, Dioxus window

- [Hyprland Xwayland](https://wiki.hypr.land/configuring/extra/xwayland/) — различие
  Wayland-native/Xwayland и HiDPI. Сначала установить фактический backend; глобальное
  force-zero-scaling не принимается как обязательное исправление Vela.
- [Hyprland Window Rules](https://wiki.hypr.land/configuring/core/rules/window-rules/) —
  способы описания geometry/rules зависят от версии compositor. В этом плане нет
  автоматически применяемых rules и нет требования менять конфигурацию пользователя.
- [Dioxus desktop Config0.7.10](https://docs.rs/dioxus-desktop/0.7.10/dioxus_desktop/struct.Config.html) —
  pinned API для window/menu configuration. Перед изменением сверить repo lockfile
  и поддержанную platform implementation, не вставлять API другой major версии.
- [Dioxus0.7 setup](https://dioxuslabs.com/learn/0.7/getting_started/) — host prerequisites.
  Существующий проект сохраняется; tutorial не является инструкцией создать новый app.

Некоторые старые Hyprland `/Configuring/...` ссылки больше не подходят. Даже
действующая latest wiki может описывать syntax новее установленного compositor.
План требует version-matched диагностики, а не копирования latest rules вслепую.

## 4. Native mobile

- [Android: Background playback with MediaSessionService](https://developer.android.com/media/media3/session/background-playback) —
  service/session lifecycle и background playback вне UI. Это основание Android
  архитектуры, но не доказательство, что Vela APK уже собирался или работает.
- [Apple: Configuring your app for media playback](https://developer.apple.com/documentation/avfoundation/configuring-your-app-for-media-playback) —
  официальная точка AVFoundation/audio session/background requirements. JS-раздел
  Apple не полностью представлен текстовым browser; детали должны быть проверены
  в полной документации и Xcode при iOS реализации. В этом проходе Apple build не было.

Exact SDK/NDK/JDK/Gradle, Media3, Xcode/deployment targets и Dioxus mobile compatibility
не объявляются «последними стабильными» без отдельной проверки на момент сборки.
[Platform plan](07_ANDROID_IOS_DESKTOP.md) разделяет compile/launch/output/background/
physical-device evidence, как требует исходный mandate.

## 5. Interaction patterns и accessibility

- [WCAG2.2](https://www.w3.org/TR/WCAG22/) — contrast, keyboard/focus, reflow и target
  accessibility criteria. Цветовые предложения в visual plan ещё не прошли отдельную
  instrumented contrast и assistive-tech проверку; палитра не означает WCAG certification.
- [Spotify: Start or join a Jam](https://support.spotify.com/ws/article/jam/) —
  публичный пример разделения host/guest, приглашений и управления общей очередью.
  Vela использует собственные названия/дизайн/permission policy, не копирует artwork
  или source и не наследует права на commercial streams.
- [Spotify: Listening Activity and Request to Jam](https://newsroom.spotify.com/2026-01-07/listening-activity-request-to-jam-messages-updates/) —
  современный пример opt-in social visibility. У Vela history, queue sync, device
  control и presence остаются разными согласиями по собственному privacy contract.

Это ориентиры взаимодействий, не утверждение, что просмотрена или измерена вся
актуальная Spotify UI. Превосходство по UX возможно заявлять только после сравнительного
user testing по конкретным задачам, а не по количеству идей в каталоге.

## 6. Сеть и восстановление данных

Для следующих network/recovery работ дополнительно проверены первичные entry points:

- [SQLite Online Backup API](https://www.sqlite.org/backup.html) — согласованный
  snapshot live DB; Vela всё равно требует отдельного media/config backup и restore test.
- [W3C ActivityPub](https://www.w3.org/TR/activitypub/) — social activity transport/model,
  не готовый стандарт прав на музыкальный stream.
- [RFC7033 WebFinger](https://www.rfc-editor.org/rfc/rfc7033.html) — identity discovery,
  не подтверждение media ownership или разрешение скачивать.
- [RFC9421 HTTP Message Signatures](https://www.rfc-editor.org/rfc/rfc9421.html) —
  стандарт signatures. Он не автоматически совместим со всеми deployed ActivityPub
  signature profiles; выбрать и проверить interoperability profile до заявления compatibility.
- [OpenSubsonic overview](https://opensubsonic.netlify.app/docs/) — client/server API
  и extension negotiation, самостоятельная от social federation capability.

## 7. Внутренние источники истины

- [AGENTS](../../../../AGENTS.md), [CONTRACTS](../../../CONTRACTS.md),
  [ARCHITECTURE](../../../ARCHITECTURE.md).
- [NATIVE_CLIENT_CONTRACTS](../../../NATIVE_CLIENT_CONTRACTS.md),
  [PLATFORM_MATRIX](../../../PLATFORM_MATRIX.md), [NETWORK_RIGHTS](../../../NETWORK_RIGHTS.md).
- [PROGRESS](../../../PROGRESS.md), [QA_EVIDENCE](../../../QA_EVIDENCE.md),
  [KNOWN_LIMITATIONS](../../../KNOWN_LIMITATIONS.md), [PERFORMANCE](../../../PERFORMANCE.md).
- [Предыдущий source audit](../01_AUDIT.md) и [новые конкретные PR-наблюдения](00_CURRENT_PROBLEMS_AND_SCOPE.md).

Противоречия разрешать actual source/build и датированными executed evidence;
не превращать предложенные здесь contracts в описание уже работающего продукта.
