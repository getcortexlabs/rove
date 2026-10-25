#!/usr/bin/env python3
import argparse
import random
import sys
from pathlib import Path


def build_tree(leaves):
    if len(leaves) == 1:
        return leaves[0]
    mid = len(leaves) // 2
    return (build_tree(leaves[:mid]), build_tree(leaves[mid:]))


def collect_clades(tree, out):
    if isinstance(tree, str):
        return [tree]
    members = collect_clades(tree[0], out) + collect_clades(tree[1], out)
    out.append(members)
    return members


def newick(tree):
    if isinstance(tree, str):
        return tree
    return f"({newick(tree[0])},{newick(tree[1])})"


def label(members):
    return members[0] if len(members) == 1 else f"{members[0]}-{members[-1]}"


def ranges(labels):
    start = 0
    for i in range(1, len(labels) + 1):
        if i == len(labels) or labels[i] != labels[start]:
            span = f"{start + 1}" if i - start == 1 else f"{start + 1}-{i}"
            yield span, labels[start]
            start = i


def main():
    p = argparse.ArgumentParser(
        description="Binary morphological matrix built from a known balanced tree, "
        "one exclusive set of synapomorphies per clade. The last taxon is the "
        "all-zero outgroup."
    )
    p.add_argument("--taxa", type=int, default=20)
    p.add_argument("--chars", type=int, default=50)
    p.add_argument("--autapomorphies", type=int, default=5)
    p.add_argument("--homoplasy", type=int, default=0, metavar="N", help="flip N informative cells against the tree")
    p.add_argument("--missing", type=float, default=0.0, metavar="P", help="replace each cell by ? with probability P")
    p.add_argument("--drop", action="append", default=[], metavar="CLADE", help="remove the synapomorphies of CLADE, e.g. T1-T4 (repeatable)")
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--out", type=Path, required=True, help="output path without extension")
    args = p.parse_args()

    rng = random.Random(args.seed)
    outgroup = f"T{args.taxa}"
    ingroup = [f"T{i}" for i in range(1, args.taxa)]
    taxa = ingroup + [outgroup]
    tree = build_tree(ingroup)
    groups = []
    collect_clades(tree, groups)
    groups.sort(key=lambda m: (-len(m), int(m[0][1:])))
    names = [label(m) for m in groups]

    for name in args.drop:
        if name not in names:
            p.error(f"unknown clade {name}; clades are {', '.join(names)}")

    informative = args.chars - args.autapomorphies
    minimum = 2 * len(groups) + args.autapomorphies
    if informative < 2 * len(groups):
        p.error(f"{len(groups)} clades need at least {minimum} characters")
    counts = {name: 2 for name in names}
    for i in range(informative - 2 * len(groups)):
        counts[names[i % len(names)]] += 1

    columns = []
    for members, name in zip(groups, names):
        if name in args.drop:
            continue
        columns.extend((name, set(members)) for _ in range(counts[name]))
    step = len(ingroup) / (args.autapomorphies + 1)
    for i in range(args.autapomorphies):
        t = ingroup[int(round(step * (i + 1))) - 1]
        columns.append((f"{t} autapomorphy", {t}))

    cells = {t: ["1" if t in members else "0" for _, members in columns] for t in taxa}
    notes = []

    informative_cols = [i for i, (name, _) in enumerate(columns) if "autapomorphy" not in name]
    pool = [(c, t) for c in informative_cols for t in taxa]
    for c, t in rng.sample(pool, min(args.homoplasy, len(pool))):
        before = cells[t][c]
        cells[t][c] = "0" if before == "1" else "1"
        notes.append(f"homoplasy: char {c + 1} {t} {before}->{cells[t][c]}")

    missing = 0
    if args.missing > 0:
        for t in taxa:
            for c in range(len(columns)):
                if rng.random() < args.missing:
                    cells[t][c] = "?"
                    missing += 1
        notes.append(f"missing: {missing} cells")

    nchar = len(columns)
    width = max(len(t) for t in taxa) + 2
    argv = " ".join(sys.argv[1:])
    tree_newick = f"({outgroup},{newick(tree)});"

    nexus = [
        "#NEXUS",
        f"[tools/synth_matrix.py {argv}]",
        "",
        "BEGIN DATA;",
        f"\tDIMENSIONS NTAX={len(taxa)} NCHAR={nchar};",
        '\tFORMAT DATATYPE=STANDARD SYMBOLS="01" MISSING=? GAP=-;',
        "\tMATRIX",
    ]
    nexus += [f"\t{t.ljust(width)}{''.join(cells[t])}" for t in taxa]
    nexus += ["\t;", "END;", "", f"[true tree: {tree_newick}]", ""]

    legend = [f"{span}\t{name}" for span, name in ranges([name for name, _ in columns])]
    if notes:
        legend += [""] + notes
    legend += ["", f"true tree: {tree_newick}", ""]

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.with_suffix(".nex").write_text("\n".join(nexus))
    args.out.with_suffix(".tre").write_text(tree_newick + "\n")
    args.out.with_suffix(".legend.txt").write_text("\n".join(legend))
    print(f"{len(taxa)} taxa, {nchar} characters, {len(groups) - len(args.drop)} supported clades -> {args.out}.{{nex,tre,legend.txt}}")


if __name__ == "__main__":
    main()
