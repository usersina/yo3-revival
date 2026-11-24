#!/usr/bin/env python3
"""The documentation a release bundle carries: README.md, docs/ and screenshots/ as in the repository, without the
developer documents (DEV_DOCS: the protocol, client internals, history, research, roadmap, development, dashboard and
deployment guides, which need the source anyway). A link to one of them from what stays points to the same file on
GitHub, at the release's tag (main for a build that has none), so `tools/docs/docs-check.py --root BUNDLE` stays clean.

    tools/release/bundle-docs.py DEST VERSION       (tools/release/release.sh)
"""
import re
import shutil
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
GITHUB = "https://github.com/usersina/yo3-revival/blob"
DEV_DOCS = ("protocol", "client-internals", "history", "research", "roadmap", "development", "dashboard", "deployment")
_LINK = re.compile(r"\]\((?:\./)?(docs/)?(" + "|".join(re.escape(d) for d in DEV_DOCS) + r")\.md(#[^)\s]*)?\)")


def ref(version):
    """The tag a released version's files are at, else main."""
    return version if re.fullmatch(r"v\d+\.\d+\.\d+", version or "") else "main"


def rewrite(text, in_docs, version):
    """`text`'s Markdown links to a developer document as GitHub links: `in_docs` for a file in docs/ (links are
    `protocol.md`), else the root README (links are `docs/protocol.md`)."""
    def to_github(m):
        if bool(m.group(1)) == in_docs:  # docs/x.md inside docs/, or x.md at the root: not a document of the repository
            return m.group(0)
        return f"]({GITHUB}/{ref(version)}/docs/{m.group(2)}.md{m.group(3) or ''})"
    return _LINK.sub(to_github, text)


def copy(dest, version, repo=REPO):
    dest = Path(dest)
    (dest / "docs").mkdir(parents=True, exist_ok=True)
    (dest / "README.md").write_text(rewrite((repo / "README.md").read_text(encoding="utf-8"), False, version), encoding="utf-8")
    for doc in sorted((repo / "docs").glob("*.md")):
        if doc.stem in DEV_DOCS:
            continue
        (dest / "docs" / doc.name).write_text(rewrite(doc.read_text(encoding="utf-8"), True, version), encoding="utf-8")
    shutil.copytree(repo / "screenshots", dest / "screenshots", dirs_exist_ok=True)
    shutil.copyfile(repo / "LICENSE", dest / "LICENSE")


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) != 2 or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0 if argv[:1] in (["-h"], ["--help"]) else 2
    copy(argv[0], argv[1])
    return 0


if __name__ == "__main__":
    sys.exit(main())
