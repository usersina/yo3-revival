import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("duel_report", Path(__file__).with_name("duel-report.py"))
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)

class NativeReportTests(unittest.TestCase):
    def test_missing_duplicate_and_failed_acknowledgements_never_pass(self):
        expected = [{"schema": 1, "sequence": 1, "expected": [123, 456]}]
        passed = expected[0] | {"status": "pass", "actual": [123, 456]}
        self.assertEqual(report.compare(expected, [passed]), [])
        for acknowledgements in [[], [passed, passed], [passed | {"status": "timeout"}], [passed | {"actual": [124, 456]}]]:
            self.assertTrue(report.compare(expected, acknowledgements))
        self.assertTrue(report.compare([], []))
        self.assertTrue(report.compare([expected[0], expected[0]], [passed, passed]), "duplicated server log parts")

    def test_identical_checkpoints_on_two_seats_require_two_acknowledgements(self):
        expected = {"schema": 1, "sequence": 1, "expected": [123, 456]}
        passed = expected | {"status": "pass", "actual": [123, 456]}
        other = expected | {"sequence": 2}
        other_passed = passed | {"sequence": 2}
        self.assertTrue(report.compare([expected, other], [passed, passed]))
        self.assertTrue(report.compare([expected, other], [passed]))
        self.assertEqual(report.compare([expected, other], [passed, other_passed]), [])

    def test_known_unsupported_mechanics_are_reported_as_incomplete_coverage(self):
        self.assertTrue(report.coverage_problems(["WARN [YO3 GAP] engine event 30: known unsupported presentation/interaction"]))
        self.assertEqual(report.coverage_problems(["INFO ordinary activity"]), [])

    def test_rotated_session_logs_are_read_without_chat_or_credentials(self):
        line = '1790000000 INFO [YO3 EXPECT] {"schema":1,"sequence":2,"expected":[123,456]}'
        self.assertEqual(report.records([line], "[YO3 EXPECT] ")[0]["sequence"], 2)
        self.assertEqual(report.records(["unrelated private game text"], "[YO3 CHECK] "), [])

if __name__ == "__main__": unittest.main()
