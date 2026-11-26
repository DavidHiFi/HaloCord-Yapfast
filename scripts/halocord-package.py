#!/usr/bin/env python3
"""HaloCord's release files, from the Windows package scripts/build-windows
makes: the one place that knows their names and layout, for the numbered
releases and for the nightly.

  HaloCord-Yapfast-<version>-<N>-windows-x64.zip
  HaloCord-Yapfast-<version>-<N>-nightly.<YYYYMMDD>.<run>-windows-x64.zip

Each ZIP holds one folder of the same name with yapfast.exe and Yapfast's
own files, SOURCE.txt saying where HaloCord's source is. SHA256SUMS.txt
beside it checks the ZIP and HaloCord-Installer.exe, which every release
carries. The updater (crates/yapfast-iced/src/halocord/updates.rs) reads a
build's number from the ZIP's name, so the two change together.

  scripts/halocord-package.py --build 18 --tag v0.5.0-...-halocord.18 \\
      --commit SHA --installer HaloCord-Installer.exe --output release
  scripts/halocord-package.py --build 18 --nightly 20261126.7 ...
"""
import argparse
import re
import shutil
import sys
import tempfile
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from release_package import ROOT, digest, zip_directory  # noqa: E402

REPO = "DavidHiFi/HaloCord-Yapfast"
INSTALLER = "HaloCord-Installer.exe"
SUMS = "SHA256SUMS.txt"
UPSTREAM_ZIP = re.compile(r"Yapfast-(?P<version>.+)-windows-x64\.zip")
NIGHTLY = re.compile(r"\d{8}\.\d+")


def package_name(version, build, nightly=None):
    """The ZIP's name without `.zip`, and the folder inside it."""
    stamp = f"-nightly.{nightly}" if nightly else ""
    return f"HaloCord-Yapfast-{version}-{build}{stamp}-windows-x64"


def source_text(version, build, tag, commit, nightly=None):
    """SOURCE.txt: where this build's source is, HaloCord's and Yapfast's."""
    which = f"the nightly of {nightly} after {build}" if nightly else str(build)
    base = tag.split("-halocord.", 1)[0] if "-halocord." in tag else f"v{version}"
    return (
        f"The source code of this build, HaloCord for Yapfast {version}, {which},\r\n"
        "is published with it:\r\n\r\n"
        f"  https://github.com/{REPO}/releases/tag/{tag}\r\n"
        f"  commit {commit}\r\n\r\n"
        f"It is Yapfast {version} with HaloCord's commits on top.\r\n"
        "Yapfast's own source for that release, vendored libraries included:\r\n\r\n"
        f"  https://github.com/yapfast/yapfast/releases/tag/{base}\r\n\r\n"
        "The licenses folder holds every dependency's own license and notices.\r\n"
    )


def upstream_zip(folder):
    """The one Windows ZIP scripts/build-windows left in `folder`."""
    found = [path for path in folder.glob("Yapfast-*-windows-x64.zip") if UPSTREAM_ZIP.fullmatch(path.name)]
    if len(found) != 1:
        raise SystemExit(f"expected one Yapfast-*-windows-x64.zip in {folder}, found {len(found)}")
    return found[0], UPSTREAM_ZIP.fullmatch(found[0].name)["version"]


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--build", type=int, required=True, help="the numbered release this build is or follows")
    parser.add_argument("--nightly", help="YYYYMMDD.run, for a nightly")
    parser.add_argument("--tag", required=True, help="the release's tag, for SOURCE.txt")
    parser.add_argument("--commit", required=True, help="the commit built, for SOURCE.txt")
    parser.add_argument("--installer", type=Path, required=True, help=f"{INSTALLER} to publish beside the ZIP")
    parser.add_argument("--dist", type=Path, default=ROOT / "target/dist/discord",
                        help="where scripts/build-windows put its package")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.build < 1:
        raise SystemExit("--build is a release number, 1 or more")
    if args.nightly and not NIGHTLY.fullmatch(args.nightly):
        raise SystemExit("--nightly is YYYYMMDD.run")
    if not args.installer.is_file():
        raise SystemExit(f"{args.installer} isn't there")

    source, version = upstream_zip(args.dist)
    name = package_name(version, args.build, args.nightly)
    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output / f"{name}.zip"
    with tempfile.TemporaryDirectory(prefix="halocord-package-") as temporary:
        temporary = Path(temporary)
        with zipfile.ZipFile(source) as stream:
            stream.extractall(temporary)
        folders = [path for path in temporary.iterdir() if path.is_dir()]
        if len(folders) != 1 or not (folders[0] / "yapfast.exe").is_file():
            raise SystemExit(f"{source.name} isn't one folder holding yapfast.exe")
        package = folders[0].rename(temporary / name)
        (package / "SOURCE.txt").write_bytes(
            source_text(version, args.build, args.tag, args.commit, args.nightly).encode("utf-8"))
        zip_directory(package, archive)
    installer = args.output / INSTALLER
    if args.installer.resolve() != installer.resolve():
        shutil.copy2(args.installer, installer)
    (args.output / SUMS).write_text(
        "".join(f"{digest(path)}  {path.name}\n" for path in (archive, installer)), encoding="utf-8")
    print(f"Packaged {archive} ({archive.stat().st_size / 1024 / 1024:.1f} MiB), {INSTALLER} and {SUMS}")


if __name__ == "__main__":
    main()
