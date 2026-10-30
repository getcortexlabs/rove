#!/usr/bin/env python3
"""Publish aggregate expanded benchmarks without private alignments or trees."""

import argparse
import json
import shutil
import statistics
import sys
from pathlib import Path

from run import ROOT, percentile, read_results, summarize
from expanded import write

sys.path.insert(0, str(ROOT / "tools/beast"))
from compare_inference import STATISTICS, clade_summary, convergence, difference, pooled
from compare_runs import read_trace


def distribution(values):
    return {"values": values, "median": statistics.median(values), "mean": statistics.fmean(values),
            "sd": statistics.stdev(values) if len(values) > 1 else None, "min": min(values), "max": max(values)}


def inference(directory, config, timings, name):
    settings = config["cases"][name]
    summaries, traces, seconds = {}, {}, {}
    result = {"settings": settings, "wall_seconds": {}, "statistics": {}, "tree": {}}
    for engine in ("rove", "beast"):
        paths = [directory / name / engine / str(seed) for seed in config["seeds"]]
        summaries[engine] = [json.loads((path / "summary.json").read_text()) for path in paths]
        traces[engine] = [read_trace(path / "trace.log", config["burnin_percent"]) for path in paths]
        if any(s["retained_samples"] != len(t["Sample"]) for s, t in zip(summaries[engine], traces[engine])):
            raise ValueError("Trace/tree counts disagree")
        seconds[engine] = [next(r["wall_seconds"] for r in timings["runs"]
                               if r["case"] == name and r["engine"] == engine and r["seed"] == seed)
                           for seed in config["seeds"]]
        result["wall_seconds"][engine] = distribution(seconds[engine])
    result["beast_over_rove_wall"] = distribution([b / r for b, r in zip(seconds["beast"], seconds["rove"])])
    for label, beast_label in STATISTICS.items():
        entry = {}
        for engine in ("rove", "beast"):
            key = beast_label if engine == "beast" else label
            divisor = 2 * settings["taxa"] - 2 if engine == "beast" and label == "mean_branch_rate" else 1
            values = [{field: value / divisor if field in ("mean", "sd", "hpd_low", "hpd_high") and value is not None else value
                       for field, value in summary["statistics"][key].items()} for summary in summaries[engine]]
            columns = [[x / divisor for x in trace[key]] for trace in traces[engine]]
            entry[engine] = {"chains": values, "pooled": pooled(values), "convergence": convergence(values, columns),
                             "ess_per_second": [v["ess"] / t if v["ess"] is not None else None
                                                for v, t in zip(values, seconds[engine])]}
        entry["comparison"] = difference(entry["rove"]["pooled"], entry["beast"]["pooled"])
        efficiencies = [entry[e]["ess_per_second"] for e in ("rove", "beast")]
        entry["rove_over_beast_ess_per_second"] = (
            statistics.median(efficiencies[0]) / statistics.median(efficiencies[1])
            if all(x is not None and x > 0 for values in efficiencies for x in values) else None)
        result["statistics"][label] = entry
    for engine in ("rove", "beast"):
        values = [s["tree_statistics"]["rooted_rf"] for s in summaries[engine]]
        result["tree"][engine] = {
            "unique_topologies": [s["unique_topologies"] for s in summaries[engine]],
            "retained_samples": [s["retained_samples"] for s in summaries[engine]],
            "rf": {"pooled": pooled(values), "convergence": convergence(values, [s["tree_traces"]["rooted_rf"]
                                                                                for s in summaries[engine]], tree=True)},
            "rf_ess_per_second": [v["ess"] / t if v["ess"] is not None else None for v, t in zip(values, seconds[engine])],
            "exact_reference_frequencies": [s["reference_topology_posterior"] for s in summaries[engine]],
        }
    clades = set().union(*(set(s["observed_clade_frequencies"]) for runs in summaries.values() for s in runs))
    constraint = ",".join(sorted(f"T{i}" for i in range(1, 20))) if settings["constrained"] else None
    comparisons = []
    for clade in sorted(clades):
        if clade == constraint:
            continue
        a, b = (clade_summary(summaries[e], clade) for e in ("rove", "beast"))
        if max(a["mean"], b["mean"]) <= 0.01:
            continue
        comparisons.append({"difference": abs(a["mean"] - b["mean"]), "rove_mcse": a["mcse"], "beast_mcse": b["mcse"]})
    result["tree"]["clades"] = {
        "compared": len(comparisons), "constraint_excluded": bool(constraint),
        "max_absolute_frequency_difference": max((c["difference"] for c in comparisons), default=None),
        "above_difference_limit": sum(c["difference"] > 0.05 for c in comparisons),
        "above_mcse_limit": sum(max(c["rove_mcse"], c["beast_mcse"]) > 0.0125 for c in comparisons),
        "max_mcse": {e: max((c[f"{e}_mcse"] for c in comparisons), default=None) for e in ("rove", "beast")},
        "passes": bool(comparisons) and all(c["difference"] <= 0.05 and max(c["rove_mcse"], c["beast_mcse"]) <= 0.0125
                                           for c in comparisons),
    }
    result["continuous_gates_pass"] = all(s["comparison"]["agrees"] and all(s[e]["convergence"]["passes"]
                                           for e in ("rove", "beast")) for s in result["statistics"].values())
    result["sampling_gates_pass"] = result["continuous_gates_pass"] and result["tree"]["clades"]["passes"] and all(
        result["tree"][e]["rf"]["convergence"]["passes"] is True for e in ("rove", "beast"))
    return result


def kernels(directory):
    rust = summarize([read_results((directory / f"rove-{i}.tsv").read_text(), 9) for i in range(3)])
    adapters = {}
    for backend in ("java", "beagle"):
        processes = [json.loads((directory / f"{backend}-{i}.json").read_text()) for i in range(3)]
        for first in processes[0]:
            name, metric = first["case"], first["metric"]
            records = [next(row for row in process if row["case"] == name and row["metric"] == metric) for process in processes]
            if any(row["implementation"] != first["implementation"] for row in records):
                raise ValueError("Kernel implementation changed between repetitions")
            samples = [[ns / row["iterations"] for ns in row["elapsed_ns"]] for row in records]
            medians = [statistics.median(values) for values in samples]
            flat = [x for values in samples for x in values]
            adapters.setdefault(f"{name}/{backend}", {})[metric] = {
                "median_ns": statistics.median(medians), "process_medians_ns": medians,
                "min_process_median_ns": min(medians), "max_process_median_ns": max(medians),
                "batch_p10_ns": percentile(flat, 0.1), "batch_p90_ns": percentile(flat, 0.9),
                "implementation": first["implementation"], "cold_setup_ns": [r["setup_ns"] for r in records],
            }
    comparisons = {}
    for name, values in rust.items():
        case, path, backend = name.split("/")
        if path != "mk" or backend != "neon":
            continue
        comparisons[case] = {metric: {engine: adapters[f"{case}/{engine}"][metric]["median_ns"] / values["metrics"][metric]["median_ns"]
                                     for engine in ("java", "beagle")}
                             for metric in ("full_cached", "full_with_branches", "tip_update", "internal_update")}
    return {"rove": rust, "adapters": adapters, "adapter_over_rove_mk_neon": comparisons}


def redact(value, directory):
    if isinstance(value, str):
        return value.replace(str(directory), "$RUN").replace(str(ROOT), "$REPO").replace(str(Path.home()), "$HOME")
    if isinstance(value, dict):
        return {k: redact(v, directory) for k, v in value.items()}
    if isinstance(value, list):
        return [redact(v, directory) for v in value]
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    directory = args.directory.resolve()
    config = json.loads((directory / "config.json").read_text())
    timings = json.loads((directory / "timings.json").read_text())
    if len(timings["runs"]) != len(config["cases"]) * len(config["seeds"]) * 2 or "finished_at" not in timings:
        raise ValueError("Incomplete measurement batch")
    result = {name: inference(directory, config, timings, name) for name in config["cases"]}
    kernel = kernels(directory / "kernel")
    args.output.mkdir(parents=True, exist_ok=False)
    write(args.output / "inference.json", result)
    write(args.output / "kernel-summary.json", kernel)
    write(args.output / "metadata.json", redact({"config": config, "timings": timings}, directory))
    fixed = {}
    for name in config["cases"]:
        case = directory / name
        fixed[name] = {"pilot": json.loads((case / "pilot/fixed-state.json").read_text()),
                       "production": {str(seed): json.loads((case / "beast" / str(seed) / "fixed-state.json").read_text())
                                      for seed in config["seeds"]}}
    write(args.output / "fixed-states.json", fixed)
    raw = args.output / "kernel"
    raw.mkdir()
    for path in (directory / "kernel").iterdir():
        if path.suffix in (".json", ".tsv"):
            shutil.copyfile(path, raw / path.name)
    shutil.copyfile(directory / "kernel-input.json", raw / "input.json")
    print(f"Published aggregates in {args.output}")


if __name__ == "__main__":
    main()
