# Platform evidence

Inspected2026-10-17. A native server executable is not a desktop music client. Status applies only to the stated scope. COMPILES, RUNS, FUNCTIONAL_TESTED and HARDWARE_VERIFIED require separate evidence; RELEASE_READY is not inferred from any one of them.

| Target | Actual implementation | Latest established evidence | Status / remaining work |
| --- | --- | --- | --- |
| Linux x86_64 server | Axum/SQLite/native binary | Frozen0.9.0 source bc50564/user runtime, schema9, original20-track library, verified backup/restore | FUNCTIONAL_TESTED for documented server scope |
| Web Chromium / Firefox Linux | Dioxus WASM, one HTMLMediaElement, Media Session |0.9 exact package:35 scenarios in each engine; responsive history/radio and original player regressions | FUNCTIONAL_TESTED for documented scenarios; physical sound/background/Safari unverified |
| Linux desktop client | None yet | GTK3.24.52, WebKitGTK2.54.1, ALSA1.2.16.1 available; dx absent | NOT_STARTED; select native audio backend, Dioxus desktop feature separation, MPRIS, packaging and actual playback |
| Windows desktop | None | No Windows target/runner available | NOT_STARTED; native engine/SMTC and Windows build/runtime required |
| macOS desktop | None | No Apple host/Xcode | BLOCKED for Apple compilation/runtime; shared-contract work remains possible |
| Android | None | adb, sdkmanager, gradle absent from PATH; no connected-device verification | NOT_STARTED; SDK/NDK/toolchain, Dioxus native bridge, Media3 service/audio focus/offline sources required |
| iOS | None | xcodebuild absent; Linux host, no simulator/device/Apple signing environment | BLOCKED for compilation/device validation; AVAudioSession/AVFoundation/remote-command bridge still required |
| ARM64 server | Deployment target only | Not compiled/run in this session | NOT_STARTED validation |
| Docker | Recipe present | docker info fails: daemon socket absent | BLOCKED runtime validation; no image success claimed |

Installed Rust targets are x86_64-unknown-linux-gnu and wasm32-unknown-unknown. No mobile/native client has been silently substituted with a browser WebView. Native next work is a shared command/event/capability contract with session/occurrence identity, followed by one actual Linux backend and Android Media3 foreground service. Audio must outlive UI route/lifecycle changes. Android's [Media3 ExoPlayer documentation](https://developer.android.com/media/media3/exoplayer) was checked during this session; no Android version or hardware compatibility is claimed without a build.
