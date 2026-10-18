# Native-платформы: один продукт, правильный audio lifecycle

## 1. Shared и platform-specific части

Общие: origin/account keys, queue occurrences/order, source resolution policy,
download manifests/journal, sync/control protocol, recommendation preferences,
UI primitives и Dioxus screens. Platform-specific: decoder/output/session, secure
store, background transfer, audio focus/route, media controls, permissions/packaging.

Dioxus остаётся UI. Закрытие/сворачивание WebView не должно быть механизмом управления
Android service или iOS audio session. Для каждой platform capability возвращается
реально поддерживаемая функция, не одинаковый набор ложных enabled controls.

## 2. Android — законченный ежедневный клиент

### Подготовка

Проверить текущую stable Dioxus mobile поддержку, Rust target, SDK/NDK/JDK/Gradle
совместимость, min/target SDK и Android permission policy. Зафиксировать toolchain
в воспроизводимом build, не угадывать latest version в инструкции. Подготовить
эмулятор и отдельно физическое устройство; добавить package ID/signing debug profile.

### Audio/service

- Media3 ExoPlayer и MediaSessionService/обоснованный MediaLibraryService владеют
  playback вне UI lifetime; Rust coordinator связывается через typed bridge.
- Notification/lockscreen/Bluetooth/headset команды идут в один command path.
- Audio focus: duck/pause/resume policy, interruption, noisy audio/unplug, звонок,
  смена route/BT и отказ output. Не auto-resume после явного pause пользователя.
- Foreground media service и требуемые platform permissions соответствуют SDK;
  service lifecycle нельзя подменять JavaScript interval.
- Bridge version/correlation/source/session IDs, bounded messages, no secret logging,
  callback freshness после activity recreation/new account.
- Source adapters: authorized progressive HTTP, scoped byte cache, permitted offline,
  cross-origin queue. Native media engine download cache согласуется с общим index,
  не вторая скрытая копия без quota.

### Offline/background

Resumable downloads в поддерживаемой OS background infrastructure; Wi-Fi/metered/
charging/storage rules. App kill/перезапуск не теряет manifest/reservation; user pause
не превращается в automatic retry spam. Android secure token storage использует
platform-backed keys/appropriate credential storage, без plaintext fallback.

### UX

Bottom tabs+mini player, full-screen artwork/player/lyrics/queue, predictive back/
system insets, accessibility font scaling, TalkBack и orientation/tablet layout.
Pull-down sheet не требует жеста: есть обычная кнопка. Download/storage и device
handoff доступны без web-admin. Локальный сервер на desktop не равен localhost
телефона: connection wizard предлагает LAN/VPN/public HTTPS discovery осознанно,
не выставляет домашний сервер в Интернет автоматически.

### Проверка

Install APK→sign in→реальные MP3/FLAC→lock screen→next/pause/headset→route change→
app background→process recreation→offline cold launch→handoff→session revoke.
Emulator PASS и physical PASS записываются раздельно. Battery/OEM restrictions,
foreground notification policy,64bit ABI и APK/AAB packaging — отдельные evidence.

## 3. iOS — настоящий AVFoundation lifecycle

Доступны macOS/Xcode/simulator/device — запускать build/test; нет — точный BLOCKED,
не объявление «shared Rust значит iOS готов». Shared design/source/sync work может
продолжаться независимо.

- AVPlayer/AVQueuePlayer либо обоснованный AVAudioEngine adapter; выбор через codec/
  buffering/seek/gapless проверку. Shared queue не заменяется неподконтрольным OS order.
- AVAudioSession playback configuration/background audio capability, interruptions,
  route changes, headphones/Bluetooth/AirPlay capabilities и media services reset.
- MPNowPlayingInfoCenter/MPRemoteCommandCenter корректно публикуют position/state,
  очищают private metadata на logout, один активный command owner.
- Swift/Objective-C/Rust bridge versioned/cancellable, no callbacks в deallocated UI.
- Keychain и scoped local files; background downloads через поддерживаемую OS механику.
- Safe areas/Dynamic Type/VoiceOver/Reduce Motion, доступный fullscreen player/sheets.
- Проверить locked/background/offline/phone interruption/network change и handoff.
- Signing/entitlements/app-store publish требуют настоящей среды/credentials/
  разрешения; не выполнять внешнюю публикацию автоматически.

## 4. Linux daily driver

Сохранить существующий Rodio/CPAL/MPRIS и исправить source/cache/Hyprland перед
добавлением новой engine abstraction ради архитектурного вида. Ввести native output
route выбор/reconnect, suspend/resume, unavailable-device feedback, background
minimize/close policy. App menu/keyboard shortcuts/About/preferences настоящие.

MPRIS показывает current session для всех поддерживаемых local/remote/offline
источников по privacy policy, а не stopped при реально играющем public track.
Шумные diagnostics доступны отдельно. Portal/native dialogs интегрировать там, где
они действительно нужны; не требовать xdotool от обычного пользователя.

Packages: portable tar уже есть; затем один полноценный формат с проверенным install/
uninstall/update (например Flatpak/AppImage после оценки sandbox/media/Secret Service),
далее deb/rpm по спросу. Нужны runtime library matrix, licensing/notices и реальные
window/audio tests на package, не просто файл с расширением.

## 5. Windows/macOS desktop

Windows: system transport/media keys/SMTC, Windows Credential Manager, device switch,
sleep/hibernate/resume, notifications/tray по user preference, installer+uninstaller.
macOS: native Now Playing/remote commands/audio route, Keychain, app lifecycle,
Apple Silicon/Intel build scope, app/dmg/signing/notarization отдельно.

На обеих платформах source/cache/queue contracts общие, low-level audio/backend
выбирается по реальному codec/output тесту. Linux package не является доказательством
запуска ни на Windows, ни на Apple. Избегать native unavailable controls в основном UI.

## 6. Клиентская compatibility matrix

Для каждого target записать: compiled, installed, styled window, real source,
decoder/output, seek, OS controls, route change, suspend/background, offline cold,
sync/handoff, secure store, screen reader, packaging, hardware/version.
Каждая клетка PASS/PARTIAL/FAIL/NOT_RUN/BLOCKED с командой/устройством/build hash.

Веб остаётся полноценным клиентом с HTMLMediaElement/Media Session, но browser cache,
autoplay/suspension/codec limits и service-worker offline имеют собственные проверки.
Нельзя выдавать PWA за прошедший mobile foreground-service/lockscreen validation.
