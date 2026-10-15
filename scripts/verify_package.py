#!/usr/bin/env python3
"""Start an extracted native review package using its bundled web assets and defaults."""
import argparse
import json
import os
from pathlib import Path, PurePosixPath
import signal
import socket
import sqlite3
import subprocess
import tarfile
import tempfile
import time
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='vela-package-check-') as temporary:
        root = Path(temporary)
        with tarfile.open(args.archive, 'r:gz') as archive:
            total = 0
            for count, entry in enumerate(archive, 1):
                name = PurePosixPath(entry.name)
                total += entry.size
                if count > 1000 or total > 128 * 1024 * 1024:
                    raise RuntimeError('Unexpected package size')
                if name.is_absolute() or '..' in name.parts or not (entry.isfile() or entry.isdir()):
                    raise RuntimeError('Unsafe package entry')
            archive.extractall(root, filter='data')
        for required in ['vela-server', 'dist/index.html', 'dist/pkg/vela_web_bg.wasm', 'BUILD.txt', 'LICENSE']:
            if not (root / required).is_file():
                raise RuntimeError('Required package entry missing: ' + required)
        build = dict(line.split('=', 1) for line in (root / 'BUILD.txt').read_text().splitlines())
        with socket.socket() as reservation:
            reservation.bind(('127.0.0.1', 0))
            port = reservation.getsockname()[1]
        # Remove inherited application settings to test the package's defaults.
        environment = {key: value for key, value in os.environ.items() if not key.startswith('VELA_')}
        environment.update(VELA_BIND=f'127.0.0.1:{port}', NO_COLOR='1')
        process = subprocess.Popen([str(root / 'vela-server')], cwd=root, env=environment,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        base = f'http://127.0.0.1:{port}'
        try:
            deadline = time.monotonic() + 10
            while True:
                try:
                    with urllib.request.urlopen(base + '/health', timeout=1) as response:
                        health = json.load(response)
                    break
                except OSError:
                    if process.poll() is not None or time.monotonic() > deadline:
                        raise RuntimeError('Packaged server failed readiness')
                    time.sleep(.05)
            assert health['version'] == build['version']
            if 'schema_version' in build:
                with sqlite3.connect(f'file:{root / "data/vela.db"}?mode=ro', uri=True) as database:
                    actual = database.execute('SELECT MAX(version) FROM _sqlx_migrations WHERE success=1').fetchone()[0]
                    assert actual == int(build['schema_version']), 'Packaged binary is missing an expected migration'
            with urllib.request.urlopen(base + '/api/setup', timeout=2) as response:
                assert json.load(response)['needs_setup'] is True
            for path, expected in [('/', b'bootstrap.js'), ('/bootstrap.js', b'vela_web.js')]:
                with urllib.request.urlopen(base + path, timeout=2) as response:
                    assert expected in response.read(1024 * 1024)
            with urllib.request.urlopen(base + '/pkg/vela_web_bg.wasm', timeout=2) as response:
                assert response.read(4) == b'\x00asm'
            process.send_signal(signal.SIGTERM)
            process.communicate(timeout=20)
            assert process.returncode == 0
            print(f"Package verified: version={build['version']} target={build['target']} commit={build['commit']}")
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()


if __name__ == '__main__':
    main()
