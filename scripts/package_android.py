#!/usr/bin/env python3
"""Build the Dioxus Android application, verify its APK signature and record it."""
from __future__ import annotations

import base64
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

from release_build import Artifact, describe, git, require_clean_tree, sha256, workspace_version, write_sidecar
from verify_release_artifacts import check_archive

ROOT = Path(__file__).resolve().parent.parent


def main() -> None:
    commit = require_clean_tree()
    version = workspace_version()
    ndk = Path(os.environ['ANDROID_NDK_HOME']) / 'toolchains/llvm/prebuilt/linux-x86_64/bin'
    env = dict(os.environ, CARGO_BUILD_JOBS='2')
    env.update(CC_aarch64_linux_android=str(ndk / 'aarch64-linux-android26-clang'),
               CXX_aarch64_linux_android=str(ndk / 'aarch64-linux-android26-clang++'),
               AR_aarch64_linux_android=str(ndk / 'llvm-ar'),
               CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=str(ndk / 'aarch64-linux-android26-clang'))
    env['PATH'] = str(ndk) + os.pathsep + env['PATH']
    names = ('ANDROID_KEYSTORE_BASE64', 'ANDROID_KEYSTORE_PASSWORD', 'ANDROID_KEY_ALIAS', 'ANDROID_KEY_PASSWORD')
    supplied = [bool(os.environ.get(name)) for name in names]
    if any(supplied) and not all(supplied):
        raise SystemExit('Android signing configuration is incomplete')
    signed_release = all(supplied)
    config = ROOT / 'apps/desktop/Dioxus.toml'
    original = config.read_bytes()
    output = ROOT / 'target/release-artifacts'
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='vela-android-') as directory:
        stage = Path(directory)
        try:
            if signed_release:
                key = stage / 'release.jks'
                key.write_bytes(base64.b64decode(os.environ[names[0]], validate=True))
                key.chmod(0o600)
                settings = {'jks_file': str(key), 'jks_password': os.environ[names[1]],
                            'key_alias': os.environ[names[2]], 'key_password': os.environ[names[3]]}
                config.write_text(original.decode() + '\n[bundle.android]\n' + ''.join(f'{k} = {json.dumps(v)}\n' for k, v in settings.items()))
            subprocess.run(['dx', 'bundle', '--android', '--release', '--package', 'vela-desktop',
                            '--bin', 'vela-mobile', '--features', 'mobile', '--target', 'aarch64-linux-android',
                            '--package-types', 'apk', '--out-dir', str(stage / 'apk'), '--locked'],
                           cwd=ROOT / 'apps/desktop', env=env, check=True)
        finally:
            config.write_bytes(original)
        apks = list((stage / 'apk').glob('*.apk'))
        if len(apks) != 1:
            raise SystemExit(f'expected one APK, found {len(apks)}')
        check_archive(apks[0], ['AndroidManifest.xml', 'classes.dex', 'lib/arm64-v8a/libmain.so'])
        subprocess.run(['apksigner', 'verify', '--verbose', str(apks[0])], check=True)
        subprocess.run(['aapt', 'dump', 'badging', str(apks[0])], check=True)
        suffix = '' if signed_release else '-preview'
        apk = output / f'vela-{version}-android-arm64{suffix}.apk'
        shutil.copy2(apks[0], apk)
    write_sidecar(apk)
    artifact = Artifact('android-arm64', apk, 'apk', 'android', 'aarch64', 'github-release',
                        sha256(apk), apk.stat().st_size, {'signing': 'release' if signed_release else 'ephemeral-preview'})
    (output / 'release-report-android.json').write_text(json.dumps(
        {'version': version, 'commit': commit, 'artifacts': [describe(artifact)]}, indent=2) + '\n')
    print(f'Verified APK: {apk.name}; signing={artifact.extra["signing"]}')


if __name__ == '__main__':
    main()
