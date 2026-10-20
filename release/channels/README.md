# Channel recipes

Each file in this directory is the executable recipe for one publishing channel.
The release pipeline calls it with two arguments:

    <recipe> <artifact-path> <target-id>

A recipe publishes the artifact and exits 0 on success. It must never publish
something that was not verified: the pipeline re-checks the artifact checksum
immediately before the recipe runs.

A channel without a recipe is reported as `not-implemented` by the pipeline. That
is deliberate. The project ships one Linux desktop today, and the other targets
are blocked on missing platform support in the code base, not on missing
upload commands. Writing an upload command for a target that cannot be built
would make the release look further along than it is.

Channels that need an account expect these environment variables (the names
come from `release/targets.yaml`):

    AUR_SSH_KEY / AUR_REPO     push the AUR PKGBUILD
    WINGET_TOKEN               submit a winget manifest
    HOMEBREW_TAP_TOKEN         push a Homebrew tap
    PLAY_SERVICE_ACCOUNT_JSON  upload an Android App Bundle
    APPLE_ID / APPLE_APP_SPECIFIC_PASSWORD / APPLE_TEAM_ID
                                upload a macOS or iOS build
