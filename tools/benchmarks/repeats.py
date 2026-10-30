#!/usr/bin/env python3
"""Measure dense and experimental site-repeat builds in sequential processes."""

import argparse
import json
import os
import random
import statistics
import subprocess
import sys
import time
from pathlib import Path

from run import hardware, power_state, sha256
from scaling import scientific_digest, write

ROOT = Path(__file__).resolve().parents[2]
VARIANTS = ["dense", "gated"]


def measure(command):
    start = time.perf_counter()
    with subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE) as child:
        # The sampler's progress goes to stdout. Stderr is small on success;
        # drain it before waiting so an error cannot fill the pipe.
        error = child.stderr.read().decode(errors="replace")
        _, status, usage = os.wait4(child.pid, 0)
        child.returncode = os.waitstatus_to_exitcode(status)
        if child.returncode:
            raise RuntimeError(f"sampler exited {child.returncode}: {error}")
    rss = usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024)
    return time.perf_counter() - start, rss


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binaries", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--large-matrix", type=Path)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--phase", choices=["kernel", "sampler"], required=True)
    args = parser.parse_args()
    if args.repetitions < 2:
        parser.error("use at least two fresh processes")
    if args.phase == "sampler" and args.large_matrix is None:
        parser.error("--large-matrix is required for sampler measurements")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binaries = args.binaries.resolve()
    sources = [ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__)]
    sources += sorted((ROOT / "src").rglob("*.rs")) + sorted((ROOT / "benches").rglob("*.rs"))
    sources += sorted((ROOT / "fixtures/benchmarks").rglob("*.nex"))
    metadata = {
        "hardware": hardware(), "power_before": power_state(), "load_before": os.getloadavg(),
        "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
        "build": "release; default target CPU; fat LTO, 1 codegen unit; gated adds --features experimental-repeats",
        "source_sha256": {str(p.relative_to(ROOT)): sha256(p) for p in sources},
        "executable_sha256": {f"{kind}-{v}": sha256(binaries / f"{kind}-{v}")
                              for v in VARIANTS for kind in ["rove", "bench"]},
        "repetitions": args.repetitions, "background_load_controlled": False,
        "processes_in_parallel": 1, "rows": [],
    }
    if args.phase == "kernel":
        groups = {}
        for repeat in range(args.repetitions):
            order = VARIANTS if repeat % 2 == 0 else list(reversed(VARIANTS))
            for variant in order:
                path = output / f"{variant}-{repeat + 1}.json"
                options = ["--reverse"] if repeat % 2 else []
                with path.open("w") as stream:
                    subprocess.run([str(binaries / f"bench-{variant}"), *options],
                                   stdout=stream, cwd=ROOT, check=True)
                rows = json.loads(path.read_text())["rows"]
                metadata["rows"].append({"variant": variant, "repetition": repeat + 1,
                                         "file": path.name, "sha256": sha256(path), "arguments": options})
                for row in rows:
                    key = (row["case"], row["backend"], row["metric"], variant)
                    groups.setdefault(key, []).append(statistics.median(row["elapsed_ns"]) / row["iterations"])
                    if row["metric"] not in ["core_setup", "setup_first"]:
                        assert row["allocation_calls"] == row["reallocation_calls"] == 0, key
                write(output / "metadata.json", metadata)
                print(f"Finished {variant}, repetition {repeat + 1}", flush=True)
        summary = []
        for case, backend, metric in sorted({k[:3] for k in groups}):
            medians = {v: statistics.median(groups[case, backend, metric, v]) for v in VARIANTS}
            summary.append({"case": case, "backend": backend, "metric": metric,
                            "process_medians_ns": {v: groups[case, backend, metric, v] for v in VARIANTS},
                            "median_ns": medians, "speedup": medians["dense"] / medians["gated"]})
        write(output / "summary.json", summary)
    else:
        cases = {
            "mk": (ROOT / "fixtures/validation/reference-mk/matrix.nex", "none", 2_000_000, 1000),
            "mkv": (ROOT / "fixtures/reference/20x50/matrix.nex", "variable", 2_000_000, 1000),
            "large": (args.large_matrix.resolve(), "variable", 20_000, 100),
        }
        metadata["cases"] = {name: {"matrix_sha256": sha256(path), "conditioning": conditioning,
                                   "steps": steps, "sample_every": interval, "model": "reference",
                                   "partitioning": "observed", "ingroup_constraint": name != "large"}
                             for name, (path, conditioning, steps, interval) in cases.items()}
        jobs = [(v, b, c, r) for v in VARIANTS for b in ["portable", "neon"]
                for c in cases for r in range(args.repetitions)]
        random.Random(17).shuffle(jobs)
        for variant, backend, name, repeat in jobs:
            matrix, conditioning, steps, interval = cases[name]
            directory = output / f"{name}-{variant}-{backend}-{repeat + 1}"
            seed = 2026103017 + repeat
            command = [str(binaries / f"rove-{variant}"), "sample", str(matrix), "--output", str(directory),
                       "--seed", str(seed), "--steps", str(steps), "--sample-every", str(interval),
                       "--model", "reference", "--conditioning", conditioning, "--backend", backend]
            if name != "large":
                command += ["--monophyly", ",".join(f"T{i}" for i in range(1, 20))]
            load = os.getloadavg()
            elapsed, rss = measure(command)
            record = json.loads((directory / "run.json").read_text())
            row = {"variant": variant, "backend": backend, "case": name, "seed": seed,
                   "wall_seconds": elapsed, "sampling_seconds": record.get("sampling_wall_seconds", record["wall_seconds"]),
                   "peak_rss_bytes": rss, "load_average": load,
                   "trace_sha256": scientific_digest(directory / "trace.log"),
                   "trees_sha256": scientific_digest(directory / "trees.nexus"), "directory": directory.name}
            metadata["rows"].append(row)
            write(output / "metadata.json", metadata)
            print(f"{name}, {variant}, {backend}, seed {seed}: {elapsed:.3f}s", flush=True)
        # Run diagnostics after all timings. Keep private matrices and trees local.
        for row in metadata["rows"]:
            directory = output / row["directory"]
            subprocess.run([str(binaries / "rove-dense"), "diagnose", str(directory / "trace.log"),
                            "--burnin-percent", "10", "--output", str(directory / "diagnostics")],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
            columns = json.loads((directory / "diagnostics/diagnostics.json").read_text())["columns"]
            row["statistics"] = [{k: c[k] for k in ["name", "ess", "mean", "ess_reason", "lag_limit_reached"]}
                                 for c in columns]
            row["minimum_ess"] = min(c["ess"] for c in columns if c["ess"] is not None)
            row["minimum_ess_per_second"] = row["minimum_ess"] / row["wall_seconds"]
            subprocess.run([str(binaries / "rove-dense"), "summarize", str(directory / "trees.nexus"),
                            "--burnin-percent", "10", "--heights", "keep", "--output", str(directory / "summary")],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
            summary = json.loads((directory / "summary/summary.json").read_text())
            row["tree_summary"] = {k: summary[k] for k in ["distinct_clades", "burnin", "convergence_assessed"]}
        lookup = {(r["case"], r["variant"], r["backend"], r["seed"]): r for r in metadata["rows"]}
        for (case, variant, backend, seed), row in lookup.items():
            for field in ["trace_sha256", "trees_sha256"]:
                assert row[field] == lookup[case, "dense", "portable", seed][field], (case, variant, backend, seed, field)
        metadata["identical_output_groups"] = len(lookup) // 4
        metadata["medians"] = [{"case": name, "variant": v, "backend": b,
                                 "wall_seconds": statistics.median(r["wall_seconds"] for r in metadata["rows"]
                                     if (r["case"], r["variant"], r["backend"]) == (name, v, b))}
                                for name in cases for v in VARIANTS for b in ["portable", "neon"]]
    metadata.update(power_after=power_state(), load_after=os.getloadavg())
    write(output / "metadata.json", metadata)


if __name__ == "__main__":
    main()
