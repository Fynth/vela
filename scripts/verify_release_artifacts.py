#!/usr/bin/env python3
"""Verify reports, hashes and payloads; fail closed before release publication."""
from __future__ import annotations

import argparse
import json
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path

from release_build import sha256


def archive_names(path: Path) -> set[str]:
    if path.suffix in {'.zip', '.apk'}:
        with zipfile.ZipFile(path) as bundle:
            return set(bundle.namelist())
    with tarfile.open(path, 'r:gz') as bundle:
        return {name.removeprefix('./') for name in bundle.getnames()}


def check_archive(path: Path, required: list[str]) -> None:
    missing = set(required) - archive_names(path)
    if missing:
        raise SystemExit(f'{path.name}: missing {sorted(missing)}')


def check_build_metadata(path: Path, version: str, commit: str) -> None:
    if path.suffix == '.zip':
        with zipfile.ZipFile(path) as bundle:
            raw = bundle.read('BUILD.txt')
    else:
        with tarfile.open(path, 'r:gz') as bundle:
            member = next(m for m in bundle.getmembers() if m.name.removeprefix('./') == 'BUILD.txt')
            raw = bundle.extractfile(member).read()
    metadata = dict(line.split('=', 1) for line in raw.decode().splitlines() if '=' in line)
    if metadata.get('version') != version or metadata.get('commit') != commit:
        raise SystemExit(f'{path.name}: build version/commit mismatch')


def load_reports(root: Path) -> dict:
    paths = sorted(root.glob('release-report-*.json'))
    if not paths:
        paths = [root / 'release-report.json']
    reports = [json.loads(path.read_text()) for path in paths]
    if len({(r['version'], r['commit']) for r in reports}) != 1:
        raise SystemExit('release reports name different versions or commits')
    combined = dict(reports[0], artifacts=[])
    seen: set[str] = set()
    for report in reports:
        for artifact in report['artifacts']:
            name = artifact['file']
            if Path(name).name != name or name in seen:
                raise SystemExit(f'duplicate or unsafe artifact filename: {name}')
            seen.add(name)
            combined['artifacts'].append(artifact)
    if not seen:
        raise SystemExit('release contains no artifacts')
    return combined


def verify(root: Path, *, require_all: bool = False) -> dict:
    report = load_reports(root)
    for artifact in report['artifacts']:
        path = root / artifact['file']
        if not path.is_file() or sha256(path) != artifact['sha256'] or path.stat().st_size != artifact['size']:
            raise SystemExit(f'{path.name}: missing or checksum/size mismatch')
        sidecar = path.with_name(path.name + '.sha256')
        if sidecar.read_text().strip() != f"{artifact['sha256']}  {path.name}":
            raise SystemExit(f'{path.name}: checksum sidecar mismatch')
        kind = artifact['kind']
        if path.name.endswith('.tar.gz') or path.suffix == '.zip':
            required = {
                'server': ['vela-server', 'dist/index.html', 'dist/pkg/vela_web_bg.wasm', 'BUILD.txt'],
                'web': ['dist/index.html', 'dist/style.css', 'dist/pkg/vela_web_bg.wasm'],
                'aur': ['usr/bin/vela-desktop', 'usr/lib/vela/vela-desktop', 'usr/share/applications/vela-desktop.desktop', 'usr/share/icons/hicolor/scalable/apps/vela-desktop.svg'],
            }.get(kind, ['bin/vela-desktop.exe' if artifact['platform'] == 'windows' else 'bin/vela-desktop', 'BUILD.txt'])
            check_archive(path, required)
            if kind in {'server', 'desktop'}:
                check_build_metadata(path, report['version'], report['commit'])
        elif path.suffix == '.exe':
            with path.open('rb') as stream:
                if stream.read(2) != b'MZ':
                    raise SystemExit(f'{path.name}: invalid PE executable')
        elif path.suffix == '.deb':
            with tempfile.TemporaryDirectory() as directory:
                subprocess.run(['dpkg-deb', '--extract', str(path), directory], check=True)
                for name in ('usr/bin/vela-desktop', 'usr/lib/vela/vela-desktop', 'usr/share/doc/vela-desktop/BUILD.txt'):
                    if not (Path(directory) / name).is_file():
                        raise SystemExit(f'{path.name}: missing {name}')
        elif path.suffix == '.apk':
            check_archive(path, ['AndroidManifest.xml', 'classes.dex'])
            if not any(n.startswith('lib/arm64-v8a/') and n.endswith('.so') for n in archive_names(path)):
                raise SystemExit(f'{path.name}: missing arm64 native library')
        elif path.suffix == '.AppImage':
            with path.open('rb') as stream:
                header = stream.read(12)
            if header[:4] != b'\x7fELF' or header[8:11] != b'AI\x02':
                raise SystemExit(f'{path.name}: invalid type 2 AppImage')
        else:
            raise SystemExit(f'{path.name}: unsupported artifact format')
    if require_all:
        expected = {'server-linux-x86-64', 'web-linux-x86-64', 'desktop-linux-x86-64', 'desktop-linux-deb', 'desktop-linux-appimage', 'desktop-linux-aur', 'desktop-windows-x86-64', 'android-arm64'}
        missing = expected - {a['target'] for a in report['artifacts']}
        if missing:
            raise SystemExit(f'release is incomplete: {sorted(missing)}')
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts', type=Path, default=Path('target/release-artifacts'))
    parser.add_argument('--require-all', action='store_true')
    arguments = parser.parse_args()
    report = verify(arguments.artifacts, require_all=arguments.require_all)
    (arguments.artifacts / 'release-report.json').write_text(json.dumps(report, indent=2) + '\n')
    checksums = ''.join(f"{a['sha256']}  {a['file']}\n" for a in sorted(report['artifacts'], key=lambda a: a['file']))
    (arguments.artifacts / 'SHA256SUMS').write_text(checksums)
    print(f"Verified {len(report['artifacts'])} artifacts, version {report['version']}, commit {report['commit']}")
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
