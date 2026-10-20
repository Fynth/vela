# Distribution (0.12.7)

The release workflow runs on main pushes, pull requests, version tags and manual
requests. Its reusable quality and packages workflows build with the tracked lock
and two Cargo jobs. Main/PR builds retain verified Actions artifacts. Only a pushed
`v<workspace version>` tag publishes a GitHub release, after quality, all required
package jobs, the real Arch makepkg build and aggregate verification succeed.
Manual runs build artifacts without publishing. Tags must refer to the source
commit named by every report. There is no deployment of the persistent local server.

| Platform | Artifacts | Scope |
| --- | --- | --- |
| Linux x86_64 | Desktop/server/web `.tar.gz`, `.deb`, `.AppImage` | Ubuntu 24.04 build; GTK3, WebKitGTK 4.1, ALSA, libxdo and libudev required; AppImage contains the app, uses host libraries and needs FUSE2 or `APPIMAGE_EXTRACT_AND_RUN=1` |
| Windows x86_64 | `.exe`, portable `.zip` with `.exe` entry | Native MSVC runner, `--version` smoke; WebView2 runtime required; no Authenticode signature/installer |
| Android arm64 | `.apk` | Dioxus CLI 0.7.10 generates the application, SDK35/NDK r26d, minimum API26; signature and payload checks; no device/background audio certification |
| Arch x86_64 | `vela-music-bin` PKGBUILD + `.SRCINFO` | Real non-root makepkg build, versioned release asset and SHA256, full binary/launcher/icon/license payload; private Ubuntu libxdo.so.3 compatibility library because Arch ships libxdo.so.4 |

macOS/iOS application packaging, notarization, Google Play, winget and Homebrew
submission remain separate work. Existing source compile reports are not store or
device evidence. RPM is outside the required release set.

Android without a configured signing identity produces a visibly named
`-preview.apk` signed by the runner's ephemeral preview key. It is installable but
may require uninstall/reinstall between builds. Stable update signatures require
GitHub secrets `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`,
`ANDROID_KEY_PASSWORD` and variable `ANDROID_KEY_ALIAS`. Partial signing config
fails. The repository owner-authorized pipeline now has a dedicated stable signing
identity; its private backup is in ignored `target/private-release-keys` (directory0700,
files0600). Preserve it across rebuilds and back it up securely. Temporary signing files are private and removed even after a failed build;
no keys enter artifacts. Android manifest disables backups and cleartext traffic.

For `yay -S vela-music-bin`, the repository owner must provide an AUR account with
its public key registered. Set GitHub secret `AUR_SSH_KEY`, variable `AUR_REPO` to
`vela-music-bin`, and variable `AUR_KNOWN_HOSTS` to the independently verified
`aur.archlinux.org` SSH host key line. The AUR job runs only after the GitHub release
exists; missing credentials fail explicitly and do not claim an AUR publication.
It pushes both PKGBUILD and `.SRCINFO`, handles an unchanged recipe idempotently,
and accepts only the selected package/repository. `vela-desktop-bin` is an unrelated
AUR vault application and must not be replaced.

Each required payload has a per-file SHA256 sidecar and an entry in the consolidated
`release-report.json` and `SHA256SUMS`. Reports from different platforms have unique
names and must agree on source commit/version. Missing, modified, incomplete or
duplicate artifact sets fail verification. Checksums detect byte changes; they are
not publisher signatures. Existing optional GPG/channel tools do not run implicitly.

Local checks and actual remote run links are recorded in QA_EVIDENCE and PROGRESS.
A queued job is not a passing job. No installation, UI, device audio, Docker or store
claim follows solely from packaging or a `--version` check.
