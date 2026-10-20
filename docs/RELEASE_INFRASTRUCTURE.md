# Release infrastructure

How a Vela release is produced, verified, signed and published. The design goal
is that every step is reproducible, every step is checked, and no step ever
claims more than it did.

## The one source of truth

`release/targets.yaml` lists every distribution target: what it is, which
platform it runs on, which channel it is published to, what it needs, and its
honest status. Nothing else hardcodes a channel.

| status | meaning |
| --- | --- |
| `ready` | built, verified and published on every release |
| `buildable` | the artifact is built and verified, publishing still needs a channel or credential |
| `blocked` | the target cannot be built from the current code base |

`python3 scripts/release_manifest.py` validates the manifest before anything is
built. It rejects a target that requires itself, an unknown or cyclic
dependency, a target that holds credentials it never uses, a store target that
publishes while it is blocked, and a ready target that keeps unused secrets.

The platform state is measured, not guessed:

| Platform | State | Why |
| --- | --- | --- |
| Linux x86_64 | `ready` | the client that exists, built, verified and launched here |
| Windows x86_64 | `buildable` | the same client compiles for `x86_64-pc-windows-msvc`; the CI job builds it on a Windows runner and checks the binary |
| macOS aarch64 | `buildable` | the same, on a macOS runner |
| Android arm64 | `buildable` | the same client builds for `aarch64-linux-android` against the Android NDK and asks for the platform audio stack (`libaaudio`); it is built and inspected by the CI job |
| iOS arm64 | `buildable` | the same, on a macOS runner |
| Store uploads | not yet | a store needs an application project that packages, signs and uploads the binary. The channels and their credential names are ready; the application projects are the remaining work |

The mobile client is not a second application. It is the same UI, the same HTTP
client, the same queue and the same audio authority behind the platform webview
shell; `apps/desktop/src/bin/vela_mobile.rs` only chooses the shell.

## Building

```bash
python3 scripts/release_build.py --output target/release-artifacts
python3 scripts/release_packages.py --output target/release-artifacts
```

The build refuses to run on a dirty tree, because an artifact must be
traceable to exactly one commit. Every archive carries a `BUILD.txt` with the
version, the target triple and the commit, a `.sha256` sidecar, and is reopened
after it is written to confirm it really contains the files it claims.

`release_packages.py` builds the Linux payloads: a Debian package, an RPM
source package, an AppImage and an AUR recipe with a real source URL and
checksum. Where the build tool is not installed (`dpkg-deb`, `rpmbuild`,
`appimagetool`) the channel is reported as `unavailable` instead of pretending
to have produced a package. The CI image installs those tools, so the packages
are built there.

## Signing

```bash
python3 scripts/release_sign.py --gpg-key <fingerprint> --artifacts target/release-artifacts
```

Each artifact gets a detached signature, and the signature is verified
immediately after it is written. The workflow signs with
`RELEASE_GPG_PRIVATE_KEY` when it is configured and says out loud, with a
workflow warning, when it is not.

## Publishing

```bash
python3 scripts/release_publish.py --dry-run     # report, never touch the network
python3 scripts/release_publish.py                # publish
```

Publishing is separate from building and deliberately hard to do by accident:

* only targets the manifest allows are considered;
* every credential a target names must be present, and a missing credential
  stops that publish instead of skipping it quietly;
* the artifact checksum is re-verified immediately before the upload, so a file
  that changed after the build cannot be published;
* each target's outcome is written to a receipt, so a rerun knows what already
  went out.

A channel is a recipe in `release/channels/`: an executable that receives the
artifact path and the target id. A channel without a recipe is reported as
`not-implemented`. That is intentional — the pipeline does not approximate an
upload for a target whose platform does not exist yet.

## Automation

`.github/workflows/release.yml` runs on a `v*` tag or a manual dispatch:

1. **validate** — the manifest is checked before any binary is produced.
2. **build** — the artifacts are built, verified, signed and attached.
3. **linux-desktop** — the desktop package is built and launched in a real
   window on CI.
4. **publish** — only on a manual run with `dry_run: false`, and only with the
   credentials the manifest names.
5. **github-release** — the verified artifacts are attached to the GitHub
   release.

The default manual run is a dry run: it builds and verifies everything and
publishes nothing.

## The frozen runtime

The live service is a separate concern from the release pipeline. It runs a
frozen build from `target/runtime/`, supervises it with systemd and is
installed with `scripts/install-user-service.py`, which refuses to install a
runtime whose archive, source archive and checksums do not match, and whose
database does not match the migrations in that build. The database is backed up
before an upgrade and the previous runtime is kept.

## What is not covered

Physical hardware output, screen readers, real touch devices, Safari/WebKit,
Windows, macOS, Android, iOS, Internet federation and a real store upload are
not exercised by this infrastructure. The manifest keeps them visible as
`blocked` or `not-implemented` instead of hiding them behind a green check.
