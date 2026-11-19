#!/usr/bin/env python3
"""tools/client/ui-scenario.py: the step language, the expectations' timing, and the scenarios in the repository."""
import importlib.util
import json
import os
import stat
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).parent
SPEC = importlib.util.spec_from_file_location('ui_scenario', HERE / 'ui-scenario.py')
scn = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(scn)


class Clock:
    def __init__(self):
        self.now = 100.0

    def __call__(self):
        return self.now

    def sleep(self, s):
        self.now += s


STATE = {'components': [{'class': 'CDeckEditScene', 'depth': 0, 'key': 1}],
         'texts': [{'id': 5, 'text': 'Input text.', 'ago_ms': 1000}, {'id': 6, 'text': 'Old dialog', 'ago_ms': 60000}],
         'flash': [{'call': 'SetCardList / Red-Eyes Black Dragon', 'ago_ms': 500}]}


class Parse(unittest.TestCase):
    def test_steps_and_quoting(self):
        steps = scn.parse('# a comment\nclick 10 20\ntype "Red Eyes"\nkey ctrl+a BackSpace\nexpect text "Input text." 5\n\nshot after\n')
        self.assertEqual([s[1] for s in steps], ['click', 'type', 'key', 'expect', 'shot'])
        self.assertEqual(steps[1], (3, 'type', ['Red Eyes']))
        self.assertEqual(steps[3][2], ['text', 'Input text.', '5'])

    def test_mistakes_name_their_line(self):
        for bad in ['click 10', 'click a b', 'jump 1', 'expect colour red', 'expect text', 'type', 'key', 'sleep soon', 'type "open']:
            with self.subTest(bad=bad), self.assertRaises((scn.ScenarioError, ValueError)):
                scn.parse('click 1 2\n' + bad)
        with self.assertRaisesRegex(scn.ScenarioError, 'line 2'):
            scn.parse('click 1 2\njump')


class Expect(unittest.TestCase):
    def test_only_what_came_after_the_last_action_counts(self):
        self.assertTrue(scn.seen(STATE, 'text', 'input', since=2))
        self.assertFalse(scn.seen(STATE, 'text', 'old dialog', since=2))
        self.assertTrue(scn.seen(STATE, 'text', 'old dialog', since=70))
        self.assertTrue(scn.seen(STATE, 'flash', 'red-eyes', since=1))
        self.assertTrue(scn.seen(STATE, 'component', 'deckedit', since=0))

    def test_waits_then_fails_with_what_was_missing(self):
        c = Clock()
        with self.assertRaisesRegex(scn.ScenarioError, "no text 'nothing' within 2 s"):
            scn.expect('text', 'nothing', 2, c.now, read=lambda: STATE, clock=c, sleep=c.sleep)
        self.assertGreaterEqual(c.now, 102)

    def test_negative_expectations(self):
        c = Clock()
        scn.expect('no-text', 'nothing', 1, c.now, read=lambda: STATE, clock=c, sleep=c.sleep)
        with self.assertRaisesRegex(scn.ScenarioError, 'was shown'):
            scn.expect('no-flash', 'Red-Eyes', 1, c.now - 1, read=lambda: STATE, clock=c, sleep=c.sleep)


class Run(unittest.TestCase):
    def test_a_scenario_drives_vclient_and_a_failure_takes_a_screenshot(self):
        with tempfile.TemporaryDirectory() as d:
            calls = Path(d, 'calls')
            fake = Path(d, 'vclient.sh')
            fake.write_text(f'#!/bin/sh\necho "$@" >> {calls}\n[ "$1 $2" = "ui --raw" ] && cat {Path(d, "ui.json")}\nexit 0\n')
            fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
            Path(d, 'ui.json').write_text(json.dumps(STATE))
            old = scn.VCLIENT
            scn.VCLIENT = str(fake)
            try:
                ok = scn.run(scn.parse('click 1 2\nexpect component DeckEdit 1\n'), 'ok.scn')
                bad = scn.run(scn.parse('type x\nexpect component Shop 0\n'), 'bad.scn')
            finally:
                scn.VCLIENT = old
            log = calls.read_text().splitlines()
        self.assertEqual((ok, bad), (0, 1))
        self.assertIn('click 1 2', log)
        self.assertIn('type x', log)
        self.assertIn('shot scenario-fail-2', log)


class Scenarios(unittest.TestCase):
    def test_the_repositorys_scenarios_parse(self):
        files = sorted((HERE / 'scenarios').glob('*.scn'))
        self.assertTrue(files)
        for f in files:
            with self.subTest(f=f.name):
                self.assertTrue(scn.parse(f.read_text()))


if __name__ == '__main__':
    unittest.main()
