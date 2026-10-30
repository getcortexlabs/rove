#!/usr/bin/env python3
"""Prepare and run the expanded, sequential cross-engine benchmark protocol."""

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from pathlib import Path

from prepare_expanded import ROOT, make_xml
from run import hardware, power_state, sha256

sys.path.insert(0, str(ROOT / "tools/beast"))
from run_reference import EXPECTED_PACKAGES
from compare_runs import read_trace

SEEDS = [20261201, 20261202, 20261203, 20261204]
CASES = {
    "reference": {"steps": 10_000_000, "interval": 1000, "conditioning": "variable", "constrained": True},
    "morphology90": {"steps": 1_000_000, "interval": 100, "conditioning": "none", "constrained": False},
    "zilla": {"steps": 10_000, "interval": 10, "conditioning": "none", "constrained": False},
}


def write(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def capture(command, **kwargs):
    return subprocess.run(command, text=True, capture_output=True, check=True, **kwargs).stdout.strip()


def execute(command, directory, console):
    start = time.perf_counter()
    with console.open("w") as stream:
        subprocess.run(command, cwd=directory, stdout=stream, stderr=subprocess.STDOUT, check=True)
    return time.perf_counter() - start


def verify_trace(path, steps, interval):
    # BEAST can return exit code zero after an XML error. Require complete output.
    trace = read_trace(path, 0)
    if trace["Sample"] != list(range(0, steps + 1, interval)):
        raise ValueError(f"Incomplete or incorrectly spaced trace: {path}")


def verify_rove(directory, settings, seed):
    record = json.loads((directory / "run.json").read_text())
    model, actual = record["model"], record["settings"]
    constraint = [[f"T{i}" for i in range(1, 20)]] if settings["constrained"] else []
    conditioning = "variable characters (Mkv)" if settings["conditioning"] == "variable" else "none (Mk)"
    if (record["status"] != "completed" or record["input"]["sha256"] != settings["input_sha256"]
            or record["input"]["gaps"] != "missing" or model["monophyly"] != constraint
            or model["conditioning"] != conditioning or model["sample_from_prior"]
            or model["partitioning"] != "observed" or model["preset"] != "reference"):
        raise ValueError("Rove target differs from the declared benchmark")
    expected = {"seed": seed, "steps": settings["steps"], "sample_every": settings["interval"],
                "initial_gamma_shape": 1.0, "initial_clock_sigma": 0.2, "initial_branch_rate": 0.5,
                "initial_birth_difference_rate": 1.0, "initial_relative_death_rate": 0.5, "tuning": True}
    if any(actual[key] != value for key, value in expected.items()):
        raise ValueError("Rove run settings differ from the protocol")
    partitions = [{"states": p["states"], "characters": p["characters"], "patterns": p["observed_patterns"]}
                  for p in model["partitions"]]
    if partitions != settings["partitions"]:
        raise ValueError("Rove partitions differ from the exported BEAST inputs")


def jvm(config, *, beagle=False, diagnostics=False):
    flags = [config["java"], "-Xms256m", "-Xmx4g" if diagnostics else "-Xmx2g", "-Xss8m",
             "-Djava.awt.headless=true", "-Duser.language=en", f"-Dbeast.user.package.dir={config['packages']}"]
    flags += ([f"-Djava.library.path={config['beagle_lib']}"] if beagle else ["-Djava.only=true"])
    return flags + ["-cp", config["classpath"]]


def beast(config, seed):
    return jvm(config) + ["beastfx.app.beast.BeastMCMC", "-batch", "-seed", str(seed), "-threads", "1", "analysis.xml"]


def snapshot(config, directory, matrix, conditioning):
    execute(jvm(config) + ["rove.reference.BenchmarkSnapshot", str(directory / "analysis.xml"),
            str(directory / "analysis.xml.state"), str(directory / "snapshot.json")],
            ROOT, directory / "snapshot.console.log")
    with (directory / "fixed-state.json").open("w") as stream:
        subprocess.run([config["input_tool"], "check", str(matrix), str(directory / "snapshot.json"), conditioning],
                       stdout=stream, check=True, cwd=ROOT)


def newick(state):
    """Use an explicitly named pilot state as an RF reference, not as biological truth."""
    def node(index):
        current = state["nodes"][index]
        if not current["children"]:
            return "'" + current["taxon"].replace("'", "''") + "'"
        return "(" + ",".join(node(child) for child in current["children"]) + ")"
    return node(state["root"]) + ";\n"


def prepare(args):
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    packages, launcher, java_home = args.packages.resolve(), args.launcher.resolve(), args.java_home.resolve()
    versions = {node.get("name"): node.get("version") for node in
                (ET.parse(path).getroot() for path in packages.glob("*/version.xml"))}
    if versions != EXPECTED_PACKAGES:
        raise ValueError(f"Unexpected packages: {versions}")
    environment = os.environ.copy()
    for name in list(environment):
        if name.startswith("CARGO_PROFILE_") or name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_TARGET_DIR"):
            del environment[name]
    environment["CARGO_INCREMENTAL"] = "0"
    build = ["cargo", "build", "--locked", "--release", "--bin", "rove", "--bench", "likelihood",
             "--example", "benchmark_input", "--message-format=json"]
    with (output / "build.stderr").open("w") as errors:
        log = subprocess.run(build, cwd=ROOT, env=environment, text=True, stdout=subprocess.PIPE, stderr=errors, check=True).stdout
    (output / "build.jsonl").write_text(log)
    executables = {message["target"]["name"]: message["executable"] for message in map(json.loads, log.splitlines())
                   if message.get("reason") == "compiler-artifact" and message.get("executable")}
    binaries = output / "bin"
    binaries.mkdir()
    for name, source in executables.items():
        shutil.copy2(source, binaries / name)
    classes = output / "classes"
    classes.mkdir()
    jars = [launcher, *sorted(packages.glob("*/lib/*.jar"))]
    classpath = os.pathsep.join(map(str, jars))
    sources = [ROOT / "tools/beast" / f"{name}.java" for name in
               ["BenchmarkSnapshot", "KernelBenchmark", "ReferenceDiagnostics"]]
    subprocess.run([str(java_home / "bin/javac"), "--release", "17", "-cp", classpath, "-d", str(classes),
                    *map(str, sources)], check=True)
    config = {
        "java": str(java_home / "bin/java"), "packages": str(packages), "beagle_lib": str(args.beagle_lib.resolve()),
        "classpath": str(classes) + os.pathsep + classpath, "rove": str(binaries / "rove"),
        "kernel": str(binaries / "likelihood"), "input_tool": str(binaries / "benchmark_input"),
        "versions": versions, "jars": {path.name: sha256(path) for path in jars},
        "java_sources": {path.name: sha256(path) for path in sources},
        "source_commit_at_build": capture(["git", "rev-parse", "HEAD"], cwd=ROOT),
        "rustc": capture(["rustc", "-Vv"]), "cargo": capture(["cargo", "-V"]),
        "java_version": subprocess.run([str(java_home / "bin/java"), "-version"], text=True, capture_output=True, check=True).stderr,
        "build_command": build, "build_settings": {"profile": "release", "opt_level": 3, "lto": "fat", "codegen_units": 1,
        "target_cpu": "Rust target default (no target-cpu override)", "CARGO_INCREMENTAL": "0", "features": []},
        "binary_sha256": {path.name: sha256(path) for path in binaries.iterdir()},
        "beagle_libraries": {path.name: sha256(path) for path in args.beagle_lib.resolve().iterdir()
                             if path.is_file() and not path.is_symlink()},
        "protocol_sha256": sha256(ROOT / "benchmarks/expanded/PROTOCOL.md"),
        "hardware": hardware(), "cases": {}, "seeds": SEEDS, "burnin_percent": 10,
    }
    write(output / "config.json", config)
    subprocess.run([config["kernel"], "--export", str(output / "kernel-input.json")], check=True)
    for backend in ("java", "beagle"):
        execute(jvm(config, beagle=backend == "beagle") + ["rove.reference.KernelBenchmark",
                str(output / "kernel-input.json"), backend, str(output / f"kernel-{backend}-check.json"), "0", "0", "10"],
                ROOT, output / f"kernel-{backend}-check.console.log")
    inputs = {"reference": ROOT / "fixtures/reference/20x50/matrix.nex",
              "morphology90": ROOT / "fixtures/benchmarks/morphology_90x250.nex"}
    if args.zilla:
        inputs["zilla"] = args.zilla.resolve()
    for name in args.case or list(inputs):
        settings = CASES[name]
        directory = output / name
        directory.mkdir()
        matrix = directory / "matrix.nex"
        shutil.copyfile(inputs[name], matrix)
        data = json.loads(capture([config["input_tool"], "export", str(matrix)]))
        write(directory / "input.json", data)
        config["cases"][name] = {**settings, "input_sha256": sha256(matrix), "taxa": len(data["taxa"]),
            "characters": data["characters"], "partitions": [{key: p[key] for key in ["states", "characters", "patterns"]}
                                                              for p in data["partitions"]]}
        pilot = directory / "pilot"
        pilot.mkdir()
        make_xml(data, settings["conditioning"], 1000, 100, settings["constrained"]).write(pilot / "analysis.xml")
        elapsed = execute(beast(config, 81991), pilot, pilot / "console.log")
        verify_trace(pilot / "trace.log", 1000, 100)
        snapshot(config, pilot, matrix, settings["conditioning"])
        (directory / "reference.newick").write_text(newick(json.loads((pilot / "snapshot.json").read_text())))
        config["cases"][name].update(pilot_wall_seconds=elapsed, reference_sha256=sha256(directory / "reference.newick"),
                                    reference_kind="BEAST pilot seed 81991, state 1000; arbitrary RF reference")
        write(output / "config.json", config)
        print(f"Prepared {name}; fixed-state checks passed", flush=True)


def measure(output):
    config = json.loads((output / "config.json").read_text())
    if capture(["git", "status", "--porcelain", "--untracked-files=normal"], cwd=ROOT):
        raise ValueError("Commit the protocol and tools before collecting production timings")
    if sha256(ROOT / "benchmarks/expanded/PROTOCOL.md") != config["protocol_sha256"]:
        raise ValueError("Protocol changed after preparation")
    for name, expected in config["binary_sha256"].items():
        if sha256(output / "bin" / name) != expected:
            raise ValueError(f"Executable changed: {name}")
    record = {"source_commit": capture(["git", "rev-parse", "HEAD"], cwd=ROOT),
              "started_at": datetime.now(timezone.utc).isoformat(), "power_before": power_state(),
              "load_before": list(os.getloadavg()), "threads": 1, "concurrent_processes": 1, "runs": []}
    if (output / "timings.json").exists():
        raise ValueError("Use a new output directory; do not overwrite measured runs")
    write(output / "timings.json", record)
    for name, settings in config["cases"].items():
        case = output / name
        if sha256(case / "matrix.nex") != settings["input_sha256"]:
            raise ValueError("Input changed after fixed-state validation")
        data = json.loads((case / "input.json").read_text())
        for index, seed in enumerate(config["seeds"]):
            for engine in (["beast", "rove"] if index % 2 == 0 else ["rove", "beast"]):
                directory = case / engine / str(seed)
                directory.parent.mkdir(exist_ok=True)
                console = directory.parent / f"{seed}.console.log"
                if engine == "beast":
                    directory.mkdir()
                    make_xml(data, settings["conditioning"], settings["steps"], settings["interval"], settings["constrained"]).write(directory / "analysis.xml")
                    command = beast(config, seed)
                    elapsed = execute(command, directory, console)
                else:
                    command = [config["rove"], "sample", str(case / "matrix.nex"), "--output", str(directory),
                        "--model", "reference", "--conditioning", settings["conditioning"], "--gaps", "missing",
                        "--steps", str(settings["steps"]), "--sample-every", str(settings["interval"]),
                        "--seed", str(seed), "--print-every", "100000", "--checkpoint-every", "100000"]
                    if settings["constrained"]:
                        command += ["--monophyly", ",".join(f"T{i}" for i in range(1, 20))]
                    elapsed = execute(command, ROOT, console)
                    verify_rove(directory, settings, seed)
                verify_trace(directory / "trace.log", settings["steps"], settings["interval"])
                record["runs"].append({"case": name, "engine": engine, "seed": seed, "wall_seconds": elapsed,
                    "command": command, "sha256": {p.name: sha256(p) for p in directory.iterdir() if p.is_file()}})
                write(output / "timings.json", record)
                print(f"{name}, {engine}, seed {seed}: {elapsed:.3f} seconds", flush=True)
    kernels = output / "kernel"
    kernels.mkdir()
    for repetition in range(3):
        for backend in (["rove", "java", "beagle"] if repetition != 1 else ["beagle", "java", "rove"]):
            if backend == "rove":
                command = [config["kernel"], "--samples", "9", "--sample-ms", "10", "--warmup-ms", "100"]
                if repetition == 1:
                    command.append("--reverse")
                with (kernels / f"rove-{repetition}.tsv").open("w") as stream, (kernels / f"rove-{repetition}.stderr").open("w") as errors:
                    subprocess.run(command, stdout=stream, stderr=errors, cwd=ROOT, check=True)
            else:
                command = jvm(config, beagle=backend == "beagle") + ["rove.reference.KernelBenchmark",
                    str(output / "kernel-input.json"), backend, str(kernels / f"{backend}-{repetition}.json"), "500", "9", "10"]
                execute(command, ROOT, kernels / f"{backend}-{repetition}.console.log")
            record.setdefault("kernel_commands", []).append(command)
            write(output / "timings.json", record)
            print(f"Kernel repetition {repetition + 1}: {backend}", flush=True)
    record.update(finished_at=datetime.now(timezone.utc).isoformat(), power_after=power_state(), load_after=list(os.getloadavg()))
    write(output / "timings.json", record)


def diagnose(output):
    config = json.loads((output / "config.json").read_text())
    for name, settings in config["cases"].items():
        case = output / name
        for engine in ("rove", "beast"):
            for seed in config["seeds"]:
                directory = case / engine / str(seed)
                command = jvm(config, diagnostics=True) + ["rove.reference.ReferenceDiagnostics", "inference-summary",
                    str(directory / "trace.log"), str(directory / "trees.nexus"), str(case / "reference.newick"),
                    "10", str(directory / "summary.json")]
                execute(command, ROOT, directory / "summary.console.log")
                if engine == "beast":
                    snapshot(config, directory, case / "matrix.nex", settings["conditioning"])
        print(f"Diagnosed {name}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["prepare", "measure", "diagnose"])
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--packages", type=Path)
    parser.add_argument("--launcher", type=Path)
    parser.add_argument("--java-home", type=Path)
    parser.add_argument("--beagle-lib", type=Path)
    parser.add_argument("--zilla", type=Path)
    parser.add_argument("--case", choices=list(CASES), action="append",
                        help="Prepare selected cases; by default use both public cases and zilla if supplied")
    args = parser.parse_args()
    if args.mode == "prepare":
        if any(getattr(args, field) is None for field in ["packages", "launcher", "java_home", "beagle_lib"]):
            parser.error("Preparation requires package, launcher, Java and BEAGLE paths")
        if args.case and (len(set(args.case)) != len(args.case) or ("zilla" in args.case and args.zilla is None)):
            parser.error("Use distinct cases and supply --zilla when selecting the private case")
        prepare(args)
    elif args.mode == "measure":
        measure(args.output.resolve())
    else:
        diagnose(args.output.resolve())


if __name__ == "__main__":
    main()
