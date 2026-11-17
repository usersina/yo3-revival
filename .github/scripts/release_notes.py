#!/usr/bin/env python3
"""The release's version and notes, from CHANGELOG.md's newest section (`## vX.Y.Z (date)`).

    release_notes.py version CHANGELOG.md        prints the newest version, e.g. v0.0.1
    release_notes.py check CHANGELOG.md Cargo.toml checks the workspace version and prints the release version
    release_notes.py notes CHANGELOG.md DIR      prints its notes: the section, then each file of DIR's SHA-256
    release_notes.py sums DIR                    writes DIR/SHA256SUMS: each other file's SHA-256, as sha256sum prints it
"""
import hashlib
import pathlib
import re
import sys
import tomllib

HEADING = re.compile(r"^## (v\d+\.\d+\.\d+)\b")


def newest(changelog):
    """(version, body lines) of the first `## vX.Y.Z` section; exits if there is none."""
    lines = pathlib.Path(changelog).read_text(encoding="utf-8").splitlines()
    for i, line in enumerate(lines):
        m = HEADING.match(line)
        if m:
            end = next((j for j in range(i + 1, len(lines)) if lines[j].startswith("## ")), len(lines))
            return m.group(1), lines[i + 1 : end]
    sys.exit(f"{changelog}: no section headed `## vX.Y.Z`")


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


SUMS = "SHA256SUMS"
SIGNATURE = "SHA256SUMS.sig"
ANTIVIRUS = "https://github.com/usersina/yo3-revival/blob/main/docs/antivirus.md"


def archives(directory):
    """The release's files in DIR, by name, without the checksum file and its signature."""
    files = sorted((p for p in pathlib.Path(directory).iterdir() if p.is_file() and p.name not in (SUMS, SIGNATURE)), key=lambda p: p.name)
    if not files:
        sys.exit(f"{directory}: no files to list")
    return files


def sums(directory):
    """Writes DIR/SHA256SUMS (`<hash>  <name>` lines, which `sha256sum -c` reads) and returns its text."""
    text = "".join(f"{sha256(f)}  {f.name}\n" for f in archives(directory))
    (pathlib.Path(directory) / SUMS).write_text(text, encoding="utf-8", newline="\n")
    return text


def main():
    if len(sys.argv) < 3 or sys.argv[1] not in ("version", "notes", "sums", "check"):
        sys.exit(__doc__)
    if sys.argv[1] == "sums":
        sums(sys.argv[2])
        return
    version, body = newest(sys.argv[2])
    if sys.argv[1] == "check":
        package=tomllib.loads(pathlib.Path(sys.argv[3]).read_text())["workspace"]["package"]["version"]
        if version != "v"+package:
            sys.exit(f"Release version mismatch: CHANGELOG.md says {version}, Cargo.toml says {package}. Update both together.")
        print(version)
        return
    if sys.argv[1] == "version":
        print(version)
        return
    files = archives(sys.argv[3])
    print("\n".join(body).strip())
    print("\n## Downloads\n")
    print("**Desktop** is recommended: a simple Play screen, optional advanced controls, and confirmed in-app updates. "
          "**Scripts** provides terminal start/play scripts without the GUI. Both include the server, engine and required connection helper/DLL. "
          "Extract the whole archive; original game files are not included. Windows 7 is not supported.\n")
    print("| Edition | Windows 10/11 (64-bit) | Linux (64-bit; Wine for the game) |")
    print("|---|---|---|")
    for edition in ("desktop", "scripts"):
        base = f"https://github.com/usersina/yo3-revival/releases/download/{version}/yo3-revival-{edition}-{version}"
        print(f"| {edition.title()} | [Download ZIP]({base}-windows-x86_64.zip) | [Download tar.gz]({base}-linux-x86_64.tar.gz) |")
    server = f"https://github.com/usersina/yo3-revival/releases/download/{version}/yo3-revival-server-{version}-linux-x86_64.tar.gz"
    if any(f.name.startswith("yo3-revival-server-") for f in files):
        print(f"| Server (a Linux server or VPS; no game client) | | [Download tar.gz]({server}) |")
    print("\nThe folders inside are always `yo3-revival-desktop/`, `yo3-revival-scripts/` and `yo3-revival-server/`, without a"
          " version number. The Server edition's README is the hosting guide.")
    print("\n## Checksums (SHA-256)\n")
    print("Windows may warn about these unsigned applications and the connection DLL. Before you allow anything, check that"
          f" your download matches the list below (also the `{SUMS}` file among this release's files). On Windows, in PowerShell,"
          " in the folder you downloaded to:\n")
    print("```powershell")
    print(f"Get-FileHash .\\yo3-revival-desktop-{version}-windows-x86_64.zip")
    print("```\n")
    print(f"The Hash it prints must equal the archive's line here (capitals don't matter). On Linux, `sha256sum -c {SUMS}"
          f" --ignore-missing` in the download folder checks every archive you have. `{SIGNATURE}` is the project's signature"
          " over that list, which Desktop checks before it installs an update. More:"
          f" [antivirus warnings and checking a download]({ANTIVIRUS}).\n")
    print("```")
    for f in files:
        print(f"{sha256(f)}  {f.name}")
    print("```")
    print("\nFan preservation project, not affiliated with or endorsed by Konami. No game files are included: you need your"
          " own copy of the client. MIT licensed; the bundled rules engine is other people's open-source work (see"
          " `engine/THIRD-PARTY.md` in a bundle).")


if __name__ == "__main__":
    main()
