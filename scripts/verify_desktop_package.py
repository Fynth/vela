#!/usr/bin/env python3
"""Validate a local portable Vela desktop archive; --launch opens a real window."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import signal
import subprocess
import tarfile
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('--launch', action='store_true', help='Launch the packaged binary and check its real window')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='vela-desktop-check-') as temporary:
        root = Path(temporary)
        with tarfile.open(args.archive, 'r:gz') as archive:
            entries = []
            total = 0
            for entry in archive:
                total += entry.size
                if len(entries) > 500 or total > 256 * 1024 * 1024:
                    raise RuntimeError('Unexpected desktop package size')
                entries.append(entry)
            expected = {'bin/vela-desktop', 'share/applications/vela-desktop.desktop',
                        'share/icons/hicolor/scalable/apps/vela-desktop.svg',
                        'share/doc/vela-desktop/README.md', 'LICENSE', 'BUILD.txt'}
            files = set()
            for entry in entries:
                name = PurePosixPath(entry.name)
                if name.is_absolute() or '..' in name.parts:
                    raise RuntimeError(f'Unsafe desktop package entry: {entry.name}')
                if entry.isdir():
                    continue
                if not entry.isfile():
                    raise RuntimeError(f'Unsupported desktop package entry: {entry.name}')
                files.add(str(name))
                target = root.joinpath(*name.parts)
                target.parent.mkdir(parents=True, exist_ok=True)
                with archive.extractfile(entry) as source, target.open('xb') as output:
                    output.write(source.read())
                os.chmod(target, entry.mode & 0o777)
        build = dict(line.split('=', 1) for line in (root / 'BUILD.txt').read_text().splitlines() if '=' in line)
        if files != expected:
            raise RuntimeError('Desktop package contents do not match the review manifest')
        if build.get('version') != '0.12.0':
            raise RuntimeError('Unexpected packaged desktop version')
        binary = root / 'bin/vela-desktop'
        if (binary.stat().st_mode & 0o777) != 0o755:
            raise RuntimeError('Packaged desktop binary is not executable')
        version = subprocess.check_output([str(binary), '--version'], text=True).strip()
        if version != f"vela-desktop {build['version']}":
            raise RuntimeError(f'Packaged version mismatch: {version}')
        missing = [line for line in subprocess.check_output(['ldd', str(binary)], text=True).splitlines() if 'not found' in line]
        if missing:
            raise RuntimeError(f'Packaged desktop has missing libraries: {missing}')
        desktop = (root / 'share/applications/vela-desktop.desktop').read_text()
        if 'Exec=vela-desktop' not in desktop or 'Icon=vela-desktop' not in desktop:
            raise RuntimeError('Desktop entry does not reference the packaged binary/icon')
        report = {'archive': str(args.archive.resolve()), 'sha256': hashlib.sha256(args.archive.read_bytes()).hexdigest(),
                  'version': build['version'], 'commit': build.get('commit'), 'files': sorted(files), 'ldd_ok': True}
        if args.launch:
            if not os.environ.get('DISPLAY') and not os.environ.get('WAYLAND_DISPLAY'):
                raise RuntimeError('--launch needs a graphical session')
            environment = {**os.environ, 'VELA_DESKTOP_SERVER_URL': 'http://127.0.0.1:9', 'GDK_BACKEND': 'x11'}
            process = subprocess.Popen([str(binary)], env=environment, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            window = None
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                found = subprocess.run(['xdotool', 'search', '--onlyvisible', '--name', '^Vela Music$'], capture_output=True, text=True)
                ids = [line for line in found.stdout.splitlines() if line.strip()]
                if ids:
                    window = ids[0]
                    break
                if process.poll() is not None:
                    raise RuntimeError('Packaged desktop exited before opening its window')
                time.sleep(.2)
            if window is None:
                process.terminate()
                raise RuntimeError('Packaged desktop window did not appear')
            screenshot = Path('/tmp') / 'vela-desktop-package-launch.png'
            subprocess.run(['import', '-window', window, str(screenshot)], check=True)
            process.send_signal(signal.SIGTERM)
            if process.wait(timeout=20) != 0:
                raise RuntimeError('Packaged desktop did not exit cleanly on SIGTERM')
            report['launched'] = True
            report['window_title'] = 'Vela Music'
            report['screenshot'] = str(screenshot)
            report['temporary_originals_after_exit'] = len(list(Path(tempfile.gettempdir()).glob('vela-native-*')))
        print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
