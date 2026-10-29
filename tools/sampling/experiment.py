#!/usr/bin/env python3
"""Run sequential proposal pilots or frozen four-chain comparisons."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/beast"))
from inference import rove_command, verify_rove
from run_reference import digest, write_json

SCHEMES = {
    "classical": {},
    "sigma-frequency": {"node_height": 21, "clock_sigma_scale": 12},
    "time-rates": {
        "node_height": 18, "node_time_rates": 12,
        "root_scale": 1, "root_time_rates": 2,
    },
    "sigma-rates": {"clock_sigma_scale": 1, "clock_sigma_rates": 2},
    "combined": {
        "node_height": 18, "node_time_rates": 12,
        "root_scale": 1, "root_time_rates": 2,
        "clock_sigma_scale": 1, "clock_sigma_rates": 2,
    },
}
CASES = ["reference-mk", "reference-mkv", "control", "homoplasy", "missing", "reduced"]
PRIMARY = ["posterior", "clock_sigma"]


def capture(command):
    return subprocess.check_output(command, cwd=ROOT, text=True).strip()


def power():
    observations = {}
    for option in ["batt", "therm"]:
        result = subprocess.run(["pmset", "-g", option], text=True, capture_output=True)
        observations[option] = (result.stdout + result.stderr).strip()
    return observations


def scientific_digest(path):
    sha = hashlib.sha256()
    with path.open("rb") as source:
        for line in source:
            if not line.startswith((b"# ", b"[command ", b"[input sha256 ", b"[executable sha256 ", b"[rove ")):
                sha.update(line)
    return sha.hexdigest()


def pilot_summary(output, metadata):
    cases = {}
    for case in metadata["cases"]:
        cases[case] = {}
        for scheme in metadata["schemes"]:
            runs = [r for r in metadata["runs"] if r["case"] == case and r["scheme"] == scheme]
            columns = {}
            for run in runs:
                directory = output / run["directory"]
                report = json.loads(subprocess.check_output([
                    metadata["binary"], "diagnose", str(directory / "trace.log"),
                    "--burnin-percent", "10",
                ]))
                write_json(directory / "diagnostics.json", report)
                for column in report["columns"]:
                    ess = column["ess"]
                    columns.setdefault(column["name"], []).append({
                        "seed": run["seed"], "mean": column["mean"], "sd": column["sd"],
                        "ess": ess, "ess_per_second": ess / run["wall_seconds"] if ess else None,
                        "lag_limit_reached": column["lag_limit_reached"],
                    })
            cases[case][scheme] = columns
    scores = {}
    for scheme in metadata["schemes"]:
        if scheme == "classical":
            continue
        ratios = []
        for case, schemes in cases.items():
            primary = []
            for parameter in PRIMARY:
                values = [r["ess_per_second"] for r in schemes[scheme][parameter]]
                baseline = [r["ess_per_second"] for r in schemes["classical"][parameter]]
                primary.append(statistics.median(values) / statistics.median(baseline)
                               if all(values) and all(baseline) else 0)
            ratios.append(min(primary))
        scores[scheme] = math.exp(statistics.fmean(map(math.log, ratios))) if all(ratios) else 0
    selected = max(scores, key=scores.get)
    result = {"phase": "exploratory_pilot", "cases": cases, "scores": scores,
              "selected": selected, "selected_weights": SCHEMES[selected],
              "protocol_sha256": metadata["protocol_sha256"],
              "caveat": "Pilot selection only; no posterior-concordance or final efficiency claim."}
    write_json(output / "pilot.json", result)
    print(json.dumps({"scores": scores, "selected": selected}), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=["pilot", "final"])
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/rove")
    parser.add_argument("--hardware", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--pilot", type=Path)
    args = parser.parse_args()
    status = capture(["git", "status", "--porcelain", "--untracked-files=normal"])
    if args.phase == "final" and (status or args.pilot is None):
        parser.error("Final runs require a clean checkout and --pilot selection record")
    selected = None
    if args.phase == "final":
        pilot = json.loads(args.pilot.read_text())
        selected = pilot["selected"]
        if pilot["selected_weights"] != SCHEMES[selected]:
            parser.error("The selected weights changed since the pilot")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    manifest = json.loads((ROOT / "fixtures/validation/manifest.json").read_text())
    cases = ["reference-mk", "reference-mkv", "missing"] if args.phase == "pilot" else CASES
    schemes = list(SCHEMES) if args.phase == "pilot" else ["classical", selected]
    protocol = ROOT / "fixtures/validation/SAMPLING-EFFICIENCY.md"
    metadata = {
        "phase": args.phase, "cases": cases, "schemes": {s: SCHEMES[s] for s in schemes},
        "selected": selected, "binary": str(binary), "binary_sha256": digest(binary),
        "source_commit": capture(["git", "rev-parse", "HEAD"]), "source_dirty": bool(status),
        "source_diff_sha256": hashlib.sha256(capture(["git", "diff"]).encode()).hexdigest(),
        "source_files": {str(p.relative_to(ROOT)): digest(p) for p in
                         [ROOT / "Cargo.toml", ROOT / "Cargo.lock", *sorted((ROOT / "src").rglob("*.rs"))]},
        "protocol_sha256": digest(protocol), "hardware": json.loads(args.hardware.read_text()),
        "rustc": capture(["rustc", "-Vv"]), "platform": platform.platform(),
        "started_at": datetime.now(timezone.utc).isoformat(),
        "build_environment": {key: value for key, value in os.environ.items()
                              if key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET")
                              or key.startswith("CARGO_PROFILE_RELEASE_")},
        "build": "cargo build --locked --release; fat LTO; one codegen unit; no target-cpu override",
        "power_before": power(), "load_before": list(os.getloadavg()), "runs": [],
        "burnin_percent": 10, "concurrent_chains": 1,
        "timing": "Process start to exit, including setup, adaptation, sampling and I/O; diagnostics excluded",
        "pilot_sha256": digest(args.pilot) if args.pilot else None,
    }
    write_json(output / "metadata.json", metadata)
    for case in cases:
        fixture = ROOT / "fixtures/validation" / case
        for name, expected in manifest[case]["sha256"].items():
            if digest(fixture / name) != expected:
                raise ValueError(f"Fixture changed: {case}/{name}")
        if args.phase == "pilot":
            steps, interval, seeds = 2_000_000, 200, [27011, 27012]
        else:
            steps, interval, seeds = (40_000_000, 1000, list(range(20261210, 20261214))) if case == "reference-mkv" else (10_000_000, 1000, list(range(20261110, 20261114)))
        for index, seed in enumerate(seeds):
            order = schemes[index % len(schemes):] + schemes[:index % len(schemes)]
            for scheme in order:
                directory = output / case / scheme / str(seed)
                directory.parent.mkdir(parents=True, exist_ok=True)
                console = directory.with_suffix(".console.log")
                command = rove_command(binary, fixture, directory, seed, steps, interval,
                                       manifest[case]["conditioning"])
                for move, weight in SCHEMES[scheme].items():
                    command += ["--weight", f"{move}={weight}"]
                if digest(binary) != metadata["binary_sha256"]:
                    raise ValueError("The experiment binary changed during the run")
                began = time.perf_counter()
                with console.open("w") as stream:
                    subprocess.run(command, cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT, check=True)
                seconds = time.perf_counter() - began
                record = verify_rove(directory, manifest[case], steps, interval)
                observation = {
                    "case": case, "scheme": scheme, "seed": seed, "steps": steps,
                    "sample_every": interval, "directory": str(directory.relative_to(output)),
                    "command": command, "wall_seconds": seconds, "moves": record["moves"],
                    "settings": record["settings"], "input_sha256": digest(fixture / "matrix.nex"),
                    "sha256": {name: digest(directory / name) for name in ["trace.log", "trees.nexus", "run.json"]},
                    "scientific_sha256": {name: scientific_digest(directory / name) for name in ["trace.log", "trees.nexus"]},
                }
                metadata["runs"].append(observation)
                write_json(output / "metadata.json", metadata)
                print(f"{case} {scheme} seed {seed}: {seconds:.3f}s", flush=True)
    metadata.update(power_after=power(), load_after=list(os.getloadavg()),
                    finished_at=datetime.now(timezone.utc).isoformat())
    write_json(output / "metadata.json", metadata)
    if args.phase == "pilot":
        pilot_summary(output, metadata)


if __name__ == "__main__":
    main()
