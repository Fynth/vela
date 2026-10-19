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
import urllib.error


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
                    if int(build['schema_version']) >= 11:
                        assert database.execute('SELECT enabled,origin,revision FROM network_settings').fetchall() == [(0, None, 0)]
                        assert database.execute('SELECT COUNT(*) FROM publications').fetchone()[0] == 0
                if int(build['schema_version']) >= 11:
                    for route in ['/.well-known/vela', '/api/public/catalog', '/api/public/catalog?limit=invalid']:
                        try:
                            urllib.request.urlopen(base + route, timeout=2)
                        except urllib.error.HTTPError as error:
                            assert error.code == 404
                        else:
                            raise AssertionError('Packaged public catalog must remain disabled')
            with urllib.request.urlopen(base + '/api/setup', timeout=2) as response:
                assert json.load(response)['needs_setup'] is True
            for path, expected in [('/', b'bootstrap.js'), ('/bootstrap.js', b'vela_web.js')]:
                with urllib.request.urlopen(base + path, timeout=2) as response:
                    assert expected in response.read(1024 * 1024)
            if 'href="/now-playing.css"' in (root / 'dist/index.html').read_text():
                with urllib.request.urlopen(base + '/now-playing.css', timeout=2) as response:
                    assert response.headers.get_content_type() == 'text/css'
                    assert b'.now-playing-dialog' in response.read(1024 * 1024)
            # Every stylesheet the shipped index links must resolve, carry CSS
            # and contain its own component rules. A missing copy would leave
            # the packaged client unstyled instead of failing the check.
            for name, marker in [('home-editorial.css', b'.home-feature'),
                                 ('queue-drawer.css', b'.queue-drawer')]:
                packaged = root / 'dist' / name
                assert packaged.is_file(), 'Packaged stylesheet missing: ' + name
                with urllib.request.urlopen(base + '/' + name, timeout=2) as response:
                    assert response.headers.get_content_type() == 'text/css'
                    assert marker in response.read(1024 * 1024)
            # Shipped static assets must resolve from the exact allowlist and be
            # byte-identical to the packaged file. An unlisted neighbour must
            # stay denied instead of exposing a directory listing.
            for name, folder in [('vela-coast.png', 'brand'), ('vela-wave.png', 'brand'),
                                 ('manrope-latin.woff2', 'fonts'), ('manrope-cyrillic.woff2', 'fonts')]:
                packaged = root / 'dist' / 'assets' / folder / name
                assert packaged.is_file(), 'Packaged asset missing: ' + name
                with urllib.request.urlopen(base + f'/assets/{folder}/{name}', timeout=2) as response:
                    assert response.status == 200
                    served = response.read()
                assert served == packaged.read_bytes(), f'Packaged asset differs from source: {name}'
            with urllib.request.urlopen(base + '/assets/brand/vela-coast.png', timeout=2) as response:
                assert response.headers.get_content_type() == 'image/png'
            with urllib.request.urlopen(base + '/assets/fonts/manrope-latin.woff2', timeout=2) as response:
                assert response.headers.get_content_type() in ('font/woff2', 'application/octet-stream')
            for denied in ['/assets/brand/', '/assets/brand/SOURCE.md', '/assets/brand/../fonts/OFL.txt']:
                try:
                    urllib.request.urlopen(base + denied, timeout=2)
                except urllib.error.HTTPError as error:
                    assert error.code == 404
                else:
                    raise AssertionError('Unlisted static path must stay denied: ' + denied)
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
