#!/usr/bin/env python3
"""Checks the Markdown docs and prints tables of contents.

    tools/docs-check.py                 every relative link and #anchor in README.md, CLAUDE.md and docs/*.md resolves
    tools/docs-check.py --toc FILE      a table of contents for FILE (H2 and H3 headings), ready to paste under its title

Anchors follow GitHub's rule: lowercase, drop everything but letters, digits, spaces, hyphens and underscores, spaces
become hyphens, repeated headings get -1, -2. Exit status 1 when a link is broken.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOCS = [ROOT / "README.md", ROOT / "CLAUDE.md", *sorted((ROOT / "docs").glob("*.md"))]
LINK = re.compile(r"(?<!\!)\[[^\]]*\]\(([^)\s]+)\)|!\[[^\]]*\]\(([^)\s]+)\)")


def lines_outside_code(path):
    fenced = False
    for line in path.read_text().splitlines():
        if line.lstrip().startswith("```"):
            fenced = not fenced
            continue
        if not fenced:
            yield line


def headings(path):
    """(level, text, anchor) for each heading, anchors made unique the way GitHub does."""
    seen, out = {}, []
    for line in lines_outside_code(path):
        m = re.match(r"^(#{1,6}) +(.*?) *#*$", line)
        if not m:
            continue
        text = re.sub(r"`([^`]*)`", r"\1", m.group(2))
        text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
        anchor = re.sub(r"[^\w\- ]", "", text.lower()).replace(" ", "-")
        n = seen.get(anchor, 0)
        seen[anchor] = n + 1
        out.append((len(m.group(1)), text, anchor if n == 0 else f"{anchor}-{n}"))
    return out


def toc(path):
    for level, text, anchor in headings(path):
        if level in (2, 3):
            print(f"{'  ' * (level - 2)}- [{text}](#{anchor})")


def check():
    bad = 0
    for doc in DOCS:
        anchors = {a for _, _, a in headings(doc)}
        for line in lines_outside_code(doc):
            line = re.sub(r"`[^`]*`", "", line)
            for m in LINK.finditer(line):
                target = m.group(1) or m.group(2)
                if re.match(r"[a-z]+:", target):
                    continue
                file_part, _, frag = target.partition("#")
                dest = doc if not file_part else (doc.parent / file_part).resolve()
                if not dest.exists():
                    print(f"{doc.relative_to(ROOT)}: missing file {target}")
                    bad += 1
                elif frag and dest.suffix == ".md":
                    if frag not in {a for _, _, a in headings(dest)}:
                        print(f"{doc.relative_to(ROOT)}: no anchor {target}")
                        bad += 1
    print("broken links:", bad)
    return 1 if bad else 0


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--toc":
        toc(Path(sys.argv[2]))
    else:
        sys.exit(check())
