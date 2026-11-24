"""The card scripts a release ships (engine-scripts.py): the game's, the added and the anime cards' and what they reference,
the shared files always. Skips without the pinned engine data (engine/fetch.sh)."""
import importlib.util
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("engine_scripts", Path(__file__).with_name("engine-scripts.py"))
es = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(es)
VENDOR = es.ROOT / "engine" / "vendor"


@unittest.skipUnless((VENDOR / "CardScripts").is_dir() and (VENDOR / "cardinfo.json").is_file(), "engine/fetch.sh")
class EngineScripts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.kept = set(es.closure(VENDOR, es.wanted()))

    def test_every_wanted_card_with_a_script_ships(self):
        files = es.scripts(VENDOR)
        missing = [c for c in es.wanted() if c in files and not set(files[c]) <= self.kept]
        self.assertEqual(missing, [])

    def test_what_scripts_bring_in_ships(self):
        # Orichalcos Shunoros (170000167) is summoned by Kyutora's script, not a card of any deck; Divine Serpent Geh's
        # anime script by Shunoros'; c420.lua is loaded by name; proc_unofficial.lua is the anime cards' procedures.
        for rel in ("unofficial/c170000167.lua", "unofficial/c170000170.lua", "unofficial/c420.lua", "unofficial/proc_unofficial.lua"):
            self.assertIn(rel, self.kept)

    def test_it_is_far_fewer_than_all(self):
        every = sum(len(v) for v in es.scripts(VENDOR).values())
        self.assertLess(len(self.kept), every / 2)


if __name__ == "__main__":
    unittest.main()
