#!/usr/bin/env python3
"""Apply the recorded inference protocol to a completed pair of engine batches."""

import argparse
import json
import math
import statistics
from pathlib import Path

from compare_runs import read_trace, rhat
from run_reference import write_json
from strict_clock import STATISTICS as STRICT_STATISTICS

STATISTICS = {
    **STRICT_STATISTICS,
    "gamma_shape": "gammaShape",
    "clock_sigma": "ORCsigma",
    "gamma_prior": "GammaShapePrior",
    "clock_rate_prior": "ORCRatePriorDistribution",
    "clock_sigma_prior": "ORCsigmaPrior",
    "mean_branch_rate": "sum(ORCRates.)",
}
CONSTRAINT = ",".join(sorted(f"T{i}" for i in range(1, 20)))
MEAN_Z = 4.0
MAX_RHAT = 1.01
MIN_CHAIN_ESS = 100
MIN_TOTAL_ESS = 400
MAX_CLADE_DIFFERENCE = 0.05
MAX_CLADE_MCSE = 0.0125


def pooled(values):
    means = [v["mean"] for v in values]
    count = len(values)
    constant = all(v["sd"] == 0 for v in values)
    variances = [0.0 if v["sd"] == 0 else v["sd"] ** 2 / v["ess"]
                 if v["ess"] is not None and v["ess"] > 0 else None for v in values]
    within = sum(variances) / count ** 2 if None not in variances else None
    between = statistics.variance(means) / count
    return {"mean": statistics.fmean(means), "chain_means": means,
            "mcse": math.sqrt(max(within, between)) if within is not None else None,
            "within_mcse": math.sqrt(within) if within is not None else None,
            "between_mcse": math.sqrt(between), "ess": [v["ess"] for v in values],
            "constant_in_every_chain": constant}


def difference(first, second):
    delta = first["mean"] - second["mean"]
    if first["mcse"] is None or second["mcse"] is None:
        return {"difference": delta, "z": None, "agrees": False}
    scale = math.hypot(first["mcse"], second["mcse"])
    z = delta / scale if scale > 0 else (0.0 if delta == 0 else math.inf)
    return {"difference": delta, "combined_mcse": scale,
            "z": z if math.isfinite(z) else None, "agrees": abs(z) <= MEAN_Z}


def convergence(values, traces, *, tree=False):
    diagnostic = rhat(traces)
    ess = [v["ess"] for v in values]
    constant = [v["sd"] == 0 for v in values]
    # Rare or fixed tree indicators cannot establish topology mixing. Report
    # that limitation explicitly; do not invent an ESS for a constant chain.
    limited = tree and any(constant)
    passed = (diagnostic is not None and math.isfinite(diagnostic) and diagnostic <= MAX_RHAT
              and all(e is not None and e >= MIN_CHAIN_ESS for e in ess)
              and sum(ess) >= MIN_TOTAL_ESS)
    return {"rhat": diagnostic if diagnostic is None or math.isfinite(diagnostic) else "infinite",
            "ess": ess, "constant_chains": constant,
            "limited_tree_mixing_diagnostic": limited,
            "passes": None if limited else passed}


def clade_summary(summaries, clade):
    means = [s["observed_clade_frequencies"].get(clade, 0.0) for s in summaries]
    variances = []
    for summary in summaries:
        batches = summary["clade_batch_frequencies"].get(clade, [0.0] * 20)
        variances.append(statistics.variance(batches) / len(batches))
    count = len(summaries)
    within = sum(variances) / count ** 2
    between = statistics.variance(means) / count
    return {"mean": statistics.fmean(means), "mcse": math.sqrt(max(within, between)),
            "within_mcse": math.sqrt(within), "between_mcse": math.sqrt(between),
            "chain_frequencies": means, "constant_frequency": len(set(means)) == 1 and means[0] in (0, 1)}


def normalize(summary, trace, engine):
    """Canonical statistics; BEAST's sum logger is not its weighted rate statistic."""
    stats, columns = {}, {}
    for label, beast in STATISTICS.items():
        name = beast if engine == "beast" else label
        divisor = 38 if engine == "beast" and label == "mean_branch_rate" else 1
        stats[label] = {key: value / divisor if value is not None and key in ("mean", "sd", "hpd_low", "hpd_high") else value
                        for key, value in summary["statistics"][name].items()}
        columns[label] = [value / divisor for value in trace[name]]
    return stats, columns


def compare(directory):
    metadata = json.loads((directory / "metadata.json").read_text())
    if metadata["smoke_only"] or len(metadata["seeds"]) < 4:
        raise ValueError("Smoke runs are not posterior comparisons")
    records, traces, canonical, seconds = {}, {}, {}, {}
    for engine in ("rove", "beast"):
        records[engine], traces[engine], canonical[engine], seconds[engine] = [], [], [], []
        for seed in metadata["seeds"]:
            path = directory / engine / str(seed)
            summary = json.loads((path / "summary.json").read_text())
            trace = read_trace(path / ("trace.log" if engine == "rove" else "analysis.log"), metadata["burnin_percent"])
            if summary["retained_samples"] != len(trace["Sample"]):
                raise ValueError("Tree and trace sample counts differ")
            stats, columns = normalize(summary, trace, engine)
            records[engine].append(summary)
            traces[engine].append(columns)
            canonical[engine].append(stats)
            seconds[engine].append(next(r["wall_seconds"] for r in metadata["runs"]
                                        if r["seed"] == seed and r["engine"] == engine))
    means, diagnostics, efficiency, tree_means = {}, {"rove": {}, "beast": {}}, {"rove": {}, "beast": {}}, {}
    for label in STATISTICS:
        means[label] = {}
        for engine in ("rove", "beast"):
            values = [s[label] for s in canonical[engine]]
            means[label][engine] = pooled(values)
            diagnostics[engine][label] = convergence(values, [t[label] for t in traces[engine]])
            efficiency[engine][label] = [v["ess"] / duration if v["ess"] is not None else None
                                          for v, duration in zip(values, seconds[engine])]
        means[label].update(difference(means[label]["rove"], means[label]["beast"]))
    for label in ("rooted_rf", "reference_topology"):
        tree_means[label] = {}
        for engine in ("rove", "beast"):
            values = [s["tree_statistics"][label] for s in records[engine]]
            tree_means[label][engine] = pooled(values)
            diagnostics[engine][label] = convergence(values, [s["tree_traces"][label] for s in records[engine]], tree=True)
        tree_means[label].update(difference(tree_means[label]["rove"], tree_means[label]["beast"]))
    all_clades = set().union(*(s["observed_clade_frequencies"] for runs in records.values() for s in runs))
    truth = set(records["rove"][0]["reference_clades"])
    clades = {}
    for clade in sorted(all_clades | truth | {CONSTRAINT}):
        r, b = (clade_summary(records[e], clade) for e in ("rove", "beast"))
        selected = max(r["mean"], b["mean"]) >= 0.01
        if not selected and clade not in truth:
            continue
        delta = r["mean"] - b["mean"]
        clades[clade] = {"rove": r, "beast": b, "difference": delta,
                         "combined_mcse": math.hypot(r["mcse"], b["mcse"]),
                         "constrained": clade == CONSTRAINT, "reference_clade": clade in truth,
                         "tested": selected and clade != CONSTRAINT,
                         "agrees": abs(delta) <= MAX_CLADE_DIFFERENCE,
                         "precision_passes": max(r["mcse"], b["mcse"]) <= MAX_CLADE_MCSE}
    constrained = clades.pop(CONSTRAINT)
    if any(f != 1.0 for e in ("rove", "beast") for f in constrained[e]["chain_frequencies"]):
        raise ValueError("The imposed ingroup constraint was violated")
    compared = [c for c in clades.values() if c["tested"]]
    result = {
        "case": metadata["case"], "source_commit": metadata["source_commit"],
        "protocol_sha256": metadata["protocol_sha256"], "steps": metadata["steps"], "seeds": metadata["seeds"],
        "burnin_percent": metadata["burnin_percent"], "means": means, "convergence": diagnostics,
        "tree_means": tree_means, "clades": clades, "imposed_ingroup": constrained,
        "unique_topologies": {e: [s["unique_topologies"] for s in records[e]] for e in records},
        "wall_seconds": seconds, "ess_per_second": efficiency,
        "all_means_agree": all(m["agrees"] for m in means.values()),
        "all_tree_means_agree": all(m["agrees"] for m in tree_means.values()),
        "all_clades_agree": all(c["agrees"] for c in compared),
        "clade_precision_passes": all(c["precision_passes"] for c in compared),
        "convergence_passes": all(d["passes"] is not False for engine in diagnostics.values() for d in engine.values()),
    }
    result["passes"] = all(result[k] for k in ("all_means_agree", "all_tree_means_agree", "all_clades_agree",
                                               "clade_precision_passes", "convergence_passes"))
    write_json(directory / "comparison.json", result)
    # Keep compact summaries; per-sample tree traces stay in local run directories.
    compact = {e: [{k: v for k, v in s.items() if k != "tree_traces"} for s in records[e]] for e in records}
    write_json(directory / "summaries.json", compact)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    result = compare(args.directory.resolve())
    print(json.dumps({k: v for k, v in result.items() if isinstance(v, bool)}, indent=2))


if __name__ == "__main__":
    main()
