---
description: "Combine morphological characters and aligned DNA with explicit sources, taxon matching and relative rates."
---

# Morphology and DNA together

Choose **Combine morphology and DNA** on the Mac welcome screen. Select one
STANDARD morphological NEXUS matrix and one aligned DNA NEXUS/FASTA file, review
the missing-taxon policy, and save a new `.mixed.json` input. The same operation
is available in the CLI:

```sh
rove combine --morphology morphology.nex --dna alignment.fasta --missing-taxa reject --output study.mixed.json
rove sample study.mixed.json --output combined-run --seed 42 --steps 30000000 --sample-every 3000 --model reference --morphology-rate estimate --morphology-rate-log-sd 1
```

The manifest embeds both original UTF-8 texts and their SHA-256 fingerprints;
it never follows external source paths. It can be reopened after the originals
move. Existing output files are not overwritten. The format/version, source
types and hashes are checked on every import. The file contains research data;
keep it private unless sharing the two original datasets is permitted.

Taxa match by **exact normalized name**, not row position. Existing NEXUS/FASTA
underscore-to-space normalization applies. The default `reject` requires the
same taxa. Explicit `--missing-taxa missing` keeps their union and fills cells
for absent taxa with missing data; the app lists these taxa for each source.
At least two taxa must occur in both blocks. Case-only name disagreements are
rejected. Review other spelling differences before treating them as biological
absences. Missing cells contribute no state information; gaps retain their
separate `--gaps reject|missing` policy. No sequences are aligned automatically.

Each block retains its own alphabet, character order, labels and observations.
Morphology supports observed, declared or complete labeled alphabets, ordered
and unordered models, and `--conditioning none|variable|informative`. These
choices apply **only to morphology**. DNA uses unconditioned JC69, all four
A/C/G/T states and all sites, including constants. IUPAC ambiguity is a mask of
possible bases. Unsupported true polymorphism still rejects inference.

Both blocks share topology, branch times, the strict or lognormal branch clock,
and the gamma-shape parameter when enabled. Categories are integrated separately
at each site. Independent block clocks or separate gamma shapes are not offered.
DNA fixes the relative rate at 1. Every morphological partition uses a common
positive multiplier `r`, so its branch distance is `time × branch-rate × r`.
The DNA block uses `time × branch-rate`. This fixes the relative-rate scale;
it does not turn undated branch lengths into calendar ages. Fossil dating retains
its explicit global clock and calibration settings.

Choose the rate model explicitly in the app or CLI:

- `--morphology-rate 2.5` fixes the ratio to that positive value.
- `--morphology-rate estimate --morphology-rate-log-sd s` estimates it with
  `log(r) ~ Normal(0, s²)`. The default log-SD is 1 when omitted; the median is 1
  and the mean is `exp(s²/2)`. This assumption needs sensitivity assessment.

The normalized log prior is
`−log(r) − log(s) − log(2π)/2 − (log(r)/s)²/2`.
A scale proposal uses a factor `f ~ Uniform(a, 1/a)` and log Hastings ratio
`−log(f)`. The `morphology_rate_scale` move defaults to weight 3 for estimation
and is unavailable for a fixed ratio. Its state is recorded in `trace.log` and
checkpoints. Likelihood equals the sum of the two blocks' log likelihoods,
including the selected morphological ascertainment correction. Priors are
counted once; the relative-rate prior is normalized for marginal estimation.

Format-12 app setups preserve source provenance, taxon policy and rate choices.
Review, History, CLI import, comparisons, resume and marginal estimates verify
that identity. Predictive checks reconstruct the sampled ratio and emit typed
`.mixed.json` replicates, retaining per-block alphabets and missing masks.
Prediction still rejects uncertainty/polymorphism until a suitable observation
model exists. Matrix inclusion in reproducibility exports is explicit and also
covers original copies kept by predictive checks. Exported simulations are
synthetic; they do not replace the original observations.

Validation includes 36 fixed states from BEAST.base 2.7.8 + MM 1.2.1, independent
partition sums, normalized-prior quadrature and stationary sampling, constraints,
cache recomputation, exact checkpoint replay, prediction and native execution.
The reference likelihood tolerance is 1e-7, including independent gamma-quantile
approximations. Fixtures and generator are in `fixtures/validation/mixed` and
`tools/beast/MixedStates.java`. These are correctness checks, not convergence
certification or comparative speed measurements.
