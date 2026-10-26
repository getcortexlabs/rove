---
description: "The synthetic control matrix that ships with Rove, the script that regenerates and degrades it, and how to run it in BEAST 2."
---

# Synthetic matrices

`fixtures/synthetic/control_20x50.nex` is a positive control: 20 taxa, 50 binary characters, no homoplasy, no missing data, known tree. T20 is the outgroup, all zeros. The ingroup tree is balanced and every one of its 18 clades gets exclusive synapomorphies, three for the nine largest clades and two for the rest. The last five characters are autapomorphies. This construction does not guarantee posterior support: rooting, clock models and priors also affect the result.

`control_20x50.tre` is the true tree in Newick, also repeated as a comment at the end of the NEXUS file. `control_20x50.legend.txt` maps each character to the clade it supports.

## Regenerating and degrading it

`tools/synth_matrix.py` regenerates the control and produces degraded variants:

```sh
python3 tools/synth_matrix.py --out fixtures/synthetic/control_20x50
python3 tools/synth_matrix.py --homoplasy 10 --seed 7 --out /tmp/homoplasy10
python3 tools/synth_matrix.py --missing 0.2 --out /tmp/missing20
python3 tools/synth_matrix.py --drop T1-T4 --out /tmp/no_t1_t4
```

Degradations are listed at the end of the legend file.

## Running it in BEAST 2

In BEAUti, install the MorphModels package, import the NEXUS file, choose the Lewis Mk model with the ascertainment correction for variable characters, a strict clock and a Yule tree prior.

This is an example setup for the generated control. It differs from the selected reference analysis below.

## Supplied reference matrix

`fixtures/reference/20x50/matrix.nex` is a separate dataset with its original legend and BEAST XML. Its topology and character assignments differ from the generated control. The recorded BEAST 2.7.8 experiment uses a relaxed clock, four gamma categories, a birth–death prior and an ingroup monophyly constraint. Inputs, provenance and results are in the [reference report](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md).

The [NEXUS reader](./nexus) imports both fixtures independently. Each has 23 observed patterns representing 50 characters, but the patterns differ. Neither matrix is substituted for the other in validation.
