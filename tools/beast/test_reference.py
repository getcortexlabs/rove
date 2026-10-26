import json
import random
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from check_snapshot import check
from compare_runs import rank_normalize, rhat
from run_reference import verify_run


class ReferenceChecks(unittest.TestCase):
    def test_exported_mkv_states_match_independent_pruning(self):
        results = Path(__file__).resolve().parents[2] / "benchmarks/beast-2.7.8/2026-10-25"
        for seed in [20261025, 20261026]:
            with self.subTest(seed=seed):
                snapshot = json.loads((results / f"snapshot-{seed}.json").read_text())
                self.assertLess(check(snapshot)["total_error"], 1e-9)

    def test_rank_normalization_preserves_ties(self):
        first, second = rank_normalize([[1, 2, 2], [2, 3, 4]])
        self.assertEqual(first[1], first[2])
        self.assertEqual(first[1], second[0])
        self.assertLess(first[0], first[1])
        self.assertLess(second[1], second[2])

    def test_rhat_distinguishes_shifted_chains(self):
        generator = random.Random(81)
        chains = [[generator.gauss(0, 1) for _ in range(2000)] for _ in range(2)]
        self.assertLess(rhat(chains), 1.01)
        chains[1] = [value + 3 for value in chains[1]]
        self.assertGreater(rhat(chains), 1.1)

    def test_rhat_does_not_report_constant_traces_as_converged(self):
        self.assertIsNone(rhat([[1.0] * 10, [1.0] * 10]))

    def test_constant_folded_ranks_keep_the_bulk_diagnostic(self):
        self.assertLess(rhat([[0.0, 1.0] * 50, [1.0, 0.0] * 50]), 1.01)

    def test_run_validation_rejects_incomplete_output(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "analysis.log").write_text("Sample\tposterior\n0\t-10\n1000\t-9\n")
            (root / "console.log").write_text("uses BeerLikelihoodCore\nTotal calculation time: 0.2 seconds\n")
            with self.assertRaises(ValueError):
                verify_run(root, 2000, 1000)
            self.assertEqual(verify_run(root, 1000, 1000), 0.2)


if __name__ == "__main__":
    unittest.main()
