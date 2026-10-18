# Platform evidence

Updated2026-10-18. A native server executable is not a desktop music client. Status applies only to the stated scope. COMPILES, RUNS, FUNCTIONAL_TESTED and HARDWARE_VERIFIED require separate evidence; RELEASE_READY is not inferred from any one of them.

| Target | Actual implementation | Latest established evidence | Status / remaining work |
| --- | --- | --- | --- |
| Linux x86_64 server | Axum/SQLite/native binary | Frozen0.11.0 sourcefe0484d/persistent enabled user runtime, schema11, original20-track/20analysis rows, verified stopped backup/restore | FUNCTIONAL_TESTED for documented server scope |
| Web Chromium / Firefox Linux | Dioxus WASM, one HTMLMediaElement, Media Session |0.11 exact package:39 scenarios per engine; explicit publication/rights UI, personal mixes/history/radio and original player regressions | FUNCTIONAL_TESTED for documented scenarios; physical sound/background/Safari unverified |
| Linux desktop client | Dioxus0.7.10 + Rodio/CPAL0.22.2 + MPRIS2; apps/desktop |0.12 source:23 desktop tests + opt-in owned-null-sink audio test + private D-Bus MPRIS test; real WebKitGTK window1280×840/640×700 against isolated fixture server/media; play/pause/seek/queue/search/theme/logout; zero leftover temp originals | FUNCTIONAL_TESTED for native library/queue playback; no installer/package, secure persistence, offline, background, favorites/playlist UI or physical-output audit |
| Windows desktop | None | No Windows target/runner available | NOT_STARTED; native engine/SMTC and Windows build/runtime required |
| macOS desktop | None | No Apple host/Xcode | BLOCKED for Apple compilation/runtime; shared-contract work remains possible |
| Android | None | adb, sdkmanager, gradle absent from PATH; no connected-device verification | NOT_STARTED; SDK/NDK/toolchain, Dioxus native bridge, Media3 service/audio focus/offline sources required |
| iOS | None | xcodebuild absent; Linux host, no simulator/device/Apple signing environment | BLOCKED for compilation/device validation; AVAudioSession/AVFoundation/remote-command bridge still required |
| ARM64 server | Deployment target only | Not compiled/run in this session | NOT_STARTED validation |
| Docker | Recipe present | docker info fails: daemon socket absent | BLOCKED runtime validation; no image success claimed |

Installed Rust targets are x86_64-unknown-linux-gnu and wasm32-unknown-unknown. No mobile/native client has been silently substituted with a browser WebView. The0.12 Linux desktop client is a real Dioxus/WebKitGTK window with a native Rodio/CPAL output thread and MPRIS observer; a WebView was not used for audio, and the web player remains a separate target. Android tooling/NDK/Gradle and Apple/Xcode are absent, so those targets remain NOT_STARTED/BLOCKED rather than inferred from Linux. Native next work is packaging/session persistence for Linux, then Android Media3 foreground service and Apple AVFoundation on their real toolchains. Audio must outlive UI route/lifecycle changes; Linux window evidence does not certify mobile background audio. Android's [Media3 ExoPlayer documentation](https://developer.android.com/media/media3/exoplayer) was checked during this session; no Android version or hardware compatibility is claimed without a build.
