#!/usr/bin/env python3
"""Render the recorded proposal comparison without changing its decision gates."""

import argparse
import json
from pathlib import Path
import statistics

from experiment import CASES
from report import STATISTICS


def span(values, digits=2):
    return f"{min(values):.{digits}f}–{max(values):.{digits}f}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    args = parser.parse_args()
    root = args.archive
    cases = {case: json.loads((root / "final" / case / "comparison.json").read_text()) for case in CASES}
    decision = json.loads((root / "final/efficiency.json").read_text())
    text = ["# Recorded sampling results", "", "Generated with `python3 tools/sampling/tables.py "
            + str(root) + "`. Ratios above one favor the joint candidate.", "",
            "## Primary efficiency and regressions", "",
            f"Primary score: **{decision['primary_score']:.3f}**, 90% whole-chain bootstrap interval "
            f"**{span(decision['primary_interval_90_percent'], 3)}**. "
            f"Scientific gates pass: **{decision['scientific_gates_pass']}**. "
            f"Promote to default: **{decision['retain_as_default']}**.", "",
            "Four chains per schedule give limited uncertainty resolution. The score is the geometric "
            "mean across cases of the smaller posterior/sigma median ESS/second ratio, not a wall-time speedup.", "",
            "| Case | Posterior ratio [90% interval] | Sigma ratio [90% interval] |",
            "| --- | ---: | ---: |"]
    for case in CASES:
        ratio = decision["ratios_candidate_over_classical"][case]
        ci = decision["ratio_intervals_90_percent"][case]
        text.append(f"| {case} | {ratio['posterior']:.3f} [{span(ci['posterior'], 3)}] | "
                    f"{ratio['clock_sigma']:.3f} [{span(ci['clock_sigma'], 3)}] |")
    text += ["", "These median regressions exceed the predeclared 20% limit:", "",
             "| Case | Parameter | Candidate / classical [90% interval] |", "| --- | --- | ---: |"]
    for case in CASES:
        for label, value in decision["ratios_candidate_over_classical"][case].items():
            if value < .8:
                ci = decision["ratio_intervals_90_percent"][case][label]
                text.append(f"| {case} | {label} | {value:.3f} [{span(ci, 3)}] |")
    text += ["", "`gamma_prior` is the negative gamma shape under its Exponential(mean=1) prior; "
             "these two rows have the same ESS and are not independent findings.", "",
             "## Process wall time", "",
             "Seconds, ranges across four chains. Startup, adaptation and file output are included. "
             "BEAST values are archived observations from the earlier ORC comparison.", "",
             "| Case | Steps per chain | Classical Rove | Joint candidate | Archived BEAST |",
             "| --- | ---: | ---: | ---: | ---: |"]
    for case, pairs in cases.items():
        a, b = pairs["candidate_vs_classical"], pairs["candidate_vs_beast"]
        steps = "40M" if case == "reference-mkv" else "10M"
        text.append(f"| {case} | {steps} | {span(a['wall_seconds']['classical'])} | "
                    f"{span(a['wall_seconds']['candidate'])} | {span(b['wall_seconds']['beast'])} |")
    for pair, title in [("candidate_vs_classical", "Candidate / current classical Rove"),
                        ("candidate_vs_beast", "Candidate / archived BEAST ORC")]:
        text += ["", "## " + title, "", "Median ESS/second ratios for every continuous statistic. "
                 "Individual chain values and classical-comparison intervals are in the JSON archive.", "",
                 "| Parameter | " + " | ".join(CASES) + " |", "| --- |" + " ---: |" * len(CASES)]
        for label in STATISTICS:
            cells = []
            for case in CASES:
                comparison = cases[case][pair]
                first, second = comparison["groups"]
                values = comparison["ess_per_second"]
                cells.append(f"{statistics.median(values[first][label]) / statistics.median(values[second][label]):.3f}")
            text.append("| " + label + " | " + " | ".join(cells) + " |")
    text += ["", "## Scientific agreement", "",
             "Maxima include both candidate comparisons. The ESS minimum and R-hat maximum include "
             "all continuous statistics in all three groups. The ingroup constraint is excluded from clade agreement.", "",
             "| Case | Largest absolute mean z | Largest clade difference | Largest clade MCSE | Largest R-hat | Lowest ESS | All gates |",
             "| --- | ---: | ---: | ---: | ---: | ---: | --- |"]
    for case, pairs in cases.items():
        z = max(abs(m["z"]) for c in pairs.values() for m in c["means"].values())
        clades = [(c, pair["groups"]) for pair in pairs.values()
                  for c in pair["clades"].values() if c["tested"]]
        delta = max(abs(c["difference"]) for c, _ in clades)
        mcse = max(c[g]["mcse"] for c, groups in clades for g in groups)
        diagnostics = [d[label] for c in pairs.values() for d in c["convergence"].values() for label in STATISTICS]
        text.append(f"| {case} | {z:.3f} | {delta:.5f} | {mcse:.5f} | "
                    f"{max(d['rhat'] for d in diagnostics):.5f} | {min(e for d in diagnostics for e in d['ess']):.1f} | "
                    f"{'Pass' if all(c['passes'] for c in pairs.values()) else 'Unresolved'} |")
    text += ["", "## Trees and autocorrelation", "",
             "Distinct sampled topologies, ranges across chains. Counts alone do not establish exploration. "
             "Rooted RF, reference-topology frequencies, ESS and limited/constant indicators are retained in each comparison JSON.", "",
             "| Case | Classical | Candidate | Archived BEAST | Posterior autocorrelation time: classical → candidate | Sigma autocorrelation time: classical → candidate |",
             "| --- | ---: | ---: | ---: | ---: | ---: |"]
    for case, pairs in cases.items():
        a, b = pairs["candidate_vs_classical"], pairs["candidate_vs_beast"]
        times = []
        for label in ["posterior", "clock_sigma"]:
            values = [statistics.median(v["integrated_autocorrelation_samples"]
                      for v in a["trace_dependence"][group][label]) for group in ["classical", "candidate"]]
            times.append(f"{values[0]:.2f} → {values[1]:.2f}")
        text.append(f"| {case} | {span(a['unique_topologies']['classical'], 0)} | "
                    f"{span(a['unique_topologies']['candidate'], 0)} | {span(b['unique_topologies']['beast'], 0)} | "
                    + " | ".join(times) + " |")
    text += ["", "Autocorrelation times are retained sample count divided by ESS, in logged samples (one per 1,000 steps). "
             "The table uses medians across chains. Lag-one autocorrelation is also recorded in the JSON.", ""]
    (root / "TABLES.md").write_text("\n".join(text))


if __name__ == "__main__":
    main()
