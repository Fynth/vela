# Operations

## Installation and access

See README for local build and Compose. Application management currently covers initial account setup, local user creation, upload/batch upload, direct imports, favorites and playlists. Storage/public URLs are administrator environment configuration; the full no-console health/storage/invitation wizard remains roadmap work.

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
5. Check /health, login, track counts, sample playback and import recovery. Do not attempt an unsupported schema downgrade; restore a matched backup/binary instead.

No privileged self-updater, automatic external exposure, paid service or external release publishing is included. FFmpeg is not required by the current server; it was used only to generate test fixtures. Optional transcoding is planned.

## Diagnostics

/health reports process/database reachability and software version; it is not a full disk/transcoding/backup dashboard. Structured logs record startup and failures without raw request URLs/cookies. Import source failures appear in authenticated job status; underlying infrastructure worker failure terminates the process for a supervisor to restart. Data-dir lock files persist harmlessly after process exit; OS locks release on exit, so deleting lock files is unnecessary and unsafe while a process is running.

Data layout: vela.db (+WAL/SHM during operation), media/hash.audio, artwork/hash.jpg, staging/upload-* and staging/imports/job-UUID.part. Never manually rename managed assets. Startup removes only its recognized unpublished regular temp files. A crash can retain a published blob absent from SQLite; reconciliation is not implemented and such blobs are retained.

## Local review packages

After a verified clean checkpoint commit, run `scripts/package-release.sh` with the matching wasm-bindgen CLI on PATH (or VELA_WASM_BINDGEN). It rebuilds native/web release artifacts, includes private backup tooling, deployment/docs and dependency notices, archives the exact Git source checkpoint and writes SHA256SUMS under target/releases. It refuses to overwrite existing archives or package uncommitted source. This does not publish a release. Build triples are labelled from the actual native host; no cross-platform validation is inferred. Review metadata-only license notices before external distribution.
