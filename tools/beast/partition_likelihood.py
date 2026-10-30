#!/usr/bin/env python3
"""Export fixed-state multistate likelihoods from BEAST 2.7.8 / MM 1.2.1."""

import argparse
import hashlib
import json
import platform
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packages", type=Path, required=True)
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--matrix", type=Path, default=Path("fixtures/validation/multistate/matrix.nex"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--java", default="java")
    parser.add_argument("--javac", default="javac")
    args = parser.parse_args()
    versions = {ET.parse(p).getroot().get("name"): ET.parse(p).getroot().get("version")
                for p in args.packages.glob("*/version.xml")}
    if versions.get("BEAST.base") != "2.7.8" or versions.get("MM") != "1.2.1":
        parser.error("Requires BEAST.base 2.7.8 and MM 1.2.1")
    args.output.mkdir(parents=True, exist_ok=False)
    jars = [args.launcher.resolve()] + sorted(args.packages.resolve().glob("*/lib/*.jar"))
    classpath = ":".join(map(str, jars))
    source = Path(__file__).with_name("PartitionLikelihood.java").resolve()
    result = args.output.resolve() / "beast-likelihoods.json"
    with tempfile.TemporaryDirectory() as classes:
        subprocess.run([args.javac, "--release", "17", "-cp", classpath, "-d", classes, str(source)], check=True)
        command = [args.java, "-Djava.awt.headless=true",
                   f"-Dbeast.user.package.dir={args.packages.resolve()}", "-cp", f"{classes}:{classpath}",
                   "rove.reference.PartitionLikelihood", str(args.matrix.resolve()), str(result)]
        with (args.output / "console.log").open("w") as console:
            subprocess.run(command, stdout=console, stderr=subprocess.STDOUT, check=True)
    values = json.loads(result.read_text())
    if len(values["cases"]) != 12:
        raise ValueError("Expected twelve fixed-state cases")
    provenance = {
        "packages": versions,
        "java_version": subprocess.run([args.java, "-version"], capture_output=True, text=True, check=True).stderr,
        "platform": platform.platform(),
        "jars": {p.name: digest(p) for p in jars},
        "source_sha256": digest(source), "matrix_sha256": digest(args.matrix),
        "result_sha256": digest(result),
        "backend": "Java BeerLikelihoodCore; java.only=true; scaling=always; useAmbiguities=true",
        "reproduction": "python3 tools/beast/partition_likelihood.py --packages <packages> --launcher <launcher.jar> --output <new-directory>",
    }
    (args.output / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")


if __name__ == "__main__":
    main()
