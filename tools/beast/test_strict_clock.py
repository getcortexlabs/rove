import tempfile
import unittest
from pathlib import Path

from strict_clock import STATISTICS, check_run, compare, configuration, rove_command

XML = """<beast>
  <run id="mcmc" spec="MCMC" chainLength="2000">
    <state id="state">
      <tree id="Tree.t:x" name="stateNode">
        <taxonset id="taxa"><alignment id="a" spec="FilteredAlignment" ascertained="true" filter="1-2"/></taxonset>
      </tree>
      <parameter id="BDBirthRate.t:x" name="stateNode">1.5</parameter>
      <parameter id="BDDeathRate.t:x" name="stateNode">0.25</parameter>
    </state>
    <distribution id="posterior" spec="CompoundDistribution">
      <distribution id="clade" spec="beast.base.evolution.tree.MRCAPrior" monophyletic="true">
        <taxonset id="members"><taxon id="A"/><taxon id="B"/></taxonset>
      </distribution>
    </distribution>
    <operator id="h" spec="Uniform" weight="30.0"/>
    <operator id="s" spec="SubtreeSlide" size="0.05" optimise="SLIDE" weight="15.0"/>
    <operator id="w" spec="Exchange" isNarrow="false" weight="3.0"/>
    <operator id="r" spec="ScaleOperator" rootOnly="true" tree="@Tree.t:x" optimise="false" weight="3.0"/>
    <operator id="b" spec="ScaleOperator" parameter="@BDBirthRate.t:x" scaleFactor="0.25" optimise="false" weight="3.0"/>
    <logger id="tracelog" logEvery="100"/>
  </run>
</beast>"""


def summary(mean, sd, ess, clades):
    return {
        "statistics": {label: {"mean": mean, "sd": sd, "ess": ess} for label in STATISTICS.values()}
        | {label: {"mean": mean, "sd": sd, "ess": ess} for label in STATISTICS},
        "observed_clade_frequencies": clades,
    }


def parse(text):
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "analysis.xml"
        path.write_text(text)
        return configuration(path)


def record(config):
    return {
        "moves": [{"name": name, "weight": m["weight"],
                   "initial_tuning": None if m["tuning"] is None else float(m["tuning"])}
                  for name, m in config["moves"].items()],
        "settings": {"tuning": False, "initial_birth_difference_rate": 1.5, "initial_relative_death_rate": 0.25},
    }


class CompareTest(unittest.TestCase):
    def test_means_within_four_standard_errors_agree(self):
        # Two runs per engine with sd 1 and ESS 100: each pooled MCSE is 0.0707.
        rove = [summary(10.0, 1.0, 100, {"A,B": 1.0})] * 2
        close = [summary(10.3, 1.0, 100, {"A,B": 0.97})] * 2
        far = [summary(10.5, 1.0, 100, {"A,B": 0.9})] * 2
        self.assertTrue(compare(rove, close)["all_means_agree"])
        self.assertTrue(compare(rove, close)["all_clades_agree"])
        result = compare(rove, far)
        self.assertFalse(result["all_means_agree"])
        self.assertAlmostEqual(result["means"]["posterior"]["z"], -0.5 / 0.1)
        self.assertFalse(result["all_clades_agree"])

    def test_rare_clades_are_ignored_and_missing_clades_count_as_zero(self):
        rove = [summary(1.0, 1.0, 100, {"A,B": 0.995, "A,C": 0.005})] * 2
        beast = [summary(1.0, 1.0, 100, {"A,B": 0.96, "B,C": 0.04})] * 2
        clades = compare(rove, beast)["clades"]
        self.assertEqual(set(clades), {"A,B", "B,C"})
        self.assertAlmostEqual(clades["B,C"]["difference"], -0.04)

    def test_undefined_effective_sample_sizes_leave_a_mean_unjudged(self):
        rove = [summary(1.0, 0.0, None, {})] * 2
        result = compare(rove, rove)
        self.assertIsNone(result["means"]["posterior"]["agrees"])
        self.assertFalse(result["all_means_agree"])


class ConfigurationTest(unittest.TestCase):
    def test_the_rove_command_follows_the_xml(self):
        config = parse(XML.replace("SLIDE", "false"))
        self.assertEqual(config["conditioning"], "variable")
        self.assertEqual(config["monophyly"], [["A", "B"]])
        self.assertEqual(config["initial"], {"birth_difference_rate": 1.5, "relative_death_rate": 0.25})
        self.assertEqual(config["moves"]["root_scale"]["tuning"], "0.75")
        self.assertIsNone(config["moves"]["wide_exchange"]["tuning"])
        self.assertEqual(rove_command("rove", "m.nex", "out", 7, config), [
            "rove", "sample", "m.nex", "--output", "out", "--seed", "7", "--steps", "2000",
            "--sample-every", "100", "--conditioning", "variable", "--monophyly", "A,B", "--no-tuning",
            "--tuning", "birth_difference_scale=0.25", "--tuning", "root_scale=0.75",
            "--tuning", "subtree_slide=0.05",
        ])
        plain = parse(XML.replace("SLIDE", "false").replace(' ascertained="true"', ""))
        self.assertEqual(plain["conditioning"], "none")

    def test_partly_tuned_or_unknown_operators_are_rejected(self):
        with self.assertRaises(ValueError):
            rove_command("rove", "m.nex", "out", 7, parse(XML.replace("SLIDE", "true")))
        with self.assertRaises(ValueError):
            parse(XML.replace('spec="Uniform"', 'spec="BactrianNodeOperator"'))

    def test_a_run_must_record_the_moves_and_start_of_the_xml(self):
        config = parse(XML.replace("SLIDE", "false"))
        check_run(record(config), config)
        extra = record(config)
        extra["moves"].append({"name": "narrow_exchange", "weight": 15.0, "initial_tuning": None})
        changed = record(config)
        changed["moves"][1]["initial_tuning"] = 0.5
        moved = record(config)
        moved["settings"]["initial_relative_death_rate"] = 0.5
        missing = record(config)
        missing["moves"].pop()
        for run in (extra, changed, moved, missing):
            with self.assertRaises(ValueError):
                check_run(run, config)


if __name__ == "__main__":
    unittest.main()
