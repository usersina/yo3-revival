#!/usr/bin/env python3
"""The release's version and notes, from CHANGELOG.md's newest section (`## vX.Y.Z (date)`).

    release_notes.py version CHANGELOG.md        prints the newest version, e.g. v0.0.1
    release_notes.py notes CHANGELOG.md DIR      prints its notes: the section, then each file of DIR's SHA-256
"""
import hashlib
import pathlib
import re
import sys

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


def main():
    if len(sys.argv) < 3 or sys.argv[1] not in ("version", "notes"):
        sys.exit(__doc__)
    version, body = newest(sys.argv[2])
    if sys.argv[1] == "version":
        print(version)
        return
    files = sorted(p for p in pathlib.Path(sys.argv[3]).iterdir() if p.is_file())
    if not files:
        sys.exit(f"{sys.argv[3]}: no files to list")
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
    print("\nThe folders inside are always `yo3-revival-desktop/` and `yo3-revival-scripts/`, without a version number.")
    print("\n## Checksums (SHA-256)\n")
    print("Windows may warn about these unsigned applications and the connection DLL; compare the archive's checksum, or build from source.\n")
    print("```")
    for f in files:
        print(f"{sha256(f)}  {f.name}")
    print("```")
    print("\nFan preservation project, not affiliated with or endorsed by Konami. No game files are included: you need your"
          " own copy of the client. MIT licensed; the bundled rules engine is other people's open-source work (see"
          " `engine/THIRD-PARTY.md` in a bundle).")


if __name__ == "__main__":
    main()
