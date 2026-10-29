#!/usr/bin/env python3
"""Run the recorded full-model cases in sequential, alternating engine pairs."""

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from pathlib import Path

from prepare_inference import ROOT
from run_reference import EXPECTED_PACKAGES, digest, verify_run, write_json

sys.path.insert(0, str(ROOT))
from tools.benchmarks.run import hardware, power_state


def capture(command):
    return subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=True).stdout.strip()


def execute(command, directory, console):
    start = time.perf_counter()
    with console.open("w") as stream:
        subprocess.run(command, cwd=directory, stdout=stream, stderr=subprocess.STDOUT, check=True)
    return time.perf_counter() - start


def rove_command(binary, case, output, seed, steps, interval, conditioning):
    return [str(binary), "sample", str(case / "matrix.nex"), "--model", "reference",
            "--output", str(output), "--seed", str(seed), "--steps", str(steps),
            "--sample-every", str(interval), "--conditioning", conditioning,
            "--monophyly", ",".join(f"T{i}" for i in range(1, 20)), "--print-every", "100000"]


def verify_rove(directory, manifest, steps, interval):
    record = json.loads((directory / "run.json").read_text())
    settings, model = record["settings"], record["model"]
    expected = "variable characters (Mkv)" if manifest["conditioning"] == "variable" else "none (Mk)"
    if model["preset"] != "reference" or model["conditioning"] != expected:
        raise ValueError("Rove model differs from the case")
    if settings["steps"] != steps or settings["sample_every"] != interval:
        raise ValueError("Rove run length differs")
    if model["monophyly"] != [[f"T{i}" for i in range(1, 20)]] or model["sample_from_prior"]:
        raise ValueError("Rove constraints or target differ")
    for name, expected in {"initial_gamma_shape": 1.0, "initial_clock_sigma": 0.2,
                           "initial_branch_rate": 0.5, "initial_birth_difference_rate": 1.0,
                           "initial_relative_death_rate": 0.5, "tuning": True}.items():
        if settings[name] != expected:
            raise ValueError(f"Unexpected Rove setting: {name}")
    rows = [line for line in (directory / "trace.log").read_text().splitlines() if line and not line.startswith("#")]
    if [int(line.split()[0]) for line in rows[1:]] != list(range(0, steps + 1, interval)):
        raise ValueError("Incomplete Rove trace")
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", type=Path, default=ROOT / "fixtures/validation")
    parser.add_argument("--case", required=True)
    parser.add_argument("--packages", type=Path, required=True)
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--java", default="java")
    parser.add_argument("--javac", default="javac")
    parser.add_argument("--steps", type=int, default=10_000_000)
    parser.add_argument("--seeds", nargs="+", type=int, default=[20261110, 20261111, 20261112, 20261113])
    parser.add_argument("--smoke", action="store_true", help="1,000 steps with seed 17 for fixed-state checks only")
    args = parser.parse_args()
    suite = args.suite.resolve()
    manifest = json.loads((suite / "manifest.json").read_text())
    if args.case not in manifest:
        parser.error(f"Unknown case; choose from {', '.join(manifest)}")
    case, data = suite / args.case, manifest[args.case]
    for name, expected in data["sha256"].items():
        if digest(case / name) != expected:
            raise ValueError(f"Changed fixture: {name}")
    steps, interval, seeds = (1000, 100, [17]) if args.smoke else (args.steps, 1000, args.seeds)
    if steps <= 0 or steps % interval or len(set(seeds)) != len(seeds) or any(seed <= 0 for seed in seeds):
        parser.error("Use distinct positive seeds and a positive multiple of the logging interval")
    status = capture(["git", "status", "--porcelain", "--untracked-files=normal"])
    if not args.smoke and (status or len(seeds) < 4):
        parser.error("Production comparisons require a clean checkout and at least four seeds")
    packages, binary = args.packages.resolve(), ROOT / "target/release/rove"
    versions = {}
    for path in packages.glob("*/version.xml"):
        package = ET.parse(path).getroot()
        versions[package.get("name")] = package.get("version")
    if versions != EXPECTED_PACKAGES:
        parser.error(f"Expected {EXPECTED_PACKAGES}; found {versions}")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    build_environment = {
        "RUSTFLAGS": "-C target-cpu=native", "CARGO_TARGET_DIR": str(ROOT / "target"),
        "CARGO_INCREMENTAL": "0", "CARGO_PROFILE_RELEASE_OPT_LEVEL": "3",
        "CARGO_PROFILE_RELEASE_LTO": "fat", "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "1",
        "CARGO_PROFILE_RELEASE_DEBUG": "false", "CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS": "false",
        "CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS": "false",
    }
    env = os.environ.copy()
    env.pop("CARGO_ENCODED_RUSTFLAGS", None)
    env.update(build_environment)
    build_command = ["cargo", "build", "--locked", "--release", "--bin", "rove"]
    with (output / "build.log").open("w") as stream:
        subprocess.run(build_command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT, check=True)
    classes = output / "classes"
    classes.mkdir()
    jars = [args.launcher.resolve(), *sorted(packages.glob("*/lib/*.jar"))]
    classpath = os.pathsep.join(map(str, jars))
    source = Path(__file__).with_name("ReferenceDiagnostics.java")
    subprocess.run([args.javac, "--release", "17", "-cp", classpath, "-d", str(classes), str(source)], check=True)
    jvm = [args.java, "-Xms256m", "-Xmx2g", "-Xss8m", "-Djava.awt.headless=true",
           "-Duser.language=en", "-Djava.only=true", f"-Dbeast.user.package.dir={packages}"]
    diagnostics = jvm + ["-cp", f"{classes}{os.pathsep}{classpath}", "rove.reference.ReferenceDiagnostics"]
    subprocess.run(diagnostics + ["self-test"], check=True)
    record = {
        "case": args.case, "fixture": data, "steps": steps, "log_every": interval,
        "seeds": seeds, "burnin_percent": 10, "smoke_only": args.smoke,
        "source_commit": capture(["git", "rev-parse", "HEAD"]), "source_dirty": bool(status),
        "executable_sha256": digest(binary), "exporter_sha256": digest(source),
        "protocol_sha256": digest(suite / "PROTOCOL.md"),
        "packages": versions, "jars": {p.name: digest(p) for p in jars},
        "rustc": capture(["rustc", "-Vv"]), "cargo": capture(["cargo", "-V"]),
        "build_command": build_command, "build_environment": build_environment,
        "java_version": subprocess.run([args.java, "-version"], text=True, capture_output=True, check=True).stderr,
        "platform": platform.platform(), "machine": platform.machine(),
        "hardware": hardware(), "power_before": power_state(),
        "load_before": list(os.getloadavg()), "threads": 1, "concurrent_chains": 1,
        "timing": "process start to exit, including I/O, startup, JVM warm-up and adaptive tuning; diagnostics excluded",
        "started_at": datetime.now(timezone.utc).isoformat(), "runs": [],
    }
    write_json(output / "metadata.json", record)
    # Alternating order avoids assigning one engine every earlier process.
    # There is no separate JVM warm-up: each fresh process's whole cost counts.
    for index, seed in enumerate(seeds):
        for engine in (["beast", "rove"] if index % 2 == 0 else ["rove", "beast"]):
            directory = output / engine / str(seed)
            directory.parent.mkdir(exist_ok=True)
            console = directory.parent / f"{seed}.console.log"
            if engine == "beast":
                directory.mkdir()
                model = ET.parse(case / "beast.xml")
                model.getroot().find("run").set("chainLength", str(steps))
                for logger in model.getroot().findall(".//logger"):
                    if logger.get("id") != "screenlog":
                        logger.set("logEvery", str(interval))
                model.write(directory / "analysis.xml", encoding="UTF-8", xml_declaration=True)
                command = jvm + ["-cp", classpath, "beastfx.app.beast.BeastMCMC", "-batch", "-seed", str(seed),
                                 "-threads", "1", "analysis.xml"]
                elapsed = execute(command, directory, console)
                shutil.copyfile(console, directory / "console.log")
                calculation = verify_run(directory, steps, interval)
            else:
                command = rove_command(binary, case, directory, seed, steps, interval, data["conditioning"])
                elapsed = execute(command, ROOT, console)
                verify_rove(directory, data, steps, interval)
                calculation = None
            run = {"engine": engine, "seed": seed, "command": command, "wall_seconds": elapsed,
                   "beast_calculation_seconds": calculation,
                   "sha256": {p.name: digest(p) for p in sorted(directory.iterdir()) if p.is_file()}}
            record["runs"].append(run)
            write_json(output / "metadata.json", record)
            print(f"{args.case}, {engine}, seed {seed}: {elapsed:.3f} seconds", flush=True)
    record["load_after"] = list(os.getloadavg())
    record["power_after"] = power_state()
    write_json(output / "metadata.json", record)
    # Analysis cannot contend with the timed chains.
    for run in record["runs"]:
        directory = output / run["engine"] / str(run["seed"])
        log = directory / ("analysis.log" if run["engine"] == "beast" else "trace.log")
        execute(diagnostics + ["inference-summary", str(log), str(directory / "trees.nexus"),
                              str(case / "truth.newick"), "10", str(directory / "summary.json")],
                ROOT, directory / "summary.console.log")
        if run["engine"] == "beast":
            execute(diagnostics + ["snapshot", str(directory / "analysis.xml"),
                                  str(directory / "analysis.xml.state"), str(directory / "snapshot.json")],
                    ROOT, directory / "snapshot.console.log")
    print(f"Diagnostics saved in {output}", flush=True)


if __name__ == "__main__":
    main()
