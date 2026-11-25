#!/usr/bin/env python3
"""The server image copies in every repository file tools/engine/engine-files.sh reads, the helper scripts it runs and
the files those read (engine/vendor comes from engine/fetch.sh inside the image). v0.1.17's image build failed on
engine/added-cards.json and v0.1.23's on tools/engine/engine-scripts.py, which only the image build itself would have
shown: this checks it in a second, without Docker."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


def copied(dockerfile):
    """The build context paths the Dockerfile's COPY lines take (not those from other stages)."""
    out = []
    for line in dockerfile.splitlines():
        words = line.split()
        if words[:1] != ['COPY'] or any(w.startswith('--from=') for w in words):
            continue
        out += [w for w in words[1:-1] if not w.startswith('--')]
    return out


def read_by_script(script):
    """The engine/ files the script reads, but the fetched ones (engine/vendor)."""
    found = re.findall(r'(?<![\w/])engine/[\w./*-]+', script)
    return sorted({p for p in found if not p.startswith('engine/vendor')})


def helpers(script):
    """The scripts beside it that the script runs (`$(dirname "$0")/NAME`), as repository paths."""
    return sorted({'tools/engine/' + n for n in re.findall(r'\$\(dirname "\$0"\)/([\w.-]+)', script)})


class ServerImageInputs(unittest.TestCase):
    def test_every_engine_file_the_script_reads_is_copied_in(self):
        sources = copied((ROOT / 'deploy/Dockerfile.server').read_text())
        script = (ROOT / 'tools/engine/engine-files.sh').read_text()
        runs = helpers(script)
        self.assertIn('tools/engine/engine-scripts.py', runs)
        needed = read_by_script(script) + runs
        for helper in runs:
            needed += read_by_script((ROOT / helper).read_text())
        self.assertIn('engine/added-cards.json', needed)
        self.assertIn('engine/anime/cards.tsv', needed)
        missing = [p for p in needed if not any(p == s or p.startswith(s.rstrip('/') + '/') for s in sources)]
        self.assertEqual(missing, [], 'deploy/Dockerfile.server needs a COPY for these before engine-files.sh runs')

    def test_the_parsers(self):
        self.assertEqual(copied('COPY a b dest/\nCOPY --from=x /y /z\nCOPY --chmod=755 c /d\n'), ['a', 'b', 'c'])
        self.assertEqual(read_by_script('cp engine/a.json "$e"\ncp engine/vendor/x "$e"\nsed engine/fetch.sh'), ['engine/a.json', 'engine/fetch.sh'])
        self.assertEqual(helpers('x=$(python3 "$(dirname "$0")/engine-scripts.py" --root .)'), ['tools/engine/engine-scripts.py'])


if __name__ == '__main__':
    unittest.main()
