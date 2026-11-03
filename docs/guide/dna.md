---
description: "Aligned NEXUS and FASTA DNA under JC69, with explicit ambiguity and gap handling."
---

# Aligned DNA with JC69

Open a NEXUS `DATATYPE=DNA` matrix or an aligned FASTA file in the Mac app.
Import selects **DNA · JC69**, keeps all four A/C/G/T states at every site,
and includes constant sites. Rove does not align, trim or assemble reads.
The same input works with `sample`, `inspect` and `marginal`:

```sh
rove sample alignment.fasta --output dna-run --seed 42 --steps 30000000 --sample-every 3000 --model reference --gaps missing
```

Choose `--gaps missing` only when gaps should contribute no state information.
The default rejects gaps. `?` is missing; `R` permits A/G, `Y` C/T, and the
other IUPAC ambiguity codes retain their allowed states. `N` permits all four.
These are likelihood masks of ones, not fractional probabilities or extra
states. Parenthesized polymorphism is preserved but unsupported for inference.
DNA defaults to `--conditioning none --partitioning declared`; explicitly
requesting Mkv, Mkparsinf or morphological alphabet recoding is rejected.

FASTA sequences may wrap across lines and must have equal nonzero lengths.
The first whitespace-delimited identifier after `>` names the taxon; the
remaining description is ignored, following the [FASTA header convention](https://www.ncbi.nlm.nih.gov/genbank/fastaformat/).
Rove's existing NEXUS naming convention also applies to FASTA: underscores
become spaces, names are checked for duplicates ignoring ASCII case, and
spelling otherwise remains intact. Review the displayed terminal names before
rooting or matching datasets. Original files remain unchanged.

For NEXUS DNA, bases are case insensitive, the alphabet is fixed A/C/G/T, and
the default gap symbol is `-`. Optional `SYMBOLS="ACGT"` is accepted; custom
alphabets, RESPECTCASE, ordered DNA, RNA/protein datatypes and unsupported
NEXUS directives are rejected. `MISSING=N` explicitly makes N missing.
The same supported sequential/interleaved layout rules apply as for morphology.

JC69 fixes frequencies at 1/4 and exchangeabilities equal, normalized to one
expected substitution per site: `Pii(t)=1/4+3/4 exp(−4t/3)` and
`Pij(t)=1/4−1/4 exp(−4t/3)` for different bases. Strict and lognormal relaxed
clocks, gamma categories, rooting and supported tree priors use the existing
controls. DNA has no morphology ascertainment correction. HKY, GTR and
mixed-data inference are outside this delivery.

Format-11 app setups, History, run records and checkpoints preserve DNA/JC69
identity and the chosen gap policy. Older apps reject these setups. CLI runs
can be imported into History; incompatible models cannot be pooled for
convergence. Trees, diagnostics and marginal estimates retain the fitted
model. Short execution tests demonstrate integration, not posterior convergence.
Posterior prediction currently accepts single bases and missing cells; IUPAC
uncertainty requires an observation model and is rejected for simulation.
Replicates are emitted as DNA NEXUS. Reproducibility exports retain original
input bytes as `matrix.fasta` or `matrix.nex`. Inclusion remains explicit and
also governs original copies saved by predictive checks.

Validation uses synthetic fixtures in `fixtures/validation/dna`, independent
summation of internal states under the analytical JC69 probabilities, six fixed
states from BEAST.base 2.7.8, simulation frequencies and exact checkpoint replay.
With identical category rates, absolute log-likelihood differences are below
1e-10. With independently computed gamma quantiles, the test tolerance is
1e-7; BEAST's AS91 approximation differs by less than 3e-8 in category rates
on these states. This is numerical validation, not a speed comparison.
