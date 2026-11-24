#!/usr/bin/env python3
"""Hermetic checks for duel-state.py: the newest complete engine state of a duel, read from yo3-log/1 or older lines."""
import contextlib
import importlib.util
import io
import json
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("duel_state", Path(__file__).with_name("duel-state.py"))
duel_state = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(duel_state)


def chunks(duel, call, answer, parts=2):
    """The engine's logged answer, hex in `parts` chunks, as the server writes it."""
    data = json.dumps(answer).encode().hex()
    size = -(-len(data) // parts)
    return [{"call": call, "data": [data[i * size:(i + 1) * size]], "direction": "answer", "duel": duel, "encoding": "hex",
             "part": i, "parts": parts} for i in range(parts)]


def state(lp, hand_codes):
    return {"status": "state", "teams": [
        {"lp": lp, "deck": 30, "hand": [{"code": c, "cid": 1} for c in hand_codes], "mzone": [None, {"code": 89631139, "attack": 3000, "defense": 2500, "position": 1}],
         "szone": [], "grave": [], "removed": []},
        {"lp": 8000, "deck": 33, "hand": [], "mzone": [], "szone": [{"code": 53129443, "position": 10}], "grave": [], "removed": []}]}


class DuelState(unittest.TestCase):
    def test_the_newest_complete_state_of_the_newest_duel_by_name(self):
        lines = []
        for c in chunks(1, 5, state(8000, [1])) + chunks(2, 3, state(7000, [2])) + chunks(2, 4, {"messages": []}):
            lines.append(json.dumps({"ts": "2026-11-24T10:00:00.000Z", "level": "info", "src": "engine", "session": "s", "msg": json.dumps(c)}))
        partial = chunks(2, 6, state(1, [3]))[:1]  # a state whose second chunk never came
        lines += ["2026-11-24T10:00:01Z  INFO [YO3 ENGINE] " + json.dumps(c) for c in partial]
        duel, answer = duel_state.newest_state(duel_state.records(lines))
        self.assertEqual((duel, answer["teams"][0]["lp"]), (2, 7000), "duel 2's call 3: call 4 holds no teams, call 6 is incomplete")
        self.assertEqual(duel_state.newest_state(duel_state.records(lines), duel=1)[1]["teams"][0]["lp"], 8000)
        text = duel_state.describe(answer, duel, {2: "Divine Serpent Geh", 89631139: "Blue-Eyes White Dragon", 53129443: "Dark Hole"})
        self.assertIn("hand: Divine Serpent Geh", text)
        self.assertIn("Blue-Eyes White Dragon 3000/2500 ATK (zone 1)", text)
        self.assertIn("spells and traps: Dark Hole (set)", text)

    def test_the_question_asked_now_names_each_choice(self):
        # Illusion Gate's selection: two cards of the opponent's Graveyard, one an added card shown as Artwork not found.
        select = {"type": 15, "player": 0, "min": 1, "max": 1, "can_cancel": False, "selects": [
            {"cid": 6960, "code": 69247929, "controller": 1, "location": 16}, {"cid": 12671, "code": 170000170, "controller": 1, "location": 16}]}
        lines = [json.dumps({"ts": "2026-11-24T10:00:00.000Z", "level": "info", "src": "engine", "session": "s", "msg": json.dumps(c)})
                 for c in chunks(1, 4, {"status": "await", "messages": [{"type": 2}, select]}) + chunks(1, 5, state(4000, []))]
        prompt = duel_state.newest_prompt(duel_state.records(lines), 1)
        text = duel_state.describe_prompt(prompt, {69247929: "Gene-Warped Warwolf", 170000170: "Divine Serpent Geh (Anime)"})
        self.assertIn("team 0 is asked to select cards (1 to 1)", text)
        self.assertIn("1. Gene-Warped Warwolf, Graveyard (the opponent's)", text)
        self.assertIn("2. Divine Serpent Geh (Anime), Graveyard (the opponent's)", text)
        ended = [json.dumps({"ts": "t", "level": "info", "src": "engine", "session": "s", "msg": json.dumps(c)})
                 for c in chunks(1, 4, {"status": "await", "messages": [select]}) + chunks(1, 6, {"status": "end", "messages": []})]
        self.assertIsNone(duel_state.newest_prompt(duel_state.records(ended), 1), "an ended duel asks nothing")

    def test_a_log_without_engine_states_says_so(self):
        with tempfile.NamedTemporaryFile("w", suffix=".log", delete=False) as f:
            f.write("2026-11-24T10:00:00Z  INFO nothing here\n")
        with self.assertRaises(SystemExit) as e, contextlib.redirect_stdout(io.StringIO()):
            duel_state.main(["--server", f.name])
        self.assertIn("--duel-check", str(e.exception))
        Path(f.name).unlink()


if __name__ == "__main__":
    unittest.main()
