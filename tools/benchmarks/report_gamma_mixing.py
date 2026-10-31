#!/usr/bin/env python3
"""Check posterior agreement and efficiency after the gamma-frequency timings."""

import argparse
import json
import math
import os
from pathlib import Path
import re
import statistics
import subprocess
import sys
import xml.etree.ElementTree as ET

from fbd_estimation import ROOT, sha
from gamma_mixing import CASES, PROTOCOL, write

sys.path.insert(0, str(ROOT / "tools/beast"))
from compare_inference import CONSTRAINT, clade_summary, convergence, difference, pooled
from compare_runs import read_trace
from report_inference import sanitize


def merge_node_metadata(line):
    # The archived dated writer emits adjacent blocks. BEAST 2's parser needs
    # one block per node; preserve both numeric values and all other text.
    if not line.startswith("tree "):
        return line, 0
    return re.subn(r"\[&height=([-+0-9.eE]+)\]\[&rate=([-+0-9.eE]+)\]",
                   r"[&height=\1,rate=\2]", line)


def compatible_trees(source, destination):
    merged = 0
    with source.open() as src, destination.open("w") as dst:
        for line in src:
            normalized, count = merge_node_metadata(line)
            dst.write(normalized)
            merged += count
    return {"merged_adjacent_blocks": merged, "source_sha256": sha(source),
            "normalized_sha256": sha(destination)}


def quantile(values, probability):
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    lower = int(position)
    return ordered[lower] + (position - lower) * (ordered[min(lower + 1, len(ordered) - 1)] - ordered[lower])


def correlation(x, y):
    mx, my = statistics.fmean(x), statistics.fmean(y)
    xx = sum((v - mx) ** 2 for v in x)
    yy = sum((v - my) ** 2 for v in y)
    return sum((a - mx) * (b - my) for a, b in zip(x, y)) / math.sqrt(xx * yy) if xx * yy else None


def validate_runs(metadata):
    expected = {(case, seed, weight) for i, case in enumerate(CASES)
                for seed in ([42101 + i * 20, 42111 + i * 20] if i < 2 else [42201 + (i - 2) * 10])
                for weight in [.1, 3.]}
    actual = [(r["case"], r["seed"], r["weight"]) for r in metadata["runs"]]
    if len(actual) != len(expected) or set(actual) != expected:
        raise ValueError("Final run set differs from the protocol")
    targets = {}
    for run in metadata["runs"]:
        steps = 40_000_000 if run["case"] == "reference-mkv" else 10_000_000
        if run["steps"] != steps or run["sample_every"] != 1000:
            raise ValueError("Final run length differs from the protocol")
        seeds = []
        for chain in run["chains"]:
            record = chain["record"]
            seeds.append(record["settings"]["seed"])
            gamma = [m["weight"] for m in record["moves"] if m["name"] == "gamma_shape_scale"]
            target = {"input": record["input"]["sha256"], "model": record["model"],
                      "settings": {k: v for k, v in record["settings"].items() if k != "seed"},
                      "gamma_initial_tuning": [m["initial_tuning"] for m in record["moves"]
                                               if m["name"] == "gamma_shape_scale"],
                      "other_moves": [(m["name"], m["weight"], m["initial_tuning"])
                                      for m in record["moves"] if m["name"] != "gamma_shape_scale"]}
            if targets.setdefault(run["case"], target) != target:
                raise ValueError("Target, initial state or other proposals changed within a case")
            if (record["executable_sha256"] != metadata["binary_sha256"]
                    or record["status"] != "completed" or record["completed_steps"] != steps
                    or record["settings"]["steps"] != steps
                    or record["settings"]["sample_every"] != 1000 or gamma != [run["weight"]]):
                raise ValueError("Chain length or gamma weight differs from its group")
        if seeds != list(range(run["seed"], run["seed"] + 4)):
            raise ValueError("Need four distinct consecutive seeds per group")


def compare(groups, fixed, constrained):
    labels = sorted(set(groups["baseline"][0]["summary"]["statistics"]) - fixed)
    means, diagnostics, efficiency = {}, {}, {}
    for label in labels:
        values, checks, rates = {}, {}, {}
        for name, runs in groups.items():
            stats = [r["summary"]["statistics"][label] for r in runs]
            values[name] = pooled(stats)
            checks[name] = convergence(stats, [r["trace"][label] for r in runs])
            rates[name] = [r["rove_ess"][label] / r["seconds"]
                           if r["rove_ess"][label] else None for r in runs]
        values.update(difference(values["baseline"], values["candidate"]))
        means[label], diagnostics[label] = values, checks
        efficiency[label] = {**rates, "median_ratio":
                             statistics.median(rates["candidate"]) / statistics.median(rates["baseline"])
                             if all(rates["candidate"] + rates["baseline"]) else None}
    tree_means, tree_checks = {}, {}
    for label in ["rooted_rf", "reference_topology"]:
        values, checks = {}, {}
        for name, runs in groups.items():
            stats = [r["summary"]["tree_statistics"][label] for r in runs]
            values[name] = pooled(stats)
            checks[name] = convergence(stats, [r["summary"]["tree_traces"][label] for r in runs], tree=True)
        values.update(difference(values["baseline"], values["candidate"]))
        tree_means[label], tree_checks[label] = values, checks
    summaries = {name: [r["summary"] for r in runs] for name, runs in groups.items()}
    observed = set().union(*(s["observed_clade_frequencies"] for runs in summaries.values() for s in runs))
    clades = {}
    for clade in sorted(observed):
        a, b = (clade_summary(summaries[name], clade) for name in groups)
        if clade == constrained:
            if any(f != 1 for f in a["chain_frequencies"] + b["chain_frequencies"]):
                raise ValueError("Imposed constraint was violated")
            continue
        if max(a["mean"], b["mean"]) < .01:
            continue
        clades[clade] = {"baseline": a, "candidate": b,
                         "difference": a["mean"] - b["mean"],
                         "agrees": abs(a["mean"] - b["mean"]) <= .05,
                         "precision_passes": max(a["mcse"], b["mcse"]) <= .0125}
    gates = {
        "means": all(s["agrees"] for s in means.values()),
        "scalar_convergence": all(s["passes"] for d in diagnostics.values() for s in d.values()),
        "tree_means": all(s["agrees"] for s in tree_means.values()),
        "tree_diagnostics": all(s["passes"] is not False for d in tree_checks.values() for s in d.values()),
        "clades": all(c["agrees"] for c in clades.values()),
        "clade_precision": all(c["precision_passes"] for c in clades.values()),
        "rove_diagnostics": all(r["rove_diagnostics_pass"] for runs in groups.values() for r in runs),
    }
    ratios = [e["median_ratio"] for e in efficiency.values()]
    no_regression = all(x is not None and x >= .8 for x in ratios)
    return {"means": means, "diagnostics": diagnostics, "efficiency": efficiency,
            "tree_means": tree_means, "tree_diagnostics": tree_checks, "clades": clades,
            "scientific_gates": gates, "scientific_pass": all(gates.values()),
            "no_median_regression_below_0_8": no_regression,
            "gamma_gain_above_2": (efficiency["gamma_shape"]["median_ratio"] or 0) > 2,
            "unique_topologies": {name: [r["summary"]["unique_topologies"] for r in runs] for name, runs in groups.items()},
            "fixed_columns": sorted(fixed), "imposed_ingroup": constrained}


def gamma_details(run):
    trace, values = run["trace"], run["trace"]["gamma_shape"]
    mean = statistics.fmean(values)
    centered = [v - mean for v in values]
    squared = sum(v * v for v in centered)
    return {"quantiles": {str(p): quantile(values, p) for p in [.001, .01, .05, .5, .95, .99, .999]},
            "fraction_below_0_01": sum(v < .01 for v in values) / len(values),
            "fraction_above_3": sum(v > 3 for v in values) / len(values),
            "lag_one_autocorrelation": sum(a * b for a, b in zip(centered, centered[1:])) / squared if squared else None,
            "integrated_autocorrelation_samples": len(values) / run["rove_ess"]["gamma_shape"] if run["rove_ess"]["gamma_shape"] else None,
            "pearson_correlations": {label: correlation(values, trace[label]) for label in
                ["posterior", "tree_height", "tree_length", "clock_rate", "clock_sigma", "mean_branch_rate", "fbd_origin"]
                if label in trace}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=Path, required=True)
    parser.add_argument("--packages", type=Path, required=True)
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--java", default="java")
    parser.add_argument("--javac", default="javac")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    runs = args.runs.resolve()
    metadata = json.loads((runs / "runs.json").read_text())
    if not metadata.get("finished_at") or metadata["protocol_sha256"] != sha(PROTOCOL):
        raise ValueError("Require completed timings and the frozen protocol")
    validate_runs(metadata)
    for path, expected_sha in metadata["engine_source_sha256"].items():
        if sha(ROOT / path) != expected_sha:
            raise ValueError(f"Analyze before changing engine source: {path}")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    classes = runs / "diagnostic-classes"
    classes.mkdir(exist_ok=True)
    jars = [args.launcher.resolve(), *sorted(args.packages.resolve().glob("*/lib/*.jar"))]
    classpath = os.pathsep.join(map(str, jars))
    helper = ROOT / "tools/beast/ReferenceDiagnostics.java"
    subprocess.run([args.javac, "--release", "17", "-cp", classpath, "-d", str(classes), str(helper)], check=True)
    command = [args.java, "-Xms256m", "-Xmx2g", "-Djava.awt.headless=true", "-Duser.language=en",
               f"-Dbeast.user.package.dir={args.packages.resolve()}", "-cp", str(classes) + os.pathsep + classpath,
               "rove.reference.ReferenceDiagnostics", "inference-summary"]
    anchor = runs / "dated-anchor.newick"
    anchor.write_text("(('Living A','Fossil B'),('Living C','Fossil D'));\n")
    report = {"metadata": sanitize(metadata, ROOT), "cases": {}, "gamma_traces": {},
              "tree_metadata_normalization": {},
              "analysis": {"script_sha256": sha(Path(__file__)), "helper_sha256": sha(helper),
                           "imported_scripts_sha256": {name: sha(ROOT / "tools/beast" / name) for name in
                               ["compare_inference.py", "compare_runs.py", "report_inference.py"]},
                           "packages": {ET.parse(p).getroot().get("name"): ET.parse(p).getroot().get("version")
                                        for p in sorted(args.packages.glob("*/version.xml"))},
                           "jars_sha256": {str(p.relative_to(args.packages.resolve())) if p.is_relative_to(args.packages.resolve()) else p.name: sha(p) for p in jars},
                           "java": subprocess.run([args.java, "-version"], text=True, capture_output=True, check=True).stderr,
                           "dated_anchor": anchor.read_text().strip()}}
    for case in CASES:
        dated = case.startswith("dated-")
        groups = {"baseline": [], "candidate": []}
        fixed_sets = []
        for observation in [r for r in metadata["runs"] if r["case"] == case]:
            name = "baseline" if observation["weight"] == .1 else "candidate"
            columns = observation["comparison"]["statistics"]
            fixed = {s["name"] for s in columns if s.get("fixed_by_model")}
            fixed_sets.append(fixed)
            sampled = [s for s in columns if s["name"] not in fixed]
            passed = all(s["rhat"] is not None and s["rhat"] <= 1.01
                         and all(e is not None and e >= 100 for e in s["ess"])
                         and not any(s["lag_limit_reached"]) for s in sampled)
            truth = anchor if dated else ROOT / "fixtures/validation" / case / "truth.newick"
            for index, chain in enumerate(observation["chains"]):
                directory = runs / chain["directory"]
                for file, expected_sha in chain["sha256"].items():
                    if sha(directory / file) != expected_sha:
                        raise ValueError(f"Run changed: {directory}/{file}")
                summary = directory / "independent-summary.json"
                trees = directory / "trees.nexus"
                if dated:
                    normalized = directory / "validation-trees.nexus"
                    report["tree_metadata_normalization"][chain["directory"]] = compatible_trees(trees, normalized)
                    trees = normalized
                with (directory / "summary.console.log").open("w") as console:
                    subprocess.run(command + [str(directory / "trace.log"), str(trees),
                                   str(truth), "10", str(summary)], stdout=console, stderr=console, check=True)
                data = json.loads(summary.read_text())
                trace = read_trace(directory / "trace.log", 10)
                if len(trace["Sample"]) != data["retained_samples"]:
                    raise ValueError("Tree and trace burn-in differ")
                entry = {"seconds": observation["group_wall_seconds"], "summary": data, "trace": trace,
                         "rove_ess": {s["name"]: s["ess"][index] for s in sampled},
                         "rove_diagnostics_pass": passed}
                groups[name].append(entry)
                report["gamma_traces"][chain["directory"]] = gamma_details(entry)
        if any(s != fixed_sets[0] for s in fixed_sets):
            raise ValueError("Fixed parameters differ between runs")
        result = compare(groups, fixed_sets[0], None if dated else CONSTRAINT)
        report["cases"][case] = result
        write(output / "report.json", report)
        worst = min(result["efficiency"].items(), key=lambda item: item[1]["median_ratio"] or 0)
        print(case, result["scientific_gates"], "gamma", result["efficiency"]["gamma_shape"]["median_ratio"],
              "lowest ratio", worst[0], worst[1]["median_ratio"], flush=True)
    eligible = {case: result["scientific_pass"] and result["no_median_regression_below_0_8"]
                and (not case.startswith("dated-") or result["gamma_gain_above_2"])
                for case, result in report["cases"].items()}
    report["decision"] = {"eligible": eligible, "scope": "global" if all(eligible.values()) else
                          "dated" if all(eligible[c] for c in CASES[:2]) else "opt_in_only"}
    write(output / "report.json", report)
    print(report["decision"], flush=True)


if __name__ == "__main__":
    main()
