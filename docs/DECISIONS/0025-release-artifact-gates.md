# ADR0025: Verified native artifacts before publication

Accepted 2026-10-20 for the user's push/distribution request.

Use reusable quality and package workflows. Build native Windows on MSVC, Linux
on Ubuntu24.04, and Android through the pinned Dioxus CLI application generator.
An Android ELF alone is not an APK. Preserve executable names and platform/source
metadata in every archive. The Linux AppImage uses host GTK/WebKit libraries;
its portability is limited to that declared ABI/runtime.

Gate tag publication on quality, every required format, actual unprivileged Arch
makepkg and consolidated hash/payload checks. Keep platform reports distinct before
merging; never silently skip a required missing packaging tool. Publish versioned
GitHub assets before AUR so makepkg can retrieve and verify the original binary
archive. AUR requires an owner account/key and pinned host identity. Use the
user-selected available name vela-music-bin; vela-desktop-bin belongs to another app.

Main and pull requests produce Actions artifacts without a public release. Tags
publish only when their version matches Cargo. Manual dispatch remains build-only.
APK debug-signing previews are labelled and distinct from an optional durable release
identity. No credentials in outputs, no local data/runtime upgrades, no store/device
validation inferred from compilation or archive verification. See DISTRIBUTION.md.
