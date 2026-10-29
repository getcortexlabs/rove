import itertools
import json
import tempfile
import unittest
from pathlib import Path

from report_inference import CASES, known_tree, minimum_changes, operator_counts, read_runs


class InferenceReportTest(unittest.TestCase):
    def test_character_scores_match_enumerated_ancestral_assignments(self):
        tree = known_tree("((T1,T2),(T3,T4));")
        edges = [(0, 1), (0, 2), (1, 3), (1, 4), (2, 5), (2, 6)]
        for observations in itertools.product("01?", repeat=4):
            scores = []
            for states in itertools.product((0, 1), repeat=7):
                if all(o == "?" or int(o) == s for o, s in zip(observations, states[3:])):
                    scores.append(sum(states[a] != states[b] for a, b in edges))
            column = dict(zip(("T1", "T2", "T3", "T4"), observations))
            with self.subTest(observations=observations):
                self.assertEqual(minimum_changes(tree, column)[1], min(scores))

    def test_archive_preserves_nested_adaptive_operator_weights(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            rove = directory / "rove/17"
            beast = directory / "beast/17"
            rove.mkdir(parents=True)
            beast.mkdir(parents=True)
            run = {"settings": {"steps": 10}, "moves": [{"accepted": 3, "rejected": 7}]}
            schedule = {"operators": [{"id": "orc", "weights": [0.2, 0.8],
                                       "operators": [{"accepted": 2}, {"accepted": 1}]}]}
            (rove / "run.json").write_text(json.dumps(run))
            (beast / "analysis.xml.state").write_text(
                '<itsabeastystate/>\n<!-- {"other": true} -->\n<!--\n'
                + json.dumps(schedule) + '\n-->'
            )
            archived = operator_counts(directory, {"seeds": [17]})
            self.assertEqual(archived["beast"]["17"], schedule)
            self.assertEqual(archived["rove"]["17"], run)

    def test_archive_selects_longest_batch_and_retains_earlier_results(self):
        with tempfile.TemporaryDirectory() as temporary:
            paths = self.make_runs(Path(temporary))
            runs, selected, protocol = read_runs(paths)
            self.assertEqual(len(runs), 7)
            self.assertEqual(protocol, "frozen")
            self.assertEqual(selected["reference-mkv"]["steps"], 40_000_000)
            self.assertTrue(selected["reference-mkv"]["passes"])
            self.assertTrue(any(not record["passes"] for _, record, _ in runs))

    def test_archive_refuses_incomplete_duplicate_or_mismatched_batches(self):
        with tempfile.TemporaryDirectory() as temporary:
            paths = self.make_runs(Path(temporary))
            with self.assertRaisesRegex(ValueError, "all six cases"):
                read_runs(paths[:1])
            with self.assertRaisesRegex(ValueError, "Duplicate"):
                read_runs(paths + paths[:1])
            path = paths[0] / "metadata.json"
            metadata = json.loads(path.read_text())
            metadata["seeds"] = [99]
            path.write_text(json.dumps(metadata))
            with self.assertRaisesRegex(ValueError, "metadata disagree"):
                read_runs(paths)
            metadata["seeds"] = [17, 18, 19, 20]
            metadata["protocol_sha256"] = "changed"
            path.write_text(json.dumps(metadata))
            with self.assertRaisesRegex(ValueError, "different protocols"):
                read_runs(paths)

    @staticmethod
    def make_runs(root):
        paths = []
        for case, steps in [(case, 10_000_000) for case in CASES] + [("reference-mkv", 40_000_000)]:
            directory = root / case / str(steps)
            directory.mkdir(parents=True)
            record = {"case": case, "steps": steps, "seeds": [17, 18, 19, 20],
                      "protocol_sha256": "frozen",
                      "passes": case != "reference-mkv" or steps == 40_000_000}
            for name in ("comparison.json", "metadata.json"):
                (directory / name).write_text(json.dumps(record))
            paths.append(directory)
        return paths


if __name__ == "__main__":
    unittest.main()
