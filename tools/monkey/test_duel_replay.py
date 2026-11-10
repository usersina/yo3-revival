import importlib.util
import json
from pathlib import Path
import sys
import unittest

spec = importlib.util.spec_from_file_location('duel_replay', Path(__file__).with_name('duel-replay.py'))
replay = importlib.util.module_from_spec(spec)
spec.loader.exec_module(replay)

class ReplayTests(unittest.TestCase):
    identity = dict(node='v24.0.0', **{k: 'a'*64 for k in ['sidecar', 'core', 'bindings', 'scripts', 'database', 'card_ids']})
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

    def test_a_missing_dependency_identity_is_not_a_reproducible_recording(self):
        records = self.records()
        for r in records:
            if r['call'] == 1 and r['direction'] == 'answer': r['data'] = ['7b7d' if r['part'] == 0 else '']
        with self.assertRaises(ValueError): replay.assemble(records, 1)

if __name__ == '__main__': unittest.main()
