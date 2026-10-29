# Full-model inference against BEAST 2.7.8

Rove and BEAST 2.7.8 produced compatible posterior summaries and clade
frequencies in six constructed, 20-taxon cases under criteria fixed before
the production runs. Five cases passed with four chains of 10 million steps
per engine. The supplied Mkv case required a new batch of 40 million steps
because some initial BEAST chains had low ESS for the log-posterior density
and tree prior. Both batches are retained; the longer batch supplies the final
Mkv comparison.

In the five unconditioned Mk cases, BEAST's median wall time was 2.84–3.30
times Rove's. Rove's median log-posterior-density ESS/second was 0.35–0.49
of BEAST's, and clock-sigma ESS/second was 0.42–0.51. Gamma-shape and
log-likelihood ESS/second favored Rove.
The tables below separate these measurements. They compare the current Rove
moves with the supplied ORC configuration, not identical proposal schedules.

## Inputs and model

| Case | Observed characters | Unique observed patterns | Missing cells | Construction |
| --- | ---: | ---: | ---: | --- |
| reference-mkv | 50 | 23 | 0 | Supplied matrix, with Mkv |
| reference-mk | 50 | 23 | 0 | Same supplied matrix, without Mkv |
| control | 50 | 23 | 0 | Existing generated control, distinct known topology |
| homoplasy | 50 | 33 | 0 | Control with ten recorded cell flips |
| missing | 50 | 50 | 180 | Control with missing-cell probability 0.2 |
| reduced | 47 | 22 | 0 | Control without the three T1–T4 synapomorphies |

The [inputs](../../../fixtures/validation/README.md) preserve every sequence,
known tree, XML and generation command. The two constant Mkv patterns are
likelihood workspace, not observed characters. The homoplasy matrix requires
60 changes on its known tree, versus 50 in the control; ten columns require
more than one change. Missing observations hide the only observed change in
two columns. All four generated-control cases therefore use unconditioned Mk,
which keeps these columns without applying an unsupported ascertainment rule.
`character-checks.json` records those calculations.

All cases use binary equal-frequency Mk, four normalized midpoint gamma
categories, explicit independent lognormal branch rates with real-space
mean one, and the supplied Gernhard birth–death prior. Gamma shape has its
Exponential(1) prior and effective [0.001, 1000] bounds; clock sigma has
Gamma(shape=5, scale=0.2). The birth–death parameters have the supplied uniform
priors. Mutation and clock multipliers stay at one. Realized branch rates are
not normalized. The [model guide](../../../docs/guide/reference-model.md)
records the full parameterization.

T1–T19 is constrained in every run. Its support of one is imposed, and it is
separate from the other 17 known clades in each comparison. The known trees
are construction references, not starting trees or posterior constraints on
those other clades. These matrices were constructed with synapomorphies;
they are not simulations from the complete fitted model or a coverage study.

## Protocol and numerical checks

The [protocol](../../../fixtures/validation/PROTOCOL.md), inputs and analysis
tools were committed in [`9f779b0`](https://github.com/getcortexlabs/rove/commit/9f779b0ce5ec707e9a4fb5a8bc9d4994d4e2b0b8)
before the production chains. Initial seeds were 20261110–20261113; the
40-million-step batch used 20261210–20261213. Each chain sampled every 1,000
steps and discarded the first 10%, retaining 9,001 or 36,001 paired tree/trace
samples. All chains ran sequentially, alternating which engine ran first.

Mean differences must be within four combined Monte Carlo standard errors.
Each engine's pooled MCSE uses the larger of its within-chain ESS estimate
and the variation between chain means. Continuous diagnostics require
rank-normalized split R-hat, including folded ranks, at most 1.01, ESS at
least 100 per chain and at least 400 in total. These are diagnostic gates,
not a proof of global convergence. Clade frequencies must differ by at most
0.05 for every unconstrained clade reaching 0.01 in either engine, with each
engine's clade MCSE at most 0.0125. Clade uncertainty uses 20 contiguous
batches per chain and a between-chain safeguard. One remainder sample per
chain is omitted from the batch estimate, but retained in the frequency.

Fixed-state agreement preceded these comparisons. The two original reference
states were already covered by the rate-model tests. Six additional states
came from 1,000-step BEAST smoke runs with seed 17, which do not contribute
posterior estimates. With the same imported gamma categories, the maximum
likelihood difference was 1.14e-13. With independently calculated categories,
it was 8.16e-8, attributable to BEAST's legacy quantile approximation: the
category differences were below 1e-8. Tests separately bound imported-category
likelihoods at 1e-9, generated-category likelihoods/posteriors at 2e-7, category
rates at 3e-8 and prior components at 1e-10. The short-state diagnosis and
bounds were fixed before production.

The first Mkv batch passed all mean, tree-mean and clade comparisons, but
BEAST tree-prior ESS was 88.2 and 41.8 in its first two chains; log-posterior and
total-prior ESS also fell below 100 in the second chain. The predeclared
extension rule therefore required new, longer chains in **both** engines.
No criterion changed and the earlier results were not discarded.

## Agreement and convergence

| Case | Steps per chain | Largest absolute mean z | Largest clade difference | Largest continuous R-hat | Lowest continuous ESS | Criteria |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| reference-mkv | 40,000,000 | 1.313 | 0.00001 | 1.00107 | 358.9 | Pass |
| reference-mk | 10,000,000 | 1.489 | 0.00108 | 1.00398 | 333.3 | Pass |
| control | 10,000,000 | 1.270 | 0.00244 | 1.00265 | 281.5 | Pass |
| homoplasy | 10,000,000 | 2.299 | 0.00161 | 1.00145 | 262.6 | Pass |
| missing | 10,000,000 | 3.200 | 0.00536 | 1.00231 | 304.9 | Pass |
| reduced | 10,000,000 | 1.651 | 0.00350 | 1.00140 | 394.9 | Pass |

These are the last batches required by the protocol. Earlier batches remain
archived below; batches of different lengths are not pooled. The clade maximum
excludes the imposed T1–T19 group. Each case has four chains per engine.

| Batch | Means agree | Clades agree | Tree means agree | Convergence gates | Clade precision |
| --- | --- | --- | --- | --- | --- |
| reference-mkv, 10M | yes | yes | yes | no | yes |
| reference-mkv, 40M | yes | yes | yes | yes | yes |
| reference-mk, 10M | yes | yes | yes | yes | yes |
| control, 10M | yes | yes | yes | yes | yes |
| homoplasy, 10M | yes | yes | yes | yes | yes |
| missing, 10M | yes | yes | yes | yes | yes |
| reduced, 10M | yes | yes | yes | yes | yes |

## Trees

| Case | Distinct topologies per chain, Rove | BEAST | Mean rooted RF, Rove | BEAST | Reference topology frequency, Rove | BEAST |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| reference-mkv | 2–6 | 2–4 | 0.0002 | 0.0001 | 0.99991 | 0.99993 |
| reference-mk | 137–158 | 138–158 | 0.4884 | 0.4901 | 0.77719 | 0.77675 |
| control | 157–169 | 148–167 | 0.4813 | 0.4657 | 0.78289 | 0.78925 |
| homoplasy | 301–311 | 285–308 | 0.7811 | 0.7736 | 0.67398 | 0.67412 |
| missing | 1119–1186 | 1138–1164 | 2.4972 | 2.4988 | 0.26622 | 0.26355 |
| reduced | 256–275 | 252–257 | 1.2414 | 1.2339 | 0.49714 | 0.49786 |

A constant topology indicator has no measurable mixing ESS. The comparison
files flag tree diagnostics with any constant chain as limited; they are
not counted as convergence evidence. All chains obeyed the imposed ingroup
constraint. Unconstrained reference clades and their Monte Carlo errors are
listed separately in each comparison file.

## Runtime and sampling efficiency

| Case | Wall seconds per chain, Rove | BEAST | BEAST/Rove median wall time |
| --- | ---: | ---: | ---: |
| reference-mkv | 67.78–68.34 | 211.06–215.28 | 3.13 |
| reference-mk | 15.76–16.28 | 51.86–53.82 | 3.30 |
| control | 15.86–16.15 | 47.87–49.14 | 3.05 |
| homoplasy | 20.11–20.52 | 58.65–59.49 | 2.92 |
| missing | 25.68–26.22 | 71.42–77.29 | 2.84 |
| reduced | 15.74–16.05 | 46.78–47.27 | 2.98 |

The following ratios divide median ESS/second in Rove by median ESS/second
in BEAST. Values above one favor Rove for that statistic. They include the
cost of the different proposal schedules and tuning rules, not just pruning.

| Case | Log-posterior density | Log likelihood | Gamma shape | Clock sigma | Tree height |
| --- | ---: | ---: | ---: | ---: | ---: |
| reference-mkv | 9.27 | 2.77 | 3.39 | 0.40 | 9.32 |
| reference-mk | 0.38 | 3.08 | 2.73 | 0.42 | 1.29 |
| control | 0.37 | 2.55 | 3.11 | 0.42 | 1.05 |
| homoplasy | 0.35 | 2.15 | 3.12 | 0.44 | 1.13 |
| missing | 0.49 | 1.87 | 3.32 | 0.47 | 0.94 |
| reduced | 0.44 | 1.96 | 3.09 | 0.51 | 0.91 |

Per-chain ESS, wall times, ESS/second and their variation remain in the JSON
files. A faster chain can have lower ESS/second for a parameter that it
explores less efficiently.

## Reading the results

Removing the T1–T4 synapomorphies reduced that clade's support from 0.9944
(Rove) and 0.9953 (BEAST) to 0.6418 and 0.6395. The reduced-case MCSEs were
0.0028 and 0.0029. Agreement here means representing uncertainty consistently;
it does not require every known clade to clear a support threshold. Missing
cells expanded the sampled topology space to over 1,100 distinct trees per
chain, while the supplied Mkv case barely moved away from the known topology.
The latter is a weak test of topology mixing by itself.

The efficiency gap in the five unconditioned cases makes proposal quality a
separate target from kernel cost. Rove uses twelve tree and parameter moves,
with square-root adaptation towards acceptance 0.234. BEAST uses
the XML's ORC adaptive mixtures, Bactrian proposals, coupled time/rate moves,
epoch and tree-stretch moves, with its own adaptation. Operator counts, final
tuning and learned mixture weights are archived for each chain. These results
do not isolate which proposal accounts for the gap; that requires a controlled
operator comparison, tracked in [#27](https://github.com/getcortexlabs/rove/issues/27).

The [BEAST adaptive sampler](https://github.com/CompEvol/beast2/blob/v2.7.8/src/beast/base/evolution/operator/AdaptableOperatorSampler.java)
uses measured proposal runtimes when learning mixture weights. Repeating a
seed and configuration can therefore change the BEAST trajectory even on the
same hardware. Reproduction here concerns the configurations and statistical
comparison, not byte-identical traces.

## Hardware and timing

The machine was a MacBook Pro with Apple M4 Max (12 performance and four
efficiency cores), 64 GB memory and macOS 26.6.2, on AC power. Both engines
used one likelihood thread, with no concurrent benchmark chains. JVM support
threads retained their defaults. Metadata records the exact Rust, Java and
package versions, commands, source commit, build settings, executable/JAR
hashes and power/load observations. Rust was 1.99.1 and Java was Temurin
17.0.15+6. BEAST.base and BEAST.app were 2.7.8, MM and ORC 1.2.1, and
BEASTLabs 2.0.3.

Rove used its NEON backend, `target-cpu=native`, optimization level 3, fat LTO
and one codegen unit. BEAST used `BeerLikelihoodCore` in a fresh Java process
with a 256 MB initial heap, 2 GB maximum heap and an 8 MB stack. Wall time
includes startup, JVM warm-up, tuning, logging and state storage. It excludes
compilation and the subsequent diagnostic exports. The parameter and tree
logging interval was 1,000 steps in both engines. BEAST also retained the
supplied checkpoint interval of 5,000 steps and its extra log columns; Rove
does not yet support checkpointing. BEAST makes one extra proposal at its
sample-zero iteration, as recorded in the operator counts.

These measurements cover the recorded configurations on one machine. CPU
frequency and affinity were not fixed, and unrelated background work was not
disabled. Thermal status was unavailable through the system API; the metadata
preserves that result rather than assuming a thermal state. The cases are
small and related, and proposals differ. The times are not pure kernel or
language speedups, nor results for BEAGLE, BEAST X, larger matrices or an M1.
[Issue #8](https://github.com/getcortexlabs/rove/issues/8) retains the broader
benchmarking work.

## Reproduce

Install the pinned packages as in the
[BEAST reference report](../../beast-2.7.8/2026-10-25/README.md#reproduce).
From a clean checkout, use a new output directory for each case:

```sh
python3 tools/beast/inference.py --case reference-mkv \
  --packages /path/to/packages --launcher /path/to/launcher.jar \
  --output /path/to/initial/reference-mkv
python3 tools/beast/compare_inference.py /path/to/initial/reference-mkv
```

Repeat with `reference-mk`, `control`, `homoplasy`, `missing` and `reduced`.
The driver builds the release binary with recorded flags. The recorded Mkv
extension is:

```sh
python3 tools/beast/inference.py --case reference-mkv \
  --steps 40000000 --seeds 20261210 20261211 20261212 20261213 \
  --packages /path/to/packages --launcher /path/to/launcher.jar \
  --output /path/to/extension/reference-mkv
python3 tools/beast/compare_inference.py /path/to/extension/reference-mkv
```

Archive completed comparisons with `tools/beast/report_inference.py --runs`
followed by the six initial case directories and the extension directory,
and `--output` pointing to a new report directory. It regenerates the compact
JSON, TSV files, character checks and `tables.md`. This README adds the model,
interpretation, reproduction instructions and scope to those generated tables.

## Files

- Each `case/10m` or `case/40m` directory contains `comparison.json`, per-chain
  `summaries.json`, provenance/timing `metadata.json` and `operators.json`.
- `estimates.tsv` contains final-batch posterior means, MCSEs and standardized
  differences. `chain-estimates.tsv` adds each chain's mean, posterior standard
  deviation, ESS and 95% HPD interval, with the arithmetic rate mean in common
  units. MCSE measures uncertainty in an estimated mean, not posterior spread.
- `character-checks.json` records minimum changes on the known trees.
- `archive.json` identifies the protocol, report-generator hash, analysis Python
  version, selected run lengths and final criteria outcome.

The raw traces, sampled trees and BEAST checkpoints remain in the local run
directories. Their hashes are archived; the input files, executable provenance,
commands and scripts allow the experiment to be repeated. Per-chain summaries
retain HPD intervals, all observed clade frequencies, batch frequencies and
reference-clade indicator ESS. Constant indicators are explicitly marked.
