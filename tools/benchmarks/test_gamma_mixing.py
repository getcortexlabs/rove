import copy
import unittest

from report_gamma_mixing import compare, merge_node_metadata


def groups():
    stats = {"mean": 1., "sd": 1., "ess": 500.}
    labels = ["gamma_shape", "posterior"]
    tree_labels = ["rooted_rf", "reference_topology"]
    run = {"seconds": 10., "rove_ess": {label: 500. for label in labels},
           "rove_diagnostics_pass": True,
           "trace": {label: [0., 2.] * 500 for label in labels},
           "summary": {"statistics": {label: dict(stats) for label in labels},
                       "tree_statistics": {label: dict(stats) for label in tree_labels},
                       "tree_traces": {label: [0., 2.] * 500 for label in tree_labels},
                       "observed_clade_frequencies": {"A,B": .5},
                       "clade_batch_frequencies": {"A,B": [.49, .51] * 10},
                       "unique_topologies": 2}}
    return {name: [copy.deepcopy(run) for _ in range(4)] for name in ["baseline", "candidate"]}


class GammaMixingReportTest(unittest.TestCase):
    def test_beast_metadata_copy_preserves_values_and_branch_lengths(self):
        line = "tree STATE_0 = (1[&height=0][&rate=1.2e-3]:2.5,2[&height=1.5][&rate=2]:1)[&height=2.5];\n"
        merged, count = merge_node_metadata(line)
        self.assertEqual(count, 2)
        self.assertEqual(merged, "tree STATE_0 = (1[&height=0,rate=1.2e-3]:2.5,2[&height=1.5,rate=2]:1)[&height=2.5];\n")
        self.assertEqual(merge_node_metadata(merged), (merged, 0))
        comment = "[command example [&height=1][&rate=2]]\n"
        self.assertEqual(merge_node_metadata(comment), (comment, 0))

    def test_efficiency_cannot_hide_density_or_clade_disagreement(self):
        runs = groups()
        for run in runs["candidate"]:
            run["rove_ess"]["gamma_shape"] = 5000.
        good = compare(runs, set(), None)
        self.assertTrue(good["scientific_pass"])
        self.assertTrue(good["gamma_gain_above_2"])
        for run in runs["candidate"]:
            run["summary"]["statistics"]["posterior"]["mean"] = 5.
            run["summary"]["observed_clade_frequencies"]["A,B"] = .8
            run["summary"]["clade_batch_frequencies"]["A,B"] = [.8] * 20
        shifted = compare(runs, set(), None)
        self.assertTrue(shifted["gamma_gain_above_2"])
        self.assertFalse(shifted["scientific_gates"]["means"])
        self.assertFalse(shifted["scientific_gates"]["clades"])

    def test_shared_group_time_and_other_parameter_regressions_count(self):
        runs = groups()
        for run in runs["candidate"]:
            run["seconds"] = 20.
            run["rove_ess"]["gamma_shape"] = 5000.
        result = compare(runs, set(), None)
        self.assertEqual(result["efficiency"]["gamma_shape"]["median_ratio"], 5.)
        self.assertEqual(result["efficiency"]["posterior"]["median_ratio"], .5)
        self.assertFalse(result["no_median_regression_below_0_8"])
        runs["candidate"][0]["rove_ess"]["gamma_shape"] = None
        unavailable = compare(runs, set(), None)
        self.assertFalse(unavailable["gamma_gain_above_2"])
        self.assertFalse(unavailable["no_median_regression_below_0_8"])

    def test_lag_limits_and_uncertain_clades_block_promotion(self):
        runs = groups()
        runs["baseline"][0]["rove_diagnostics_pass"] = False
        for run in runs["candidate"]:
            run["summary"]["clade_batch_frequencies"]["A,B"] = [0., 1.] * 10
        result = compare(runs, set(), None)
        self.assertFalse(result["scientific_gates"]["rove_diagnostics"])
        self.assertFalse(result["scientific_gates"]["clade_precision"])
        self.assertFalse(result["scientific_pass"])


if __name__ == "__main__":
    unittest.main()
