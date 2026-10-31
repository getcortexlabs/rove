---
description: "Simulate morphological matrices from a completed Rove run and compare paired posterior predictive discrepancies."
---

# Posterior predictive checks

`rove predict` asks whether a fitted model can generate matrices with properties like the observed data. It uses posterior trees and parameters, generates new characters, and compares the original and simulated matrix on the **same sampled tree**. This checks aspects of model adequacy; it does not establish convergence or give the probability that a model is true. The [RevBayes morphology tutorial](https://revbayes.github.io/tutorials/pps_morpho/) introduces the approach.

This feature is available through the Rust CLI and, for completed runs, from History in the [Mac app](./macos-preview#posterior-predictive-checks). No BEAST, Java, R or RevBayes installation is required.

Dated runs are also supported: predictions use the sampled tip ages, global clock rate and relative branch rates. See [fossil tip dating](./tip-dating).

## Run a check

First inspect sampling and convergence diagnostics. Then supply a completed run directory containing `run.json`, `trace.log` and `trees.nexus`, plus its exact original NEXUS input:

```sh
cargo run --release --bin rove -- predict runs/seed-1 \
  --matrix fixtures/reference/20x50/matrix.nex \
  --output runs/prediction-1 --burnin-percent 25 \
  --replicates 100 --seed 42
```

Use the matrix actually fitted, including any taxon selection made by the app. Its SHA-256 must match the run record. For parallel analyses, select one completed cold-chain run directory; this command does not pool chains. Existing output directories, prior-only runs, incomplete files and mismatched trace/tree states are rejected. Source files are copied into the new output directory and remain unchanged.

Burn-in discards `floor(total samples × percent / 100)` rows, including `STATE_0` in the total. Each replicate draws a retained posterior row uniformly **with replacement** and receives a recorded simulation seed. Repeated rows are expected. The fitted likelihood is reconstructed for each selected row before simulation, checking the pairing of its topology, branch rates and gamma shape.

## Model and observation rules

- Preserve each character's fitted alphabet and ordering, even when a replicate lacks some possible states. Ordered characters retain intermediate states. Do not re-infer alphabets from simulated observations.
- Use the fitted strict clock or explicit relaxed branch rates and the fitted four-category gamma mixture where present.
- For Mkv and Mkparsinf, reject excluded **complete latent characters**, then restore the original missing/gap mask. Redraw the gamma category on every attempt. A masked replicate may appear constant even though its latent character was variable; this is intentional.
- Support single states, missing cells and gaps explicitly treated as missing. Uncertainty sets and polymorphism are rejected because there is no generative observation model for them. Ordered Mkparsinf remains unsupported, as in inference.

Rare conditioning events can exceed the default 100,000 attempts per character. The command fails without publishing a partial result; `--max-attempts` can raise that limit to 10,000,000. Owner-pipe cancellation (`--cancel-on-stdin-close`) also removes partial output. Killing the process can leave an incomplete directory; only a complete `predictive.json` marks a finished check. Replicate counts and output/workspace sizes are bounded.

## Read the result

`predictive.json` records model settings, input/build fingerprints, burn-in, seeds, frozen character alphabets and global/per-character tail estimates. `replicates.jsonl` links each matrix to its source state and seed, with paired statistics. `characters.tsv` contains per-character paired scores. `matrices/` contains readable NEXUS replicates; their alphabets must be interpreted using the report's `frozen_character_sets`.

Parsimony uses Sankoff scores on each posterior tree, not a search for a new optimal topology. Unordered changes cost one; ordered changes cost `|i − j|`. For each character, `s` is its tree score, `m` the minimum possible score and `g` its maximum possible score. Global consistency is `Σm / Σs`; retention is `(Σg − Σs) / (Σg − Σm)`. Zero denominators are undefined, not zero. The informative count uses `g > m`, including the effect of ordered costs. Missing observations have no change cost.

Per-character fit also includes **negative unconditioned log pattern likelihood** at the paired posterior draw, using the original missing mask. This is a discrepancy statistic, not a conditional predictive density. Ascertainment is enforced during simulation.

`p_upper` is the fraction of valid pairs where the simulated statistic is at least the observed statistic; `p_lower` uses at most. Ties count in both tails and are reported separately. Undefined CI/RI pairs are counted and excluded. Small tail fractions identify a discrepancy to investigate; no automatic pass/fail threshold is applied. Per-character checks are exploratory and have no multiple-testing adjustment.

The 95% Wilson intervals describe simulation error **conditional on the finite source posterior**. They do not include MCMC error. Increasing replicates cannot fix poor posterior sampling, and a non-extreme result does not prove model adequacy.

## Validation

Tests compare simulated pattern frequencies with independent closed-form probabilities on four-tip trees, including conditioned gamma mixtures and missing masks. Exhaustive ancestral-state and quartet-topology enumeration checks parsimony, consistency, retention and pattern likelihoods for ordered and unordered characters. CLI tests cover both clock presets, all supported conditioning modes, reproducibility, source-file mismatch, cancellation and rejection of partial results.
