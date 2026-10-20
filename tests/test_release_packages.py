"""Regression checks for payload loss, merged reports and false-green releases."""
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
import release_build
import release_packages
import verify_release_artifacts as verification


class ReleasePackagesTest(unittest.TestCase):
    def test_windows_archive_keeps_exe_extension_and_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / 'vela-desktop.exe'
            binary.write_bytes(b'MZfixture')
            archive = release_build.stage_desktop(binary, '1.2.3', 'a' * 40, root, 'winget', 'windows')
            verification.check_archive(archive, ['bin/vela-desktop.exe', 'BUILD.txt'])
            verification.check_build_metadata(archive, '1.2.3', 'a' * 40)
            with self.assertRaises(SystemExit):
                verification.check_build_metadata(archive, '9.9.9', 'a' * 40)

    def test_tar_payload_and_metadata_are_actually_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / 'vela-desktop'
            binary.write_bytes(b'fixture')
            archive = release_build.stage_desktop(binary, '1.2.3', 'a' * 40, root, 'local-review')
            verification.check_archive(archive, ['bin/vela-desktop', 'BUILD.txt'])
            verification.check_build_metadata(archive, '1.2.3', 'a' * 40)
            with self.assertRaises(SystemExit):
                verification.check_archive(archive, ['missing'])

    def test_reports_merge_without_losing_a_platform_and_reject_tampering(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for platform in ('windows', 'linux'):
                # A small PE fixture exercises the transport, hash and size gates.
                path = root / f'{platform}.exe'
                path.write_bytes(b'MZfixture')
                release_build.write_sidecar(path)
                artifact = release_build.Artifact(platform, path, 'exe', platform, 'x86-64', 'local-review', release_build.sha256(path), path.stat().st_size)
                (root / f'release-report-{platform}.json').write_text(json.dumps({'version': '1.2.3', 'commit': 'a' * 40, 'artifacts': [release_build.describe(artifact)]}))
            self.assertEqual(len(verification.verify(root)['artifacts']), 2)
            with self.assertRaises(SystemExit):
                verification.verify(root, require_all=True)
            (root / 'windows.exe').write_bytes(b'MZtampered')
            with self.assertRaises(SystemExit):
                verification.verify(root)

    def test_reports_reject_mixed_commits_and_unsafe_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = root / 'release-report-a.json'
            second = root / 'release-report-b.json'
            first.write_text(json.dumps({'version': '1', 'commit': 'a', 'artifacts': [{'file': '../escape'}]}))
            with self.assertRaises(SystemExit):
                verification.load_reports(root)
            second.write_text(json.dumps({'version': '1', 'commit': 'b', 'artifacts': []}))
            with self.assertRaises(SystemExit):
                verification.load_reports(root)

    def test_aur_has_real_source_arrays_and_complete_installed_payload(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            # Use the real payload layout; only the executable is a test fixture.
            binary = root / 'binary'
            binary.write_bytes(b'#!/bin/sh\nexit 0\n')
            stage = release_packages.desktop_payload('1.2.3', 'a' * 40, binary)
            self.addCleanup(release_packages.shutil.rmtree, stage)
            with patch.dict(os.environ, {'VELA_RELEASE_BASE_URL': 'https://github.com/Fynth/vela/releases/download/v1.2.3'}), patch.object(release_packages, 'bundle_xdo'):
                archive = release_packages.build_aur(stage, '1.2.3', 'a' * 40, root)
            verification.check_archive(archive, ['usr/bin/vela-desktop', 'usr/lib/vela/vela-desktop', 'usr/share/icons/hicolor/scalable/apps/vela-desktop.svg'])
            recipe = root / 'aur/PKGBUILD'
            subprocess.run(['bash', '-n', str(recipe)], check=True)
            result = subprocess.run(['bash', '-c', 'source "$1"; test "${#source[@]}" = 1; test "${sha256sums[0]}" != SKIP; test "$pkgname" = vela-music-bin', 'test', str(recipe)], check=False)
            self.assertEqual(result.returncode, 0)


if __name__ == '__main__':
    unittest.main()
