#!/usr/bin/env python3
"""Check native multi-chain diagnostics against archived reference summaries."""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/beast"))
from compare_runs import read_trace, rhat


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, default=ROOT / "target/release/rove")
    parser.add_argument("--archive", type=Path, default=ROOT / ".local/issue12/production")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = {"engine_sha256": hashlib.sha256(args.engine.read_bytes()).hexdigest(),
              "rhat_oracle": "tools/beast/compare_runs.py; Python statistics.NormalDist",
              "ess_oracle": "archived ReferenceDiagnostics.java summaries, BEAST 2.7.8 convention",
              "cases": []}
    for case in ["reference-mk", "reference-mkv", "control", "homoplasy", "missing", "reduced"]:
        directories = sorted(path for path in (args.archive / case / "rove").iterdir() if path.is_dir())
        command = [str(args.engine), "compare", *map(str, directories), "--burnin-percent", "10"]
        actual = json.loads(subprocess.check_output(command, text=True))
        traces = [read_trace(path / "trace.log", 10) for path in directories]
        archived = [json.loads((path / "summary.json").read_text()) for path in directories]
        expected = [x["statistics"] for x in archived]
        max_rhat_error = max_ess_relative_error = 0.0
        for column in actual["statistics"]:
            name = column["name"]
            oracle = rhat([trace[name] for trace in traces])
            if oracle is None:
                assert column["rhat"] is None
            else:
                error = abs(oracle - column["rhat"])
                max_rhat_error = max(max_rhat_error, error)
                assert error < 1e-8, (case, name, "R-hat", error)
            for index, chain in enumerate(expected):
                reference = chain[name]["ess"]
                value = column["ess"][index]
                if reference is None:
                    assert value is None
                else:
                    error = abs(reference - value) / max(1.0, abs(reference))
                    max_ess_relative_error = max(max_ess_relative_error, error)
                    assert error < 1e-8, (case, name, "ESS", error)
        frequencies = [{",".join(sorted(c["taxa"])): c["frequencies"][i] for c in actual["clades"]}
                       for i in range(len(directories))]
        for native, reference in zip(frequencies, archived):
            oracle = reference["observed_clade_frequencies"]
            for clade in native.keys() | oracle.keys():
                assert native.get(clade, 0.0) == oracle.get(clade, 0.0), (case, clade)
        report["cases"].append({"case": case, "chains": len(directories),
            "retained_per_chain": actual["retained_per_chain"], "statistics": len(actual["statistics"]), "all_clade_frequencies_match": True,
            "maximum_rhat_absolute_error": max_rhat_error,
            "maximum_ess_relative_error": max_ess_relative_error,
            "sources": actual["sources"]})
        print(f"{case}: R-hat error {max_rhat_error:.3g}; ESS relative error {max_ess_relative_error:.3g}", flush=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
