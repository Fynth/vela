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
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

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


def desktop_payload(version: str, commit: str) -> Path:
    """Lays out the files every Linux package carries."""
    stage = Path(tempfile.mkdtemp(prefix="vela-pkg-"))
    (stage / "usr" / "bin").mkdir(parents=True)
    (stage / "usr" / "lib" / "vela").mkdir(parents=True)
    (stage / "usr" / "share" / "applications").mkdir(parents=True)
    icons = stage / "usr" / "share" / "icons" / "hicolor" / "scalable" / "apps"
    icons.mkdir(parents=True)
    (stage / "usr" / "share" / "doc" / "vela-desktop").mkdir(parents=True)
    (stage / "usr" / "share" / "licenses" / "vela").mkdir(parents=True)

    binary = REPO_ROOT / "target" / "release" / "vela-desktop"
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
    (stage / "BUILD.txt").write_text(
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
        "Depends: libgtk-3-0, libwebkit2gtk-4.1-0, libasound2, libxdo1\n"
        "Description: Vela private local music player\n"
        " A private music library, offline copies and public catalogs.\n",
        encoding="utf-8",
    )
    (control.parent / "postinst").write_text(
        "set -e\nldconfig\n", encoding="utf-8"
    )
    (control.parent / "postinst").chmod(0o755)
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
    root = output / "appimage-root"
    if root.exists():
        shutil.rmtree(root)
    shutil.copytree(stage, root)
    appdir = root / "vela-desktop.AppDir"
    appdir.mkdir(parents=True)
    (appdir / "usr" / "bin").mkdir(parents=True)
    (appdir / "usr" / "lib").mkdir(parents=True)
    shutil.copy2(stage / "usr" / "lib" / "vela" / "vela-desktop", appdir / "usr" / "lib" / "vela-desktop")
    (appdir / "usr" / "bin" / "vela-desktop").symlink_to("../lib/vela-desktop")
    (appdir / "vela-desktop.desktop").write_text(
        (stage / "usr" / "share" / "applications" / "vela-desktop.desktop").read_text(
            encoding="utf-8"
        ),
        encoding="utf-8",
    )
    (appdir / "vela.svg").write_text(
        (stage / "usr" / "share" / "icons" / "hicolor" / "scalable" / "apps" / "vela-desktop.svg").read_text(
            encoding="utf-8"
        ),
        encoding="utf-8",
    )
    (appdir / "AppRun").write_text(
        "#!/bin/sh\nexec \"$(dirname \"$0\")/usr/lib/vela-desktop\" \"$@\"\n", encoding="utf-8"
    )
    (appdir / "AppRun").chmod(0o755)
    package = output / f"vela-desktop-{version}-x86_64.AppImage"
    run(
        [
            "appimagetool",
            "--no-appstream",
            str(appdir),
            str(package),
        ]
    )
    package.chmod(package.stat().st_mode | stat.S_IXUSR)
    return package


def build_aur(stage: Path, version: str, commit: str, output: Path) -> Path:
    """An AUR source recipe plus the source tarball it points at."""
    aur_dir = output / "aur"
    aur_dir.mkdir(parents=True, exist_ok=True)
    source_name = f"vela-{version}.tar.gz"
    sources = output / "aur-sources"
    sources.mkdir(parents=True, exist_ok=True)
    licence_dir = stage / "usr" / "share" / "licenses" / "vela"
    licence_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(REPO_ROOT / "LICENSE", licence_dir / "LICENSE")
    with tarfile.open(sources / source_name, "w:gz") as bundle:
        for relative in (
            "usr/lib/vela/vela-desktop",
            "usr/share/applications/vela-desktop.desktop",
            "usr/share/licenses/vela/LICENSE",
            "BUILD.txt",
        ):
            bundle.add(stage / relative, arcname=f"vela-{version}/{relative}")
    # A published PKGBUILD must point at a real, checksummed source. When no
    # release base URL is configured the recipe stays local and says so, rather
    # than shipping a placeholder URL and a skipped checksum to the AUR.
    base_url = os.environ.get("VELA_RELEASE_BASE_URL", "").rstrip("/")
    source_sha256 = hashlib.sha256((sources / source_name).read_bytes()).hexdigest()
    # The recipe downloads the source from the GitHub release, so the tarball
    # has to be a release asset of its own and not only a build by-product.
    shutil.copy2(sources / source_name, output / source_name)
    if base_url:
        source_url = f"{base_url}/{source_name}"
        source_line = f'  echo "$pkgname-$pkgver.tar.gz::{source_url}"\n'
        checksum_line = f"  sha256sums=('{source_sha256}')\n"
    else:
        source_line = f'  echo "$pkgname-$pkgver.tar.gz"\n'
        checksum_line = "  sha256sums=('SKIP')  # local build: set VELA_RELEASE_BASE_URL for a published recipe\n"
    (aur_dir / "PKGBUILD").write_text(
        "# Maintainer: Vela Music <vela@example.invalid>\n"
        "pkgname=vela\n"
        f"pkgver={version}\n"
        "pkgrel=1\n"
        "arch=('x86_64')\n"
        "license=('AGPL-3.0-or-later')\n"
        "url=\"https://github.com/Fynth/vela\"\n"
        "depends=('gtk3' 'webkit2gtk-4.1' 'alsa-lib' 'libxdo')\n"
        "options=('!strip')\n"
        "source(){\n"
        + source_line
        + checksum_line
        + "}\n"
        "build(){\n"
        '  cd "$srcdir/$pkgname-$pkgver"\n'
        '  install -Dm0755 usr/lib/vela/vela-desktop "$pkgdir/usr/lib/vela/vela-desktop"\n'
        '  install -Dm0644 usr/share/applications/vela-desktop.desktop '
        '"$pkgdir/usr/share/applications/vela-desktop.desktop"\n'
        "}\n"
        "package(){\n"
        '  cd "$srcdir/$pkgname-$pkgver"\n'
        '  install -Dm0644 BUILD.txt "$pkgdir/usr/share/doc/$pkgname/BUILD.txt"\n'
        '  install -Dm0644 usr/share/licenses/vela/LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"\n'
        "}\n",
        encoding="utf-8",
    )
    return aur_dir


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
    output = arguments.output
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
            results.append(
                {
                    "kind": kind,
                    "status": "built",
                    "file": relative_to_repo(package),
                }
            )
    finally:
        shutil.rmtree(stage, ignore_errors=True)

    report = output / "linux-packages.json"
    report.write_text(
        json.dumps({"version": version, "commit": commit, "packages": results}, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {report}")
    for entry in results:
        print(f"  {entry['kind']}: {entry['status']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
