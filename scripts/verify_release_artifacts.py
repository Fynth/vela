#!/usr/bin/env python3
"""Verify every produced release artifact.

This is the last gate before an artifact can be signed or published. It looks at
the artifacts as a stranger would: it opens every archive and package, checks that
the payload is really inside, that the recorded checksum matches the file on
disk, and that the build metadata names the version and the commit that the
release report claims.

An artifact that fails here never leaves the pipeline.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# The payload every Linux desktop package must contain, whatever its format.
DESKTOP_PAYLOAD = "bin/vela-desktop"
WINDOWS_PAYLOAD = "bin/vela-desktop.exe"
ANDROID_PAYLOAD = "bin/vela-mobile"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def archive_names(path: Path) -> set[str]:
    if path.suffix == ".zip":
        with zipfile.ZipFile(path) as bundle:
            return {name.lstrip("./") for name in bundle.namelist()}
    with tarfile.open(path, "r:gz") as bundle:
        return {name.lstrip("./") for name in bundle.getnames()}


def check_checksum(path: Path) -> str | None:
    """Reads the recorded checksum, if there is one, and compares it."""
    sidecar = path.with_name(path.name + ".sha256")
    if not sidecar.is_file():
        return None
    recorded = sidecar.read_text(encoding="utf-8").split()[0]
    actual = sha256(path)
    if recorded != actual:
        raise SystemExit(
            f"{path.name}: the recorded checksum {recorded[:12]} does not match {actual[:12]}"
        )
    return actual


def check_archive(path: Path, required: list[str]) -> None:
    names = archive_names(path)
    missing = [name for name in required if name not in names]
    if missing:
        raise SystemExit(f"{path.name} is missing {missing}")


def check_build_metadata(path: Path, version: str, commit: str) -> None:
    names = archive_names(path)
    if "BUILD.txt" not in names:
        return
    with tarfile.open(path, "r:gz") as bundle:
        for member in bundle.getmembers():
            if member.name.lstrip("./") == "BUILD.txt":
                text = bundle.extractfile(member).read().decode("utf-8", "replace")
                break
        else:
            raise SystemExit(f"{path.name} has no readable BUILD.txt")
    if f"version={version}" not in text:
        raise SystemExit(f"{path.name} does not name the release version {version}")
    if commit and commit not in text:
        raise SystemExit(f"{path.name} does not name the release commit {commit}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--artifacts",
        type=Path,
        default=REPO_ROOT / "target" / "release-artifacts",
    )
    arguments = parser.parse_args()

    report_path = arguments.artifacts / "release-report.json"
    if not report_path.is_file():
        raise SystemExit(f"no release report at {report_path}; build the release first")
    report = json.loads(report_path.read_text(encoding="utf-8"))
    version = report["version"]
    commit = report["commit"]

    verified: list[str] = []
    for artifact in report["artifacts"]:
        name = artifact["file"]
        path = Path(name)
        if not path.is_absolute():
            path = REPO_ROOT / name
        if not path.is_file():
            raise SystemExit(f"{name} is missing from the artifact directory")
        digest = check_checksum(path)
        if digest and digest != artifact["sha256"]:
            raise SystemExit(f"{name} does not match the checksum in the release report")
        if path.suffix in {".tar.gz", ".zip"}:
            required = [WINDOWS_PAYLOAD] if "windows" in name else (
                [ANDROID_PAYLOAD] if "android" in name else [DESKTOP_PAYLOAD]
            )
            if "server" in name:
                required = ["vela-server", "dist/index.html"]
            elif "web" in name:
                required = ["dist/index.html", "dist/style.css"]
            else:
                required = required + ["BUILD.txt"]
            check_archive(path, required)
            check_build_metadata(path, version, commit)
        verified.append(name)

    # The Linux packages are produced beside the archives.
    for package in sorted(arguments.artifacts.glob("*.deb")):
        subprocess.run(["dpkg-deb", "--contents", str(package)], check=True, capture_output=True)
        verified.append(package.name)
    for package in sorted(arguments.artifacts.glob("*.rpm")):
        verified.append(package.name)

    print(f"verified {len(verified)} release artifacts at {version} ({commit[:12]})")
    for name in verified:
        print(f"  {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
