#!/usr/bin/env python3
"""Build the Android application, sign and verify its APK, then record its hash."""
from __future__ import annotations

import base64
import json
import os
import secrets
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

from release_build import Artifact, describe, require_clean_tree, sha256, workspace_version, write_sidecar
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
    # The NDK names its compilers after the API level, while the crates that
    # compile C code look for the plain target name. The plain name is created
    # here as well, so a build outside the workflow finds the same tools.
    for prefix in ('aarch64-linux-android', 'armv7a-linux-androideabi', 'x86_64-linux-android'):
        for suffix in ('clang', 'clang++'):
            plain = ndk / f'{prefix}-{suffix}'
            versioned = ndk / f'{prefix}26-{suffix}'
            if not plain.exists() and versioned.exists():
                plain.symlink_to(versioned)
    names = ('ANDROID_KEYSTORE_BASE64', 'ANDROID_KEYSTORE_PASSWORD', 'ANDROID_KEY_ALIAS', 'ANDROID_KEY_PASSWORD')
    supplied = [bool(os.environ.get(name)) for name in names]
    if any(supplied) and not all(supplied):
        raise SystemExit('Android signing configuration is incomplete')
    signed_release = all(supplied)
    output = ROOT / 'target/release-artifacts'
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='vela-android-') as directory:
        stage = Path(directory)
        # Let the pinned CLI generate the WebView/JNI project and Rust shared
        # library. Signing is separate: credentials never enter Dioxus/Gradle.
        subprocess.run(['dx', 'bundle', '--android', '--release', '--package', 'vela-desktop',
                        '--bin', 'vela-mobile', '--features', 'mobile', '--target', 'aarch64-linux-android',
                        '--package-types', 'apk', '--out-dir', str(stage / 'apk'), '--locked'],
                       cwd=ROOT / 'apps/desktop', env=env, check=True)
        project = ROOT / 'target/dx/vela-mobile/release/android/app'
        gradle = project / 'app/build.gradle.kts'
        if not gradle.is_file():
            raise SystemExit('Dioxus did not generate the expected Android application project')
        major, minor, patch = map(int, version.split('.'))
        version_code = major * 1_000_000 + minor * 1_000 + patch
        text = gradle.read_text()
        if 'versionCode = 1' not in text or not 0 < version_code <= 2_100_000_000:
            raise SystemExit('Unexpected Android versionCode template or version')
        gradle.write_text(text.replace('versionCode = 1', f'versionCode = {version_code}', 1))
        # JNI and JavaScript bridge class names are referenced from native code.
        with (project / 'app/proguard-rules.pro').open('a') as rules:
            rules.write('\n-keep class dev.dioxus.** { *; }\n')
        strings = project / 'app/src/main/res/values/strings.xml'
        xml = ET.parse(strings)
        label = xml.getroot().find("string[@name='app_name']")
        if label is None:
            raise SystemExit('Android app label resource is missing')
        label.text = 'Vela Music'
        xml.write(strings, encoding='utf-8', xml_declaration=True)
        # CLI 0.7.10 falls back to assembleDebug without inline signing settings.
        # Build a non-debuggable release from the generated JNI/assets instead.
        subprocess.run(['bash', 'gradlew', '--no-daemon', '--max-workers=2', 'assembleRelease'],
                       cwd=project, env=env, check=True)
        unsigned = project / 'app/build/outputs/apk/release/app-release-unsigned.apk'
        if not unsigned.is_file():
            raise SystemExit('Gradle did not produce the unsigned release APK')
        key = stage / 'signing.jks'
        if signed_release:
            key.write_bytes(base64.b64decode(os.environ[names[0]], validate=True))
            alias = os.environ[names[2]]
        else:
            alias = 'vela-preview'
            env['ANDROID_KEYSTORE_PASSWORD'] = secrets.token_urlsafe(32)
            env['ANDROID_KEY_PASSWORD'] = env['ANDROID_KEYSTORE_PASSWORD']
            subprocess.run(['keytool', '-genkeypair', '-keystore', str(key), '-storetype', 'JKS',
                            '-storepass:env', 'ANDROID_KEYSTORE_PASSWORD', '-keypass:env', 'ANDROID_KEY_PASSWORD',
                            '-alias', alias, '-keyalg', 'RSA', '-keysize', '2048', '-validity', '3650',
                            '-dname', 'CN=Vela Preview'], env=env, check=True)
        key.chmod(0o600)
        aligned = stage / 'aligned.apk'
        subprocess.run(['zipalign', '-f', '-p', '4', str(unsigned), str(aligned)], check=True)
        signed = stage / 'signed.apk'
        keystore_type = 'JKS' if key.read_bytes().startswith(b'\xfe\xed\xfe\xed') else 'PKCS12'
        subprocess.run(['apksigner', 'sign', '--ks', str(key), '--ks-type', keystore_type, '--ks-key-alias', alias,
                        '--ks-pass', 'env:ANDROID_KEYSTORE_PASSWORD', '--key-pass', 'env:ANDROID_KEY_PASSWORD',
                        '--out', str(signed), str(aligned)], env=env, check=True)
        check_archive(signed, ['AndroidManifest.xml', 'classes.dex', 'lib/arm64-v8a/libmain.so'])
        subprocess.run(['apksigner', 'verify', '--verbose', str(signed)], check=True)
        badging = subprocess.check_output(['aapt', 'dump', 'badging', str(signed)], text=True)
        if "name='io.github.fynth.vela'" not in badging or "sdkVersion:'26'" not in badging or 'application-debuggable' in badging:
            raise SystemExit('APK package, minimum API or release mode does not match')
        manifest = subprocess.check_output(['aapt', 'dump', 'xmltree', str(signed), 'AndroidManifest.xml'], text=True)
        for attribute in ('allowBackup', 'usesCleartextTraffic'):
            if not any(f'android:{attribute}(' in line and line.rstrip().endswith('0x0') for line in manifest.splitlines()):
                raise SystemExit(f'APK must disable {attribute}')
        print(badging)
        suffix = '' if signed_release else '-preview'
        apk = output / f'vela-{version}-android-arm64{suffix}.apk'
        shutil.copy2(signed, apk)
    write_sidecar(apk)
    artifact = Artifact('android-arm64', apk, 'apk', 'android', 'aarch64', 'github-release',
                        sha256(apk), apk.stat().st_size, {'signing': 'release' if signed_release else 'ephemeral-preview'})
    (output / 'release-report-android.json').write_text(json.dumps(
        {'version': version, 'commit': commit, 'artifacts': [describe(artifact)]}, indent=2) + '\n')
    print(f'Verified APK: {apk.name}; signing={artifact.extra["signing"]}')


if __name__ == '__main__':
    main()
