import gzip
import importlib.util
import os
import tempfile
import json
from pathlib import Path
import sys
import unittest

spec = importlib.util.spec_from_file_location('duel_replay', Path(__file__).with_name('duel-replay.py'))
replay = importlib.util.module_from_spec(spec)
spec.loader.exec_module(replay)

class ReplayTests(unittest.TestCase):
    identity = dict(node='native 0.0.10', **{k: 'a'*64 for k in ['sidecar', 'core', 'bindings', 'scripts', 'database', 'card_ids']})
    def records(self):
        result = []
        for call in range(1, 3):
            for direction in ['request', 'answer']:
                body = json.dumps({'op': 'identity'} if direction == 'request' else self.identity if call == 1 else {'fixture': True})
                middle = len(body)//2
                for part, data in enumerate([body[:middle], body[middle:]]):
                    result.append(dict(duel=1, call=call, direction=direction, part=part, parts=2, encoding='hex',
                                       data=[data.encode().hex()]))
        return result

    def test_incomplete_duplicate_and_missing_calls_fail(self):
        records = self.records()
        self.assertEqual(len(replay.assemble(records, 1)), 2)
        for changed in [records[:-1], records + [records[0]], [r for r in records if r['call'] != 1]]:
            with self.assertRaises(ValueError): replay.assemble(changed, 1)

    def test_replay_checks_answers_and_never_executes_commands_from_the_recording(self):
        calls = replay.assemble(self.records(), 1)
        command = [sys.executable, '-u', '-c', f'import sys,json; identity={self.identity!r}; [print(json.dumps(identity if json.loads(line)["op"]=="identity" else {{"fixture":True}}),flush=True) for line in sys.stdin]']
        calls[1][0]['op'] = 'state'
        self.assertEqual(replay.run(calls, command), [])
        calls[1][1]['fixture'] = False
        self.assertTrue(replay.run(calls, command))
        calls[0][0]['op'] = 'shell'
        with self.assertRaises(ValueError): replay.run(calls, command)

    def test_the_bots_thinking_is_not_replayed(self):
        # think's answer depends on the machine's speed (the bot's time budget); the duel goes on through respond.
        calls = replay.assemble(self.records(), 1)
        command = [sys.executable, '-u', '-c', f'import sys,json; identity={self.identity!r}; [print(json.dumps(identity if json.loads(line)["op"]=="identity" else {{"fixture":True}}),flush=True) for line in sys.stdin]']
        calls[1][0]['op'] = 'state'
        calls.append(({'op': 'think', 'style': {}}, {'status': 'thought', 'response': {'type': 1}, 'ms': 5}))
        self.assertEqual(replay.run(calls, command), [])

    def test_hints_may_come_in_another_order_but_nothing_else(self):
        hint = lambda n: {'type': 160, 'controller': 0, 'location': 4, 'sequence': n, 'position': 1, 'card_hint': 7, 'description': 1}
        move = {'type': 50, 'card': 1}
        recorded = {'status': 'await', 'messages': [move, hint(1), hint(2), move]}
        self.assertEqual(replay.canonical(recorded), replay.canonical({**recorded, 'messages': [move, hint(2), hint(1), move]}))
        self.assertNotEqual(replay.canonical(recorded), replay.canonical({**recorded, 'messages': [hint(1), move, hint(2), move]}))

    def test_a_missing_dependency_identity_is_not_a_reproducible_recording(self):
        records = self.records()
        for r in records:
            if r['call'] == 1 and r['direction'] == 'answer': r['data'] = ['7b7d' if r['part'] == 0 else '']
        with self.assertRaises(ValueError): replay.assemble(records, 1)



class RecordingTests(unittest.TestCase):
    identity = ReplayTests.identity
    def write(self, rows, cut=0):
        path = Path(tempfile.mkdtemp())/'20261122-120000-001.jsonl.gz'
        # Flushed line by line, as the server writes (a sync flush after each), so a cut file keeps its earlier lines.
        with gzip.open(path, 'wb') as f:
            for r in rows: f.write((json.dumps(r)+'\n').encode()); f.flush()
        if cut: path.write_bytes(path.read_bytes()[:-cut])
        return path

    def header(self, **changes):
        return {'format': 'yo3-duel-recording', 'version': 1, 'id': '20261122-120000-001', 'kind': 'npc', 'anime': False,
                'players': [{'player_id': 1001, 'name': 'Yugi'}, {'player_id': None, 'name': 'Pegasus'}], 'identity': self.identity, **changes}

    def test_a_server_recording_replays_with_its_identity_first(self):
        rows = [self.header(), {'call': 1, 'request': {'op': 'new', 'seed': 1}, 'answer': {'fixture': True}},
                {'call': 2, 'request': {'op': 'respond', 'response': {}}, 'answer': {'fixture': True}}]
        header, calls = replay.read_recording(self.write(rows))
        self.assertEqual((header['kind'], len(calls), calls[0][0], calls[1][0]['op']), ('npc', 3, {'op': 'identity'}, 'new'))
        command = [sys.executable, '-u', '-c', f'import sys,json; identity={self.identity!r}; [print(json.dumps(identity if json.loads(line)["op"]=="identity" else {{"fixture":True}}),flush=True) for line in sys.stdin]']
        self.assertEqual(replay.run(calls, command, dict(os.environ)), [])

    def test_a_duel_still_running_reads_up_to_its_open_end(self):
        rows = [self.header()] + [{'call': n, 'request': {'op': 'respond', 'response': {'n': n}}, 'answer': {'x': 'y'*200}} for n in range(1, 40)]
        header, calls = replay.read_recording(self.write(rows, cut=30))
        self.assertGreater(len(calls), 1)
        self.assertEqual([r['response']['n'] for r, _ in calls[1:]], list(range(1, len(calls))))

    def test_a_recording_without_identity_or_with_gaps_is_refused(self):
        with self.assertRaises(ValueError): replay.read_recording(self.write([self.header(identity=None)]))
        with self.assertRaises(ValueError): replay.read_recording(self.write([self.header(), {'call': 2, 'request': {'op': 'new'}, 'answer': {}}]))
        with self.assertRaises(ValueError): replay.read_recording(self.write([{'format': 'other'}]))

if __name__ == '__main__': unittest.main()
