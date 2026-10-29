#!/usr/bin/env python3
"""Prepare the recorded full-model inference cases from the archived inputs."""

import argparse
import json
import re
import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

from run_reference import digest, write_json

ROOT = Path(__file__).resolve().parents[2]
REFERENCE = ROOT / "fixtures/reference/20x50"
CONTROL = ROOT / "fixtures/synthetic/control_20x50"
CASES = {
    "reference-mkv": ("reference", "variable", []),
    "reference-mk": ("reference", "none", []),
    "control": ("control", "none", []),
    "homoplasy": ("generated", "none", ["--homoplasy", "10", "--seed", "7"]),
    "missing": ("generated", "none", ["--missing", "0.2", "--seed", "1"]),
    "reduced": ("generated", "none", ["--drop", "T1-T4", "--seed", "1"]),
}


def read_matrix(path):
    """The sequential, binary NEXUS subset used by these recorded fixtures."""
    text = re.sub(r"\[.*?\]", "", path.read_text(), flags=re.S)
    block = re.search(r"\bmatrix\b(.*?);", text, re.S | re.I)
    if block is None:
        raise ValueError("Missing MATRIX block")
    rows = [line.split() for line in block[1].splitlines() if line.strip()]
    if any(len(row) != 2 or not set(row[1]) <= set("01?") for row in rows):
        raise ValueError("Expected simple binary sequences with optional missing cells")
    sequences = dict(rows)
    if len(sequences) != len(rows) or len({len(s) for s in sequences.values()}) != 1:
        raise ValueError("Duplicate taxon or unequal sequence lengths")
    return sequences


def make_xml(sequences, conditioning, steps=10_000_000, interval=1000):
    model = ET.parse(REFERENCE / "analysis.xml")
    root = model.getroot()
    size = len(next(iter(sequences.values())))
    if set(sequences) != {s.get("taxon") for s in root.findall("data/sequence")}:
        raise ValueError("These configurations require the reference taxon set")
    for sequence in root.findall("data/sequence"):
        sequence.set("value", sequences[sequence.get("taxon")] + ("01" if conditioning == "variable" else ""))
    alignment = root.find(".//alignment[@spec='FilteredAlignment']")
    alignment.set("filter", f"1-{size + (2 if conditioning == 'variable' else 0)}")
    for key in ("ascertained", "excludefrom", "excludeto"):
        alignment.attrib.pop(key, None)
    if conditioning == "variable":
        if any(set(column) - {"?"} != {"0", "1"} for column in zip(*sequences.values())):
            raise ValueError("Mkv requires every observed character to be demonstrably variable")
        alignment.attrib.update(ascertained="true", excludefrom=str(size), excludeto=str(size + 2))
    root.find("run").set("chainLength", str(steps))
    root.find(".//weightvector[@id='weightparameter']").text = str(size)
    trace = root.find(".//logger[@id='tracelog']")
    # Keep the supplied ORC operators and probability model. Add diagnostics only.
    ET.SubElement(trace, "log", id="TreeStat.t:matriz_20x50",
                  spec="beast.base.evolution.tree.TreeStatLogger", tree="@Tree.t:matriz_20x50")
    for prefix in ("GammaShapePrior.s:matriz_20x502", "ORCRatePriorDistribution.c:matriz_20x50",
                   "ORCsigmaPrior.c:matriz_20x50"):
        ET.SubElement(trace, "log", idref=prefix)
    ET.SubElement(trace, "log", id="branch_rate_sum", spec="beast.base.evolution.Sum",
                  arg="@ORCRates.c:matriz_20x50")
    for logger in root.findall(".//logger"):
        logger.set("logEvery", str(100_000 if logger.get("id") == "screenlog" else interval))
        if logger.get("mode") == "tree":
            logger.set("fileName", "trees.nexus")
    ET.indent(model, space="    ")
    return model


def prepare(output):
    output.mkdir(parents=True, exist_ok=False)
    manifest = {}
    for name, (source, conditioning, flags) in CASES.items():
        directory = output / name
        directory.mkdir()
        command = None
        if source == "generated":
            stem = directory / "matrix"
            command = ["python3", "tools/synth_matrix.py", *flags, "--out", str(stem.relative_to(ROOT))]
            subprocess.run([sys.executable, *command[1:]], cwd=ROOT, check=True)
            (directory / "matrix.tre").rename(directory / "truth.newick")
        else:
            matrix = REFERENCE / "matrix.nex" if source == "reference" else CONTROL.with_suffix(".nex")
            truth = REFERENCE / "truth.newick" if source == "reference" else CONTROL.with_suffix(".tre")
            shutil.copyfile(matrix, directory / "matrix.nex")
            shutil.copyfile(truth, directory / "truth.newick")
        sequences = read_matrix(directory / "matrix.nex")
        make_xml(sequences, conditioning).write(directory / "beast.xml", encoding="UTF-8", xml_declaration=True)
        columns = list(zip(*sequences.values()))
        manifest[name] = {
            "source": source, "conditioning": conditioning, "generation_command": command,
            "taxa": len(sequences), "characters": len(columns), "patterns": len(set(columns)),
            "missing_cells": sum(s.count("?") for s in sequences.values()),
            "sha256": {p.name: digest(p) for p in sorted(directory.iterdir())},
        }
    write_json(output / "manifest.json", manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    prepare(args.output.resolve())


if __name__ == "__main__":
    main()
