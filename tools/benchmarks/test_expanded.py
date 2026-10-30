"""Check model export and incomplete-run handling before expensive measurements."""

import tempfile
import unittest
from pathlib import Path

from expanded import verify_trace
from prepare_expanded import make_xml


class ExpandedTests(unittest.TestCase):
    def data(self):
        return {"taxa": ["A", "B", "C"], "characters": 5, "partitions": [
            {"states": 2, "characters": 3, "sequences": ["010", "1?0", "{01}11"], "ambiguities": "01"},
            {"states": 3, "characters": 2, "sequences": ["02", "12", "?1"], "ambiguities": ""}]}

    def test_partitioned_model_shares_tree_clock_and_gamma(self):
        root = make_xml(self.data(), "none", 1000, 100).getroot()
        likelihoods = root.findall(".//distribution[@id='likelihood']/distribution")
        self.assertEqual(len(likelihoods), 2)
        self.assertEqual(likelihoods[0].get("tree"), likelihoods[1].get("tree"))
        self.assertEqual(likelihoods[0].find("branchRateModel").get("id"), likelihoods[1].get("branchRateModel")[1:])
        self.assertEqual(likelihoods[0].find("siteModel").get("shape"), likelihoods[1].find("siteModel").get("shape"))
        self.assertIsNone(root.find(".//distribution[@id='root.prior']"))
        self.assertEqual(root.find(".//parameter[@id='ORCRates.c:matriz_20x50']").get("dimension"), "4")
        self.assertEqual([d.get("data") for d in likelihoods], ["@partition_02", "@partition_03"])

    def test_mkv_indices_count_characters_including_ambiguities(self):
        root = make_xml(self.data(), "variable", 1000, 100).getroot()
        parts = root.findall("data")
        self.assertEqual([(p.get("excludefrom"), p.get("excludeto")) for p in parts], [("3", "5"), ("2", "5")])
        self.assertEqual(parts[0].findall("sequence")[2].get("value"), "{01}1101")
        self.assertEqual(parts[1].findall("sequence")[0].get("value"), "02012")

    def test_constraint_and_lengths_reject_mismatched_inputs(self):
        with self.assertRaises(ValueError):
            make_xml(self.data(), "none", 1000, 100, constrained=True)
        with self.assertRaises(ValueError):
            make_xml(self.data(), "none", 1001, 100)
        data = self.data()
        data["partitions"][1]["sequences"].pop()
        with self.assertRaises(ValueError):
            make_xml(data, "none", 1000, 100)

    def test_trace_must_contain_every_expected_state(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.log"
            path.write_text("Sample\tposterior\n0\t-10\n100\t-9\n200\t-8\n")
            verify_trace(path, 200, 100)
            with self.assertRaises(ValueError):
                verify_trace(path, 300, 100)
            path.write_text("Sample\tposterior\n0\t-10\n200\t-8\n")
            with self.assertRaises(ValueError):
                verify_trace(path, 200, 100)


if __name__ == "__main__":
    unittest.main()
