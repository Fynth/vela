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


MAX_TEXT_BYTES = 1024 * 1024
# These exact crate packages place their full MIT grant and copyright in AUTHORS;
# their packaged README explicitly identifies AUTHORS as the license source.
REVIEWED_AUTHORS_LICENSES = {('r-efi', '5.3.0'), ('r-efi', '6.0.0')}


def shipped_license_files(package):
    root = Path(package['manifest_path']).parent
    files = set()
    if package.get('license_file'):
        files.add(root / package['license_file'])
    for path in root.iterdir():
        if path.is_file() and path.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'COPYRIGHT', 'NOTICE')):
            files.add(path)
        elif path.is_dir() and path.name.upper() in ('LICENSES', 'LICENCES'):
            files.update(child for child in path.iterdir() if child.is_file())
    if (package['name'], package['version']) in REVIEWED_AUTHORS_LICENSES:
        files.add(root / 'AUTHORS')
    return files


def cached_license(path):
    """Fail closed on missing provenance, oversized text or changed cached bytes."""
    with path.open('rb') as stream:
        data = stream.read(MAX_TEXT_BYTES + 1)
    if len(data) > MAX_TEXT_BYTES:
        raise ValueError(f'Cached license exceeds size limit: {path}')
    source = json.loads(path.with_name(path.name + '.source.json').read_text())
    if not isinstance(source.get('url'), str) or not source['url'].startswith('https://'):
        raise ValueError(f'Cached license needs an HTTPS source URL: {path}')
    if source.get('sha256') != hashlib.sha256(data).hexdigest():
        raise ValueError(f'Cached license checksum mismatch: {path}')
    return data.decode()


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
        files = shipped_license_files(package)
        found = False
        for path in sorted(files):
            try:
                if path.stat().st_size > MAX_TEXT_BYTES:
                    continue
                text = path.read_text()
            except (OSError, UnicodeDecodeError):
                continue
            label = str(path.relative_to(root)) if path.is_relative_to(root) else path.name
            sections.append(f'\n--- {label} ---\n{text}\n')
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
                                data = response.read(MAX_TEXT_BYTES + 1)
                            if len(data) > MAX_TEXT_BYTES: return None
                            data.decode()  # License text must be valid UTF-8.
                            return name, data
                        except (OSError, UnicodeDecodeError): return None
                    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
                        for result in executor.map(fetch, ['LICENSE-MIT', 'LICENSE-APACHE', 'LICENSE', 'LICENSE.md', 'LICENSE.txt', 'COPYING', 'LICENSE_APACHE_2_0.txt', 'LICENSE_BOOST_1_0.txt', 'LICENSE_MIT.txt']):
                            if result:
                                name, data = result
                                cache.mkdir(parents=True, exist_ok=True)
                                (cache / name).write_bytes(data)
                                (cache / (name + '.source.json')).write_text(json.dumps({'url': base+name, 'sha256': hashlib.sha256(data).hexdigest()}, indent=2)+'\n')
        if not found and cache.exists():
            for path in sorted(cache.iterdir()):
                if path.is_file() and not path.name.endswith('.json'):
                    sections.append(f'\n--- Upstream {path.name} (verified source URL/hash in repository) ---\n{cached_license(path)}\n')
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
