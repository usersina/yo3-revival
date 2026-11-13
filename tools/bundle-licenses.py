#!/usr/bin/env python3
"""Copy license notices for the Rust executables shipped in a platform bundle."""
import json
from pathlib import Path
import shutil
import subprocess
import sys

out, target, edition = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
roots = {"yo3-server", "yo3-launcher", "yo3-engine"}
if edition == "desktop":
    roots.add("yo3-desktop")
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", target], text=True))
packages = {p["id"]: p for p in metadata["packages"]}
nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
pending = [p["id"] for p in packages.values() if p["name"] in roots]
used = set()
while pending:
    package = pending.pop()
    if package in used:
        continue
    used.add(package)
    pending.extend(nodes[package]["dependencies"])

if edition == "desktop":
    shutil.copytree(Path("crates/yo3-desktop/notices"), out / "licenses/desktop", dirs_exist_ok=True)
    shutil.copyfile(Path("crates/yo3-desktop/assets/icons/LICENSE"), out / "licenses/LUCIDE.txt")
rows = []
for package in sorted((packages[p] for p in used), key=lambda p: (p["name"], p["version"])):
    if package["source"] is None:  # This project's own license is already in the bundle.
        continue
    root = Path(package["manifest_path"]).parent
    notices = [p for p in root.rglob("*") if p.is_file() and p.name.upper().startswith(("LICENSE", "COPYING", "UNLICENSE", "OFL"))]
    if package.get("license_file"):
        explicit = root / package["license_file"]
        if explicit.is_file() and explicit not in notices:
            notices.append(explicit)
    folder = Path("licenses") / f"{package['name']}-{package['version']}"
    for notice in notices:
        relative = notice.relative_to(root) if notice.is_relative_to(root) else Path(notice.name)
        dest = out / folder / relative
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(notice, dest)
    license_name = (package.get("license") or "see upstream source").replace("|", "\\|")
    url = package.get("repository") or f"https://crates.io/crates/{package['name']}"
    rows.append(f"| [{package['name']}]({url}) | {package['version']} | {license_name} | {'`' + str(folder) + '/`' if notices else 'see upstream'} |")
(out / "THIRD-PARTY.md").write_text(
    "# Third-party notices\n\nThis project's code is MIT licensed (LICENSE). Rust dependency notices are collected below.\n"
    "The rules engine (yo3-engine, AGPL-3.0-or-later: EDOPro's core is built into it) and its data are described in "
    "[engine/THIRD-PARTY.md](engine/THIRD-PARTY.md). No game files are included.\n\n"
    + ("The embedded Fira Sans font (SIL OFL 1.1) and Iced's MIT notice are in [licenses/desktop/](licenses/desktop/). "
       "Lucide icons (ISC and MIT notices) are in [licenses/LUCIDE.txt](licenses/LUCIDE.txt). "
       "Sources: https://github.com/mozilla/Fira , https://github.com/iced-rs/iced/tree/0.14.0 , https://github.com/lucide-icons/lucide .\n\n"
       if edition == "desktop" else "") +
    "| Crate | Version | License | Included notices |\n|---|---|---|---|\n" + "\n".join(rows) + "\n")
print(f"Collected Rust notices for {len(rows)} dependencies ({target}).")
