import json
import math
import random
import unittest
import xml.etree.ElementTree as ET

from compare_inference import clade_summary, convergence, difference, normalize, pooled, STATISTICS
from prepare_inference import make_xml, read_matrix, ROOT
from run_reference import digest


class InferenceStatisticsTest(unittest.TestCase):
    def test_between_chain_variation_limits_precision(self):
        values = [{"mean": x, "sd": 1.0, "ess": 10000} for x in (0.0, 0.0, 2.0, 2.0)]
        result = pooled(values)
        self.assertAlmostEqual(result["mean"], 1.0)
        self.assertAlmostEqual(result["mcse"], math.sqrt(1 / 3))
        self.assertGreater(result["between_mcse"], result["within_mcse"])

    def test_comparison_uses_combined_error_and_rejects_undefined_precision(self):
        a = {"mean": 1, "mcse": 0.1}
        self.assertTrue(difference(a, {"mean": 1.4, "mcse": 0.1})["agrees"])
        self.assertFalse(difference(a, {"mean": 1.8, "mcse": 0.1})["agrees"])
        self.assertFalse(difference(a, {"mean": 1, "mcse": None})["agrees"])

    def test_constant_or_divergent_chains_are_not_convergence_evidence(self):
        constant = [{"mean": 1.0, "sd": 0.0, "ess": None}] * 4
        self.assertFalse(convergence(constant, [[1.0] * 100] * 4)["passes"])
        tree = convergence(constant, [[1.0] * 100] * 4, tree=True)
        self.assertIsNone(tree["passes"])
        self.assertTrue(tree["limited_tree_mixing_diagnostic"])
        rng = random.Random(17)
        chains = [[rng.gauss(offset, 1) for _ in range(1000)] for offset in (0, 0, 4, 4)]
        values = [{"mean": 0.0, "sd": 1.0, "ess": 1000}] * 4
        self.assertFalse(convergence(values, chains)["passes"])

    def test_clade_uncertainty_detects_chains_stuck_in_different_topologies(self):
        summaries = [{"observed_clade_frequencies": {"A,B": p},
                      "clade_batch_frequencies": {"A,B": [p] * 20}} for p in (0, 0, 1, 1)]
        result = clade_summary(summaries, "A,B")
        self.assertEqual(result["mean"], 0.5)
        self.assertGreater(result["mcse"], 0.28)
        self.assertFalse(result["constant_frequency"])
        absent = clade_summary(summaries, "A,C")
        self.assertEqual(absent["mean"], 0)
        self.assertTrue(absent["constant_frequency"])

    def test_branch_rate_mean_uses_explicit_rates_not_weighted_statistic(self):
        summary = {"statistics": {name: {"mean": 38.0, "sd": 3.8, "ess": 200,
                                          "hpd_low": 19, "hpd_high": 76} for name in STATISTICS.values()}}
        trace = {name: [19.0, 38.0, 76.0] for name in STATISTICS.values()}
        stats, columns = normalize(summary, trace, "beast")
        self.assertEqual(stats["mean_branch_rate"]["mean"], 1)
        self.assertAlmostEqual(stats["mean_branch_rate"]["sd"], 0.1)
        self.assertEqual(stats["mean_branch_rate"]["ess"], 200)
        self.assertEqual(columns["mean_branch_rate"], [0.5, 1.0, 2.0])


class InferenceConfigurationTest(unittest.TestCase):
    def test_generated_xml_preserves_model_and_operators(self):
        original = ET.parse(ROOT / "fixtures/reference/20x50/analysis.xml").getroot()
        manifest = json.loads((ROOT / "fixtures/validation/manifest.json").read_text())
        for name, case in manifest.items():
            with self.subTest(case=name):
                directory = ROOT / "fixtures/validation" / name
                for file, expected in case["sha256"].items():
                    self.assertEqual(digest(directory / file), expected)
                sequences = read_matrix(directory / "matrix.nex")
                root = make_xml(sequences, case["conditioning"]).getroot()
                for tag in ("run/state", "run/distribution", "run/init", "run/operatorschedule"):
                    a, b = original.find(tag), root.find(tag)
                    # Formatting whitespace differs; compare structure and meaningful text.
                    def structure(node):
                        return (node.tag, dict(node.attrib), (node.text or "").strip(), [structure(c) for c in node])
                    if tag == "run/state":
                        # Only the alignment's ascertainment/filter inputs can differ.
                        a = ET.fromstring(ET.tostring(a))
                        alignment = a.find(".//alignment")
                        alignment.attrib = dict(b.find(".//alignment").attrib)
                    self.assertEqual(structure(a), structure(b))
                for a, b in zip(original.findall("run/operator"), root.findall("run/operator")):
                    a = ET.fromstring(ET.tostring(a))
                    weights = a.find("weightvector")
                    if weights is not None:
                        weights.text = str(case["characters"])
                    self.assertEqual(structure(a), structure(b))
                suffix = "01" if case["conditioning"] == "variable" else ""
                for row in root.findall("data/sequence"):
                    self.assertEqual(row.get("value"), sequences[row.get("taxon")] + suffix)

    def test_missing_columns_that_may_be_constant_cannot_use_mkv(self):
        sequences = read_matrix(ROOT / "fixtures/validation/missing/matrix.nex")
        self.assertTrue(any(set(column) - {"?"} != {"0", "1"} for column in zip(*sequences.values())))
        with self.assertRaises(ValueError):
            make_xml(sequences, "variable")


if __name__ == "__main__":
    unittest.main()
