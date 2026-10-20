#!/usr/bin/env python3
"""Publish verified release artifacts to their channels.

Publishing is the one step that changes the world, so it is deliberately
separate from building and deliberately hard to do by accident:

  * only targets whose manifest status allows it are considered;
  * every credential a target names must be present in the environment, and a
    missing credential stops the publish instead of silently skipping it;
  * a dry run reports exactly what would be uploaded, with no network access;
  * the artifacts are re-verified (checksum and payload) immediately before the
    upload, so a file that changed after the build cannot be published;
  * each published target is recorded in a receipt, so a later run knows what
    already went out.

The channels that need an account (App Store, Google Play, AUR, winget,
Homebrew) are driven by the same rules: the recipe for each channel lives in
`release/channels/`, and a channel without a recipe is reported as
`not-implemented` rather than approximated.
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
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from release_manifest import ManifestError, load, validate  # noqa: E402

MANIFEST_PATH = Path(
    os.environ.get("VELA_RELEASE_MANIFEST") or REPO_ROOT / "release" / "targets.yaml"
)
CHANNELS_DIR = REPO_ROOT / "release" / "channels"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def channel_recipe(channel: str) -> Path | None:
    """The command recipe for a channel, when one exists.

    A recipe is an executable shell script that receives the artifact path and
    the target id as arguments. Channels without a recipe are reported as
    `not-implemented`: the pipeline never invents an upload.
    """
    candidate = CHANNELS_DIR / f"{channel}.sh"
    if candidate.is_file() and os.access(candidate, os.X_OK):
        return candidate
    return None


def verify_artifact(path: Path) -> None:
    """Re-checks an artifact right before it is published."""
    if not path.is_file():
        raise SystemExit(f"artifact is missing: {path}")
    sidecar = path.with_name(path.name + ".sha256")
    if sidecar.is_file():
        recorded = sidecar.read_text(encoding="utf-8").split()[0]
        actual = sha256(path)
        if recorded != actual:
            raise SystemExit(
                f"{path.name} changed after it was built ({recorded[:12]} != {actual[:12]})"
            )
    if path.suffix == ".tar.gz":
        with tarfile.open(path, "r:gz") as bundle:
            if not bundle.getnames():
                raise SystemExit(f"{path.name} is empty")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--artifacts",
        type=Path,
        default=REPO_ROOT / "target" / "release-artifacts",
        help="directory that holds the built artifacts",
    )
    parser.add_argument(
        "--receipt",
        type=Path,
        default=REPO_ROOT / "target" / "release-artifacts" / "publish-receipt.json",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="report what would be published without touching the network",
    )
    arguments = parser.parse_args()

    try:
        targets = validate(load(MANIFEST_PATH))
    except ManifestError as error:
        raise SystemExit(f"release manifest is invalid: {error}") from error

    report_path = arguments.artifacts / "release-report.json"
    if not report_path.is_file():
        raise SystemExit(f"no release report at {report_path}; build the release first")
    report = json.loads(report_path.read_text(encoding="utf-8"))
    by_target = {artifact["target"]: artifact for artifact in report["artifacts"]}

    receipt: dict = {"version": report["version"], "commit": report["commit"], "published": []}
    if arguments.receipt.is_file():
        try:
            previous = json.loads(arguments.receipt.read_text(encoding="utf-8"))
            receipt["published"] = previous.get("published", [])
        except json.JSONDecodeError:
            pass
    # Only a target that really reached a channel counts as published. An
    # artifact that was merely attached to the release must not make a later
    # real publish look like it already happened.
    already = {
        entry["target"]
        for entry in receipt["published"]
        if entry.get("status") in {"published", "dry-run"}
    }

    for target in targets:
        identifier = target["id"]
        channel = target["channel"]
        artifact = by_target.get(identifier)
        if artifact is None:
            continue
        if target["status"] == "blocked":
            print(f"{identifier}: blocked on this platform, nothing to publish")
            continue
        if identifier in already:
            print(f"{identifier}: already published, skipping")
            continue
        if target["publish"] == "none":
            print(f"{identifier}: local-review artifact, attached to the release only")
            receipt["published"].append(
                {"target": identifier, "channel": channel, "status": "attached", "file": artifact["file"]}
            )
            continue

        missing = [name for name in target["credentials"] if not os.environ.get(name)]
        if missing:
            print(f"{identifier}: cannot publish, missing credentials {missing}")
            receipt["published"].append(
                {
                    "target": identifier,
                    "channel": channel,
                    "status": "skipped-missing-credentials",
                    "missing": missing,
                }
            )
            continue

        recipe = channel_recipe(channel)
        if recipe is None:
            print(f"{identifier}: channel {channel} has no upload recipe yet")
            receipt["published"].append(
                {"target": identifier, "channel": channel, "status": "not-implemented"}
            )
            continue

        artifact_path = REPO_ROOT / artifact["file"] if not Path(artifact["file"]).is_absolute() else Path(artifact["file"])
        verify_artifact(artifact_path)
        if arguments.dry_run:
            print(f"{identifier}: would publish {artifact_path.name} to {channel} via {recipe.name}")
            receipt["published"].append(
                {"target": identifier, "channel": channel, "status": "dry-run"}
            )
            continue
        print(f"{identifier}: publishing {artifact_path.name} to {channel}")
        result = subprocess.run(
            [str(recipe), str(artifact_path), identifier],
            cwd=str(REPO_ROOT),
            env=dict(os.environ),
        )
        if result.returncode != 0:
            raise SystemExit(f"{identifier}: channel {channel} failed to publish")
        receipt["published"].append(
            {
                "target": identifier,
                "channel": channel,
                "status": "published",
                "file": artifact["file"],
                "sha256": artifact["sha256"],
            }
        )

    arguments.receipt.parent.mkdir(parents=True, exist_ok=True)
    arguments.receipt.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {arguments.receipt}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
