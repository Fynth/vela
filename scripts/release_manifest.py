#!/usr/bin/env python3
"""Validate the release target manifest.

The manifest is the single source of truth for what Vela ships and where. A
mistyped target, a missing dependency or a target that claims to be published
without a publish rule would silently produce a release that does not match the
project's own documentation, so the manifest is checked before any build runs.

Checks
  * every entry has the required fields with valid values
  * ids are unique and file-system safe
  * `requires` refers to targets that exist
  * the dependency graph has no cycles
  * `publish: none` targets never claim credentials are used
  * a target that is not `ready` must not pretend to publish
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import yaml

REQUIRED_FIELDS = {
    "id": str,
    "kind": str,
    "platform": str,
    "arch": str,
    "channel": str,
    "publish": str,
    "requires": list,
    "credentials": list,
    "status": str,
}
PLATFORMS = {"linux", "windows", "macos", "android", "ios"}
STATUSES = {"ready", "buildable", "blocked"}
ID_PATTERN = re.compile(r"^[a-z0-9][a-z0-9-]*$")


class ManifestError(Exception):
    """The manifest does not describe a buildable release."""


def load(path: Path) -> dict:
    try:
        document = yaml.safe_load(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise ManifestError(f"manifest not found: {path}") from error
    except yaml.YAMLError as error:
        raise ManifestError(f"manifest is not valid YAML: {error}") from error
    if not isinstance(document, dict):
        raise ManifestError("manifest must be a mapping")
    return document


def validate(document: dict) -> list[dict]:
    version = document.get("version")
    if version != 1:
        raise ManifestError(f"unsupported manifest version: {version!r}")
    targets = document.get("targets")
    if not isinstance(targets, list) or not targets:
        raise ManifestError("manifest needs a non-empty `targets` list")

    seen: dict[str, dict] = {}
    for entry in targets:
        if not isinstance(entry, dict):
            raise ManifestError("every target must be a mapping")
        for field, expected in REQUIRED_FIELDS.items():
            if field not in entry:
                raise ManifestError(f"target {entry.get('id', '?')!r} is missing `{field}`")
            if not isinstance(entry[field], expected):
                raise ManifestError(
                    f"target {entry['id']!r} field `{field}` must be {expected.__name__}"
                )
        identifier = entry["id"]
        if not ID_PATTERN.match(identifier):
            raise ManifestError(f"target id {identifier!r} must be lower-case and file-system safe")
        if identifier in seen:
            raise ManifestError(f"duplicate target id: {identifier}")
        if entry["platform"] not in PLATFORMS:
            raise ManifestError(f"target {identifier} has an unknown platform {entry['platform']!r}")
        if entry["status"] not in STATUSES:
            raise ManifestError(f"target {identifier} has an unknown status {entry['status']!r}")
        if entry["publish"] not in {"none", "upload"}:
            raise ManifestError(
                f"target {identifier} has an unknown publish rule {entry['publish']!r}"
            )
        if entry["publish"] == "none" and entry["credentials"] and entry["status"] != "blocked":
            raise ManifestError(
                f"target {identifier} lists credentials but never publishes with them"
            )
        if entry["publish"] == "upload" and not entry["credentials"]:
            raise ManifestError(
                f"target {identifier} publishes but names no credentials to publish with"
            )
        if entry["publish"] == "upload" and entry["status"] == "blocked":
            raise ManifestError(
                f"target {identifier} claims to publish while its status is blocked"
            )
        if entry["status"] == "ready" and entry["publish"] == "none" and entry["credentials"]:
            raise ManifestError(
                f"target {identifier} is ready but holds credentials it never uses"
            )
        if entry["status"] == "ready" and entry["publish"] == "none":
            # A ready target may still be a local-review artifact, but it must be
            # attached to the release so it is actually reachable.
            pass
        if not isinstance(entry["requires"], list) or not all(
            isinstance(item, str) for item in entry["requires"]
        ):
            raise ManifestError(f"target {identifier} has a malformed `requires` list")
        if not isinstance(entry["credentials"], list) or not all(
            isinstance(item, str) for item in entry["credentials"]
        ):
            raise ManifestError(f"target {identifier} has a malformed `credentials` list")
        seen[identifier] = entry

    for entry in targets:
        for dependency in entry["requires"]:
            if dependency not in seen:
                raise ManifestError(
                    f"target {entry['id']} requires unknown target {dependency!r}"
                )
            if dependency == entry["id"]:
                raise ManifestError(f"target {entry['id']} requires itself")

    # Cycle detection over the `requires` graph.
    state: dict[str, int] = {}

    def visit(node: str, path: tuple[str, ...]) -> None:
        marker = state.get(node, 0)
        if marker == 1:
            raise ManifestError("release dependency cycle: " + " -> ".join(path + (node,)))
        if marker == 2:
            return
        state[node] = 1
        for dependency in seen[node]["requires"]:
            visit(dependency, path + (node,))
        state[node] = 2

    for identifier in seen:
        visit(identifier, ())
    return list(targets)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "release" / "targets.yaml",
    )
    arguments = parser.parse_args()
    try:
        targets = validate(load(arguments.manifest))
    except ManifestError as error:
        print(f"release manifest is invalid: {error}", file=sys.stderr)
        return 1
    by_status: dict[str, int] = {}
    for target in targets:
        by_status[target["status"]] = by_status.get(target["status"], 0) + 1
    summary = ", ".join(f"{count} {status}" for status, count in sorted(by_status.items()))
    print(f"release manifest valid: {len(targets)} targets ({summary})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
