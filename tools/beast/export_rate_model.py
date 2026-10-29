#!/usr/bin/env python3
"""Export BEAST 2.7.8's gamma categories and clock prior densities."""

import argparse
import json
import platform
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

from run_reference import EXPECTED_PACKAGES, digest, write_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packages", type=Path, required=True)
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--java", default="java")
    parser.add_argument("--javac", default="javac")
    args = parser.parse_args()
    if args.output.exists():
        parser.error("The output file must not exist")
    packages = args.packages.resolve()
    versions = {}
    for path in packages.glob("*/version.xml"):
        package = ET.parse(path).getroot()
        versions[package.get("name")] = package.get("version")
    if versions != EXPECTED_PACKAGES:
        parser.error(f"Expected {EXPECTED_PACKAGES}; found {versions}")
    source = Path(__file__).with_name("ReferenceDiagnostics.java").resolve()
    jars = [args.launcher.resolve()] + sorted(packages.glob("*/lib/*.jar"))
    classpath = ":".join(map(str, jars))
    with tempfile.TemporaryDirectory(prefix="rove-rate-model-") as work:
        subprocess.run([args.javac, "--release", "17", "-cp", classpath, "-d", work, str(source)], check=True)
        output = Path(work) / "rates.json"
        subprocess.run([
            args.java, "-Djava.awt.headless=true", f"-Dbeast.user.package.dir={packages}",
            "-cp", f"{work}:{classpath}", "rove.reference.ReferenceDiagnostics", "rate-model", str(output),
        ], check=True)
        data = json.loads(output.read_text())
    data["provenance"] = {
        "beast_version": versions["BEAST.base"],
        "java_version": subprocess.run([args.java, "-version"], capture_output=True, text=True, check=True).stderr,
        "machine": platform.machine(),
        "source_sha256": digest(source),
        "jar_sha256": {jar.name: digest(jar) for jar in jars},
        "command": "ReferenceDiagnostics rate-model <output.json>",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    write_json(args.output, data)


if __name__ == "__main__":
    main()
