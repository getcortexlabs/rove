#!/usr/bin/env python3
"""Compare saved scaling implementations with the likelihood harness and sampler."""

import argparse
import hashlib
import json
import os
import random
import statistics
import subprocess
import time
from pathlib import Path

from run import hardware, power_state, read_results, sha256, summarize

ROOT = Path(__file__).resolve().parents[2]
VARIANTS = ["before", "power-sum", "subtree"]


def write(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def scientific_digest(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for line in stream:
            if not line.startswith((b"#", b"[command", b"[input sha256", b"[executable sha256", b"[rove")):
                digest.update(line)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binaries", type=Path, required=True,
                        help="directory containing rove-{before,power-sum,subtree} and likelihood-{...}")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--large-matrix", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--phase", choices=["kernel", "sampler"], required=True)
    args = parser.parse_args()
    if args.repetitions < 2:
        parser.error("use at least two process repetitions")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binaries = args.binaries.resolve()
    metadata = {"hardware": hardware(), "power_before": power_state(),
                "load_before": os.getloadavg(), "source_commit": subprocess.check_output(
                    ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
                "build": "cargo build --release / cargo bench --bench likelihood --no-run; default target CPU; fat LTO, 1 codegen unit",
                "source_sha256": {str(p.relative_to(ROOT)): sha256(p) for p in sorted((ROOT / "src").rglob("*.rs"))},
                "executable_sha256": {f"{kind}-{v}": sha256(binaries / f"{kind}-{v}")
                                      for v in VARIANTS for kind in ["rove", "likelihood"]},
                "repetitions": args.repetitions, "background_load_controlled": False,
                "processes_in_parallel": 1, "rows": []}
    if args.phase == "kernel":
        runs = {v: [] for v in VARIANTS}
        arguments = ["--samples", "5", "--sample-ms", "5", "--warmup-ms", "10"]
        metadata["arguments"] = arguments
        for repeat in range(args.repetitions):
            order = VARIANTS[repeat % 3:] + VARIANTS[:repeat % 3]
            for variant in order:
                path = output / f"{variant}-{repeat + 1}.tsv"
                options = arguments + (["--reverse"] if repeat % 2 else [])
                start = time.perf_counter()
                with path.open("w") as stream:
                    subprocess.run([str(binaries / f"likelihood-{variant}"), *options],
                                   stdout=stream, cwd=ROOT, check=True)
                runs[variant].append(read_results(path.read_text(), 5))
                metadata["rows"].append({"variant": variant, "repetition": repeat,
                                         "wall_seconds": time.perf_counter() - start,
                                         "file": path.name, "sha256": sha256(path), "arguments": options})
                write(output / "metadata.json", metadata)
                print(f"Finished {variant}, kernel repetition {repeat + 1}", flush=True)
        write(output / "summary.json", {v: summarize(rows) for v, rows in runs.items()})
    else:
        cases = {
            "mk": (ROOT / "fixtures/validation/reference-mk/matrix.nex", "none", 2_000_000, 1000),
            "mkv": (ROOT / "fixtures/reference/20x50/matrix.nex", "variable", 2_000_000, 1000),
            "large": (args.large_matrix.resolve(), "variable", 20_000, 100),
        }
        metadata["cases"] = {name: {"matrix_sha256": sha256(path), "conditioning": conditioning,
                                   "steps": steps, "sample_every": interval,
                                   "model": "reference", "partitioning": "observed",
                                   "ingroup_constraint": name != "large"}
                             for name, (path, conditioning, steps, interval) in cases.items()}
        jobs = [(v, b, c, r) for v in VARIANTS for b in ["portable", "neon"]
                for c in cases for r in range(args.repetitions)]
        random.Random(18).shuffle(jobs)
        for variant, backend, name, repeat in jobs:
            matrix, conditioning, steps, interval = cases[name]
            directory = output / f"{name}-{variant}-{backend}-{repeat + 1}"
            seed = 2026103000 + repeat
            command = [str(binaries / f"rove-{variant}"), "sample", str(matrix), "--output", str(directory),
                       "--seed", str(seed), "--steps", str(steps), "--sample-every", str(interval),
                       "--model", "reference", "--conditioning", conditioning, "--backend", backend]
            if name != "large":
                command += ["--monophyly", ",".join(f"T{i}" for i in range(1, 20))]
            load = os.getloadavg()
            start = time.perf_counter()
            subprocess.run(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
            elapsed = time.perf_counter() - start
            record = json.loads((directory / "run.json").read_text())
            row = {"variant": variant, "backend": backend, "case": name, "seed": seed,
                   "wall_seconds": elapsed, "sampling_seconds": record.get("sampling_wall_seconds", record["wall_seconds"]),
                   "load_average": load, "trace_sha256": scientific_digest(directory / "trace.log"),
                   "trees_sha256": scientific_digest(directory / "trees.nexus"), "directory": directory.name}
            metadata["rows"].append(row)
            write(output / "metadata.json", metadata)
            print(f"{name}, {variant}, {backend}, seed {seed}: {elapsed:.3f}s", flush=True)
        # Diagnostics run after all timings, using the same final executable.
        for row in metadata["rows"]:
            directory = output / row["directory"]
            subprocess.run([str(binaries / "rove-subtree"), "diagnose", str(directory / "trace.log"),
                            "--burnin-percent", "10", "--output", str(directory / "diagnostics")],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
            columns = json.loads((directory / "diagnostics/diagnostics.json").read_text())["columns"]
            row["statistics"] = [{k: c[k] for k in ["name", "ess", "mean", "ess_reason", "lag_limit_reached"]}
                                 for c in columns]
            row["minimum_ess"] = min(c["ess"] for c in columns if c["ess"] is not None)
            row["minimum_ess_per_second"] = row["minimum_ess"] / row["wall_seconds"]
            subprocess.run([str(binaries / "rove-subtree"), "summarize", str(directory / "trees.nexus"),
                            "--burnin-percent", "10", "--heights", "keep", "--output", str(directory / "summary")],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
            summary = json.loads((directory / "summary/summary.json").read_text())
            row["tree_summary"] = {k: summary[k] for k in
                                   ["distinct_clades", "burnin", "convergence_assessed"]}
        lookup = {(r["case"], r["variant"], r["backend"], r["seed"]): r for r in metadata["rows"]}
        for (case, variant, backend, seed), row in lookup.items():
            portable = lookup[case, variant, "portable", seed]
            for field in ["trace_sha256", "trees_sha256"]:
                assert row[field] == portable[field], (case, variant, seed, field)
                if variant == "subtree":
                    assert row[field] == lookup[case, "power-sum", backend, seed][field], (case, seed, field)
        metadata["backend_identity_pairs"] = len(lookup) // 2
        metadata["cumulative_identity_pairs"] = len(lookup) // 3
        metadata["medians"] = [{"case": name, "variant": v, "backend": b,
                                 "wall_seconds": statistics.median(r["wall_seconds"] for r in metadata["rows"]
                                     if (r["case"], r["variant"], r["backend"]) == (name, v, b))}
                                for name in cases for v in VARIANTS for b in ["portable", "neon"]]
    metadata.update(power_after=power_state(), load_after=os.getloadavg())
    write(output / "metadata.json", metadata)


if __name__ == "__main__":
    main()
