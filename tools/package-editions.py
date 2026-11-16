#!/usr/bin/env python3
"""Split prebuilt platform files into Desktop, Scripts and (Linux) Server releases with stable archive roots."""
from pathlib import Path
import os
import re
import shutil
import subprocess
import sys
import tarfile

REPO = Path(__file__).resolve().parent.parent
# What only the players' editions carry: the launchers, the connection DLL and its loader, the play scripts.
PLAYER_FILES = ("yo3-revival", "yo3-revival.exe", "yo3-launcher", "yo3-launcher.exe", "yo3-launcher-windows.exe", "yo3-auth.dll",
                "yo3-auth-launch.exe", "yo3-bootstrap.dll", "yo3-revival.ico", "yo3-revival.png")


def editions(platform):
    """The editions of a platform: the server bundle is for a Linux server (#37)."""
    return ("desktop", "scripts", "server") if platform.startswith("linux") else ("desktop", "scripts")


def relocated(guide):
    """A guide from docs/ as an edition's entry README: sibling-document links point into docs/, anchors and URLs stay."""
    return re.sub(r"\]\((?![a-z]+:|#)([^)]+)\)", r"](docs/\1)", (REPO / "docs" / guide).read_text())


def prepare_server(destination):
    """The server bundle: the server, the rules engine, the systemd units, the monitor and off-machine backup scripts,
    an example server file and the hosting guide as its README. Nothing a player runs."""
    for name in PLAYER_FILES:
        (destination / name).unlink(missing_ok=True)
    (destination / "README.md").write_text(relocated("hosting.md"))
    host = REPO / "tools/host"
    shutil.copyfile(host / "server.toml.example", destination / "server.toml.example")
    units = destination / "systemd"
    units.mkdir()
    for source in sorted(host.iterdir()):
        if source.name == "server.toml.example":
            continue
        shutil.copyfile(source, units / source.name)
        (units / source.name).chmod(0o755 if source.suffix == ".sh" else 0o644)


def prepare(source, destination, edition, windows):
    shutil.copytree(source, destination)
    for name in ("run.sh", "run.cmd"):
        (destination / name).unlink(missing_ok=True)
    if edition == "server":
        return prepare_server(destination)
    if edition == "desktop":
        branding = REPO / "crates/yo3-desktop/assets/branding"
        for source_name, bundle_name in (("launcher.ico", "yo3-revival.ico"), ("launcher-window.png", "yo3-revival.png")):
            shutil.copyfile(branding / source_name, destination / bundle_name)
        return
    for name in ("yo3-revival", "yo3-revival.exe", "yo3-bootstrap.dll", "yo3-launcher-windows.exe", "yo3-revival.ico", "yo3-revival.png"):
        (destination / name).unlink(missing_ok=True)
    # The Scripts guide lives in docs/ in source, but is also this edition's entry README.
    # Preserve its local anchors and URLs while relocating sibling-document links.
    (destination / "README.md").write_text(relocated("scripts.md"))
    if windows:
        scripts = {
            "start-server.cmd": '@echo off\ncd /d "%~dp0"\n"%~dp0yo3-server.exe" --bind 0.0.0.0 --data-dir data --log-dir logs %*\n',
            "play.cmd": '@echo off\n"%~dp0yo3-launcher.exe" --injector "%~dp0yo3-auth-launch.exe" --dll "%~dp0yo3-auth.dll" %*\n',
        }
        for name, text in scripts.items():
            text += 'set "YO3_SCRIPT_EXIT=%errorlevel%"\nif not "%YO3_SCRIPT_EXIT%"=="0" pause\nexit /b %YO3_SCRIPT_EXIT%\n'
            (destination / name).write_bytes(text.replace('\n', '\r\n').encode())
    else:
        scripts = {
            "start-server.sh": '#!/bin/sh\ncd "$(dirname "$0")" || exit 1\nexec ./yo3-server --bind 0.0.0.0 --data-dir data --log-dir logs "$@"\n',
            "play.sh": '#!/bin/sh\nbundle=$(CDPATH= cd -P "$(dirname "$0")" && pwd) || exit 1\nexec "$bundle/yo3-launcher" --injector "$bundle/yo3-auth-launch.exe" --dll "$bundle/yo3-auth.dll" "$@"\n',
        }
        for name, text in scripts.items():
            path = destination / name
            path.write_text(text)
            path.chmod(0o755)


def archive(directory, dist, version, platform):
    name = f"{directory.name}-{version}-{platform}"
    if platform.startswith("windows"):
        return Path(shutil.make_archive(str(dist / name), "zip", directory.parent, directory.name))
    path = dist / f"{name}.tar.gz"
    with tarfile.open(path, "w:gz") as tar:
        tar.add(directory, arcname=directory.name)
    return path


def main():
    source, platform, version, dist = sys.argv[1:]
    if platform not in ("linux-x86_64", "windows-x86_64") or not re.fullmatch(r"[A-Za-z0-9._+-]+", version):
        sys.exit("Unsupported platform or version")
    dist = Path(dist).resolve()
    windows = platform.startswith("windows")
    target = "x86_64-pc-windows-gnu" if windows else "x86_64-unknown-linux-gnu"
    for edition in editions(platform):
        directory = dist / ".editions" / platform / edition / f"yo3-revival-{edition}"
        if directory.exists():
            shutil.rmtree(directory)
        prepare(Path(source), directory, edition, windows)
        subprocess.run([sys.executable, str(REPO / "tools/bundle-licenses.py"), str(directory), target, edition], cwd=REPO, check=True)
        subprocess.run([sys.executable, str(REPO / "tools/bundle-manifest.py"), str(directory), edition], cwd=REPO,
                       env={**os.environ, "YO3_BUILD_VERSION": version}, check=True)
        subprocess.run([sys.executable, str(REPO / "tools/docs-check.py"), "--root", str(directory)], cwd=REPO, check=True)
        print(archive(directory, dist, version, platform))


if __name__ == "__main__":
    main()
