# Homebrew cask for the Vela desktop client.
#
# The version and URL are rewritten by the release workflow from the verified
# artifact; the repository copy stays a valid, reviewable template.
cask "vela" do
  version "0.0.0"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"

  url "https://example.invalid/vela/vela-desktop-0.0.0-x86_64.dmg"
  name "Vela Music"
  desc "Private music library, offline copies and public catalogs"
  homepage "https://example.invalid/vela"

  depends_on macos: ">= :big_sur"

  app "Vela Music.app"
  zap trash: [
    "~/Library/Application Support/Vela",
    "~/Library/Preferences/com.vela.music.plist",
  ]
end
