"""Tests for the release pipeline.

The release path is the one that can publish code to other people's devices,
so its rules are tested rather than trusted: the manifest must reject
contradictions, the build must refuse to run on a dirty tree, the produced
archives must contain what they claim, and publishing must never run without
the credentials the manifest names.
"""
from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tarfile
import tempfile
import unittest
import unittest.mock
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

import release_build  # noqa: E402
import release_manifest  # noqa: E402
import release_publish  # noqa: E402


def load_manifest_module():
    spec = importlib.util.spec_from_file_location(
        "release_manifest_local", REPO_ROOT / "scripts" / "release_manifest.py"
    )
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


MANIFEST = load_manifest_module()


def write_manifest(directory: Path, body: str) -> Path:
    path = directory / "targets.yaml"
    path.write_text(body, encoding="utf-8")
    return path


class ManifestRulesTest(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = Path(tempfile.mkdtemp())

    def test_the_shipped_manifest_is_valid(self) -> None:
        document = MANIFEST.load(REPO_ROOT / "release" / "targets.yaml")
        targets = MANIFEST.validate(document)
        self.assertTrue(targets)
        identifiers = {target["id"] for target in targets}
        self.assertEqual(len(identifiers), len(targets), "target ids must be unique")

    def test_a_target_cannot_require_itself(self) -> None:
        path = write_manifest(
            self.directory,
            """
version: 1
targets:
  - id: loop
    kind: desktop
    platform: linux
    arch: x86_64
    channel: local-review
    publish: none
    requires: [loop]
    credentials: []
    status: buildable
""",
        )
        with self.assertRaises(MANIFEST.ManifestError):
            MANIFEST.validate(MANIFEST.load(path))

    def test_an_unknown_requirement_is_rejected(self) -> None:
        path = write_manifest(
            self.directory,
            """
version: 1
targets:
  - id: needs-ghost
    kind: desktop
    platform: linux
    arch: x86_64
    channel: local-review
    publish: none
    requires: [ghost]
    credentials: []
    status: buildable
""",
        )
        with self.assertRaises(MANIFEST.ManifestError):
            MANIFEST.validate(MANIFEST.load(path))

    def test_a_dependency_cycle_is_rejected(self) -> None:
        path = write_manifest(
            self.directory,
            """
version: 1
targets:
  - id: one
    kind: desktop
    platform: linux
    arch: x86_64
    channel: local-review
    publish: none
    requires: [two]
    credentials: []
    status: buildable
  - id: two
    kind: desktop
    platform: linux
    arch: x86_64
    channel: local-review
    publish: none
    requires: [one]
    credentials: []
    status: buildable
""",
        )
        with self.assertRaises(MANIFEST.ManifestError):
            MANIFEST.validate(MANIFEST.load(path))

    def test_credentials_without_publishing_are_rejected_for_a_live_target(self) -> None:
        path = write_manifest(
            self.directory,
            """
version: 1
targets:
  - id: holds-secret
    kind: desktop
    platform: linux
    arch: x86_64
    channel: local-review
    publish: none
    requires: []
    credentials: [TOKEN]
    status: ready
""",
        )
        with self.assertRaises(MANIFEST.ManifestError):
            MANIFEST.validate(MANIFEST.load(path))

    def test_a_buildable_target_may_reserve_credentials_it_does_not_use_yet(self) -> None:
        # A target that waits for an application project keeps the credential
        # names it will need, without claiming to publish today.
        path = write_manifest(
            self.directory,
            """
version: 1
targets:
  - id: reserves-secret
    kind: desktop
    platform: android
    arch: aarch64
    channel: google-play
    publish: none
    requires: []
    credentials: [KEYSTORE]
    status: buildable
""",
        )
        targets = MANIFEST.validate(MANIFEST.load(path))
        self.assertEqual(len(targets), 1)

    def test_a_ready_target_may_not_hold_unused_credentials(self) -> None:
        path = write_manifest(
            self.directory,
            """
version: 1
targets:
  - id: ready-with-secret
    kind: desktop
    platform: android
    arch: aarch64
    channel: google-play
    publish: none
    requires: []
    credentials: [KEYSTORE]
    status: ready
""",
        )
        with self.assertRaises(MANIFEST.ManifestError):
            MANIFEST.validate(MANIFEST.load(path))

    def test_a_blocked_target_may_not_claim_to_publish(self) -> None:
        path = write_manifest(
            self.directory,
            """
version: 1
targets:
  - id: blocked-but-publishing
    kind: desktop
    platform: ios
    arch: aarch64
    channel: app-store-ios
    publish: upload
    requires: []
    credentials: [TOKEN]
    status: blocked
""",
        )
        with self.assertRaises(MANIFEST.ManifestError):
            MANIFEST.validate(MANIFEST.load(path))


class BuildGuardTest(unittest.TestCase):
    def test_a_dirty_tree_stops_the_release(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory) / "repo"
            repository.mkdir()
            subprocess.run(["git", "init", "-q"], cwd=repository, check=True)
            subprocess.run(
                ["git", "config", "user.email", "t@example.invalid"], cwd=repository, check=True
            )
            subprocess.run(
                ["git", "config", "user.name", "t"], cwd=repository, check=True
            )
            (repository / "file.txt").write_text("x", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=repository, check=True)
            subprocess.run(["git", "commit", "-qm", "one"], cwd=repository, check=True)
            (repository / "file.txt").write_text("changed", encoding="utf-8")
            original = release_build.REPO_ROOT
            release_build.REPO_ROOT = repository
            try:
                with self.assertRaises(SystemExit):
                    release_build.require_clean_tree()
            finally:
                release_build.REPO_ROOT = original


class ArchiveVerificationTest(unittest.TestCase):
    def test_an_archive_missing_a_claimed_file_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "empty.tar.gz"
            with tarfile.open(archive, "w:gz") as bundle:
                bundle.add(__file__, arcname="other.txt")
            with self.assertRaises(SystemExit):
                release_build.verify_archive(archive, ["dist/index.html"])


class PublishRulesTest(unittest.TestCase):
    def test_a_missing_credential_stops_publishing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            artifacts = Path(directory)
            report = {
                "version": "0.0.0",
                "commit": "0" * 40,
                "artifacts": [
                    {
                        "target": "store-target",
                        "file": "store.tar.gz",
                        "kind": "desktop",
                        "platform": "linux",
                        "arch": "x86_64",
                        "channel": "aur",
                        "sha256": "0" * 64,
                        "size": 1,
                    }
                ],
            }
            (artifacts / "release-report.json").write_text(
                json.dumps(report), encoding="utf-8"
            )
            with tarfile.open(artifacts / "store.tar.gz", "w:gz") as bundle:
                bundle.add(__file__, arcname="payload")
            manifest = write_manifest(
                artifacts,
                """
version: 1
targets:
  - id: store-target
    kind: desktop
    platform: linux
    arch: x86_64
    channel: aur
    publish: upload
    requires: []
    credentials: [A_TOKEN_THAT_IS_MISSING]
    status: ready
""",
            )
            environment = dict(os.environ)
            environment.pop("A_TOKEN_THAT_IS_MISSING", None)
            environment["VELA_RELEASE_MANIFEST"] = str(manifest)
            with unittest.mock.patch.dict(os.environ, environment, clear=True):
                result = subprocess.run(
                    [
                        sys.executable,
                        str(REPO_ROOT / "scripts" / "release_publish.py"),
                        "--artifacts",
                        str(artifacts),
                        "--receipt",
                        str(artifacts / "receipt.json"),
                        "--dry-run",
                    ],
                    capture_output=True,
                    text=True,
                )
            self.assertEqual(result.returncode, 0, result.stderr)
            receipt = json.loads((artifacts / "receipt.json").read_text(encoding="utf-8"))
            statuses = {entry["target"]: entry["status"] for entry in receipt["published"]}
            self.assertEqual(statuses["store-target"], "skipped-missing-credentials")


if __name__ == "__main__":
    unittest.main()
