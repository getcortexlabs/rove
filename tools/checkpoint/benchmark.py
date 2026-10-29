#!/usr/bin/env python3
"""Compare checkpoint overhead with a previously built Rove executable."""
import argparse
import hashlib
import json
import os
import random
import statistics
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path, default=ROOT / "target/release/rove")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--steps", type=int, default=2_000_000)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    engines = {"before": args.before.resolve(), "after": args.after.resolve()}
    matrix = ROOT / "fixtures/validation/reference-mk/matrix.nex"
    fingerprint = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    report = {"engines": {k: fingerprint(p) for k, p in engines.items()},
              "matrix_sha256": fingerprint(matrix), "steps": args.steps,
              "sample_every": 1000, "checkpoint_seconds": 30, "rows": [],
              "rust_sources_sha256": {str(p.relative_to(ROOT)): fingerprint(p) for p in sorted((ROOT / "src").rglob("*.rs"))},
              "rustc": subprocess.check_output(["rustc", "--version", "--verbose"], text=True)}
    jobs = [(engine, chains, repeat) for engine in engines for chains in [1, 4] for repeat in range(3)]
    random.Random(22).shuffle(jobs)
    for engine, chains, repeat in jobs:
        directory = args.output / f"{engine}-{chains}-{repeat}"
        command = [str(engines[engine]), "sample", str(matrix), "--output", str(directory),
                   "--seed", "2026102900", "--steps", str(args.steps), "--sample-every", "1000",
                   "--chains", str(chains), "--model", "reference", "--conditioning", "none",
                   "--monophyly", ",".join(f"T{i}" for i in range(1, 20))]
        load = os.getloadavg()
        start = time.perf_counter()
        subprocess.run(command, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        elapsed = time.perf_counter() - start
        metadata = json.loads((directory / "run.json").read_text())
        row = {"engine": engine, "chains": chains, "repeat": repeat, "wall_seconds": elapsed,
               "sampling_seconds": metadata.get("sampling_wall_seconds", metadata["wall_seconds"]),
               "load_average": load}
        report["rows"].append(row)
        (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"{engine}, {chains} chains: {elapsed:.3f}s", flush=True)
    report["summary"] = []
    for chains in [1, 4]:
        medians = {engine: statistics.median(r["wall_seconds"] for r in report["rows"]
                                            if r["engine"] == engine and r["chains"] == chains)
                   for engine in engines}
        report["summary"].append({"chains": chains, "median_wall_seconds": medians,
                                  "overhead_percent": 100 * (medians["after"] / medians["before"] - 1)})
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report["summary"], indent=2))


if __name__ == "__main__":
    main()
