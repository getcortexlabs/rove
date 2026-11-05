import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
from coupled_study import choose_ladder, ladder, run_group


def runs(trips, accepted=20, attempted=100):
    return [{'coupling': {'cold_hot_cold_round_trips_per_state': [trip, 0, 0, 0],
                         'swaps': [{'attempted': attempted, 'accepted': accepted}] * 3}}
            for trip in trips]


class CoupledStudyTests(unittest.TestCase):
    def test_selection_requires_each_replica_and_each_edge(self):
        candidates = {'0.01': runs([3, 4]), '0.05': runs([2, 2]), '0.2': runs([4, 4])}
        candidates['0.2'][1]['coupling']['swaps'] = [
            {'attempted': 100, 'accepted': 20},
            {'attempted': 100, 'accepted': 0},
            {'attempted': 100, 'accepted': 20}]
        self.assertEqual(choose_ladder(candidates), .05)
        candidates['0.05'] = runs([2, 0])
        self.assertEqual(choose_ladder(candidates), .01)

    def test_fallback_uses_the_weaker_replica_and_then_smallest_delta(self):
        candidates = {'0.01': runs([20, 0], 0), '0.05': runs([3, 2], 0), '0.2': runs([2, 2], 0)}
        self.assertEqual(choose_ladder(candidates), .05)
        self.assertEqual(ladder(.2), [1., 1/1.2, 1/1.4, 1/1.6])

    def test_group_preserves_failure_and_timestamps_prefixes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            code = 'import json; print(json.dumps({"step":100})); print(json.dumps({"step":200}));'
            jobs = [([sys.executable, '-c', code], root)] * 2
            report = run_group(root/'success', jobs, 100)
            for member in report['runs']:
                self.assertLessEqual(member['prefix_group_seconds']['100'], member['prefix_group_seconds']['200'])
                self.assertLessEqual(member['prefix_group_seconds']['200'], report['wall_seconds'])
            jobs[1] = ([sys.executable, '-c', 'raise SystemExit(7)'], root)
            with self.assertRaisesRegex(ValueError, 'Failed group'):
                run_group(root/'failed', jobs, 100)
            record = json.loads((root/'failed/timing.json').read_text())
            self.assertEqual([r['returncode'] for r in record['runs']], [0, 7])


if __name__ == '__main__':
    unittest.main()
