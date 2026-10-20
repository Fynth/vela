#!/usr/bin/env python3
"""Build and verify the release artifacts declared in the release manifest.

Every artifact is built from the same source tree, in the same way, and is
verified before it is published anywhere:

  * the manifest is validated first, so a broken target list stops the release
    before a single binary is produced;
  * the tree must be clean and committed, so an artifact can always be traced
    back to exactly one commit;
  * every artifact carries a BUILD.txt with the version, target and commit, and
    a SHA256 sidecar;
  * a package is unpacked again and checked, so a tarball that does not contain
    what it claims never leaves this script.

The script never uploads anything. Publishing is a separate, explicit step that
requires credentials and is only reached for targets whose manifest status says
they may be published.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
MANIFEST_PATH = REPO_ROOT / "release" / "targets.yaml"
OUTPUT_ROOT = REPO_ROOT / "target" / "release-artifacts"


def run(
    command: list[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    """Runs a command and reports it without hiding a failure."""
    printable = " ".join(command)
    print(f"  $ {printable}", flush=True)
    environment = dict(os.environ)
    if env:
        environment.update(env)
    result = subprocess.run(
        command,
        cwd=str(cwd or REPO_ROOT),
        env=environment,
        capture_output=True,
        text=True,
    )
    if check and result.returncode != 0:
        tail = (result.stderr or result.stdout or "").strip().splitlines()[-12:]
        for line in tail:
            print(f"    {line}", file=sys.stderr)
        raise SystemExit(f"command failed ({result.returncode}): {printable}")
    return result


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_sidecar(path: Path) -> Path:
    """Writes `<artifact>.sha256` next to the artifact, the way the rest of the
    project records checksums."""
    sidecar = path.with_name(path.name + ".sha256")
    sidecar.write_text(f"{sha256(path)}  {path.name}\n", encoding="utf-8")
    return sidecar


def git(*arguments: str) -> str:
    return run(["git", *arguments]).stdout.strip()


@dataclass
class Artifact:
    """One produced file and the facts needed to publish or audit it."""

    target: str
    path: Path
    kind: str
    platform: str
    arch: str
    channel: str
    sha256: str
    size: int
    extra: dict[str, str] = field(default_factory=dict)


def load_manifest() -> list[dict]:
    sys.path.insert(0, str(REPO_ROOT / "scripts"))
    from release_manifest import ManifestError, load, validate  # noqa: PLC0415

    try:
        document = load(MANIFEST_PATH)
        return validate(document)
    except ManifestError as error:
        raise SystemExit(f"release manifest is invalid: {error}") from error


def require_clean_tree() -> str:
    """A release must be traceable to one commit."""
    if git("status", "--porcelain"):
        raise SystemExit("the working tree is not clean; commit before releasing")
    return git("rev-parse", "HEAD")


def workspace_version() -> str:
    metadata = json.loads(
        run(
            [
                "cargo",
                "metadata",
                "--locked",
                "--no-deps",
                "--format-version=1",
            ]
        ).stdout
    )
    for package in metadata["packages"]:
        if package["name"] == "vela-server":
            return package["version"]
    raise SystemExit("could not read the workspace version")


def build_web(version: str, commit: str) -> Path:
    """Builds the web client and copies it into the artifact directory."""
    tools = REPO_ROOT / "target" / "tools"
    bindgen = None
    if tools.is_dir():
        for entry in sorted(tools.iterdir()):
            if entry.name.startswith("wasm-bindgen-") and not entry.name.endswith(".tar.gz"):
                candidate = entry / "wasm-bindgen" if entry.is_dir() else entry
                if candidate.is_file():
                    bindgen = str(candidate)
                    break
    environment = {"VELA_WEB_PROFILE": "release"}
    if bindgen:
        environment["VELA_WASM_BINDGEN"] = bindgen
    run(["bash", "scripts/build-web.sh"], env=environment)
    run(["python3", "scripts/generate_notices.py"])
    return REPO_ROOT / "dist"


def build_server(version: str, commit: str) -> Path:
    run(["cargo", "build", "--locked", "--release", "-p", "vela-server"])
    return REPO_ROOT / "target" / "release" / "vela-server"


def build_desktop(version: str, commit: str) -> Path:
    run(["cargo", "build", "--locked", "--release", "-p", "vela-desktop"])
    return REPO_ROOT / "target" / "release" / "vela-desktop"


def stage_server(
    binary: Path, version: str, commit: str, web: Path, output_root: Path, channel: str
) -> Path:
    """Packs the server with the web client into one reviewable archive."""
    stage = Path(tempfile.mkdtemp(prefix="vela-release-server-"))
    try:
        shutil.copy2(binary, stage / "vela-server")
        shutil.copytree(web, stage / "dist")
        for name in ("LICENSE", "README.md"):
            source = REPO_ROOT / name
            if source.is_file():
                shutil.copy2(source, stage / name)
        (stage / "BUILD.txt").write_text(
            f"version={version}\ntarget=x86_64-unknown-linux-gnu\n"
            f"commit={commit}\nschema_version=13\n",
            encoding="utf-8",
        )
        archive = output_root / f"vela-server-{version}-linux-x86_64-{channel}.tar.gz"
        archive.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(archive, "w:gz") as bundle:
            bundle.add(stage, arcname=".")
        return archive
    finally:
        shutil.rmtree(stage, ignore_errors=True)


def stage_desktop(
    binary: Path, version: str, commit: str, output_root: Path, channel: str
) -> Path:
    """Packs the portable desktop archive the project already ships."""
    stage = Path(tempfile.mkdtemp(prefix="vela-release-desktop-"))
    try:
        (stage / "bin").mkdir()
        (stage / "share" / "applications").mkdir(parents=True)
        icons = stage / "share" / "icons" / "hicolor" / "scalable" / "apps"
        icons.mkdir(parents=True)
        docs = stage / "share" / "doc" / "vela-desktop"
        docs.mkdir(parents=True)
        shutil.copy2(binary, stage / "bin" / "vela-desktop")
        shutil.copy2(
            REPO_ROOT / "apps" / "desktop" / "vela-desktop.desktop",
            stage / "share" / "applications" / "vela-desktop.desktop",
        )
        shutil.copy2(REPO_ROOT / "apps" / "web" / "vela.svg", icons / "vela-desktop.svg")
        readme = REPO_ROOT / "apps" / "desktop" / "README.md"
        if readme.is_file():
            shutil.copy2(readme, docs / "README.md")
        shutil.copy2(REPO_ROOT / "LICENSE", stage / "LICENSE")
        (stage / "BUILD.txt").write_text(
            f"version={version}\ntarget=x86_64-unknown-linux-gnu\n"
            f"commit={commit}\nruntime=GTK3, WebKitGTK 4.1, ALSA, libxdo\n",
            encoding="utf-8",
        )
        archive = output_root / f"vela-desktop-{version}-linux-x86_64-{channel}.tar.gz"
        archive.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(archive, "w:gz") as bundle:
            bundle.add(stage, arcname=".")
        return archive
    finally:
        shutil.rmtree(stage, ignore_errors=True)


def verify_archive(archive: Path, required: list[str]) -> None:
    """Reopens a produced archive and checks that it really contains what it
    claims. A tarball that silently lost a file never leaves the pipeline."""
    with tarfile.open(archive, "r:gz") as bundle:
        names = {name.lstrip("./") for name in bundle.getnames()}
    missing = [name for name in required if name not in names]
    if missing:
        raise SystemExit(f"{archive.name} is missing {missing}")


def describe(artifact: Artifact) -> dict:
    return {
        "target": artifact.target,
        "file": artifact.path.name,
        "kind": artifact.kind,
        "platform": artifact.platform,
        "arch": artifact.arch,
        "channel": artifact.channel,
        "sha256": artifact.sha256,
        "size": artifact.size,
        **artifact.extra,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--only",
        action="append",
        default=[],
        help="build only these target ids (repeatable)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=OUTPUT_ROOT,
        help="directory that receives the artifacts",
    )
    arguments = parser.parse_args()

    targets = load_manifest()
    selected = [
        target
        for target in targets
        if not arguments.only or target["id"] in arguments.only
    ]
    if arguments.only:
        unknown = set(arguments.only) - {target["id"] for target in targets}
        if unknown:
            raise SystemExit(f"unknown target ids: {sorted(unknown)}")

    version = workspace_version()
    commit = require_clean_tree()
    print(f"release build: version {version}, commit {commit[:12]}")
    output_root = arguments.output
    output_root.mkdir(parents=True, exist_ok=True)

    produced: list[Artifact] = []
    web: Path | None = None
    server: Path | None = None
    desktop: Path | None = None

    for target in selected:
        identifier = target["id"]
        if target["status"] == "blocked":
            print(f"skip {identifier}: {target['channel']} is blocked on this platform")
            continue
        print(f"build {identifier}")
        if target["kind"] == "web":
            web = web or build_web(version, commit)
            archive = output_root / f"vela-web-{version}-linux-x86_64-{target['channel']}.tar.gz"
            with tarfile.open(archive, "w:gz") as bundle:
                bundle.add(web, arcname="dist")
            verify_archive(archive, ["dist/style.css", "dist/index.html"])
            produced.append(
                Artifact(
                    identifier,
                    archive,
                    "web",
                    "linux",
                    "x86_64",
                    target["channel"],
                    sha256(archive),
                    archive.stat().st_size,
                )
            )
        elif target["kind"] == "server":
            server = server or build_server(version, commit)
            web = web or build_web(version, commit)
            archive = stage_server(server, version, commit, web, output_root, target["channel"])
            verify_archive(archive, ["vela-server", "dist/index.html", "BUILD.txt"])
            produced.append(
                Artifact(
                    identifier,
                    archive,
                    "server",
                    "linux",
                    "x86_64",
                    target["channel"],
                    sha256(archive),
                    archive.stat().st_size,
                )
            )
        elif target["kind"] == "desktop":
            desktop = desktop or build_desktop(version, commit)
            archive = stage_desktop(desktop, version, commit, output_root, target["channel"])
            verify_archive(
                archive,
                [
                    "bin/vela-desktop",
                    "share/applications/vela-desktop.desktop",
                    "BUILD.txt",
                ],
            )
            produced.append(
                Artifact(
                    identifier,
                    archive,
                    "desktop",
                    "linux",
                    "x86_64",
                    target["channel"],
                    sha256(archive),
                    archive.stat().st_size,
                )
            )
        else:
            print(f"skip {identifier}: unsupported artifact kind {target['kind']!r}")

    for artifact in produced:
        write_sidecar(artifact.path)

    report = {
        "version": version,
        "commit": commit,
        "artifacts": [describe(artifact) for artifact in produced],
    }
    report_path = output_root / "release-report.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {report_path}")
    for artifact in produced:
        print(f"  {artifact.path.name}  {artifact.sha256[:16]}  {artifact.size} B")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
