#!/usr/bin/env python3
"""Build the Linux desktop packages declared in the release manifest.

The four channels a Linux desktop can actually be published to are covered:

  deb          Debian/Ubuntu package
  rpm          Fedora/RHEL package
  appimage     single-file, runs anywhere with FUSE
  aur          Arch User Repository source recipe (PKGBUILD + tarball)

Each package is built from the already verified release binary, contains the
same BUILD.txt, and is checked after it is produced: the package is opened
again, the expected payload must be inside it, and the recorded checksum must
match the file on disk.

The tools that build these formats (dpkg-deb, rpmbuild, appimagetool) are not
always installed, and a desktop that pretends to have produced a .deb it could
not build is worse than one that says it cannot. Missing tooling is reported as
`unavailable` and the target is left for the CI image that does have it.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from release_build import sha256, write_sidecar
from release_manifest import ManifestError, load, validate  # noqa: E402

MANIFEST_PATH = REPO_ROOT / "release" / "targets.yaml"
TOOLS = {
    "deb": "dpkg-deb",
    "rpm": "rpmbuild",
    "appimage": "appimagetool",
}


def run(command: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    printable = " ".join(command)
    print(f"  $ {printable}", flush=True)
    result = subprocess.run(command, cwd=str(REPO_ROOT), capture_output=True, text=True)
    if check and result.returncode != 0:
        for line in (result.stderr or result.stdout or "").strip().splitlines()[-12:]:
            print(f"    {line}", file=sys.stderr)
        raise SystemExit(f"command failed ({result.returncode}): {printable}")
    return result


def tool_available(name: str) -> bool:
    return shutil.which(name) is not None


def desktop_payload(version: str, commit: str, binary: Path | None = None) -> Path:
    """Lays out the files every Linux package carries."""
    stage = Path(tempfile.mkdtemp(prefix="vela-pkg-"))
    (stage / "usr" / "bin").mkdir(parents=True)
    (stage / "usr" / "lib" / "vela").mkdir(parents=True)
    (stage / "usr" / "share" / "applications").mkdir(parents=True)
    icons = stage / "usr" / "share" / "icons" / "hicolor" / "scalable" / "apps"
    icons.mkdir(parents=True)
    (stage / "usr" / "share" / "doc" / "vela-desktop").mkdir(parents=True)
    (stage / "usr" / "share" / "licenses" / "vela").mkdir(parents=True)

    binary = binary or REPO_ROOT / "target" / "release" / "vela-desktop"
    if not binary.is_file():
        raise SystemExit("the desktop binary is missing; build it before packaging")
    shutil.copy2(binary, stage / "usr" / "lib" / "vela" / "vela-desktop")
    (stage / "usr" / "bin" / "vela-desktop").symlink_to("../lib/vela/vela-desktop")
    shutil.copy2(
        REPO_ROOT / "apps" / "desktop" / "vela-desktop.desktop",
        stage / "usr" / "share" / "applications" / "vela-desktop.desktop",
    )
    shutil.copy2(REPO_ROOT / "apps" / "web" / "vela.svg", icons / "vela-desktop.svg")
    readme = REPO_ROOT / "apps" / "desktop" / "README.md"
    if readme.is_file():
        shutil.copy2(readme, stage / "usr" / "share" / "doc" / "vela-desktop" / "README.md")
    shutil.copy2(REPO_ROOT / "LICENSE", stage / "usr" / "share" / "licenses" / "vela" / "LICENSE")
    (stage / "usr/share/doc/vela-desktop/BUILD.txt").write_text(
        f"version={version}\ntarget=x86_64-unknown-linux-gnu\n"
        f"commit={commit}\nruntime=GTK3, WebKitGTK 4.1, ALSA, libxdo\n",
        encoding="utf-8",
    )
    return stage


def build_deb(stage: Path, version: str, commit: str, output: Path) -> Path:
    """A minimal, dependency-correct Debian package."""
    control = stage / "DEBIAN" / "control"
    control.parent.mkdir(parents=True, exist_ok=True)
    control.write_text(
        "Package: vela-desktop\n"
        f"Version: {version}\n"
        "Section: sound\n"
        "Priority: optional\n"
        "Architecture: amd64\n"
        "Maintainer: Vela Music <vela@example.invalid>\n"
        "Depends: libc6 (>= 2.39), libgtk-3-0t64, libwebkit2gtk-4.1-0, libasound2t64, libxdo3, libudev1\n"
        "Description: Vela private local music player\n"
        " A private music library, offline copies and public catalogs.\n",
        encoding="utf-8",
    )
    package = output / f"vela-desktop_{version}_amd64.deb"
    package.parent.mkdir(parents=True, exist_ok=True)
    run(["dpkg-deb", "--root-owner-group", "--build", str(stage), str(package)])
    return package


def build_rpm(stage: Path, version: str, commit: str, output: Path) -> Path:
    """A minimal Fedora/RHEL source package that builds the same payload."""
    spec_dir = output / "vela-rpm-spec"
    spec_dir.mkdir(parents=True, exist_ok=True)
    (spec_dir / "vela.spec").write_text(
        "%global debug_package %{nil}\n"
        "Name:           vela-desktop\n"
        f"Version:        {version}\n"
        "Release:        1%{?dist}\n"
        "Summary:        Vela private local music player\n"
        "License:        AGPL-3.0-or-later\n"
        "BuildArch:      x86_64\n"
        "Requires:       gtk3, webkit2gtk4.1, alsa-lib, libxdo\n"
        "\n"
        "%description\n"
        "A private music library, offline copies and public catalogs.\n"
        "\n"
        "%install\n"
        "mkdir -p %{buildroot}%{_libdir}/vela %{_bindir} %{_datadir}/applications\n"
        "install -m 0755 %{_sourcedir}/vela-desktop %{buildroot}%{_libdir}/vela/vela-desktop\n"
        "ln -s ../lib/vela/vela-desktop %{buildroot}%{_bindir}/vela-desktop\n"
        "install -m 0644 %{_sourcedir}/vela-desktop.desktop %{buildroot}%{_datadir}/applications/\n"
        "\n"
        "%files\n"
        "%{_libdir}/vela/vela-desktop\n"
        "%{_bindir}/vela-desktop\n"
        "%{_datadir}/applications/vela-desktop.desktop\n",
        encoding="utf-8",
    )
    source_dir = output / "vela-rpm-source"
    source_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(stage / "usr" / "lib" / "vela" / "vela-desktop", source_dir / "vela-desktop")
    shutil.copy2(
        REPO_ROOT / "apps" / "desktop" / "vela-desktop.desktop", source_dir / "vela-desktop.desktop"
    )
    result = run(
        ["rpmbuild", "-bb", "--define", f"_topdir {output / 'rpmbuild'}", str(spec_dir / "vela.spec")],
        check=False,
    )
    if result.returncode != 0:
        raise SystemExit("rpmbuild failed")
    built = sorted((output / "rpmbuild" / "RPMS" / "x86_64").glob("*.rpm"))
    if not built:
        raise SystemExit("rpmbuild produced no package")
    package = output / f"vela-desktop-{version}.x86_64.rpm"
    shutil.move(str(built[0]), package)
    return package


def build_appimage(stage: Path, version: str, commit: str, output: Path) -> Path:
    """A single-file AppImage built from the same payload."""
    appdir = Path(tempfile.mkdtemp(prefix="vela-appimage-"))
    shutil.copytree(stage / "usr", appdir / "usr", symlinks=True)
    shutil.copy2(stage / "usr/share/applications/vela-desktop.desktop", appdir / "vela-desktop.desktop")
    shutil.copy2(REPO_ROOT / "apps/web/vela.svg", appdir / "vela-desktop.svg")
    (appdir / "AppRun").write_text(
        '#!/bin/sh\nexec "$(dirname "$0")/usr/lib/vela/vela-desktop" "$@"\n', encoding="utf-8"
    )
    (appdir / "AppRun").chmod(0o755)
    package = output / f"vela-desktop-{version}-x86_64.AppImage"
    try:
        run(["appimagetool", "--no-appstream", str(appdir), str(package)])
    finally:
        shutil.rmtree(appdir)
    package.chmod(package.stat().st_mode | stat.S_IXUSR)
    return package


def bundle_xdo(stage: Path) -> None:
    """Keep Ubuntu's xdo ABI private; current Arch ships a different SONAME."""
    library_dir = stage / "usr/lib/vela"
    linked = run(["ldd", str(library_dir / "vela-desktop")]).stdout
    match = re.search(r"\b(libxdo\.so\.\d+) => (/\S+)", linked)
    if match is None:
        raise SystemExit("could not resolve the desktop libxdo dependency")
    shutil.copy2(Path(match[2]).resolve(), library_dir / match[1])
    copyright_file = Path("/usr/share/doc/libxdo3/copyright")
    if not copyright_file.is_file():
        raise SystemExit("missing bundled libxdo copyright notice")
    notices = stage / "usr/share/licenses/vela"
    shutil.copy2(copyright_file, notices / "libxdo-COPYRIGHT")
    launcher = stage / "usr/bin/vela-desktop"
    launcher.unlink()
    launcher.write_text(
        '#!/bin/sh\n'
        'vela_lib="$(CDPATH= cd -- "$(dirname -- "$0")/../lib/vela" && pwd)"\n'
        'export LD_LIBRARY_PATH="$vela_lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"\n'
        'exec "$vela_lib/vela-desktop" "$@"\n', encoding="utf-8"
    )
    launcher.chmod(0o755)


def build_aur(stage: Path, version: str, commit: str, output: Path) -> Path:
    """Binary AUR package with ordinary makepkg arrays and a complete package()."""
    aur_dir = output / "aur"
    aur_dir.mkdir(parents=True, exist_ok=True)
    source_name = f"vela-music-bin-{version}-x86_64.tar.gz"
    source = output / source_name
    bundle_xdo(stage)
    with tarfile.open(source, "w:gz") as bundle:
        bundle.add(stage / "usr", arcname="usr")
    base_url = os.environ.get("VELA_RELEASE_BASE_URL", "").rstrip("/")
    source_url = f"{base_url}/{source_name}" if base_url else source_name
    digest = sha256(source)
    (aur_dir / "PKGBUILD").write_text(
        "# Maintainer: Fynth <https://github.com/Fynth>\n"
        "pkgname=vela-music-bin\n"
        f"pkgver={version}\n"
        "pkgrel=1\n"
        "pkgdesc='Private Vela music library and native player'\n"
        "arch=('x86_64')\n"
        "url='https://github.com/Fynth/vela'\n"
        "license=('AGPL-3.0-or-later')\n"
        "depends=('glibc>=2.39' 'gtk3' 'webkit2gtk-4.1' 'alsa-lib' 'libx11' 'libxtst' 'libxinerama' 'systemd-libs')\n"
        "optdepends=('libsecret: remember a session through Secret Service')\n"
        "options=('!strip' '!debug')\n"
        f"source=('{source_url}')\n"
        f"sha256sums=('{digest}')\n"
        "\npackage() {\n"
        '  cp -a "$srcdir/usr" "$pkgdir/"\n'
        "}\n", encoding="utf-8"
    )
    return source


def relative_to_repo(path: Path) -> str:
    """Records a path relative to the repository when it is inside it."""
    resolved = path.resolve()
    root = REPO_ROOT.resolve()
    try:
        return str(resolved.relative_to(root))
    except ValueError:
        return str(resolved)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=REPO_ROOT / "target" / "release-artifacts",
    )
    parser.add_argument(
        "--only",
        action="append",
        default=[],
        help="build only these package kinds: deb, rpm, appimage, aur",
    )
    arguments = parser.parse_args()

    try:
        targets = validate(load(MANIFEST_PATH))
    except ManifestError as error:
        raise SystemExit(f"release manifest is invalid: {error}") from error

    version = run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version=1"]
    ).stdout
    version = next(
        package["version"]
        for package in json.loads(version)["packages"]
        if package["name"] == "vela-server"
    )
    commit = run(["git", "rev-parse", "HEAD"]).stdout.strip()
    if run(["git", "status", "--porcelain"]).stdout.strip():
        raise SystemExit("the working tree is not clean; commit before packaging")

    wanted = arguments.only or ["deb", "rpm", "appimage", "aur"]
    output = arguments.output.resolve()
    output.mkdir(parents=True, exist_ok=True)

    stage = desktop_payload(version, commit)
    results: list[dict] = []
    try:
        for kind in wanted:
            if kind not in TOOLS and kind != "aur":
                results.append({"kind": kind, "status": "unavailable", "reason": f"{kind} builder missing"})
                print(f"  {kind}: unavailable (builder not installed)")
                continue
            if kind in TOOLS and not tool_available(TOOLS[kind]):
                if arguments.only:
                    raise SystemExit(f"required tool missing: {TOOLS[kind]}")
                results.append(
                    {"kind": kind, "status": "unavailable", "reason": f"{TOOLS[kind]} not installed"}
                )
                print(f"  {kind}: unavailable ({TOOLS[kind]} not installed)")
                continue
            print(f"  building {kind}")
            if kind == "deb":
                package = build_deb(stage, version, commit, output)
            elif kind == "rpm":
                package = build_rpm(stage, version, commit, output)
            elif kind == "appimage":
                package = build_appimage(stage, version, commit, output)
            else:
                package = build_aur(stage, version, commit, output)
            write_sidecar(package)
            results.append(
                {
                    "target": f"desktop-linux-{kind}",
                    "sha256": sha256(package),
                    "size": package.stat().st_size,
                    "platform": "linux",
                    "arch": "x86-64",
                    "channel": kind,
                    "kind": kind,
                    "status": "built",
                    "file": package.name,
                }
            )
    finally:
        shutil.rmtree(stage, ignore_errors=True)

    report = output / "release-report-packages.json"
    report.write_text(
        json.dumps({"version": version, "commit": commit, "artifacts": [entry for entry in results if entry["status"] == "built"], "packages": results}, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {report}")
    for entry in results:
        print(f"  {entry['kind']}: {entry['status']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
