# Publishing channels

The current release workflow calls `aur.sh <validated-recipe-directory> vela-music-bin`
only after GitHub assets exist. It publishes PKGBUILD and .SRCINFO, using the configured
AUR account key and pinned host identity. The recipe uses the actual package archive
checksum and carries the private xdo ABI compatibility library plus its notice.

Other channel scripts and `release_publish.py` are legacy/manual helpers; they are
not automatically invoked by the current release workflow. Google Play, App Store,
winget and Homebrew submissions remain separate work. Android APK signing does not
imply Play upload or device/background validation. See [distribution](../../docs/DISTRIBUTION.md).
