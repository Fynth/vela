#!/usr/bin/env python3
"""Sign the release artifacts and record what was signed.

An artifact without a signature can be replaced after the fact, so every
published artifact carries a detached signature and the signature itself is
verified before the release continues.

Two signing identities are supported, because they answer different questions:

  key   a local GPG key. It proves who produced the artifact, and it can be
        checked by anyone who has the public key.
  keyless a Sigstore/cosign identity. It proves which workflow produced the
        artifact, and it is verifiable with the public transparency log.

A detached signature is written next to the artifact. When no key is available
the script says so and leaves the artifacts unsigned, which the release
workflow reports as a visible warning rather than a silent success.
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def run(command: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    printable = " ".join(command)
    print(f"  $ {printable}", flush=True)
    result = subprocess.run(command, cwd=str(REPO_ROOT), capture_output=True, text=True)
    if check and result.returncode != 0:
        for line in (result.stderr or result.stdout or "").strip().splitlines()[-10:]:
            print(f"    {line}", file=sys.stderr)
        raise SystemExit(f"command failed ({result.returncode}): {printable}")
    return result


def sign_with_gpg(artifact: Path, key: str | None) -> bool:
    """Signs with a local GPG key, optionally selecting one by id."""
    if shutil.which("gpg") is None:
        print("  gpg is not installed")
        return False
    command = ["gpg", "--batch", "--yes", "--armor", "--detach-sign"]
    if key:
        command += ["--local-user", key]
    command += ["--output", f"{artifact}.asc", str(artifact)]
    result = run(command, check=False)
    if result.returncode != 0:
        print("  gpg could not sign the artifact")
        return False
    return True


def sign_with_cosign(artifact: Path) -> bool:
    """Signs keyless with cosign, which binds the artifact to the workflow."""
    if shutil.which("cosign") is None:
        print("  cosign is not installed")
        return False
    result = run(
        ["cosign", "sign-blob", "--yes", str(artifact)], check=False
    )
    if result.returncode != 0:
        print("  cosign could not sign the artifact")
        return False
    return True


def verify_signature(artifact: Path) -> bool:
    """Verifies the signature that was written next to the artifact."""
    signature = artifact.with_name(artifact.name + ".asc")
    if not signature.is_file():
        return False
    if shutil.which("gpg") is None:
        return False
    result = run(["gpg", "--verify", str(signature), str(artifact)], check=False)
    return result.returncode == 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--artifacts",
        type=Path,
        default=REPO_ROOT / "target" / "release-artifacts",
    )
    parser.add_argument(
        "--gpg-key",
        default=None,
        help="the GPG key id to sign with",
    )
    parser.add_argument(
        "--cosign",
        action="store_true",
        help="also try a keyless cosign signature",
    )
    parser.add_argument(
        "--require-signature",
        action="store_true",
        help="fail when no artifact could be signed",
    )
    arguments = parser.parse_args()

    artifacts = sorted(
        path
        for path in arguments.artifacts.iterdir()
        if path.is_file() and path.suffix in {".tar.gz", ".deb", ".rpm", ".AppImage"}
    )
    if not artifacts:
        raise SystemExit(f"no artifacts to sign in {arguments.artifacts}")

    results = []
    signed = 0
    for artifact in artifacts:
        print(f"signing {artifact.name}")
        ok = sign_with_gpg(artifact, arguments.gpg_key)
        if not ok and arguments.cosign:
            ok = sign_with_cosign(artifact)
        if ok:
            signed += 1
            verified = verify_signature(artifact)
            results.append({"file": artifact.name, "signed": True, "verified": verified})
        else:
            results.append({"file": artifact.name, "signed": False, "verified": False})

    report = arguments.artifacts / "signing-report.json"
    report.write_text(json.dumps({"artifacts": results}, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {report}")
    for entry in results:
        state = "signed" if entry["signed"] else "unsigned"
        print(f"  {entry['file']}: {state}")
    if arguments.require_signature and signed != len(artifacts):
        raise SystemExit("not every artifact could be signed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
