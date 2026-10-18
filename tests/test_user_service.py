"""Real package/SQLite/HTTP validation plus isolated systemd-transaction regressions.

These tests never call a mutating systemctl command or run a Vela server.
"""
from contextlib import closing, redirect_stdout
import hashlib
import http.server
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import socket
import sqlite3
import subprocess
import tarfile
import tempfile
import threading
import types
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('vela_user_service', ROOT / 'scripts/install-user-service.py')
service = importlib.util.module_from_spec(spec)
spec.loader.exec_module(service)


class UserService(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='vela-service-test-')
        self.root = Path(self.temporary.name)
        self.runtime = self.root / 'frozen 100% $HOME'
        self.runtime.mkdir(mode=0o700)
        self.data = self.root / 'original data "quoted" \\ literal'
        self.data.mkdir(mode=0o700)
        self.units = self.root / 'config/systemd/user'
        self.archive = self.root / 'vela-native.tar.gz'
        self.source = self.root / 'vela-source.tar.gz'
        self.manifest = self.root / 'SHA256SUMS'
        self.build = ('version=0.10.0\ntarget=' + service.platform.machine() +
                      '-unknown-linux-gnu\ncommit=' + 'a' * 40 + '\nschema_version=2\n').encode()
        (self.runtime / 'BUILD.txt').write_bytes(self.build)
        (self.runtime / 'vela-server').write_bytes(b'isolated executable fixture')
        (self.runtime / 'vela-server').chmod(0o700)
        for name in service.REQUIRED:
            path = self.runtime / 'dist' / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(('fixture:' + name).encode())
        self.sql = {1: b'CREATE TABLE private_user(value TEXT);',
                    2: b"INSERT INTO private_user VALUES('preserve this original');"}
        self.migrations = {version: hashlib.sha384(sql).digest() for version, sql in self.sql.items()}
        with closing(sqlite3.connect(self.data / 'vela.db')) as db, db:
            db.execute('CREATE TABLE _sqlx_migrations(version INTEGER PRIMARY KEY, success INTEGER, checksum BLOB)')
            for version, sql in self.sql.items():
                db.executescript(sql.decode())
                db.execute('INSERT INTO _sqlx_migrations VALUES(?,1,?)', (version, self.migrations[version]))
        self.repackage()
        with socket.socket() as reservation:
            reservation.bind(('127.0.0.1', 0))
            port = reservation.getsockname()[1]
        self.args = types.SimpleNamespace(runtime_dir=self.runtime, data_dir=self.data,
                                         archive=self.archive, source_archive=self.source,
                                         checksums=self.manifest, unit='vela-test.service',
                                         port=port, replace_transient=False)
        self.absent = {'Id': self.args.unit, 'LoadState': 'not-found', 'ActiveState': 'inactive'}

    def tearDown(self):
        self.temporary.cleanup()

    def repackage(self, extra=None):
        with tarfile.open(self.archive, 'w:gz') as archive:
            archive.add(self.runtime / 'BUILD.txt', arcname='BUILD.txt')
            archive.add(self.runtime / 'vela-server', arcname='vela-server')
            archive.add(self.runtime / 'dist', arcname='dist')
            if extra:
                archive.addfile(*extra)
        with tarfile.open(self.source, 'w:gz', format=tarfile.PAX_FORMAT,
                          pax_headers={'comment': 'a' * 40}) as archive:
            for version, sql in self.sql.items():
                info = tarfile.TarInfo(f'migrations/{version:04}_fixture.sql')
                info.size = len(sql)
                archive.addfile(info, io.BytesIO(sql))
        self.manifest.write_text(''.join(hashlib.sha256(path.read_bytes()).hexdigest() +
                                         '  ' + path.name + '\n' for path in [self.archive, self.source]))

    def plan(self):
        with patch.object(service, 'unit_state', return_value=self.absent):
            return service.preflight(self.args, self.units)

    def cli_arguments(self):
        return ['--runtime-dir', str(self.runtime), '--data-dir', str(self.data),
                '--archive', str(self.archive), '--source-archive', str(self.source),
                '--checksums', str(self.manifest), '--unit', self.args.unit,
                '--port', str(self.args.port)]

    def test_default_validation_preserves_original_and_writes_no_configuration(self):
        before = (self.data / 'vela.db').read_bytes()
        with patch.object(service.pwd, 'getpwuid', return_value=types.SimpleNamespace(pw_dir=str(self.root))), \
                patch.dict(os.environ, {'XDG_CONFIG_HOME': str(self.root / 'config')}), \
                patch.object(service, 'unit_state', return_value=self.absent), \
                patch.object(service, 'control', side_effect=AssertionError('No systemctl mutation')), \
                redirect_stdout(io.StringIO()) as output:
            self.assertEqual(service.main(self.cli_arguments()), 0)
        self.assertIn('Validated only', output.getvalue())
        self.assertFalse(self.units.exists())
        self.assertEqual((self.data / 'vela.db').read_bytes(), before)

    def test_frozen_assets_archive_hash_and_source_commit_are_independent_gates(self):
        self.plan()
        asset = self.runtime / 'dist/player.js'
        original = asset.read_bytes()
        asset.write_bytes(b'changed after package')
        with self.assertRaisesRegex(ValueError, 'binary/static assets differ'):
            self.plan()
        asset.write_bytes(original)
        (self.runtime / 'dist/unpackaged.js').write_bytes(b'extra')
        with self.assertRaisesRegex(ValueError, 'binary/static assets differ'):
            self.plan()
        (self.runtime / 'dist/unpackaged.js').unlink()
        self.manifest.write_text('0' * 64 + '  ' + self.archive.name + '\n')
        with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
            self.plan()
        self.repackage()
        (self.runtime / 'BUILD.txt').write_bytes(self.build.replace(b'a' * 40, b'b' * 40))
        # Keep native metadata consistent while source provenance remains different.
        self.repackage()
        with self.assertRaisesRegex(ValueError, 'Source archive commit'):
            self.plan()

    def test_rejects_symlink_and_traversal_package_members_without_extraction(self):
        for name, kind in [('../escape', tarfile.REGTYPE), ('dist/link', tarfile.SYMTYPE)]:
            info = tarfile.TarInfo(name)
            info.type = kind
            info.linkname = '/etc/passwd' if kind == tarfile.SYMTYPE else ''
            self.repackage((info, io.BytesIO(b'')))
            with self.assertRaisesRegex(ValueError, 'Unsafe archive'):
                self.plan()
        self.repackage()
        target = self.runtime / 'dist/player.js'
        target.unlink()
        target.symlink_to(self.data / 'vela.db')
        with self.assertRaisesRegex(ValueError, 'symlink'):
            self.plan()
        self.assertFalse((self.root / 'escape').exists())

    def test_database_gate_reads_committed_wal_and_rejects_schema_changes(self):
        with closing(sqlite3.connect(self.data / 'vela.db')) as writer:
            writer.execute('PRAGMA journal_mode=WAL')
            service.verify_database(self.data, self.migrations)
            for statement in ["UPDATE _sqlx_migrations SET checksum=x'00' WHERE version=2",
                              'UPDATE _sqlx_migrations SET success=0 WHERE version=2',
                              'DELETE FROM _sqlx_migrations WHERE version=2',
                              "INSERT INTO _sqlx_migrations VALUES(3,1,x'00')"]:
                writer.execute(statement)
                writer.commit()
                with self.assertRaisesRegex(ValueError, 'schema/checksums differ'):
                    service.verify_database(self.data, self.migrations)
                writer.execute('DELETE FROM _sqlx_migrations')
                writer.executemany('INSERT INTO _sqlx_migrations VALUES(?,1,?)', self.migrations.items())
                writer.commit()
            self.assertEqual(writer.execute('SELECT value FROM private_user').fetchone(),
                             ('preserve this original',))
        missing = self.root / 'missing'
        missing.mkdir(mode=0o700)
        with self.assertRaisesRegex(ValueError, 'does not exist'):
            service.verify_database(missing, self.migrations)
        self.assertFalse((missing / 'vela.db').exists())

    def test_conflicts_and_data_lock_are_rejected_before_any_mutation(self):
        self.units.mkdir(parents=True)
        unit = self.units / self.args.unit
        unit.write_text('[Service]\nExecStart=/usr/bin/unrelated\n')
        with self.assertRaisesRegex(ValueError, 'unrelated or edited'):
            self.plan()
        unit.unlink()
        with patch.object(service, 'unit_state', return_value={**self.absent, 'LoadState': 'loaded',
                                                             'Transient': 'yes', 'ActiveState': 'active'}):
            with self.assertRaisesRegex(ValueError, '--replace-transient'):
                service.preflight(self.args, self.units)
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', self.args.port))
            listener.listen()
            with self.assertRaisesRegex(ValueError, 'already in use'):
                self.plan()
        lock = self.data / '.vela.lock'
        lock.touch(mode=0o600)
        with lock.open('rb') as handle:
            service.fcntl.flock(handle, service.fcntl.LOCK_EX | service.fcntl.LOCK_NB)
            with self.assertRaisesRegex(ValueError, 'owned by another process'):
                self.plan()
        for unsafe in ['vela-test.service\nExecStart=/bad', 'unrelated.service', '../vela.service']:
            self.args.unit = unsafe
            with self.assertRaisesRegex(ValueError, 'unit name'):
                self.plan()

    def test_loopback_health_checks_real_responses_and_served_asset_bytes(self):
        plan = self.plan()
        test = self
        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                if self.path == '/health':
                    body = json.dumps({'status': 'ok', 'version': '0.10.0'}).encode()
                else:
                    name = 'index.html' if self.path == '/' else self.path.removeprefix('/')
                    body = (test.runtime / 'dist' / name).read_bytes()
                self.send_response(200)
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *arguments):
                pass
        with http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler) as server:
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                service.check_health(server.server_port, '0.10.0', plan['assets'])
                with self.assertRaisesRegex(ValueError, 'health/version'):
                    service.check_health(server.server_port, '0.9.0', plan['assets'])
                (self.runtime / 'dist/player.js').write_bytes(b'wrong served bytes')
                with self.assertRaisesRegex(ValueError, 'Served static asset differs'):
                    service.check_health(server.server_port, '0.10.0', plan['assets'])
            finally:
                server.shutdown()
                thread.join()

    def test_unit_quoting_is_accepted_by_systemd_without_starting_a_service(self):
        unit = service.render_unit(self.runtime, self.data, self.args.port)
        self.assertIn('100%% $HOME', unit)
        self.assertIn('data \\"quoted\\" \\\\ literal', unit)
        self.assertIn('ExecStart=:"', unit)
        self.assertIn('WantedBy=default.target', unit)
        self.assertNotIn('User=', unit)
        if not shutil.which('systemd-analyze'):
            self.skipTest('systemd-analyze is unavailable')
        path = self.root / self.args.unit
        path.write_text(unit)
        result = subprocess.run(['systemd-analyze', '--user', 'verify', str(path)],
                                capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_successful_install_is_private_and_enables_only_after_readiness(self):
        plan = self.plan()
        self.units.mkdir(parents=True)
        calls = []
        def control(*arguments):
            calls.append(arguments)
            if arguments[0] == 'enable':
                plan['link'].parent.mkdir()
                plan['link'].symlink_to(plan['file'])
        with patch.object(service, 'control', side_effect=control), \
                patch.object(service, 'unit_state', return_value={'Transient': 'no', 'FragmentPath': str(plan['file'])}), \
                patch.object(service, 'wait_ready', side_effect=lambda *a: calls.append(('ready',))):
            self.assertIsNone(service.install(self.args, plan))
        self.assertEqual(calls, [('daemon-reload',), ('start', self.args.unit), ('ready',), ('enable', self.args.unit)])
        self.assertEqual(plan['file'].stat().st_mode & 0o777, 0o600)
        self.assertEqual(plan['file'].read_text(), plan['text'])

    def test_each_install_failure_restores_previous_unit_and_enablement(self):
        plan = self.plan()
        self.units.mkdir(parents=True)
        previous = plan['text'].encode()
        plan['file'].write_bytes(previous)
        plan['file'].chmod(0o600)
        plan.update(old_bytes=previous, old_mode=0o600, old_active=True)
        for failure in ['stop', 'write', 'daemon-reload', 'start', 'ready', 'enable']:
            calls, failed = [], False
            plan['file'].write_bytes(previous)
            def control(*arguments):
                nonlocal failed
                calls.append(arguments)
                if arguments[0] == 'enable' and not plan['link'].exists():
                    plan['link'].parent.mkdir(exist_ok=True)
                    plan['link'].symlink_to(plan['file'])
                if arguments[0] == failure and not failed:
                    failed = True
                    raise ValueError('injected ' + failure)
            def ready(*arguments):
                nonlocal failed
                if failure == 'ready' and not failed:
                    failed = True
                    raise ValueError('injected readiness')
            original_write = service.atomic_write
            def write(*arguments):
                nonlocal failed
                if failure == 'write' and not failed:
                    failed = True
                    raise ValueError('injected atomic write')
                return original_write(*arguments)
            with self.subTest(failure=failure), \
                    patch.object(service, 'control', side_effect=control), \
                    patch.object(service, 'atomic_write', side_effect=write), \
                    patch.object(service, 'unit_state', return_value={'ActiveState': 'inactive', 'Transient': 'no',
                                                                    'FragmentPath': str(plan['file'])}), \
                    patch.object(service, 'wait_ready', side_effect=ready):
                with self.assertRaisesRegex(ValueError, 'previous service/configuration restored'):
                    service.install(self.args, plan)
                self.assertEqual(plan['file'].read_bytes(), previous)
                self.assertEqual(plan['file'].stat().st_mode & 0o777, 0o600)
                self.assertFalse(plan['link'].exists())
                self.assertIn(('start', self.args.unit), calls)
        backups = list(self.units.glob(self.args.unit + '.before-*'))
        self.assertEqual(len(backups), 6)
        self.assertTrue(all(path.stat().st_mode & 0o777 == 0o600 for path in backups))

    def test_failed_install_preserves_preexisting_enablement_and_inactive_state(self):
        plan = self.plan()
        self.units.mkdir(parents=True)
        previous = plan['text'].encode()
        plan['file'].write_bytes(previous)
        plan['file'].chmod(0o600)
        plan['link'].parent.mkdir()
        plan['link'].symlink_to(plan['file'])
        plan.update(old_bytes=previous, old_mode=0o600, old_link=True)
        calls = []
        def control(*arguments):
            calls.append(arguments)
            if arguments[0] == 'start':
                raise ValueError('injected startup failure')
        with patch.object(service, 'control', side_effect=control), \
                patch.object(service, 'unit_state', return_value={'Transient': 'no', 'FragmentPath': str(plan['file'])}):
            with self.assertRaisesRegex(ValueError, 'previous service/configuration restored'):
                service.install(self.args, plan)
        self.assertEqual(plan['file'].read_bytes(), previous)
        self.assertEqual(plan['link'].resolve(), plan['file'])
        self.assertEqual(calls.count(('start', self.args.unit)), 1)

    def test_process_validation_requires_exact_executable_arguments_and_private_configuration(self):
        proc = self.root / 'proc'
        fake = proc / '12345'
        fake.mkdir(parents=True)
        (fake / 'exe').symlink_to(self.runtime / 'vela-server')
        (fake / 'cmdline').write_bytes(os.fsencode(self.runtime / 'vela-server') + b'\0')
        values = service.environment(self.runtime, self.data, self.args.port)
        def write_environment():
            (fake / 'environ').write_bytes('\0'.join(key + '=' + value for key, value in values.items()).encode() + b'\0')
        write_environment()
        original_path = service.Path
        with patch.object(service, 'Path', side_effect=lambda value: proc if value == '/proc' else original_path(value)):
            service.process_matches(12345, self.runtime, self.data, self.args.port)
            values['VELA_BIND'] = '0.0.0.0:3210'
            write_environment()
            with self.assertRaisesRegex(ValueError, 'different data, assets or bind'):
                service.process_matches(12345, self.runtime, self.data, self.args.port)
            values.update(service.environment(self.runtime, self.data, self.args.port))
            values['VELA_SCAN_ROOTS'] = '[]'
            write_environment()
            with self.assertRaisesRegex(ValueError, 'Custom VELA settings'):
                service.process_matches(12345, self.runtime, self.data, self.args.port)
            (fake / 'cmdline').write_bytes(os.fsencode(self.runtime / 'vela-server') + b'\0--other\0')
            with self.assertRaisesRegex(ValueError, 'unexpected executable arguments'):
                service.process_matches(12345, self.runtime, self.data, self.args.port)

    def test_new_install_failure_removes_candidate_and_surfaces_rollback_failure(self):
        plan = self.plan()
        self.units.mkdir(parents=True)
        def control(*arguments):
            if arguments[0] == 'start':
                raise ValueError('injected startup failure')
        with patch.object(service, 'control', side_effect=control), \
                patch.object(service, 'unit_state', return_value={'Transient': 'no', 'FragmentPath': str(plan['file'])}):
            with self.assertRaisesRegex(ValueError, 'previous service/configuration restored'):
                service.install(self.args, plan)
        self.assertFalse(plan['file'].exists())
        plan.update(old_active=True, old_bytes=plan['text'].encode(), old_mode=0o600)
        plan['file'].write_text(plan['text'])
        with patch.object(service, 'control', side_effect=control), \
                patch.object(service, 'unit_state', return_value={'ActiveState': 'inactive', 'Transient': 'no',
                                                                'FragmentPath': str(plan['file'])}), \
                patch.object(service, 'wait_ready', return_value=None):
            with self.assertRaisesRegex(ValueError, 'rollback errors:.*startup failure'):
                service.install(self.args, plan)

    def test_transient_rollback_uses_structured_arguments_without_shell(self):
        plan = self.plan()
        state = {'Description': 'old review', 'Type': 'simple', 'Restart': 'on-failure',
                 'RestartUSec': '2s', 'TimeoutStopUSec': '1min', 'StartLimitIntervalUSec': '1min',
                 'StartLimitBurst': '3', 'UMask': '0077'}
        plan['transient'] = {'state': state, 'cwd': str(self.root),
                             'environment': service.environment(self.runtime, self.data, self.args.port)}
        with patch.object(service, 'unit_state', return_value=self.absent), \
                patch.object(service, 'run') as run:
            service.restart_transient(self.args, plan)
        arguments = run.call_args.args[0]
        self.assertEqual(arguments[-2:], ['--', str(self.runtime / 'vela-server')])
        self.assertIn('--expand-environment=no', arguments)
        self.assertIn('--setenv=VELA_DATA_DIR=' + str(self.data), arguments)
        self.assertIn('--property=TimeoutStopSec=1min', arguments)


if __name__ == '__main__':
    unittest.main()
