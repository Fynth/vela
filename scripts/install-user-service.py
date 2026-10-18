#!/usr/bin/env python3
"""Validate a frozen, already-migrated Vela runtime; install only with --install.

This is a same-build supervision change, not an updater or database migrator.
Only Python's standard library and the current user's systemd manager are used.
"""
import argparse
from contextlib import closing, contextmanager
import fcntl
import hashlib
import http.client
import json
import os
from pathlib import Path, PurePosixPath
import platform
import pwd
import re
import shlex
import socket
import sqlite3
import stat
import subprocess
import sys
import tarfile
import tempfile
import time


MARKER = '# Managed by Vela install-user-service.py v1\n'
PROPERTIES = (
    'Id', 'LoadState', 'ActiveState', 'SubState', 'Transient', 'FragmentPath',
    'DropInPaths', 'UnitFileState', 'MainPID', 'Environment', 'Description',
    'Type', 'WorkingDirectory', 'Restart', 'RestartUSec', 'TimeoutStopUSec',
    'StartLimitIntervalUSec', 'StartLimitBurst', 'UMask', 'KillMode', 'KillSignal',
)
DEFAULTS = {'VELA_COOKIE_SECURE': 'false', 'VELA_MAX_FILE_BYTES': '262144000',
            'VELA_MAX_STORAGE_BYTES': '10737418240'}
REQUIRED = {'index.html', 'bootstrap.js', 'player.js', 'style.css', 'now-playing.css',
            'vela.svg', 'pkg/vela_web.js', 'pkg/vela_web_bg.wasm'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def safe_path(value, *, exists=True):
    """Keep literal paths, reject control characters and symlink ancestors."""
    value = os.fspath(value)
    require(value and not any(ord(c) < 32 or ord(c) == 127 for c in value),
            'Paths must not contain control characters')
    path = Path(os.path.abspath(os.path.expanduser(value)))
    for part in [*reversed(path.parents), path]:
        require(not part.is_symlink(), f'Symlink paths are unsupported: {part}')
    if exists:
        require(path.exists(), f'Required path does not exist: {path}')
    return path


def owned(path, *, directory=False, private=False):
    info = path.lstat()
    require(info.st_uid == os.getuid(), f'Path is not owned by the current user: {path}')
    require(stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode),
            f'Unexpected file type: {path}')
    require(not info.st_mode & 0o022, f'Path is writable by another account: {path}')
    if private:
        require(not info.st_mode & 0o077, f'Data directory must be private (0700): {path}')


def digest(stream, algorithm='sha256'):
    result = hashlib.new(algorithm)
    while chunk := stream.read(1024 * 1024):
        result.update(chunk)
    return result.hexdigest()


def file_digest(path):
    with path.open('rb') as source:
        return digest(source)


def parse_build(raw):
    require(len(raw) <= 4096, 'BUILD.txt is too large')
    fields = {}
    for line in raw.decode('utf-8').splitlines():
        require('=' in line, 'Invalid BUILD.txt')
        key, value = line.split('=', 1)
        require(key not in fields, 'Duplicate BUILD.txt field')
        fields[key] = value
    require(set(fields) == {'version', 'target', 'commit', 'schema_version'},
            'BUILD.txt must identify version, target, commit and schema_version')
    require(re.fullmatch(r'\d+\.\d+\.\d+', fields['version']), 'Invalid build version')
    require(re.fullmatch(r'[0-9a-f]{40}', fields['commit']), 'Invalid build commit')
    require(fields['schema_version'].isdigit() and 0 < int(fields['schema_version']) <= 10000,
            'Invalid build schema version')
    architecture = {'x86_64': 'x86_64', 'aarch64': 'aarch64'}.get(platform.machine())
    require(platform.system() == 'Linux' and architecture and
            fields['target'] == architecture + '-unknown-linux-gnu',
            'This installer supports matching native GNU/Linux builds only')
    return fields


def archive_members(archive):
    entries, size = {}, 0
    for entry in archive:
        path = PurePosixPath(entry.name)
        size += entry.size
        require(len(entries) < 20000 and size <= 256 * 1024 * 1024,
                'Archive exceeds the bounded review-package budget')
        require(not path.is_absolute() and '..' not in path.parts and
                (entry.isfile() or entry.isdir()), 'Unsafe archive member')
        name = str(path)
        require(name not in entries, 'Duplicate archive member')
        entries[name] = entry
    return entries


def verify_archive_hash(path, manifest):
    owned(path)
    owned(manifest)
    require(path.stat().st_size <= 256 * 1024 * 1024, 'Archive exceeds the compressed review-package budget')
    require(manifest.stat().st_size <= 16384, 'Checksum manifest is too large')
    matches = []
    for line in manifest.read_text().splitlines():
        match = re.fullmatch(r'([0-9a-f]{64}) [ *](.+)', line)
        require(match, 'Malformed SHA256SUMS entry')
        if Path(match[2]).name == path.name:
            matches.append(match[1])
    require(len(matches) == 1 and matches[0] == file_digest(path),
            f'Archive checksum mismatch or missing entry: {path.name}')


def verify_package(runtime, archive, source_archive, manifest):
    """Compare every frozen binary/static byte and the actual SQLx checksums."""
    owned(runtime, directory=True)
    require(not any(c in str(runtime) for c in ('"', "'", '\\')),
            'systemd executable paths cannot contain quotes or backslashes; use another frozen directory')
    binary, web = runtime / 'vela-server', runtime / 'dist'
    for path in (binary, runtime / 'BUILD.txt'):
        safe_path(path)
        owned(path)
    owned(safe_path(web), directory=True)
    require(os.access(binary, os.X_OK), 'Frozen server is not executable')
    require(binary.stat().st_size <= 64 * 1024 * 1024, 'Frozen binary exceeds the review-package budget')
    build_raw = (runtime / 'BUILD.txt').read_bytes()
    build = parse_build(build_raw)
    for path in (archive, source_archive):
        verify_archive_hash(path, manifest)
    expected = {}
    with tarfile.open(archive, 'r:gz') as package:
        entries = archive_members(package)
        require('BUILD.txt' in entries and entries['BUILD.txt'].isfile(), 'Package lacks BUILD.txt')
        require(package.extractfile(entries['BUILD.txt']).read(4097) == build_raw,
                'Frozen BUILD.txt differs from the verified package')
        for name, entry in entries.items():
            if name == 'vela-server' or name.startswith('dist/') and entry.isfile():
                require(entry.isfile(), 'Package server must be a regular file')
                expected[name] = digest(package.extractfile(entry))
    require('vela-server' in expected and {'dist/' + p for p in REQUIRED} <= expected.keys(),
            'Package lacks required binary/static assets')
    actual = {'vela-server': file_digest(binary)}
    size = binary.stat().st_size
    for count, path in enumerate(web.rglob('*'), 1):
        require(count <= 20000, 'Frozen web tree exceeds the entry budget')
        require(not path.is_symlink(), 'Frozen web assets must not contain symlinks')
        owned(path, directory=path.is_dir())
        if path.is_file():
            name = path.relative_to(runtime).as_posix()
            require(name in expected, 'Frozen binary/static assets differ from the verified package')
            size += path.stat().st_size
            require(path.stat().st_size <= 64 * 1024 * 1024 and size <= 256 * 1024 * 1024,
                    'Frozen assets exceed the review-package budget')
            actual[name] = file_digest(path)
    require(actual == expected, 'Frozen binary/static assets differ from the verified package')
    migrations = {}
    with tarfile.open(source_archive, 'r:gz') as source:
        entries = archive_members(source)
        require(source.pax_headers.get('comment') == build['commit'],
                'Source archive commit differs from BUILD.txt')
        for name, entry in entries.items():
            match = re.fullmatch(r'migrations/(\d{4})_[^/]+\.sql', name)
            if match:
                version = int(match[1])
                require(entry.isfile() and version not in migrations, 'Invalid source migration entry')
                migrations[version] = bytes.fromhex(digest(source.extractfile(entry), 'sha384'))
    require(set(migrations) == set(range(1, int(build['schema_version']) + 1)),
            'Source migration versions differ from BUILD.txt')
    return build, expected, migrations


def verify_database(data, migrations):
    owned(data, directory=True, private=True)
    path = safe_path(data / 'vela.db')
    owned(path)
    # Do not use immutable=1: that would ignore committed live WAL transactions.
    with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True, timeout=3)) as db:
        deadline = time.monotonic() + 15
        db.set_progress_handler(lambda: int(time.monotonic() > deadline), 10000)
        db.execute('PRAGMA query_only=ON')
        db.execute('BEGIN')
        require(db.execute('PRAGMA quick_check').fetchall() == [('ok',)],
                'Database quick_check failed')
        require(not db.execute('PRAGMA foreign_key_check').fetchone(),
                'Database has a foreign-key violation')
        rows = db.execute('SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version').fetchall()
        require(len(rows) == len(migrations) and {row[0] for row in rows} == set(migrations) and
                all(success == 1 and migrations.get(version) == checksum for version, success, checksum in rows),
                'Database schema/checksums differ from the frozen build; migrate separately after backup')


def quote(value):
    """systemd quoted-list syntax; literal percent signs escape specifiers."""
    value = str(value)
    require(not any(ord(c) < 32 or ord(c) == 127 for c in value), 'Unsafe unit value')
    value = value.replace('\\', '\\\\').replace('"', '\\"').replace('%', '%%')
    return '"' + value + '"'


def environment(runtime, data, port):
    return {'NO_COLOR': '1', 'VELA_BIND': f'127.0.0.1:{port}',
            'VELA_DATA_DIR': str(data), 'VELA_WEB_DIR': str(runtime / 'dist')}


def render_unit(runtime, data, port):
    values = {**environment(runtime, data, port), **DEFAULTS}
    env = '\n'.join('Environment=' + quote(key + '=' + value) for key, value in values.items())
    return (MARKER + '[Unit]\nDescription=Vela private local music\n'
            'StartLimitIntervalSec=60\nStartLimitBurst=3\n\n[Service]\nType=exec\n'
            f'WorkingDirectory=/\nExecStart=:{quote(runtime / "vela-server")}\n'
            f'{env}\nUnsetEnvironment=VELA_SCAN_ROOTS\nUMask=0077\nNoNewPrivileges=yes\n'
            'Restart=on-failure\nRestartSec=2\nTimeoutStopSec=60\nKillMode=control-group\n'
            '\n[Install]\nWantedBy=default.target\n')


def run(arguments, *, acceptable=(0,), timeout=75):
    result = subprocess.run(arguments, check=False, capture_output=True, text=True, timeout=timeout)
    require(result.returncode in acceptable,
            f'{arguments[0]} {arguments[2] if len(arguments) > 2 else ""} failed (exit {result.returncode})')
    return result.stdout


def control(*arguments, acceptable=(0,)):
    return run(['systemctl', '--user', *arguments], acceptable=acceptable)


def unit_state(unit):
    output = control('show', '--no-pager', '--property=' + ','.join(PROPERTIES), unit,
                     acceptable=(0, 4))
    fields = dict(line.split('=', 1) for line in output.splitlines() if '=' in line)
    require(fields.get('LoadState') in ('loaded', 'not-found'), 'Unit is masked or could not be loaded')
    require(not fields.get('DropInPaths'), 'Unit drop-ins require manual review; refusing to replace them')
    require(fields.get('Id', unit) == unit, 'Unit aliases are unsupported')
    return fields


def process_matches(pid, runtime, data, port):
    process = Path('/proc') / str(pid)
    require(process.stat().st_uid == os.getuid(), 'Service process is not owned by this account')
    require(os.readlink(process / 'exe') == str(runtime / 'vela-server'),
            'Running executable differs from the frozen binary')
    require((process / 'cmdline').read_bytes().split(b'\0') ==
            [os.fsencode(runtime / 'vela-server'), b''], 'Service has unexpected executable arguments')
    values = dict(item.split('=', 1) for item in
                  (process / 'environ').read_bytes().decode().split('\0') if '=' in item)
    expected = environment(runtime, data, port)
    require(all(values.get(key) == value for key, value in expected.items()),
            'Running process uses different data, assets or bind settings')
    allowed = {**expected, **DEFAULTS}
    require(all(key in allowed and allowed[key] == value for key, value in values.items()
                if key.startswith('VELA_')), 'Custom VELA settings require a manually maintained service')
    return values


def request(port, path, limit):
    # http.client avoids proxy environment variables and never follows redirects.
    connection = http.client.HTTPConnection('127.0.0.1', port, timeout=2)
    try:
        connection.request('GET', path, headers={'Connection': 'close'})
        response = connection.getresponse()
        body = response.read(limit + 1)
        require(response.status == 200 and len(body) <= limit, 'Local readiness response is invalid')
        return body
    finally:
        connection.close()


def check_health(port, version, assets):
    health = json.loads(request(port, '/health', 4096))
    require(health == {'status': 'ok', 'version': version}, 'Local health/version does not match BUILD.txt')
    for name in sorted(REQUIRED):
        raw = request(port, '/' if name == 'index.html' else '/' + name, 64 * 1024 * 1024)
        require(hashlib.sha256(raw).hexdigest() == assets['dist/' + name],
                f'Served static asset differs from the frozen package: {name}')


def port_free(port):
    with socket.socket() as reservation:
        reservation.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            reservation.bind(('127.0.0.1', port))
        except OSError as error:
            raise ValueError('Loopback port is already in use by an unverified process') from error


def data_unlocked(data):
    lock = data / '.vela.lock'
    if lock.exists():
        owned(safe_path(lock))
        with lock.open('rb') as handle:
            try:
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as error:
                raise ValueError('Data directory is owned by another process') from error


def transient_snapshot(state, unit, runtime, data, port):
    """Only the small, known local-review unit format is automatically restorable."""
    fragment = safe_path(state['FragmentPath'])
    require(fragment == Path(f'/run/user/{os.getuid()}/systemd/transient') / unit,
            'Unexpected transient-unit location')
    owned(fragment)
    raw = fragment.read_text()
    allowed = {'Unit': {'Description', 'StartLimitIntervalSec', 'StartLimitBurst'},
               'Service': {'Type', 'Restart', 'RestartSec', 'TimeoutStopSec', 'UMask',
                           'WorkingDirectory', 'Environment', 'ExecStart'}}
    section = None
    for line in raw.splitlines():
        if not line or line.startswith('#'):
            continue
        if line.startswith('[') and line.endswith(']'):
            section = line[1:-1]
        else:
            key, separator, _ = line.partition('=')
            require(separator and key in allowed.get(section, set()),
                    'Transient unit has unsupported settings; preserve it manually')
    require(state['Type'] in ('simple', 'exec') and state['Restart'] == 'on-failure' and
            state['KillMode'] == 'control-group' and state['KillSignal'] == '15',
            'Transient supervision settings require manual review')
    values = dict(item.split('=', 1) for item in shlex.split(state['Environment']))
    expected = environment(runtime, data, port)
    require(values == expected, 'Transient environment is not the supported local-review configuration')
    cwd = safe_path(state['WorkingDirectory'])
    return {'raw': raw, 'environment': values, 'cwd': str(cwd), 'state': state}


def preflight(args, unit_dir):
    require(os.getuid() != 0, 'Run as the ordinary Vela owner, never root or sudo')
    require(re.fullmatch(r'vela(?:-[A-Za-z0-9_-]+)?\.service', args.unit), 'Use a plain vela*.service unit name')
    require(1024 <= args.port <= 65535, 'Port must be in 1024..65535; bind is always 127.0.0.1')
    runtime, data = safe_path(args.runtime_dir), safe_path(args.data_dir)
    archive, source = safe_path(args.archive), safe_path(args.source_archive)
    manifest = safe_path(args.checksums)
    build, assets, migrations = verify_package(runtime, archive, source, manifest)
    verify_database(data, migrations)
    unit_file = safe_path(unit_dir / args.unit, exists=False)
    unit_text = render_unit(runtime, data, args.port)
    old_bytes, old_mode = None, None
    if unit_file.exists():
        owned(unit_file)
        old_bytes, old_mode = unit_file.read_bytes(), stat.S_IMODE(unit_file.stat().st_mode)
        require(old_bytes == unit_text.encode(), 'Refusing to overwrite an unrelated or edited unit file')
        require(old_mode == 0o600, 'Managed user unit must retain private 0600 permissions')
    if unit_dir.exists():
        owned(unit_dir, directory=True)
    link = safe_path(unit_dir / 'default.target.wants', exists=False) / args.unit
    if link.is_symlink():
        require(link.resolve() == unit_file, 'Existing enablement link points to another unit')
    else:
        require(not link.exists(), 'Existing enablement path is not a service symlink')
    state = unit_state(args.unit)
    active = state.get('ActiveState') == 'active'
    require(state.get('ActiveState') in ('active', 'inactive', 'failed', None),
            'Unit is in transition; retry after it settles')
    transient = state.get('Transient') == 'yes'
    snapshot = None
    if state['LoadState'] != 'not-found':
        if transient:
            require(args.replace_transient, 'Existing transient unit requires explicit --replace-transient')
            require(active and old_bytes is None, 'Transient unit must be active with no persistent shadow file')
            require('--expand-environment=' in run(['systemd-run', '--help'], timeout=10),
                    'Transient rollback requires systemd-run with --expand-environment (systemd 254+)')
            snapshot = transient_snapshot(state, args.unit, runtime, data, args.port)
        else:
            require(old_bytes is not None and state['FragmentPath'] == str(unit_file),
                    'Refusing to replace a unit from another location')
            require(state.get('UnitFileState') in ('enabled', 'disabled'), 'Unsupported unit enablement state')
    if active:
        require(state.get('SubState') == 'running' and int(state.get('MainPID', '0')) > 0,
                'Service is not running normally')
        process_matches(int(state['MainPID']), runtime, data, args.port)
        check_health(args.port, build['version'], assets)
    else:
        port_free(args.port)
        data_unlocked(data)
    return {'runtime': runtime, 'data': data, 'build': build, 'assets': assets,
            'migrations': migrations, 'file': unit_file, 'text': unit_text,
            'old_bytes': old_bytes, 'old_mode': old_mode, 'old_active': active,
            'old_link': link.is_symlink(), 'link': link, 'transient': snapshot}


def atomic_write(path, content, mode=0o600):
    descriptor, temporary = tempfile.mkstemp(prefix='.' + path.name + '.', dir=path.parent)
    try:
        with os.fdopen(descriptor, 'wb') as output:
            os.fchmod(output.fileno(), mode)
            output.write(content)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


@contextmanager
def install_lock(unit_dir):
    # Validation never reaches this function; installation is explicitly mutating.
    unit_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    for path in (unit_dir.parent.parent, unit_dir.parent, unit_dir):
        safe_path(path)
        owned(path, directory=True)
    descriptor = os.open(unit_dir / '.vela-install.lock', os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    try:
        info = os.fstat(descriptor)
        require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and not info.st_mode & 0o077,
                'Unsafe installer lock file')
        try:
            fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise ValueError('Another Vela installer is running') from error
        yield
    finally:
        os.close(descriptor)


def wait_ready(args, plan, timeout=20):
    deadline = time.monotonic() + timeout
    while True:
        try:
            state = unit_state(args.unit)
            require(state.get('ActiveState') == 'active' and state.get('SubState') == 'running',
                    'Service is not active/running')
            process_matches(int(state.get('MainPID', '0')), plan['runtime'], plan['data'], args.port)
            check_health(args.port, plan['build']['version'], plan['assets'])
            verify_database(plan['data'], plan['migrations'])
            return
        except (ValueError, OSError, json.JSONDecodeError, http.client.HTTPException, sqlite3.Error):
            if time.monotonic() >= deadline:
                raise ValueError('Service failed bounded executable/health/static/schema readiness')
            time.sleep(0.2)


def restart_transient(args, plan):
    old = plan['transient']
    state = old['state']
    current = unit_state(args.unit)
    if current.get('Transient') == 'yes' and current.get('LoadState') == 'loaded':
        control('start', args.unit)
        return
    arguments = ['systemd-run', '--user', '--quiet', '--unit=' + args.unit,
                 '--expand-environment=no', '--description=' + state['Description'],
                 '--working-directory=' + old['cwd']]
    for key, field in [('Type', 'Type'), ('Restart', 'Restart'), ('RestartSec', 'RestartUSec'),
                       ('TimeoutStopSec', 'TimeoutStopUSec'), ('StartLimitIntervalSec', 'StartLimitIntervalUSec'),
                       ('StartLimitBurst', 'StartLimitBurst'), ('UMask', 'UMask')]:
        arguments.append('--property=' + key + '=' + state[field])
    arguments.extend('--setenv=' + key + '=' + value for key, value in old['environment'].items())
    arguments.extend(['--', str(plan['runtime'] / 'vela-server')])
    run(arguments)


def install(args, plan):
    """Restore the previous config/enablement/service after any failed step."""
    backup = None
    previous = plan['old_bytes']
    if plan['transient']:
        previous = plan['transient']['raw'].encode()
    if previous is not None:
        descriptor, name = tempfile.mkstemp(prefix=args.unit + '.before-', dir=plan['file'].parent)
        backup = Path(name)
        with os.fdopen(descriptor, 'wb') as output:
            os.fchmod(output.fileno(), 0o600)
            output.write(previous)
            output.flush()
            os.fsync(output.fileno())
    wrote, stopped = False, False
    try:
        if plan['old_active']:
            stopped = True
            control('stop', args.unit)
            require(unit_state(args.unit).get('ActiveState') not in ('active', 'activating', 'deactivating'),
                    'Previous service did not stop')
        port_free(args.port)
        data_unlocked(plan['data'])
        # Recheck every frozen byte and migration checksum after stopping, before startup.
        current = verify_package(plan['runtime'], safe_path(args.archive),
                                 safe_path(args.source_archive), safe_path(args.checksums))
        require(current == (plan['build'], plan['assets'], plan['migrations']),
                'Runtime changed after preflight')
        verify_database(plan['data'], plan['migrations'])
        atomic_write(plan['file'], plan['text'].encode())
        wrote = True
        control('daemon-reload')
        loaded = unit_state(args.unit)
        require(loaded.get('Transient') != 'yes' and loaded.get('FragmentPath') == str(plan['file']),
                'User manager did not load the persistent unit')
        control('start', args.unit)
        wait_ready(args, plan)
        control('enable', args.unit)
        require(plan['link'].is_symlink() and plan['link'].resolve() == plan['file'],
                'Persistent startup link was not created')
    except BaseException as failure:
        errors = []
        def attempt(action):
            try:
                action()
            except Exception as error:
                errors.append(str(error))
        if wrote:
            attempt(lambda: control('stop', args.unit))
            if not plan['old_link'] and plan['link'].is_symlink() and plan['link'].resolve() == plan['file']:
                attempt(plan['link'].unlink)
            if plan['old_bytes'] is None:
                attempt(lambda: plan['file'].unlink(missing_ok=True))
            else:
                attempt(lambda: atomic_write(plan['file'], plan['old_bytes'], plan['old_mode']))
            attempt(lambda: control('daemon-reload'))
        if stopped:
            attempt(lambda: restart_transient(args, plan) if plan['transient'] else control('start', args.unit))
            attempt(lambda: wait_ready(args, plan))
        detail = '; rollback errors: ' + '; '.join(errors) if errors else '; previous service/configuration restored'
        raise ValueError(str(failure) + detail + (f'; private previous config: {backup}' if backup else '')) from failure
    return backup


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-dir', required=True, type=Path, help='Existing frozen package directory')
    parser.add_argument('--data-dir', required=True, type=Path, help='Original existing 0700 data directory')
    parser.add_argument('--archive', required=True, type=Path, help='Matching native review .tar.gz')
    parser.add_argument('--source-archive', required=True, type=Path, help='Matching git-archive source .tar.gz')
    parser.add_argument('--checksums', required=True, type=Path, help='Matching local release SHA256SUMS')
    parser.add_argument('--unit', default='vela-local-development.service')
    parser.add_argument('--port', default=3210, type=int, help='Unprivileged loopback port (default: 3210)')
    parser.add_argument('--replace-transient', action='store_true', help='Permit verified same-build transient-unit handover')
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--install', action='store_true', help='Explicitly write, start and enable the persistent user unit')
    mode.add_argument('--dry-run', action='store_true', help='Validate only (the default); never write or restart anything')
    parser.add_argument('--print-unit', action='store_true', help='Also print the validated unit definition')
    args = parser.parse_args(argv)
    home = Path(pwd.getpwuid(os.getuid()).pw_dir)
    config = Path(os.environ.get('XDG_CONFIG_HOME', home / '.config'))
    require(config.is_absolute(), 'XDG_CONFIG_HOME must be absolute')
    unit_dir = safe_path(config / 'systemd/user', exists=False)
    plan = preflight(args, unit_dir)
    if args.print_unit:
        print(plan['text'], end='')
    if args.install:
        with install_lock(unit_dir):
            plan = preflight(args, unit_dir)
            backup = install(args, plan)
        print(f'Installed/enabled {args.unit}: 127.0.0.1:{args.port}, build {plan["build"]["version"]}, schema {len(plan["migrations"])}.')
        if backup:
            print(f'Private previous unit configuration: {backup}')
        print('Starts with the user manager, normally at login. No linger or boot-without-login setting was changed.')
    else:
        print(f'Validated only: build {plan["build"]["version"]}, schema {len(plan["migrations"])}, '
              f'{len(plan["assets"])} frozen binary/static files; no service/configuration changes requested.')
        print(f'Would install {plan["file"]}; add --install to write/start/enable it.')
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (ValueError, OSError, sqlite3.Error, tarfile.TarError, UnicodeError,
            subprocess.SubprocessError, http.client.HTTPException) as error:
        print(f'Vela user-service validation/install failed: {error}', file=sys.stderr)
        sys.exit(1)
