#!/usr/bin/env python3
"""Build matching BEAST inputs from the exact partitions exported by Rove."""

import argparse
import copy
import json
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TEMPLATE = ROOT / "fixtures/validation/reference-mkv/beast.xml"


def make_xml(data, conditioning, steps, interval, constrained=False, backend="java"):
    """Keep the supplied ORC probability model; replace observations and taxon sets."""
    if conditioning not in ("none", "variable") or backend not in ("java", "beagle"):
        raise ValueError("Unsupported conditioning or backend")
    if steps <= 0 or interval <= 0 or steps % interval:
        raise ValueError("Steps must be a positive multiple of the logging interval")
    if not data["partitions"] or len(set(data["taxa"])) != len(data["taxa"]):
        raise ValueError("Expected partitions and distinct taxa")
    if constrained and set(data["taxa"]) != {f"T{i}" for i in range(1, 21)}:
        raise ValueError("The reference constraint requires T1 through T20")
    if any(len(p["sequences"]) != len(data["taxa"]) for p in data["partitions"]):
        raise ValueError("Every partition must contain every taxon")
    model = ET.parse(TEMPLATE)
    root = model.getroot()
    run = root.find("run")
    run.set("chainLength", str(steps))
    run.find("state").set("storeEvery", "100000")
    root.remove(root.find("data"))
    likelihood = root.find(".//distribution[@id='likelihood']")
    prototype = copy.deepcopy(likelihood[0])
    likelihood.clear()
    likelihood.attrib.update(id="likelihood", spec="CompoundDistribution", useThreads="false")
    for index, part in enumerate(data["partitions"]):
        k = part["states"]
        name = f"partition_{k:02d}"
        alignment = ET.Element("data", id=name, spec="Alignment", dataType="standard")
        if conditioning == "variable":
            alignment.attrib.update(ascertained="true", excludefrom=str(part["characters"]), excludeto=str(part["characters"]+k))
        for taxon, sequence in zip(data["taxa"], part["sequences"]):
            if conditioning == "variable":
                sequence += "".join(map(str, range(k)))
            ET.SubElement(alignment, "sequence", spec="Sequence", taxon=taxon, totalcount=str(k), value=sequence)
        ET.SubElement(alignment, "userDataType", id=f"type_{k:02d}", spec="beast.base.evolution.datatype.StandardData",
                      nrOfStates=str(k), ambiguities=part["ambiguities"])
        root.insert(index, alignment)
        tree_likelihood = copy.deepcopy(prototype)
        tree_likelihood.set("id", f"tree_likelihood_{k:02d}")
        tree_likelihood.set("spec", "BeagleTreeLikelihood" if backend == "beagle" else "TreeLikelihood")
        tree_likelihood.set("data", "@" + name)
        tree_likelihood.set("useAmbiguities", "true")
        tree_likelihood.set("scaling", "always")
        site = tree_likelihood.find("siteModel")
        site.set("id", f"site_model_{k:02d}")
        site.find("substModel").attrib.update(id=f"mk_{k:02d}", datatype=f"@type_{k:02d}")
        if index:
            tree_likelihood.remove(tree_likelihood.find("branchRateModel"))
            tree_likelihood.set("branchRateModel", "@OptimisedRelaxedClock.c:matriz_20x50")
        likelihood.append(tree_likelihood)
    first = f"partition_{data['partitions'][0]['states']:02d}"
    taxonset = run.find("state/tree/taxonset")
    taxonset.remove(taxonset[0])
    ET.SubElement(taxonset, "alignment", idref=first)
    run.find("init").set("taxa", "@" + first)
    run.find(".//parameter[@id='ORCRates.c:matriz_20x50']").set("dimension", str(2*len(data["taxa"])-2))
    # The mutation rate is fixed at one: the one-parameter exchange is a no-op.
    run.find(".//weightvector[@id='weightparameter']").text = str(data["characters"])
    prior = run.find(".//distribution[@id='prior']")
    if not constrained:
        prior.remove(prior.find("distribution[@id='root.prior']"))
    for logger in run.findall("logger"):
        logger.set("logEvery", str(100000 if logger.get("id") == "screenlog" else interval))
        if logger.get("id") == "tracelog":
            logger.set("fileName", "trace.log")
            for element in list(logger):
                if element.get("idref") == "morphTreeLikelihood.matriz_20x502" or (not constrained and element.get("idref") == "root.prior"):
                    logger.remove(element)
    ET.indent(model, space="  ")
    return model


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--conditioning", choices=["none", "variable"], default="none")
    parser.add_argument("--steps", type=int, default=100000)
    parser.add_argument("--interval", type=int, default=100)
    parser.add_argument("--constrained", action="store_true")
    parser.add_argument("--backend", choices=["java", "beagle"], default="java")
    args = parser.parse_args()
    data = json.loads(args.input.read_text())
    make_xml(data, args.conditioning, args.steps, args.interval, args.constrained, args.backend).write(
        args.output, encoding="utf-8", xml_declaration=True)


if __name__ == "__main__":
    main()
