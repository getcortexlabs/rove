#!/usr/bin/env python3
"""Build and measure the Rust likelihood benchmark in fresh, sequential processes."""

import argparse
import csv
import hashlib
import io
import json
import math
import os
import platform
import re
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
METRICS = {
    "parse", "compress", "core_setup", "full_cached", "full_with_branches",
    "tip_update", "internal_update", "accept_proposal", "reject_proposal",
    "store", "store_restore",
}
TRANSITIONS = {"matrices", "mk"}
BACKENDS = {"portable", "neon"}
COUNTS = ["allocation_calls", "reallocation_calls", "requested_bytes", "deallocation_calls"]
CASE_FIELDS = ["taxa", "characters", "patterns", "core_patterns", "stride", "states", "categories",
               "missing_cells"]


def command(args, *, check=True, env=None):
    return subprocess.run(args, cwd=ROOT, env=env, text=True, capture_output=True, check=check)


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def percentile(values, fraction):
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    low = math.floor(position)
    high = math.ceil(position)
    return ordered[low] + (ordered[high] - ordered[low]) * (position - low)


def read_results(text, samples):
    cases = {}
    for row in csv.DictReader(io.StringIO(text), delimiter="\t"):
        metric, transitions, backend = row["metric"], row["transitions"], row["backend"]
        if metric not in METRICS:
            raise ValueError(f"unknown metric: {metric}")
        if transitions not in TRANSITIONS:
            raise ValueError(f"unknown transitions: {transitions}")
        if backend not in BACKENDS:
            raise ValueError(f"unknown backend: {backend}")
        name = f"{row['case']}/{transitions}/{backend}"
        metadata = {field: int(row[field]) for field in CASE_FIELDS}
        metadata.update(case=row["case"], transitions=transitions, backend=backend, shape=row["shape"],
                        baseline_log_likelihood=float(row["baseline_log_likelihood"]))
        if not math.isfinite(metadata["baseline_log_likelihood"]):
            raise ValueError(f"nonfinite likelihood: {name}")
        case = cases.setdefault(name, {"configuration": metadata, "metrics": {}})
        if case["configuration"] != metadata:
            raise ValueError(f"inconsistent configuration: {name}")
        observation = {field: int(row[field]) for field in COUNTS}
        entry = case["metrics"].setdefault(metric, {
            "dirty_nodes": int(row["dirty_nodes"]), "allocations": observation, "samples": [],
        })
        if entry["allocations"] != observation or entry["dirty_nodes"] != int(row["dirty_nodes"]):
            raise ValueError(f"inconsistent metric metadata: {name}/{metric}")
        sample, iterations, elapsed = (int(row[field]) for field in ["sample", "iterations", "elapsed_ns"])
        if sample != len(entry["samples"]) or iterations <= 0 or elapsed <= 0:
            raise ValueError(f"invalid batch: {name}/{metric}")
        entry["samples"].append({"iterations": iterations, "elapsed_ns": elapsed})
    if not cases:
        raise ValueError("empty benchmark output")
    for name, case in cases.items():
        if set(case["metrics"]) != METRICS:
            raise ValueError(f"incomplete metrics: {name}")
        for metric, entry in case["metrics"].items():
            if len(entry["samples"]) != samples:
                raise ValueError(f"incomplete samples: {name}/{metric}")
    return cases


def summarize(runs):
    if not runs or any(set(run) != set(runs[0]) for run in runs):
        raise ValueError("runs must contain the same cases")
    summary = {}
    for name, first in runs[0].items():
        if any(run[name]["configuration"] != first["configuration"] for run in runs):
            raise ValueError(f"configuration changed between processes: {name}")
        result = {"configuration": first["configuration"], "metrics": {}}
        for metric, entry in first["metrics"].items():
            observations = [run[name]["metrics"][metric] for run in runs]
            if any(o["dirty_nodes"] != entry["dirty_nodes"] for o in observations):
                raise ValueError(f"update path changed: {name}/{metric}")
            by_process = [
                [s["elapsed_ns"] / s["iterations"] for s in o["samples"]]
                for o in observations
            ]
            medians = [statistics.median(values) for values in by_process]
            batches = [value for values in by_process for value in values]
            result["metrics"][metric] = {
                "median_ns": statistics.median(medians),
                "process_medians_ns": medians,
                "min_process_median_ns": min(medians),
                "max_process_median_ns": max(medians),
                "batch_p10_ns": percentile(batches, 0.1),
                "batch_p90_ns": percentile(batches, 0.9),
                "dirty_nodes": entry["dirty_nodes"],
                "allocations_by_process": [o["allocations"] for o in observations],
            }
        summary[name] = result
    return summary


def ratios(numerator, denominator):
    """Per-process time ratios of one variant over another, for each metric."""
    result = {}
    for metric, entry in denominator["metrics"].items():
        values = [n / d for n, d in zip(numerator["metrics"][metric]["process_medians_ns"],
                                        entry["process_medians_ns"])]
        result[metric] = {"process_ratios": values, "median_ratio": statistics.median(values),
                          "min_ratio": min(values), "max_ratio": max(values)}
    return result


def compare(summary):
    """The matrix path over the Mk path on each backend, and the portable
    backend over NEON on each path."""
    transitions, backends = {}, {}
    for name, result in summary.items():
        case, path, backend = name.rsplit("/", 2)
        if path == "mk" and f"{case}/matrices/{backend}" in summary:
            transitions.setdefault(case, {})[backend] = ratios(summary[f"{case}/matrices/{backend}"], result)
        if backend == "neon" and f"{case}/{path}/portable" in summary:
            backends.setdefault(case, {})[path] = ratios(summary[f"{case}/{path}/portable"], result)
    return {"matrices_over_mk": transitions, "portable_over_neon": backends}


def load_average():
    try:
        return list(os.getloadavg())
    except OSError:
        return None


def hardware():
    result = {"architecture": platform.machine(), "system": platform.system()}
    if sys.platform == "darwin":
        result["os"] = command(["sw_vers"]).stdout.strip()
        for key in ["machdep.cpu.brand_string", "hw.memsize", "hw.physicalcpu", "hw.logicalcpu",
                    "hw.perflevel0.physicalcpu", "hw.perflevel1.physicalcpu"]:
            output = command(["sysctl", "-n", key], check=False)
            result[key] = output.stdout.strip() if output.returncode == 0 else None
        # Sandboxed macOS environments may deny sysctl while allowing this report.
        output = command(["system_profiler", "SPHardwareDataType", "-json"], check=False)
        if output.returncode == 0:
            report = json.loads(output.stdout)["SPHardwareDataType"][0]
            result["system_profiler"] = {
                key: report.get(key) for key in ["machine_name", "machine_model", "chip_type",
                                               "number_processors", "physical_memory"]
            }
    elif sys.platform.startswith("linux"):
        result["kernel"] = platform.release()
        result["cpu"] = command(["lscpu", "--json"], check=False).stdout.strip()
    return result


def redact_power_identifier(text):
    return re.sub(r"\s*\(id=\d+\)", "", text)


def power_state():
    if sys.platform != "darwin":
        return None
    return {option: redact_power_identifier(command(["pmset", "-g", option], check=False).stdout.strip())
            for option in ["batt", "therm"]}


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--samples", type=int, default=15)
    parser.add_argument("--sample-ms", type=int, default=20)
    parser.add_argument("--warmup-ms", type=int, default=100)
    args = parser.parse_args()
    if args.repetitions < 2 or args.samples < 3 or args.sample_ms < 1 or args.warmup_ms < 0:
        parser.error("use at least two repetitions, three samples, and a positive sample duration")
    status = command(["git", "status", "--porcelain", "--untracked-files=normal"]).stdout
    if status:
        raise SystemExit("benchmark from a clean source checkout; commit changes first")
    command([sys.executable, "tools/benchmarks/prepare.py", "--check"])
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    env = os.environ.copy()
    env.pop("CARGO_ENCODED_RUSTFLAGS", None)
    overrides = {
        "RUSTFLAGS": "-C target-cpu=native", "CARGO_INCREMENTAL": "0",
        "CARGO_PROFILE_BENCH_OPT_LEVEL": "3", "CARGO_PROFILE_BENCH_LTO": "fat",
        "CARGO_PROFILE_BENCH_CODEGEN_UNITS": "1", "CARGO_PROFILE_BENCH_DEBUG": "false",
        "CARGO_PROFILE_BENCH_DEBUG_ASSERTIONS": "false", "CARGO_PROFILE_BENCH_OVERFLOW_CHECKS": "false",
    }
    env.update(overrides)
    rustc = command(["rustc", "-Vv"], env=env).stdout.strip()
    target = next(line.removeprefix("host: ") for line in rustc.splitlines() if line.startswith("host: "))
    build = ["cargo", "bench", "--locked", "--bench", "likelihood", "--no-run", "--target", target, "--message-format=json"]
    print("Building the benchmark with target-cpu=native...", flush=True)
    built = command(build, check=False, env=env)
    (output / "build.log").write_text(built.stderr)
    built.check_returncode()
    artifacts = [json.loads(line) for line in built.stdout.splitlines() if line.startswith("{")]
    executables = [Path(a["executable"]) for a in artifacts
                   if a.get("reason") == "compiler-artifact" and a.get("executable")
                   and a["target"]["name"] == "likelihood"]
    if len(executables) != 1:
        raise RuntimeError("expected one likelihood benchmark executable")
    executable = executables[0]
    arguments = ["--samples", str(args.samples), "--sample-ms", str(args.sample_ms), "--warmup-ms", str(args.warmup_ms)]
    sources = command(["git", "ls-files", "src", "benches", "tools/benchmarks", "Cargo.toml", "Cargo.lock",
                       "fixtures/reference/20x50/matrix.nex", "fixtures/benchmarks",
                       "benchmarks/beast-2.7.8/2026-10-25/snapshot-20261025.json"]).stdout.splitlines()
    metadata = {
        "source_commit": command(["git", "rev-parse", "HEAD"]).stdout.strip(),
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "hardware": hardware(), "rustc": rustc,
        "cargo": command(["cargo", "-V"], env=env).stdout.strip(),
        "python": platform.python_version(), "build_command": build, "build_environment": overrides,
        "native_cpu": next((line.strip() for line in command(["rustc", "--print", "target-cpus"]).stdout.splitlines()
                            if line.strip().startswith("native ")), None),
        "executable_sha256": sha256(executable), "benchmark_arguments": arguments,
        "repetitions": args.repetitions, "samples_per_metric_per_process": args.samples,
        "source_sha256": {path: sha256(ROOT / path) for path in sources},
        "threads": 1, "processes_run_in_parallel": 1, "cpu_affinity": None,
        "background_load_controlled": False, "scaling": "Always",
        "model": "Mk on observed patterns; reference_20x50_mkv also conditions on variable characters; no priors",
        "variants": "each metric on every combination of transition path and backend back to back, in reverse order on alternate metrics and alternate processes",
        "power_before": power_state(), "load_average_before": load_average(), "runs": [],
    }
    runs = []
    for repetition in range(args.repetitions):
        path = output / f"run-{repetition + 1}.tsv"
        print(f"Starting process {repetition + 1}/{args.repetitions}...", flush=True)
        start = time.perf_counter()
        # Alternate processes also reverse the order of the variants.
        order = ["--reverse"] if repetition % 2 == 1 else []
        with path.open("w") as stream:
            subprocess.run([str(executable), *arguments, *order], cwd=ROOT, env=env, stdout=stream, check=True)
        elapsed = time.perf_counter() - start
        runs.append(read_results(path.read_text(), args.samples))
        metadata["runs"].append({"file": path.name, "sha256": sha256(path), "wall_seconds": elapsed,
                                 "arguments": [*arguments, *order]})
    metadata["power_after"] = power_state()
    metadata["load_average_after"] = load_average()
    metadata["finished_utc"] = datetime.now(timezone.utc).isoformat()
    write_json(output / "metadata.json", metadata)
    summary = summarize(runs)
    write_json(output / "summary.json", summary)
    write_json(output / "comparison.json", compare(summary))
    print(f"Saved {len(runs)} runs to {output}")


if __name__ == "__main__":
    main()
