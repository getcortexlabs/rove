#!/usr/bin/env python3
"""Run `rove sample` with each backend on the strict-clock configurations,
alternating the order, and check that both backends write the same samples."""

import argparse
import hashlib
import json
import statistics
import subprocess
import sys
import time
from pathlib import Path


def digest(lines):
    return hashlib.sha256("".join(lines).encode()).hexdigest()


def samples(directory):
    trace = [l for l in (directory / "trace.log").read_text().splitlines(True) if not l.startswith("#")]
    trees = [l for l in (directory / "trees.nexus").read_text().splitlines(True) if not l.startswith("[command")]
    return digest(trace), digest(trees)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rove", type=Path, required=True)
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seeds", nargs="+", type=int, required=True)
    parser.add_argument("--steps", type=int, default=10_000_000)
    args = parser.parse_args()
    sys.path.insert(0, str(args.repository / "tools/beast"))
    from strict_clock import configuration, rove_command
    args.output.mkdir(parents=True, exist_ok=False)
    matrix = args.repository / "fixtures/reference/20x50/matrix.nex"
    results = {}
    index = 0
    for name in ["mkv", "mk"]:
        config = configuration(args.repository / f"benchmarks/strict-clock/2026-10-28/beast-{name}.xml")
        config["chain_length"] = args.steps
        runs = []
        for seed in args.seeds:
            order = ["portable", "neon"] if index % 2 == 0 else ["neon", "portable"]
            index += 1
            entry = {"seed": seed, "order": order}
            for backend in order:
                directory = args.output / name / f"seed-{seed}-{backend}"
                directory.parent.mkdir(parents=True, exist_ok=True)
                command = rove_command(args.rove, matrix, directory, seed, config) + ["--backend", backend]
                start = time.perf_counter()
                subprocess.run(command, stderr=subprocess.DEVNULL, check=True)
                wall = time.perf_counter() - start
                record = json.loads((directory / "run.json").read_text())
                assert record["settings"]["backend"] == backend
                entry[backend] = {"wall_seconds": wall, "chain_seconds": record["wall_seconds"],
                                  "samples_sha256": samples(directory)}
            entry["identical"] = entry["portable"]["samples_sha256"] == entry["neon"]["samples_sha256"]
            entry["ratio"] = entry["portable"]["chain_seconds"] / entry["neon"]["chain_seconds"]
            runs.append(entry)
            print(f"{name} seed {seed}: portable {entry['portable']['chain_seconds']:.3f} s, "
                  f"neon {entry['neon']['chain_seconds']:.3f} s, ratio {entry['ratio']:.3f}, "
                  f"identical {entry['identical']}", flush=True)
        results[name] = {"runs": runs, "median_ratio": statistics.median(r["ratio"] for r in runs),
                         "all_identical": all(r["identical"] for r in runs)}
    (args.output / "sampler.json").write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    main()
