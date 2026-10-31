#!/usr/bin/env python3
"""Run the frozen gamma-frequency comparison; keep raw output local."""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from fbd_estimation import ROOT, SETTINGS, metadata, sha

sys.path.insert(0, str(ROOT / "tools/beast"))
from inference import rove_command

PROTOCOL = ROOT / "fixtures/validation/GAMMA-MIXING.md"
CASES = ["dated-fixed", "dated-estimated", "reference-mk", "reference-mkv",
         "control", "homoplasy", "missing", "reduced"]


def write(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def run(binary, output):
    output.mkdir(parents=True, exist_ok=False)
    record = metadata(binary)
    record.update(protocol="gamma-frequency-v1", protocol_sha256=sha(PROTOCOL),
                  runner_sha256=sha(Path(__file__)),
                  started_at=datetime.now(timezone.utc).isoformat(),
                  load_before=list(os.getloadavg()), runs=[])
    manifest = json.loads((ROOT / "fixtures/validation/manifest.json").read_text())
    for index, case in enumerate(CASES):
        dated = case.startswith("dated-")
        steps = 40_000_000 if case == "reference-mkv" else 10_000_000
        seeds = [42101 + index * 20, 42111 + index * 20] if dated else [42201 + (index - 2) * 10]
        for repetition, seed in enumerate(seeds):
            weights = [0.1, 3.0] if (index + repetition) % 2 == 0 else [3.0, 0.1]
            for weight in weights:
                directory = output / f"{case}-{seed}-{weight:g}"
                fixture = ROOT / "fixtures/validation" / ("dating" if dated else case)
                if dated:
                    config = ROOT / SETTINGS[case.removeprefix("dated-")]
                    command = [str(binary), "sample", str(fixture / "matrix.nex"),
                               "--model", "reference", "--dating", str(config),
                               "--output", str(directory), "--seed", str(seed),
                               "--steps", str(steps), "--sample-every", "1000"]
                else:
                    for name, expected in manifest[case]["sha256"].items():
                        if sha(fixture / name) != expected:
                            raise ValueError(f"Fixture changed: {case}/{name}")
                    command = rove_command(binary, fixture, directory, seed, steps,
                                           1000, manifest[case]["conditioning"])
                command += ["--chains", "4", "--weight", f"gamma_shape_scale={weight:g}"]
                if sha(binary) != record["binary_sha256"] or sha(PROTOCOL) != record["protocol_sha256"]:
                    raise ValueError("Executable or protocol changed during timing")
                with directory.with_suffix(".console.log").open("w") as console:
                    began = time.perf_counter()
                    subprocess.run(command, cwd=ROOT, stdout=console, stderr=console, check=True)
                    elapsed = time.perf_counter() - began
                comparison = json.loads((directory / "comparison.json").read_text())
                chains = []
                for path in sorted(directory.glob("chain-*/run.json")):
                    data = json.loads(path.read_text())
                    if data["status"] != "completed" or data["completed_steps"] != steps:
                        raise ValueError(f"Incomplete chain: {path}")
                    chains.append({"directory": str(path.parent.relative_to(output)),
                                   "record": data, "sha256": {name: sha(path.parent / name)
                                   for name in ["trace.log", "trees.nexus", "run.json"]}})
                if [c["record"]["settings"]["seed"] for c in chains] != list(range(seed, seed + 4)):
                    raise ValueError("Unexpected chain seeds")
                observation = {"case": case, "weight": weight, "seed": seed,
                               "steps": steps, "sample_every": 1000,
                               "directory": directory.name, "command": command,
                               "group_wall_seconds": elapsed, "chains": chains,
                               "comparison": comparison}
                record["runs"].append(observation)
                write(output / "runs.json", record)
                gamma = next(s for s in comparison["statistics"] if s["name"] == "gamma_shape")
                print(f"{directory.name}: {elapsed:.2f}s, minimum gamma ESS {min(gamma['ess']):.1f}", flush=True)
    record.update(finished_at=datetime.now(timezone.utc).isoformat(), load_after=list(os.getloadavg()))
    write(output / "runs.json", record)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.output.resolve())
