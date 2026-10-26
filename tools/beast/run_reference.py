#!/usr/bin/env python3
"""Run the pinned BEAST reference in fresh JVMs, then export diagnostics."""

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
import time
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from pathlib import Path

EXPECTED_PACKAGES = {
    "BEAST.base": "2.7.8",
    "BEAST.app": "2.7.8",
    "MM": "1.2.1",
    "ORC": "1.2.1",
    "BEASTLabs": "2.0.3",
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def verify_run(directory, chain_length, log_every):
    rows = [line for line in (directory / "analysis.log").read_text().splitlines()
            if line and not line.startswith("#")]
    samples = [int(row.split()[0]) for row in rows[1:]]
    expected = list(range(0, chain_length + 1, log_every))
    if samples != expected:
        raise ValueError("Incomplete or unexpected parameter trace")
    console = (directory / "console.log").read_text()
    match = re.search(r"Total calculation time: ([\d.]+) seconds", console)
    if not match or "uses BeerLikelihoodCore" not in console:
        raise ValueError("Missing completion marker or unexpected likelihood backend")
    # BeastMCMC.main can return zero even after an exception.
    if "Exception" in console or "Error parsing" in console:
        raise ValueError("BEAST reported an error; inspect console.log")
    return float(match.group(1))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packages", type=Path, required=True)
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--xml", type=Path, required=True)
    parser.add_argument("--truth", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--java", default="java")
    parser.add_argument("--javac", default="javac")
    parser.add_argument("--seeds", nargs="+", type=int, default=[20261025, 20261026])
    parser.add_argument("--burnin", type=int, default=10)
    args = parser.parse_args()
    if len(set(args.seeds)) != len(args.seeds) or any(seed <= 0 for seed in args.seeds):
        parser.error("Seeds must be distinct positive integers")
    if not 0 <= args.burnin < 100:
        parser.error("Burn-in must be between 0 and 99 percent")
    packages = args.packages.resolve()
    versions = {}
    for file in packages.glob("*/version.xml"):
        package = ET.parse(file).getroot()
        versions[package.get("name")] = package.get("version")
    if versions != EXPECTED_PACKAGES:
        parser.error(f"Expected exactly {EXPECTED_PACKAGES}; found {versions}")
    xml = args.xml.resolve()
    model = ET.parse(xml).getroot()
    chain_length = int(model.find("run").get("chainLength"))
    log_every = int(model.find(".//logger[@id='tracelog']").get("logEvery"))
    if chain_length % log_every:
        parser.error("Chain length must be a multiple of the logging interval")
    tree_logger = model.find(".//logger[@mode='tree']")
    tree_id = model.find(".//state/tree").get("id").split(":")[-1]
    tree_file = tree_logger.get("fileName").replace("$(tree)", tree_id).replace("$(filebase)", "analysis")
    if Path(tree_file).name != tree_file:
        parser.error("Expected a tree filename within each run directory")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    classes = output / "classes"
    classes.mkdir()
    jars = [args.launcher.resolve()] + sorted(packages.glob("*/lib/*.jar"))
    classpath = ":".join(str(path) for path in jars)
    source = Path(__file__).with_name("ReferenceDiagnostics.java")
    subprocess.run([args.javac, "--release", "17", "-cp", classpath, "-d", str(classes), str(source)], check=True)
    jvm = [args.java, "-Xms256m", "-Xmx2g", "-Xss8m", "-Djava.awt.headless=true",
           "-Duser.language=en", f"-Dbeast.user.package.dir={packages}"]
    diagnostics = jvm + ["-cp", f"{classes}:{classpath}", "rove.reference.ReferenceDiagnostics"]
    subprocess.run(diagnostics + ["self-test"], check=True)
    manifest = {
        "packages": versions,
        "java_version": subprocess.run([args.java, "-version"], capture_output=True, text=True, check=True).stderr,
        "platform": platform.platform(),
        "machine": platform.machine(),
        "xml_sha256": digest(xml),
        "truth_sha256": digest(args.truth),
        "jars": {path.name: digest(path) for path in jars},
        "chain_length": chain_length,
        "log_every": log_every,
        "threads": 1,
        "concurrent_chains": 1,
        "seeds": args.seeds,
        "burnin_percent": args.burnin,
    }
    write_json(output / "manifest.json", manifest)
    # Finish all timed chains before running the diagnostic calculations.
    for seed in args.seeds:
        directory = output / f"seed-{seed}"
        directory.mkdir()
        shutil.copyfile(xml, directory / "analysis.xml")
        command = jvm + ["-cp", classpath, "beastfx.app.beast.BeastMCMC", "-batch",
                         "-seed", str(seed), "-threads", "1", "analysis.xml"]
        metadata = {"seed": seed, "command": command, "started_at": datetime.now(timezone.utc).isoformat()}
        write_json(directory / "command.json", metadata)
        start = time.perf_counter()
        with (directory / "console.log").open("w") as console:
            subprocess.run(command, cwd=directory, stdout=console, stderr=subprocess.STDOUT, check=True)
        metadata["wall_seconds"] = time.perf_counter() - start
        metadata["beast_calculation_seconds"] = verify_run(directory, chain_length, log_every)
        write_json(directory / "timing.json", metadata)
        print(f"Seed {seed}: {metadata['wall_seconds']:.3f} seconds", flush=True)
    for seed in args.seeds:
        directory = output / f"seed-{seed}"
        commands = {
            "summary": [str(directory / "analysis.log"), str(directory / tree_file),
                        str(args.truth.resolve()), str(args.burnin)],
            "snapshot": [str(directory / "analysis.xml"), str(directory / "analysis.xml.state")],
        }
        for mode, arguments in commands.items():
            with (directory / f"{mode}.console.log").open("w") as console:
                subprocess.run(diagnostics + [mode] + arguments + [str(directory / f"{mode}.json")],
                               stdout=console, stderr=subprocess.STDOUT, check=True)


if __name__ == "__main__":
    main()
