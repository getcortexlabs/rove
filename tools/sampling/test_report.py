import copy
from pathlib import Path
import tempfile
import unittest

from experiment import CASES, SCHEMES, scientific_digest
from report import CONSTRAINT, STATISTICS, compare_pair, efficiency_summary, validate_runs


class SamplingReportTest(unittest.TestCase):
    def test_constant_twofold_efficiency_has_exact_interval_and_regressions_block_promotion(self):
        comparison = {"ess_per_second": {
            "candidate": {label: [2.0] * 4 for label in STATISTICS},
            "classical": {label: [1.0] * 4 for label in STATISTICS}}, "passes": True}
        cases = {case: {"candidate_vs_classical": copy.deepcopy(comparison),
                        "candidate_vs_beast": {"passes": True}} for case in CASES}
        result = efficiency_summary(cases)
        self.assertEqual(result["primary_score"], 2)
        self.assertEqual(result["primary_interval_90_percent"], [2, 2])
        self.assertTrue(result["retain_as_default"])
        cases["missing"]["candidate_vs_classical"]["ess_per_second"]["candidate"]["gamma_shape"] = [.5] * 4
        result = efficiency_summary(cases)
        self.assertFalse(result["retain_as_default"])
        self.assertFalse(result["no_median_regression_below_0_8"])
        self.assertEqual(result["primary_score"], 2)
        cases["missing"]["candidate_vs_classical"]["ess_per_second"]["candidate"]["gamma_shape"][0] = None
        result = efficiency_summary(cases)
        self.assertFalse(result["retain_as_default"])
        self.assertEqual(result["undefined_efficiency"], {"missing": ["gamma_shape"]})

    def test_negative_controls_detect_shifted_density_and_clade_distributions(self):
        stats = {"mean": 1.0, "sd": 1.0, "ess": 500.0}
        run = {"seconds": 10, "statistics": {label: dict(stats) for label in STATISTICS},
               "trace": {label: [0., 2.] * 500 for label in STATISTICS},
               "summary": {"tree_statistics": {label: dict(stats) for label in ["rooted_rf", "reference_topology"]},
                           "tree_traces": {label: [0., 2.] * 500 for label in ["rooted_rf", "reference_topology"]},
                           "observed_clade_frequencies": {"T1,T2": .5, CONSTRAINT: 1.0},
                           "clade_batch_frequencies": {"T1,T2": [.49, .51] * 10, CONSTRAINT: [1.] * 20},
                           "reference_clades": ["T1,T2", CONSTRAINT], "unique_topologies": 2}}
        first = [copy.deepcopy(run) for _ in range(4)]
        second = copy.deepcopy(first)
        result = compare_pair(first, second, ["candidate", "classical"])
        self.assertTrue(result["passes"])
        dependence = result["trace_dependence"]["candidate"]["posterior"][0]
        self.assertEqual(dependence["integrated_autocorrelation_samples"], 2)
        self.assertAlmostEqual(dependence["lag_one_autocorrelation"], -.999)
        for record in second:
            record["statistics"]["likelihood"]["mean"] = 5
            record["summary"]["observed_clade_frequencies"]["T1,T2"] = .9
            record["summary"]["clade_batch_frequencies"]["T1,T2"] = [.9] * 20
        result = compare_pair(first, second, ["candidate", "classical"])
        self.assertFalse(result["gates"]["means"])
        self.assertFalse(result["gates"]["clades"])
        self.assertTrue(result["gates"]["clade_precision"])

    def test_final_metadata_rejects_duplicate_chains_and_changed_weights(self):
        metadata = {"phase": "final", "source_dirty": False, "selected": "combined",
                    "schemes": {"classical": {}, "combined": SCHEMES["combined"]}, "runs": []}
        for case in CASES:
            steps, start = (40_000_000, 20261210) if case == "reference-mkv" else (10_000_000, 20261110)
            metadata["runs"].extend({"case": case, "scheme": scheme, "seed": seed,
                                     "steps": steps, "sample_every": 1000}
                                    for scheme in metadata["schemes"] for seed in range(start, start + 4))
        validate_runs(metadata)
        broken = copy.deepcopy(metadata)
        broken["runs"][-1] = broken["runs"][0]
        with self.assertRaisesRegex(ValueError, "distinct chains"):
            validate_runs(broken)
        broken = copy.deepcopy(metadata)
        broken["schemes"]["combined"]["clock_sigma_rates"] = 3
        with self.assertRaisesRegex(ValueError, "weights differ"):
            validate_runs(broken)

    def test_scientific_digest_ignores_provenance_but_preserves_branch_annotations(self):
        with tempfile.TemporaryDirectory() as temporary:
            first, second = (Path(temporary) / name for name in ["a.trees", "b.trees"])
            body = "#NEXUS\ntree STATE_0 = [&R] (1[&rate=1.0]:.1,2:.1);\n"
            first.write_text('[command "a"]\n[executable sha256 abc]\n' + body)
            second.write_text('[command "b"]\n[executable sha256 def]\n' + body)
            self.assertEqual(scientific_digest(first), scientific_digest(second))
            second.write_text(body.replace("rate=1.0", "rate=2.0"))
            self.assertNotEqual(scientific_digest(first), scientific_digest(second))


if __name__ == "__main__":
    unittest.main()
