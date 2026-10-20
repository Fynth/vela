# Public release 0.12.8 — 2026-10-20

[GitHub release](https://github.com/Fynth/vela/releases/tag/v0.12.8) published at **2026-10-20 19:21:09 UTC**, non-draft.
Tagged source: `eb0fb8cd059e2da03ef94abe9265d4a4d3505c63`; annotated tag `v0.12.8`.
[Workflow 39479453423](https://github.com/Fynth/vela/actions/runs/39479453423) completed **SUCCESS** across all required jobs:
quality, dependencies, Docker build, Linux, Windows, Android, Arch makepkg,
aggregate verification, GitHub publication and AUR publication.

## Public payload ledger

Downloaded public `release-report.json` matches version/source above. Each payload's
size and SHA256 below also matched its public GitHub release-asset digest/size.
The release contains nine payloads, nine SHA256 sidecars, the consolidated report,
`SHA256SUMS` and PKGBUILD: **21 assets**. Hashes detect corruption, not publisher identity.

| Published file | Bytes | SHA256 |
| --- | ---: | --- |
| `vela-0.12.8-android-arm64.apk` | 6,740,396 | `b14637e9e89ab1d879955448cc278fb435fdf03c3067918b7cfd1cdb6daea70d` |
| `vela-server-0.12.8-linux-x86_64-local-review.tar.gz` | 10,287,730 | `49a5e14a3d7071c8d04a4af4561af17649ba7bd252d3afd2078a0288c8338e2f` |
| `vela-web-0.12.8-linux-x86_64-local-review.tar.gz` | 4,116,880 | `8bc3b46eb641e942b09205800bc1d6bd5830e30d5981bde8e548917a2dbbdef6` |
| `vela-desktop-0.12.8-linux-x86_64-local-review.tar.gz` | 5,341,758 | `fe31268546c5ee50c2d4c97d2bc4a9ca3c4947202fd8c71a3d88939ec8e3c766` |
| `vela-desktop_0.12.8_amd64.deb` | 4,258,104 | `8e68213eb34a670624c4f37e8bec6a59a25773c77cb38d689b8645877e99ed1a` |
| `vela-desktop-0.12.8-x86_64.AppImage` | 5,863,928 | `9fb5fd11861e476904773b0ae5ea66c67c4340f9c16fec2aaa362eca91f8a506` |
| `vela-music-bin-0.12.8-x86_64.tar.gz` | 5,366,683 | `5d4662e34841a45b428eae66542cbe2c7f6c44c80e2e238416a18ecf92773777` |
| `vela-desktop-0.12.8-windows-x86_64-winget.zip` | 4,398,106 | `2be5489fde837e9bc5ff19e644899d180a8bb6d82a1f6b0318b52c1a92626ee4` |
| `vela-desktop-0.12.8-windows-x86_64.exe` | 12,846,080 | `217d4114e6c8a681931c765d2c0017c404e49f2349b3137d6150ac9567b95e9c` |

The inherited `local-review` archive suffix and ZIP `winget` channel name are
packaging names; these are public release assets, without a winget store submission.

## AUR and yay

Public package: [vela-music-bin 0.12.8-1](https://aur.archlinux.org/packages/vela-music-bin).
Public recipe git: `https://aur.archlinux.org/vela-music-bin.git`.
Recipe commit: `10c82b3c5810d9b6a28a8497433d2e861468c887` (PKGBUILD and `.SRCINFO`).
The publish job succeeded at 19:21:41 UTC after GitHub assets existed. An anonymous
clone ran `makepkg --noconfirm --nodeps`: downloaded the public versioned AUR source
tar, SHA256 passed and package built successfully. Its executable returned
`vela-desktop 0.12.8`. Final `yay -Si vela-music-bin` returned 0.12.8-1/maintainer Rasul
after the external AUR metadata snapshot updated. No system installation was run.

```sh
yay -S vela-music-bin
```

Public clone/build evidence is in ignored
`target/checks/distribution/public-0.12.8/aur` and `makepkg-public.log` beside it.
The Arch recipe carries Ubuntu's compatible libxdo.so.3 privately plus its license;
its launcher scopes that library path to Vela. This avoids requiring obsolete
system xdotool3 on current Arch (which supplies libxdo.so.4).

## Executed validation and limits

- fmt, strict workspace/all-target Clippy, strict WASM and native-QA Clippy PASS;
  release WASM compilation PASS; Rust 374 PASS/3 opt-in ignored, including 33 API tests.
- Python 38 PASS and actionlint 1.7.12 PASS; full required artifact set, checksums,
  sidecars, archive payloads and common source/version verification PASS.
- Final CI Chromium 60 PASS/1 SKIP (4.3m), Firefox 60 PASS/1 SKIP (5.7m). The sole
  skip per engine is the pre-existing opt-in live Internet import. Firefox uses
  an owned PulseAudio null sink; no physical output certification follows.
- Optimized server extracted from release tar: real startup/assets/WASM/schema 13,
  shutdown and supervised-worker-fault checks PASS. Linux DEB/AppImage CLI PASS.
- Native Windows MSVC EXE/ZIP build and CLI PASS. WebView2 required; no Authenticode.
- Arch container unprivileged makepkg, `.SRCINFO`, linkage and CLI PASS; public
  release download/build/checksum/CLI and yay query PASS as described above.
- Dioxus-generated Android arm64 APK build, durable release v2/v3 signature,
  package/version/API 26 minimum, non-debuggable, backup-off and cleartext-off checks PASS.
- Docker image BUILD PASS; container runtime NOT_RUN. AUD-20 dependency maintenance
  notices remain informational; a strict security audit is not claimed clean.

New native GUI/physical audio/hardware, Windows GUI and Android device/background
checks are NOT_RUN for this release. Earlier Linux window/audio evidence remains
anchored to its dated source in QA_EVIDENCE and PLATFORM_MATRIX. AppImage/DEB/tar
require Ubuntu 24.04+ compatible host GTK3/WebKitGTK4.1/ALSA/libudev/libxdo.so.3;
AppImage contains the application, not these host libraries. Native package build
and signing do not establish device behavior. No macOS/iOS/store publication claim.

The v0.12.7 tag and failed gate remain preserved: a packaging test improperly used
a prebuilt release binary, then was corrected to supply its own fixture before
0.12.8. No check was bypassed or tag rewritten. Listening GET writer contention
and TR03 short-fixture EOF timing were fixed with retained regression assertions.

Frozen runtime, original data and preserved target checkpoints were not upgraded
or deleted. No fresh live-health statement is made. Dedicated AUR/Android private
key backups remain ignored in `target/private-release-keys` (0700 directory,
0600 files); repository secrets provide CI identities. No credentials are in
tracked files or payloads. See [DISTRIBUTION](DISTRIBUTION.md),
[QA_EVIDENCE](QA_EVIDENCE.md) and [ADR0025](DECISIONS/0025-release-artifact-gates.md).
