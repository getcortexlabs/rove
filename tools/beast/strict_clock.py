#!/usr/bin/env python3
"""Run a strict-clock configuration in BEAST 2.7.8 and in Rove, then compare
their posterior summaries with criteria fixed before the runs."""

import argparse
import json
import math
import platform
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from pathlib import Path

from compare_runs import read_trace, rhat

HERE = Path(__file__).resolve().parent

# Rove trace columns and the BEAST columns they correspond to.
STATISTICS = {
    "posterior": "posterior",
    "likelihood": "likelihood",
    "prior": "prior",
    "birth_death": "BirthDeath",
    "birth_difference_rate": "BDBirthRate",
    "relative_death_rate": "BDDeathRate",
    "tree_height": "Tree.height",
    "tree_length": "Tree.treeLength",
}
# A posterior mean agrees when the engines differ by at most this many
# combined Monte Carlo standard errors.
MEAN_TOLERANCE = 4.0
# A clade agrees when its pooled posterior frequencies differ by at most this.
CLADE_TOLERANCE = 0.05
CLADE_THRESHOLD = 0.01
# BEAST's defaults for the operator inputs that set the step of a move.
DEFAULT_STEP = {"size": "1.0", "scaleFactor": "0.75"}
TREE_DIAGNOSTICS = ("unique_topologies", "reference_topology_posterior", "mean_rooted_rf_distance_to_reference")


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def move(operator):
    """The Rove move that follows a BEAST operator, and the input holding its step."""
    spec = operator.get("spec")
    if spec == "Uniform":
        return "node_height", None
    if spec == "SubtreeSlide":
        return "subtree_slide", "size"
    if spec == "Exchange":
        return ("wide_exchange" if operator.get("isNarrow") == "false" else "narrow_exchange"), None
    if spec == "WilsonBalding":
        return "wilson_balding", None
    if spec == "ScaleOperator":
        if operator.get("tree"):
            return ("root_scale" if operator.get("rootOnly") == "true" else "tree_scale"), "scaleFactor"
        parameter = operator.get("parameter", "")
        if parameter.startswith("@BDBirthRate"):
            return "birth_difference_scale", "scaleFactor"
        if parameter.startswith("@BDDeathRate"):
            return "relative_death_scale", "scaleFactor"
    raise ValueError(f"Rove has no move for operator {operator.get('id')}")


def configuration(xml):
    """The settings of the XML that the Rove run has to reproduce."""
    model = ET.parse(xml).getroot()
    run = model.find("run")
    moves = {}
    for operator in run.iter("operator"):
        name, step = move(operator)
        if name in moves:
            raise ValueError(f"More than one operator for {name}")
        moves[name] = {
            "weight": float(operator.get("weight")),
            "tuning": operator.get(step, DEFAULT_STEP[step]) if step else None,
            "optimise": operator.get("optimise", "true") == "true",
        }
    start = {parameter.get("id").split(".")[0]: float(parameter.text)
             for parameter in run.find("state").iter("parameter")}
    return {
        "chain_length": int(run.get("chainLength")),
        "log_every": int(model.find(".//logger[@id='tracelog']").get("logEvery")),
        "monophyly": [
            [taxon.get("id") for taxon in prior.iter("taxon")]
            for prior in model.iter("distribution")
            if prior.get("spec", "").endswith("MRCAPrior") and prior.get("monophyletic") == "true"
        ],
        "conditioning": "variable" if any(alignment.get("ascertained") == "true"
                                          for alignment in model.iter("alignment")) else "none",
        "moves": moves,
        "initial": {"birth_difference_rate": start["BDBirthRate"], "relative_death_rate": start["BDDeathRate"]},
    }


def tuned(config):
    """Whether the operators adapt their steps; Rove tunes every move or none."""
    flags = {m["optimise"] for m in config["moves"].values() if m["tuning"] is not None}
    if len(flags) > 1:
        raise ValueError("Rove tunes every move or none")
    return flags != {False}


def rove_command(rove, matrix, directory, seed, config):
    command = [str(rove), "sample", str(matrix), "--output", str(directory), "--seed", str(seed),
               "--steps", str(config["chain_length"]), "--sample-every", str(config["log_every"]),
               "--conditioning", config["conditioning"]]
    for taxa in config["monophyly"]:
        command += ["--monophyly", ",".join(taxa)]
    if not tuned(config):
        command.append("--no-tuning")
    for name, m in sorted(config["moves"].items()):
        if m["tuning"] is not None:
            command += ["--tuning", f"{name}={m['tuning']}"]
    return command


def check_run(record, config):
    """Raise unless a Rove run.json records the moves and start of the XML."""
    problems = []
    names = {entry["name"] for entry in record["moves"]}
    for name in sorted(set(config["moves"]) - names):
        problems.append(f"Rove has no {name}")
    for entry in record["moves"]:
        expected = config["moves"].get(entry["name"], {"weight": 0.0, "tuning": None})
        if entry["weight"] != expected["weight"]:
            problems.append(f"{entry['name']} has weight {entry['weight']}, not {expected['weight']}")
        step = None if expected["tuning"] is None else float(expected["tuning"])
        if entry["initial_tuning"] != step:
            problems.append(f"{entry['name']} starts at {entry['initial_tuning']}, not {step}")
    if record["settings"]["tuning"] != tuned(config):
        problems.append("tuning differs")
    for key, value in config["initial"].items():
        if record["settings"][f"initial_{key}"] != value:
            problems.append(f"{key} starts at {record['settings'][f'initial_{key}']}, not {value}")
    if problems:
        raise ValueError("; ".join(problems))


def pooled(summaries, label):
    """Mean of the per-run means and its Monte Carlo standard error."""
    runs = [summary["statistics"][label] for summary in summaries]
    mean = sum(run["mean"] for run in runs) / len(runs)
    ess = [run["ess"] for run in runs]
    if any(value is None or value <= 0 for value in ess):
        return {"mean": mean, "mcse": None, "ess": ess}
    variance = sum(run["sd"] ** 2 / run["ess"] for run in runs) / len(runs) ** 2
    return {"mean": mean, "mcse": math.sqrt(variance), "ess": ess}


def clade_frequencies(summaries):
    clades = set().union(*(summary["observed_clade_frequencies"] for summary in summaries))
    return {clade: sum(summary["observed_clade_frequencies"].get(clade, 0.0) for summary in summaries)
            / len(summaries) for clade in clades}


def compare(rove, beast):
    """Apply the agreement criteria to two lists of ReferenceDiagnostics summaries."""
    means = {}
    for label, beast_label in STATISTICS.items():
        r, b = pooled(rove, label), pooled(beast, beast_label)
        if r["mcse"] is None or b["mcse"] is None:
            means[label] = {"rove": r, "beast": b, "z": None, "agrees": None}
            continue
        scale = math.hypot(r["mcse"], b["mcse"])
        z = (r["mean"] - b["mean"]) / scale if scale > 0 else (0.0 if r["mean"] == b["mean"] else math.inf)
        means[label] = {"rove": r, "beast": b, "z": z if math.isfinite(z) else None,
                        "agrees": abs(z) <= MEAN_TOLERANCE}
    r_clades, b_clades = clade_frequencies(rove), clade_frequencies(beast)
    clades = {}
    for clade in sorted(set(r_clades) | set(b_clades)):
        r, b = r_clades.get(clade, 0.0), b_clades.get(clade, 0.0)
        if max(r, b) >= CLADE_THRESHOLD:
            clades[clade] = {"rove": r, "beast": b, "difference": r - b,
                             "agrees": abs(r - b) <= CLADE_TOLERANCE}
    defined = [m["agrees"] for m in means.values() if m["agrees"] is not None]
    return {
        "criteria": {
            "means": f"|Rove mean - BEAST mean| <= {MEAN_TOLERANCE} combined Monte Carlo standard errors, "
                     "each the root of the summed sd^2/ESS over runs divided by the number of runs",
            "clades": f"|difference in pooled frequency| <= {CLADE_TOLERANCE} for every clade "
                      f"with frequency >= {CLADE_THRESHOLD} in either engine",
        },
        "means": means,
        "clades": clades,
        "all_means_agree": bool(defined) and all(defined),
        "all_clades_agree": all(c["agrees"] for c in clades.values()),
    }


def efficiency(summaries, columns, seconds):
    """Effective sample size per second of wall time, for each run."""
    return {label: [None if s["statistics"][column]["ess"] is None else s["statistics"][column]["ess"] / t
                    for s, t in zip(summaries, seconds)]
            for label, column in columns.items()}


def java_command(packages, launcher, classes):
    jars = [launcher.resolve()] + sorted(packages.resolve().glob("*/lib/*.jar"))
    classpath = ":".join(str(path) for path in [classes, *jars])
    return ["java", "-Xms256m", "-Xmx2g", "-Xss8m", "-Djava.awt.headless=true", "-Duser.language=en",
            f"-Dbeast.user.package.dir={packages.resolve()}", "-cp", classpath,
            "rove.reference.ReferenceDiagnostics"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packages", type=Path, required=True)
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--xml", type=Path, required=True)
    parser.add_argument("--matrix", type=Path, required=True)
    parser.add_argument("--truth", type=Path, required=True)
    parser.add_argument("--rove", type=Path, required=True, help="the rove executable")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seeds", nargs="+", type=int, required=True)
    parser.add_argument("--burnin", type=int, default=10)
    args = parser.parse_args()
    if len(args.seeds) < 2:
        parser.error("use at least two seeds")
    config = configuration(args.xml)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    seeds = [str(seed) for seed in args.seeds]
    subprocess.run([sys.executable, str(HERE / "run_reference.py"), "--packages", str(args.packages),
                    "--launcher", str(args.launcher), "--xml", str(args.xml), "--truth", str(args.truth),
                    "--output", str(output / "beast"), "--burnin", str(args.burnin), "--seeds", *seeds],
                   check=True)
    diagnostics = java_command(args.packages, args.launcher, output / "beast" / "classes")
    timings = {}
    for seed in seeds:
        directory = output / "rove" / f"seed-{seed}"
        command = rove_command(args.rove.resolve(), args.matrix.resolve(), directory, seed, config)
        directory.parent.mkdir(parents=True, exist_ok=True)
        start = time.perf_counter()
        with (output / "rove" / f"seed-{seed}.console.log").open("w") as console:
            subprocess.run(command, stderr=console, check=True)
        timings[seed] = time.perf_counter() - start
        check_run(json.loads((directory / "run.json").read_text()), config)
        print(f"Rove seed {seed}: {timings[seed]:.3f} seconds", flush=True)
    for seed in seeds:
        directory = output / "rove" / f"seed-{seed}"
        subprocess.run(diagnostics + ["summary", str(directory / "trace.log"), str(directory / "trees.nexus"),
                                      str(args.truth.resolve()), str(args.burnin), str(directory / "summary.json")],
                       check=True, capture_output=True)
    def load(engine, name):
        return [json.loads((output / engine / f"seed-{seed}" / name).read_text()) for seed in seeds]

    rove, beast = load("rove", "summary.json"), load("beast", "summary.json")
    beast_tree = next(Path(output / "beast" / f"seed-{seeds[0]}").glob("*.trees")).name
    traces = {
        "rove": [read_trace(output / "rove" / f"seed-{seed}" / "trace.log", args.burnin) for seed in seeds],
        "beast": [read_trace(output / "beast" / f"seed-{seed}" / "analysis.log", args.burnin) for seed in seeds],
    }
    def finite(value):
        return value if value is None or math.isfinite(value) else None

    convergence = {
        engine: {label: finite(rhat([trace[label if engine == "rove" else STATISTICS[label]] for trace in runs]))
                 for label in STATISTICS}
        for engine, runs in traces.items()
    }
    seconds = {"rove": [timings[seed] for seed in seeds],
               "beast": [run["wall_seconds"] for run in load("beast", "timing.json")]}
    result = compare(rove, beast)
    result.update({
        "seeds": args.seeds, "chain_length": config["chain_length"], "log_every": config["log_every"],
        "burnin_percent": args.burnin, "conditioning": config["conditioning"], "monophyly": config["monophyly"],
        "moves": config["moves"], "beast_tree_file": beast_tree, "rhat": convergence,
        "trees": {engine: {key: [summary[key] for summary in summaries] for key in TREE_DIAGNOSTICS}
                  for engine, summaries in (("rove", rove), ("beast", beast))},
        "wall_seconds": seconds,
        "ess_per_second": {"rove": efficiency(rove, {label: label for label in STATISTICS}, seconds["rove"]),
                           "beast": efficiency(beast, STATISTICS, seconds["beast"])},
        "rove_executable": str(args.rove), "platform": platform.platform(),
    })
    write_json(output / "comparison.json", result)
    print(json.dumps({key: result[key] for key in ("all_means_agree", "all_clades_agree")}, indent=2))


if __name__ == "__main__":
    main()
