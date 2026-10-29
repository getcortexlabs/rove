#!/usr/bin/env python3
"""Compare frozen proposal runs with classical Rove and archived BEAST ORC runs."""

import argparse
import json
import math
import os
from pathlib import Path
import random
import statistics
import subprocess
import sys
import xml.etree.ElementTree as ET

from experiment import CASES, PRIMARY, ROOT, SCHEMES, scientific_digest

sys.path.insert(0, str(ROOT / "tools/beast"))
from compare_inference import (
    CONSTRAINT, MAX_CLADE_DIFFERENCE, MAX_CLADE_MCSE, STATISTICS,
    clade_summary, convergence, difference, normalize, pooled,
)
from compare_runs import read_trace
from report_inference import operator_counts, sanitize
from run_reference import EXPECTED_PACKAGES, digest, write_json


def trace_dependence(values, ess):
    mean = statistics.fmean(values)
    centered = [value - mean for value in values]
    variance_sum = sum(value * value for value in centered)
    lag_one = sum(a * b for a, b in zip(centered, centered[1:])) / variance_sum if variance_sum else None
    return {"lag_one_autocorrelation": lag_one,
            "integrated_autocorrelation_samples": len(values) / ess if ess else None}


def compare_pair(first, second, names):
    groups = dict(zip(names, [first, second]))
    means, tree_means = {}, {}
    diagnostics = {name: {} for name in names}
    efficiency = {name: {} for name in names}
    for label in STATISTICS:
        values = {}
        for name, runs in groups.items():
            stats = [r["statistics"][label] for r in runs]
            values[name] = pooled(stats)
            diagnostics[name][label] = convergence(stats, [r["trace"][label] for r in runs])
            efficiency[name][label] = [s["ess"] / r["seconds"] if s["ess"] else None
                                       for s, r in zip(stats, runs)]
        values.update(difference(values[names[0]], values[names[1]]))
        means[label] = values
    for label in ["rooted_rf", "reference_topology"]:
        values = {}
        for name, runs in groups.items():
            stats = [r["summary"]["tree_statistics"][label] for r in runs]
            values[name] = pooled(stats)
            diagnostics[name][label] = convergence(
                stats, [r["summary"]["tree_traces"][label] for r in runs], tree=True)
        values.update(difference(values[names[0]], values[names[1]]))
        tree_means[label] = values
    summaries = {name: [r["summary"] for r in runs] for name, runs in groups.items()}
    all_clades = set().union(*(s["observed_clade_frequencies"] for runs in summaries.values() for s in runs))
    truth = set(first[0]["summary"]["reference_clades"])
    clades = {}
    for clade in sorted(all_clades | truth | {CONSTRAINT}):
        a, b = (clade_summary(summaries[name], clade) for name in names)
        selected = max(a["mean"], b["mean"]) >= 0.01
        if not selected and clade not in truth:
            continue
        if clade == CONSTRAINT:
            if any(value != 1 for value in a["chain_frequencies"] + b["chain_frequencies"]):
                raise ValueError("A run violated the imposed ingroup constraint")
            continue
        delta = a["mean"] - b["mean"]
        clades[clade] = {
            names[0]: a, names[1]: b, "difference": delta,
            "combined_mcse": math.hypot(a["mcse"], b["mcse"]),
            "tested": selected, "reference_clade": clade in truth,
            "agrees": abs(delta) <= MAX_CLADE_DIFFERENCE,
            "precision_passes": max(a["mcse"], b["mcse"]) <= MAX_CLADE_MCSE,
        }
    tested = [c for c in clades.values() if c["tested"]]
    gates = {
        "means": all(m["agrees"] for m in means.values()),
        "tree_means": all(m["agrees"] for m in tree_means.values()),
        "clades": all(c["agrees"] for c in tested),
        "clade_precision": all(c["precision_passes"] for c in tested),
        "convergence": all(d["passes"] is not False for v in diagnostics.values() for d in v.values()),
    }
    return {
        "groups": names, "means": means, "tree_means": tree_means,
        "convergence": diagnostics, "clades": clades, "gates": gates,
        "passes": all(gates.values()), "ess_per_second": efficiency,
        "unique_topologies": {name: [r["summary"]["unique_topologies"] for r in runs] for name, runs in groups.items()},
        "wall_seconds": {name: [r["seconds"] for r in runs] for name, runs in groups.items()},
        "trace_dependence": {
            name: {label: [trace_dependence(r["trace"][label], r["statistics"][label]["ess"])
                           for r in runs] for label in STATISTICS}
            for name, runs in groups.items()},
        "imposed_ingroup": CONSTRAINT,
    }


def quantile(values, p):
    ordered = sorted(values)
    position = (len(ordered) - 1) * p
    lower = int(position)
    return ordered[lower] + (position - lower) * (ordered[min(lower + 1, len(ordered) - 1)] - ordered[lower])


def ratio(first, second):
    if not all(v is not None and math.isfinite(v) and v > 0 for v in first + second):
        return None
    return statistics.median(first) / statistics.median(second)


def validate_runs(metadata):
    selected = metadata["selected"]
    if metadata["phase"] != "final" or metadata["source_dirty"] or selected == "classical":
        raise ValueError("Need a clean final comparison with a pilot-selected candidate")
    if metadata["schemes"] != {"classical": {}, selected: SCHEMES[selected]}:
        raise ValueError("Final weights differ from the frozen candidate")
    expected = set()
    for case in CASES:
        steps, start = (40_000_000, 20261210) if case == "reference-mkv" else (10_000_000, 20261110)
        expected.update((case, scheme, seed, steps, 1000)
                        for scheme in ["classical", selected] for seed in range(start, start + 4))
    observed = [(r["case"], r["scheme"], r["seed"], r["steps"], r["sample_every"])
                for r in metadata["runs"]]
    if len(observed) != len(expected) or set(observed) != expected:
        raise ValueError("Need all six cases and four distinct chains per schedule")


def efficiency_summary(comparisons):
    rng = random.Random(27099)
    ratios, replicates = {}, {}
    for case, comparison in comparisons.items():
        eff = comparison["candidate_vs_classical"]["ess_per_second"]
        ratios[case] = {label: ratio(eff["candidate"][label], eff["classical"][label]) for label in STATISTICS}
        replicates[case] = {label: [] for label in STATISTICS}
    undefined = {case: [label for label, value in values.items() if value is None]
                 for case, values in ratios.items() if None in values.values()}
    if undefined:
        return {"ratios_candidate_over_classical": ratios, "retain_as_default": False,
                "undefined_efficiency": undefined, "primary_score": None,
                "scientific_gates_pass": False,
                "reason": "Undefined ESS prevents a default-efficiency decision."}
    primary = []
    for _ in range(10_000):
        case_scores = []
        for case, comparison in comparisons.items():
            eff = comparison["candidate_vs_classical"]["ess_per_second"]
            a = [rng.randrange(4) for _ in range(4)]
            b = [rng.randrange(4) for _ in range(4)]
            sampled = {}
            for label in STATISTICS:
                sampled[label] = ratio([eff["candidate"][label][i] for i in a],
                                       [eff["classical"][label][i] for i in b])
                replicates[case][label].append(sampled[label])
            case_scores.append(min(sampled[label] for label in PRIMARY))
        primary.append(math.exp(statistics.fmean(map(math.log, case_scores))))
    score = math.exp(statistics.fmean(math.log(min(values[label] for label in PRIMARY)) for values in ratios.values()))
    intervals = {case: {label: [quantile(values, .05), quantile(values, .95)]
                        for label, values in parameters.items()} for case, parameters in replicates.items()}
    lower, upper = quantile(primary, .05), quantile(primary, .95)
    no_large_regression = min(value for parameters in ratios.values() for value in parameters.values()) >= .8
    scientific = all(pair["passes"] for comparison in comparisons.values() for pair in comparison.values())
    return {
        "ratios_candidate_over_classical": ratios, "ratio_intervals_90_percent": intervals,
        "primary_score": score, "primary_interval_90_percent": [lower, upper],
        "scientific_gates_pass": scientific, "no_median_regression_below_0_8": no_large_regression,
        "retain_as_default": scientific and score > 1.2 and lower > 1 and no_large_regression,
        "bootstrap": {"seed": 27099, "replicates": 10000,
                      "method": "independent resampling of four whole chains per schedule within each case; only four chains give limited uncertainty resolution"},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=Path, required=True)
    parser.add_argument("--reference-root", type=Path, required=True)
    parser.add_argument("--packages", type=Path, required=True)
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--java", default="java")
    parser.add_argument("--javac", default="javac")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    runs = args.runs.resolve()
    metadata = json.loads((runs / "metadata.json").read_text())
    validate_runs(metadata)
    for name, sha in metadata["source_files"].items():
        if digest(ROOT / name) != sha:
            raise ValueError(f"Engine source changed since the frozen comparison: {name}")
    versions = {}
    for file in args.packages.glob("*/version.xml"):
        package = ET.parse(file).getroot()
        versions[package.get("name")] = package.get("version")
    if versions != EXPECTED_PACKAGES:
        parser.error(f"Reference packages changed: {versions}")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    classes = runs / "diagnostic-classes"
    classes.mkdir(exist_ok=True)
    jars = [args.launcher.resolve(), *sorted(args.packages.resolve().glob("*/lib/*.jar"))]
    cp = os.pathsep.join(map(str, jars))
    helper = ROOT / "tools/beast/ReferenceDiagnostics.java"
    subprocess.run([args.javac, "--release", "17", "-cp", cp, "-d", str(classes), str(helper)], check=True)
    command = [args.java, "-Xms256m", "-Xmx2g", "-Djava.awt.headless=true", "-Duser.language=en",
               f"-Dbeast.user.package.dir={args.packages.resolve()}", "-cp", str(classes) + os.pathsep + cp,
               "rove.reference.ReferenceDiagnostics"]
    comparisons = {}
    scripts = [Path(__file__), ROOT / "tools/sampling/experiment.py",
               *[ROOT / "tools/beast" / name for name in
                 ["compare_inference.py", "compare_runs.py", "strict_clock.py", "report_inference.py", "run_reference.py"]]]
    archive = {"metadata": sanitize(metadata, ROOT),
               "analysis_scripts_sha256": {str(p.relative_to(ROOT)): digest(p) for p in scripts},
               "cases": {}, "reference": {
        "packages": versions, "jars": {p.name: digest(p) for p in jars}, "helper_sha256": digest(helper),
        "java": subprocess.run([args.java, "-version"], capture_output=True, text=True, check=True).stderr,
        "source": "archived BEAST 2.7.8 ORC runs, not fresh timing observations",
    }}
    for case in CASES:
        reference = args.reference_root / ("extension-40m" if case == "reference-mkv" else "production") / case
        ref_meta = json.loads((reference / "metadata.json").read_text())
        groups = {"candidate": [], "classical": [], "beast": []}
        for observation in [r for r in metadata["runs"] if r["case"] == case]:
            directory = runs / observation["directory"]
            for name, sha in observation["sha256"].items():
                if digest(directory / name) != sha:
                    raise ValueError(f"Run changed: {directory}/{name}")
            summary = directory / "summary.json"
            with (directory / "summary.console.log").open("w") as console:
                subprocess.run(command + ["inference-summary", str(directory / "trace.log"),
                    str(directory / "trees.nexus"), str(ROOT / "fixtures/validation" / case / "truth.newick"),
                    "10", str(summary)], stdout=console, stderr=subprocess.STDOUT, check=True)
            group = "classical" if observation["scheme"] == "classical" else "candidate"
            data = json.loads(summary.read_text())
            trace = read_trace(directory / "trace.log", 10)
            stats, canonical = normalize(data, trace, "rove")
            if len(trace["Sample"]) != data["retained_samples"]:
                raise ValueError("Tree and trace burn-in selections disagree")
            groups[group].append({"seed": observation["seed"], "seconds": observation["wall_seconds"],
                                  "summary": data, "statistics": stats, "trace": canonical})
        archive["cases"][case] = {
            "reference_metadata_sha256": digest(reference / "metadata.json"),
            "reference_metadata": sanitize(ref_meta, ROOT),
            "reference_operators": operator_counts(reference, ref_meta)["beast"],
            "classical_matches_archive": {},
        }
        for seed in ref_meta["seeds"]:
            directory = reference / "beast" / str(seed)
            observation = next(r for r in ref_meta["runs"] if r["engine"] == "beast" and r["seed"] == seed)
            for name in ["analysis.log", "trees.nexus", "analysis.xml", "analysis.xml.state"]:
                if digest(directory / name) != observation["sha256"][name]:
                    raise ValueError(f"BEAST archive changed: {directory}/{name}")
            data = json.loads((directory / "summary.json").read_text())
            stats, trace = normalize(data, read_trace(directory / "analysis.log", 10), "beast")
            if len(next(iter(trace.values()))) != data["retained_samples"]:
                raise ValueError("Archived tree and trace burn-in selections disagree")
            groups["beast"].append({"seed": seed, "seconds": observation["wall_seconds"],
                                    "summary": data, "statistics": stats, "trace": trace})
            current = runs / case / "classical" / str(seed)
            archive["cases"][case]["classical_matches_archive"][str(seed)] = {
                name: scientific_digest(current / name) == scientific_digest(reference / "rove" / str(seed) / name)
                for name in ["trace.log", "trees.nexus"]}
        for group in groups.values():
            group.sort(key=lambda r: r["seed"])
        comparisons[case] = {
            "candidate_vs_classical": compare_pair(groups["candidate"], groups["classical"], ["candidate", "classical"]),
            "candidate_vs_beast": compare_pair(groups["candidate"], groups["beast"], ["candidate", "beast"]),
        }
        destination = output / case
        destination.mkdir()
        write_json(destination / "comparison.json", comparisons[case])
        compact = {name: [{"seed": r["seed"], "wall_seconds": r["seconds"],
                          **{k: v for k, v in r["summary"].items() if k != "tree_traces"}} for r in group]
                   for name, group in groups.items()}
        write_json(destination / "summaries.json", compact)
        print(case, {k: v["gates"] for k, v in comparisons[case].items()}, flush=True)
    write_json(output / "archive.json", archive)
    result = efficiency_summary(comparisons)
    write_json(output / "efficiency.json", result)
    print(json.dumps({k: v for k, v in result.items() if k not in ["ratios_candidate_over_classical", "ratio_intervals_90_percent"]}), flush=True)


if __name__ == "__main__":
    main()
