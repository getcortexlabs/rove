# Synthetic matrices

`control_20x50.nex` is a positive control: 20 taxa, 50 binary characters, no
homoplasy, no missing data, known tree. T20 is the outgroup, all zeros. The
ingroup tree is balanced and every one of its 18 clades gets exclusive
synapomorphies (3 for the 9 largest clades, 2 for the rest); the last 5
characters are autapomorphies. The known topology describes how the matrix was
constructed. Posterior support depends on the model, rooting assumptions,
priors, and sampling; the construction alone does not guarantee a threshold.

This generated control is distinct from the supplied
[reference dataset](../reference/20x50/README.md). The measured BEAST 2 results
use that dataset and its XML, not this fixture or the example settings below.

`control_20x50.tre` is the true tree in Newick, also repeated as a comment at
the end of the NEXUS. `control_20x50.legend.txt` maps each character to the
clade it supports.

Regenerate or degrade it with `tools/synth_matrix.py`:

    python3 tools/synth_matrix.py --out fixtures/synthetic/control_20x50
    python3 tools/synth_matrix.py --homoplasy 10 --seed 7 --out /tmp/homoplasy10
    python3 tools/synth_matrix.py --missing 0.2 --out /tmp/missing20
    python3 tools/synth_matrix.py --drop T1-T4 --out /tmp/no_t1_t4

Degradations are listed at the end of the legend file.

In BEAUti: install the MorphModels package, import the NEXUS, choose the Lewis
Mk model with the ascertainment correction for variable characters, a strict
clock and a Yule tree prior.
