#!/usr/bin/env python3
"""Collect package metadata and shipped license texts for the resolved Cargo graph."""
import argparse
import json
from pathlib import Path
import subprocess
import re
import urllib.request
import concurrent.futures
import hashlib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('dist/THIRD_PARTY_NOTICES.txt'))
    parser.add_argument('--fetch-missing', action='store_true', help='Fetch public upstream license files at packaged VCS commits')
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--offline', '--format-version=1']))
    used = {node['id'] for node in metadata['resolve']['nodes']}
    sections = ['Vela Music — dependency notices\nGenerated from the resolved Cargo.lock graph.\nProject license: AGPL-3.0-or-later; see LICENSE.\n\nLicense expressions and upstream shipped texts are preserved below.\n']
    missing = []
    for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
        if package['id'] not in used or package['source'] is None:
            continue
        root = Path(package['manifest_path']).parent
        sections.append('\n' + '=' * 76 + f"\n{package['name']} {package['version']}\nLicense: {package.get('license') or 'See upstream license file'}\nRepository: {package.get('repository') or 'See package metadata'}\n")
        files = set()
        if package.get('license_file'):
            files.add(root / package['license_file'])
        for path in root.iterdir():
            if path.is_file() and path.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'COPYRIGHT', 'NOTICE')):
                files.add(path)
        found = False
        for path in sorted(files):
            try:
                if path.stat().st_size > 1024 * 1024:
                    continue
                text = path.read_text()
            except (OSError, UnicodeDecodeError):
                continue
            sections.append(f'\n--- {path.name} ---\n{text}\n')
            found = True
        cache = Path('docs/third-party-license-texts') / (package['name'] + '-' + package['version'])
        if not found and args.fetch_missing and not cache.exists():
            repository = re.fullmatch(r'https://github.com/([\w.-]+/[\w.-]+?)(?:\.git)?(?:/tree/[^?#]+)?/?', package.get('repository') or '')
            vcs = root / '.cargo_vcs_info.json'
            if repository and vcs.exists():
                commit = json.loads(vcs.read_text()).get('git', {}).get('sha1', '')
                if re.fullmatch(r'[a-f0-9]{40}', commit):
                    base = f'https://raw.githubusercontent.com/{repository[1]}/{commit}/'
                    def fetch(name):
                        try:
                            with urllib.request.urlopen(base + name, timeout=8) as response:
                                data = response.read(1024 * 1024 + 1)
                            if len(data) > 1024 * 1024: return None
                            return name, data.decode()
                        except (OSError, UnicodeDecodeError): return None
                    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
                        for result in executor.map(fetch, ['LICENSE-MIT', 'LICENSE-APACHE', 'LICENSE', 'LICENSE.md', 'LICENSE.txt', 'COPYING']):
                            if result:
                                name, text = result
                                cache.mkdir(parents=True, exist_ok=True)
                                (cache / name).write_text(text)
                                (cache / (name + '.source.json')).write_text(json.dumps({'url': base+name, 'sha256': hashlib.sha256(text.encode()).hexdigest()}, indent=2)+'\n')
        if not found and cache.exists():
            for path in sorted(cache.iterdir()):
                if path.is_file() and not path.name.endswith('.json'):
                    sections.append(f'\n--- Upstream {path.name} (recorded source URL/hash in repository) ---\n{path.read_text()}\n')
                    found = True
        if not found:
            missing.append(f"{package['name']} {package['version']}")
            sections.append('Upstream crate package did not include a standalone license text. Refer to the upstream repository and license expression.\n')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text('\n'.join(sections))
    print(f'Notices generated: {args.output}; {len(missing)} packages have metadata-only notices')
    if missing:
        print('Review before external distribution: ' + ', '.join(missing))


if __name__ == '__main__':
    main()
