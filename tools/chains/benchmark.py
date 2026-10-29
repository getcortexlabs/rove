#!/usr/bin/env python3
"""Measure independent-chain throughput under the macOS scheduler."""
import argparse
import hashlib
import json
import os
import random
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/benchmarks"))
from run import hardware


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, default=ROOT / "target/release/rove")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--steps", type=int, default=1_000_000)
    parser.add_argument("--repeats", type=int, default=3)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    args.engine = args.engine.resolve()
    matrix = ROOT / "fixtures/validation/reference-mk/matrix.nex"
    report = {"hardware": hardware(), "engine_sha256": hashlib.sha256(args.engine.read_bytes()).hexdigest(),
        "matrix_sha256": hashlib.sha256(matrix.read_bytes()).hexdigest(),
        "rustc": subprocess.check_output(["rustc", "--version", "--verbose"], text=True),
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "rust_sources_sha256": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((ROOT / "src").rglob("*.rs"))},
        "steps_per_chain": args.steps, "sample_every": 1000, "repeats": args.repeats,
        "scheduling": "OS-managed threads at default QoS; no CPU affinity or attribution to a specific core class",
        "chain_counts": [1, 2, 4, 8, 12, 16], "rows": []}
    jobs = [(count, repeat) for repeat in range(args.repeats) for count in report["chain_counts"]]
    random.Random(23).shuffle(jobs)
    def command(count, steps, output):
        return [str(args.engine), "sample", str(matrix), "--output", str(output), "--seed", "2026102900",
            "--steps", str(steps), "--sample-every", "1000", "--chains", str(count),
            "--model", "reference", "--conditioning", "none",
            "--monophyly", ",".join(f"T{i}" for i in range(1,20))]
    subprocess.run(command(1, 500_000, args.output / "warmup"), check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for count, repeat in jobs:
        directory = args.output / f"chains-{count:02}-repeat-{repeat+1}"
        cmd = command(count, args.steps, directory)
        with (args.output / f"{directory.name}.log").open("w") as log:
            load_before = list(os.getloadavg())
            began = time.perf_counter()
            process = subprocess.Popen(cmd, stdout=log, stderr=log)
            _, status, usage = os.wait4(process.pid, 0)
            process.returncode = os.waitstatus_to_exitcode(status)
            elapsed = time.perf_counter() - began
        if process.returncode != 0:
            raise RuntimeError(f"run failed: {directory}")
        record = json.loads((directory / "run.json").read_text())
        sampling = record.get("sampling_wall_seconds", record["wall_seconds"])
        row = {"load_average_before": load_before, "load_average_after": list(os.getloadavg()), "chains": count, "repeat": repeat+1, "command": cmd,
            "wall_seconds": elapsed, "sampling_wall_seconds": sampling,
            "user_seconds": usage.ru_utime, "system_seconds": usage.ru_stime,
            "peak_rss_bytes": usage.ru_maxrss if sys.platform == "darwin" else usage.ru_maxrss * 1024,
            "aggregate_steps_per_second": count * args.steps / elapsed,
            "sampling_steps_per_second": count * args.steps / sampling}
        report["rows"].append(row)
        (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"{count:2} chains, repetition {repeat+1}: {elapsed:.3f}s total, {sampling:.3f}s sampling", flush=True)
    base = statistics.median(r["aggregate_steps_per_second"] for r in report["rows"] if r["chains"] == 1)
    report["summary"] = [{"chains": count,
        "median_wall_seconds": statistics.median(r["wall_seconds"] for r in report["rows"] if r["chains"] == count),
        "median_sampling_seconds": statistics.median(r["sampling_wall_seconds"] for r in report["rows"] if r["chains"] == count),
        "aggregate_speedup": statistics.median(r["aggregate_steps_per_second"] for r in report["rows"] if r["chains"] == count) / base}
        for count in report["chain_counts"]]
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
