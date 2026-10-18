# Persistent local user service

`scripts/install-user-service.py` converts the verified local review process into
a persistent service for the current ordinary Linux account. Running it without
`--install`, or with `--dry-run`, only validates. It does not download packages,
change listening consent, migrate a database, replace frozen assets, configure
root services, enable linger, or expose a network interface.

The service is enabled for `default.target` in the user's systemd manager. Its
unit survives reboot and starts when that manager starts, normally at login.
Whether it continues after logout depends on the host's existing user-manager
policy. Starting before any login requires separately administered linger or a
system service; this installer never configures either. No boot or logout
behavior is inferred from a successful local health check.

## Validate the existing instance

Run from the Vela checkout as the account that owns the runtime and original
private data. Python 3 and a working systemd user manager are required. Automatic
transient-unit rollback additionally needs `systemd-run --expand-environment`
(systemd 254 or later). The current native GNU/Linux architecture must match the
review package.

```bash
python3 scripts/install-user-service.py \
  --runtime-dir target/runtime/vela-0.11.0 \
  --data-dir data \
  --archive target/releases/vela-0.11.0-x86_64-unknown-linux-gnu.tar.gz \
  --source-archive target/releases/vela-0.11.0-source.tar.gz \
  --checksums target/releases/vela-0.11.0-SHA256SUMS \
  --replace-transient --dry-run --print-unit
```

Use the actual current paths/version rather than assuming this historical
example is the live process. `--replace-transient` authorizes the known,
same-build local-review handover; it does not itself install anything.
`--unit` defaults to `vela-local-development.service`; `--port` defaults to 3210.
The bind address is always `127.0.0.1`, on an unprivileged port.

Validation checks both archives against the supplied local SHA256SUMS, matches
the frozen `BUILD.txt` to the native archive, and compares every executable and
`dist/` file with the archive. The source archive's Git commit must match
`BUILD.txt`; its complete numbered migration set must match every successful
SQLx migration version and SHA384 checksum in the existing database. A missing,
older, newer, failed or differently checksummed schema is rejected before
installation. Archives are streamed without extraction and bounded by entry
count and expanded size. SHA256SUMS is a local integrity checkpoint, not a
publisher signature; use retained, trusted release artifacts.

The database is opened read-only in one SQLite snapshot, with WAL support,
`quick_check` and foreign-key checks. No database is created and no SQL mutation
is issued. The already running workers can continue their ordinary work during
validation. When inactive, a held data lock or occupied port blocks installation.
For an active service, the installer checks the account, `/proc` executable,
arguments and effective data/assets/bind environment, health version and all
eight served application assets. A process merely answering `/health` is not
accepted as ownership evidence.

Paths must have no symlink component or control characters. Runtime files and
data must belong to this account and must not be writable by another account;
the data directory must already be 0700. Executable paths containing quotes or
backslashes are rejected because systemd does not accept them. Spaces, literal
percent signs and dollar signs are escaped for systemd; data paths also support
quotes and backslashes. `ExecStart` disables environment substitution and uses
the absolute frozen executable without a shell. The working directory is `/`;
data and web paths are absolute.

## Install after validation

Repeat the same command with `--install` in place of `--dry-run`. The installer
repeats preflight while holding a private installation lock. It saves the old
unit definition in a unique 0600 `*.before-*` file, gracefully stops the verified
old service, rechecks the frozen bytes and matching database, and atomically
writes a 0600 persistent unit under
`${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/`. It reloads the user manager,
starts the service, verifies executable/health/assets/schema again, and enables
it only after readiness succeeds. It does not change the data directory or
restore a backup over user data.

The generated unit sets a 0077 umask, `NoNewPrivileges=yes`,
`Restart=on-failure`, a 2s restart delay, at most 3 starts per 60s and a 60s graceful
stop timeout. It pins the existing loopback/data/web settings and ordinary
application size defaults. Custom `VELA_*` configuration, edited/unrelated unit
files, aliases, unit drop-ins and unfamiliar transient directives require a
manually maintained service and are refused. An existing managed unit is
accepted only when its definition exactly matches the requested same-build
configuration. Changing versions or schema remains the separate backed-up
upgrade procedure in [OPERATIONS.md](OPERATIONS.md).

On a failed stop/write/reload/start/readiness/enable step, the installer restores
the prior persistent file and startup link state. If it stopped the supported
transient local-review service, it recreates its original executable,
environment, working directory and supervision properties using structured
`systemd-run` arguments. Rollback readiness is checked too. Failures during
rollback are reported explicitly with the retained private configuration path;
they are never reported as success. A power loss, forced kill or user-manager
failure can interrupt that recovery; retain the printed previous-config path
and existing runtime/backup checkpoints.

Inspect after installation:

```bash
systemctl --user show vela-local-development.service \
  -p ActiveState -p SubState -p MainPID -p FragmentPath -p Transient -p UnitFileState
systemctl --user is-enabled vela-local-development.service
curl --fail http://127.0.0.1:3210/health
```

## Reverse the supervision change

To stop automatic user-manager startup while retaining the installed unit:

```bash
systemctl --user disable --now vela-local-development.service
```

This leaves the frozen runtime, original data and all backups intact. A manual
`systemctl --user start vela-local-development.service` can still start it.
For complete unit removal, first disable/stop it, verify `FragmentPath` is the
installer's exact user-unit path and inspect its `# Managed by Vela` marker,
then remove only that file and run `systemctl --user daemon-reload`. Preserve
the `.before-*` files until recovery is no longer needed. If restoring a saved
previous definition, stop the service first, copy that exact retained definition
back to the same unit path with 0600 permissions, reload and start. A saved
transient definition has no `[Install]` section: when restored as a file it is
manually started, rather than automatically enabled. This changes supervision
only; never delete the original data or roll back its schema in place.

## Verification scope

`python3 -m unittest discover -s tests -p test_user_service.py -v` exercises real
isolated archives, source provenance, SQLite WAL/schema/checksums, file and port
conflicts, actual loopback HTTP responses, systemd unit parsing, private file
writes and injected transactional failures. Mutating systemd calls are mocked;
these regressions do not install a service or validate reboot, logout, native
audio or Docker behavior. Live `--help` and validation-only execution against a
retained runtime are separate from an actual successful installation, which
must be recorded in PROGRESS/QA evidence when executed.
