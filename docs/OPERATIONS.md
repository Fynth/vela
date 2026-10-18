# Operations

## Installation and access

See README for local build and Compose. Application management currently covers initial account setup, local user creation, upload/batch upload, direct imports, configured folder scans, the private Jobs center, catalog editing, typed library filters, favorites and playlists. Storage roots are administrator environment configuration; public catalog origin/enablement and explicit owner publication are managed in Network; the full no-console health/storage/invitation wizard remains roadmap work.

A native deployment uses one binary, built web assets, writable private local data directory and systemd unit in deploy/vela.service. Install an unprivileged `vela` account and copy the binary/web files into /opt/vela; systemd StateDirectory creates /var/lib/vela. SQLite must use local storage with working locks; network filesystems and multi-process use are unsupported.

Compose publishes loopback3210 only. Use SSH forwarding/private networking or an administrator-managed HTTPS reverse proxy for access. deploy/Caddyfile.example is an example, not an automatically deployed proxy. Preserve Host to match browser origins, set VELA_COOKIE_SECURE=true with HTTPS, and complete setup before public exposure. Do not enable raw HTTP access on an untrusted network. Incoming forwarded-IP headers are ignored to prevent spoofed rate-limit identity.

## Backups

The Python standard-library helper snapshots SQLite with the backup API, verifies database integrity, and archives only immutable audio/artwork referenced by that snapshot plus a size/SHA256 manifest. Creation can run while the current immutable-only media model is active; stop the server for upgrades, migrations or future deletion features. Store archives securely: this is a full instance backup, including credentials and sessions, not a public metadata export.

```bash
python3 scripts/backup.py create --data-dir data --output backups/vela-2026-10-13.tar.gz
python3 scripts/backup.py restore --archive backups/vela-2026-10-13.tar.gz --data-dir restored-data
```

Restore requires a new/empty directory; it validates path allowlists, regular files only, entry uniqueness, declared sizes, total disk budget, SHA256 and SQLite/FK integrity. Default restore budget20GiB; raise --max-bytes explicitly for a larger instance. Archive destinations are never overwritten. Before changing VELA_DATA_DIR to the restored directory, stop the old process and verify media/account state with the new process. Keep the original backup and data until verification completes.

## Upgrades

1. Review release notes and known limitations; save a private backup and test its restore.
2. Stop Vela gracefully. Only one process may own a data directory. SIGTERM stops receiving uploads, completes atomic processing and drains workers.
3. Replace binary/web assets or rebuild the container.
4. Start the server. SQLx applies forward migrations before serving.
5. Check /health, login, track counts, sample playback and import/scan recovery. Do not attempt an unsupported schema downgrade; restore a matched backup/binary instead.

No privileged self-updater, automatic external exposure, paid service or external release publishing is included. FFmpeg is not required by the current server; it was used only to generate test fixtures. Optional transcoding is planned.

## Diagnostics

/health reports process/database reachability and software version; it is not a full disk/transcoding/backup dashboard. Structured logs record startup and failures without raw request URLs/cookies. Import/scan source failures appear in authenticated job status; underlying infrastructure worker failure terminates the process for a supervisor to restart. Data-dir lock files persist harmlessly after process exit; OS locks release on exit, so deleting lock files is unnecessary and unsafe while a process is running.

Data layout: vela.db (+WAL/SHM during operation), media/hash.audio, artwork/hash.jpg, staging/upload-*, staging/scan-<16 random characters>.part and staging/imports/job-UUID.part. The default sources/ folder contains operator-supplied read-only input, not managed audio. Never manually rename managed assets. Startup removes only its recognized unpublished regular temp files. A crash can retain a published blob absent from SQLite; reconciliation is not implemented and such blobs are retained.

## Local review packages

After a verified clean checkpoint commit, run `scripts/package-release.sh` with the matching wasm-bindgen CLI on PATH (or VELA_WASM_BINDGEN). It rebuilds native/web release artifacts, includes private backup tooling, deployment/docs and dependency notices, archives the exact Git source checkpoint and writes a version-specific vela-VERSION-SHA256SUMS under target/releases. Previous archives/manifests are preserved. It refuses to overwrite existing archives or package uncommitted source. This does not publish a release. Build triples are labelled from the actual native host; no cross-platform validation is inferred. Review metadata-only license notices before external distribution.

Validate a local native archive with `python3 scripts/verify_package.py target/releases/vela-0.8.0-x86_64-unknown-linux-gnu.tar.gz` and `sha256sum -c target/releases/vela-0.8.0-SHA256SUMS`. The verifier extracts only into its own temporary directory, starts a loopback-only server, checks the expected embedded schema version, first-run state/bundled HTML/WASM and graceful exit, then removes its fixture. To run manually, extract the native archive into a new directory, enter it and run `./vela-server`; bundled `dist/` is the default web root.

The native host artifact dynamically links libc/libm/libgcc_s; inspection found GLIBC2.34 symbol requirements. This is a minimum ABI requirement, not validation on every Linux distribution. Only the actual x86_64 GNU/Linux build host was tested; other systems/ARM64 need their own builds and runtime validation.

## Read-only music folders

On Linux, the default Music folder is VELA_DATA_DIR/sources, created on first start. The installation operator can place owned music there or mount an existing folder read-only. Then open Settings → Music folders, choose the folder, name the source, click Add source and Scan now. The browser shows durable counters and supports cancellation. Scan again after adding/changing files. Removing a source clears its scan tracking/history and keeps all imported music.

For approved external folders, configure an array of at most32 absolute, nonoverlapping roots before starting the server:

```bash
export VELA_SCAN_ROOTS='[{"id":"collection","name":"Music collection","path":"/srv/music"}]'
```

The service user must have read/traverse permissions on each path component. Paths and ancestors must not be symlinks. System directories and managed storage are rejected. An unavailable/missing external mount appears unavailable in Settings; do not point a source at host secrets. `VELA_SCAN_ROOTS='[]'` disables folder access. Changing installation configuration requires a restart. No path can be entered through the browser/API.

For Compose, put the following beside compose.yaml as compose.override.yaml, using an existing operator-owned /srv/music folder; validate with docker compose config before deploying:

```yaml
services:
  vela:
    environment:
      VELA_SCAN_ROOTS: '[{"id":"collection","name":"Music collection","path":"/music"}]'
    volumes:
      - /srv/music:/music:ro
```

Sources are read-only inputs. Supported audio is copied into immutable managed storage, so budget for the additional copy; exact duplicates reuse managed bytes. Existing edited catalog tags survive unchanged and exact-duplicate rescans. Changed files create/deduplicate a new snapshot; missing files never delete tracks. Signature-based change detection is best-effort, not a full integrity audit. Cancellation/shutdown checks occur between I/O calls and finish any atomic publication safely. Broken/network filesystem reads have no hard timeout guarantee; use local storage.

Instance backups include private source configuration records, job state/fingerprints and managed audio, but do not copy the original sources/ directory or external roots. Back up original collections independently. Restores need the intended VELA_SCAN_ROOTS configuration/mounts; managed music remains playable without sources. No downgrade to0.1.x after migration0004 is supported; restore a matching backup and old binary instead.

## Catalog index upgrade (0.3.0)

Migration0005 creates five owner-prefixed indexes for filtering and ordering;0006 rebuilds the genre index with a covering column. Startup applies them before serving, so large existing catalogs can take additional startup time and disk. Use the normal private backup/upgrade procedure. No source audio/tags/sharing grants are rewritten. Measured SQL fixture index sizes/build times are in PERFORMANCE; they are not a guaranteed runtime estimate for a real VPS. Downgrading schema in place is unsupported; restore a matched pre-upgrade backup with its previous binary.

## Task history upgrade (0.4.0)

Migration0007 rebuilds the import/scan owner-history indexes with UUID tie-breakers and adds owner/status/history indexes. Follow the same backup/upgrade procedure; startup time and disk depend on retained history. Worker state, source audio and permissions are unchanged. Jobs displays your durable imports and, for administrators, your folder scans. Cancel/retry/Scan again revalidate existing service policies. A new scan preserves earlier passes; removing its source removes those history rows while keeping imported music. Opening Jobs loads the current history; an already idle page requires Refresh to discover work started elsewhere. Known active work polls while visible. Upload transfer progress remains in the upload dialog.

## Web player upgrade0.8

This increment keeps schema7, media ownership, original files, HTTP APIs and upstream pins. Replace binary and complete generated web assets together; index.html now references now-playing.css and the matching server explicitly permits that path. Never partially update a running frozen directory or delete target/runtime, target/releases or target/upgrade-backups during development. Extract each package into a new versioned directory, verify its hashes, gracefully stop the owned old process, take/restore-check a private backup, and start the new binary/assets against the original data directory. Validate health/version/setup state and compare database/media integrity. Retain the matching old package and backup for rollback; restoration is tested in a new fixture directory, never over the live library. See PROGRESS for the actual current PID/source/evidence.

## Historical transient review supervision (superseded by0.11 installer)

The following describes the earlier review setup. Current supervision is recorded
in PROGRESS;0.11 adds the persistent same-build USER_SERVICE workflow below.

A tool-owned foreground process can end independently of a source/build checkpoint. Reinspect health, executable and original data before starting another process; never assume a recorded PID is still current. The local review instance can be run as the current user's transient systemd unit vela-local-development.service, pointing at one frozen versioned binary/asset directory and the original private data directory, bound only to127.0.0.1:3210. Its Restart=on-failure policy is bounded to3 starts per60s with a2s delay. SIGTERM/20s shutdown still uses the application’s normal worker cleanup.

Inspect with `systemctl --user show vela-local-development.service -p MainPID -p ExecStart -p ActiveState -p NRestarts` and confirm the expected executable before stopping this owned unit for an upgrade. This unit needs no root account or global package installation. It is transient: automatic startup after a reboot/user-manager restart is not configured. Use the existing deploy/vela.service installation recipe for an operator-managed persistent installation. PROGRESS and ignored runtime JSON identify the actually running version/unit/PID; do not replace frozen files in place.

## Local review0.10 checkpoint (2026-10-18)

Frozen source902dd47 is installed from its verified native archive to
target/runtime/vela-0.10.0 with original data/ on loopback3210. The owned non-root
transient user unit has Restart=on-failure and60s graceful stop; it is not enabled
for reboot/user-manager startup. On resume first verify unit/PID/executable, health
and SQLite schema/integrity rather than assume it remains active. Reuse the frozen
assets/binary when restarting the matching schema; never use an old0.9 binary on
schema10. Before any forward upgrade stop gracefully, create a fresh private
backup and restore it into a disposable empty directory; compare all logical rows
and media checksums. Keep all older archives/snapshots and original data. Details
and actual backup paths are in PROGRESS. Production persistent systemd/Compose
installation remains administrator-controlled; no public deployment occurred.


## Persistent local review supervision

See [USER_SERVICE.md](USER_SERVICE.md) for the validation-first same-build user
installer. It compares the frozen executable/assets with retained native/source
archives and the existing database's exact migration checksums before replacing
a supported transient review unit. It binds loopback only and does not migrate,
configure root/linger or publish music. The unit is enabled with the existing user
manager, normally at login; boot/logout behavior needs independent host validation.

Upgrade this pinned service separately: inspect current process/unit/data, retain
the private old unit, stop normally, make and restore-test a fresh backup, verify
a new frozen package, replace only its executable/assets references, then start
and check migration/row/media preservation. Do not run the same-build installer
against an older schema or silently retarget its unit. No in-place schema downgrade
or backup restore over original user data is supported.
