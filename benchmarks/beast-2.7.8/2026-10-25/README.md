# BEAST 2.7.8 reference: 20 taxa × 50 characters

Two independent 10-million-step runs completed with the supplied XML unchanged.
Both retained the known topology in 9,000 of 9,001 post-burn-in tree samples.
This establishes a numerical and sampling reference for Rove; it is not a
comparison with Rove or BEAST X.

## Environment and timing

MacBook Pro, Apple M4 Max (12 performance and 4 efficiency cores), 64 GB RAM,
macOS 26.6.2. Eclipse Temurin OpenJDK 17.0.15+6 for aarch64. Each chain ran
sequentially in a fresh JVM with one BEAST worker thread, `-Xms256m -Xmx2g
-Xss8m`, and output redirected to a file. The selected likelihood backend
was `BeerLikelihoodCore`; the native BEAGLE library was unavailable.

The headless entry point was `beastfx.app.beast.BeastMCMC`. It enables BEAST's
consistency checks (`beast.debug=true`). These are validation timings under
the recorded configuration, not a tuned benchmark of the fastest BEAST setup.
No cores were pinned, and background system activity was not controlled.

| Seed | Wall time, including JVM startup | BEAST calculation time | Posterior ESS | Posterior ESS / wall second | Smallest nonconstant logged ESS |
| --- | ---: | ---: | ---: | ---: | ---: |
| 20261025 | 54.181 s | 53.984 s | 582.40 | 10.75 | 329.43 (gamma shape) |
| 20261026 | 57.803 s | 57.271 s | 380.05 | 6.57 | 245.07 (birth-death log density) |

The runs logged every 1,000 steps and saved state every 5,000 steps. We
discarded the first 10% of logged samples: 1,000 of 10,001, retaining 9,001.
ESS values use BEAST.app 2.7.8 `LogAnalyser` and its autocorrelation estimator.
ESS per second divides the retained trace's ESS by the complete wall time,
including burn-in and startup.

The largest rank-normalized split R-hat, including folded ranks, was
**1.00323** for relative death rate. The implementation follows
[Vehtari et al. (2021)](https://doi.org/10.1214/20-BA1221).
Constant traces are reported as undefined, not as evidence of convergence.
These diagnostics support agreement for this example; they do not prove
convergence for every possible feature of the posterior.

## Model details

The supplied [input files](../../../fixtures/reference/20x50) are archived
with their original hashes. They are distinct from the earlier generated
control fixture.

- Binary, equal-frequency Lewis Mk; 50 observed characters, 23 observed
  patterns; two additional zero-weight constant patterns for Mkv.
- Four discrete gamma categories; shape prior Exponential(mean = 1).
- The mutation-rate parameter stays at 1. Its only proposal exchanges a
  fixed total within a one-dimensional vector, so the proposal has no effect.
- Lognormal relaxed clock with 38 explicit branch rates. The lognormal mean
  in real space is 1; its log-scale standard deviation has Gamma(shape = 5,
  scale = 0.2). The clock-rate multiplier is fixed at 1. Realized mean branch
  rates can vary.
- Gernhard birth-death prior with `type=unscaled`, sampling proportion 1,
  no origin-height input, and `conditionalOnRoot=false`. The parameter named
  `BDBirthRate` represents **lambda − mu**, with Uniform(0, 1000); `BDDeathRate`
  represents **mu / lambda**, with Uniform(0, 1). The names alone should not
  be interpreted as the separate birth and death rates.
- The initial tree is random, subject to the T1–T19 monophyly constraint.
  No fossil ages or calendar-time calibrations are present.

All proposal weights and priors were preserved. BEAST reports that
`FixMeanMutationRatesOperator` has no effect with this one-dimensional input.
Its weight is 2 of 117.1, about 1.71% of scheduled proposals. Changing it
would define a separate tuning experiment, not reproduce this configuration.

## Tree diagnostics

The T1–T19 ingroup is constrained and is reported separately from the 17 free
reference clades. Its support of 1 is not recovery from the observations.

Each run sampled two distinct rooted topologies after burn-in. The reference
topology had probability **0.9998889** in each run. All 17 free reference
clades had support at least **0.9998889**:

- Seed 20261025: T13–T19 was absent in one retained tree.
- Seed 20261026: T9–T19 was absent in one retained tree.
- Every other reference clade appeared in every retained tree in that run.

The maximum clade-frequency difference between runs, across all observed
clades, was 0.0001111. The mean rooted Robinson–Foulds distance to the known
tree was 0.0002222 in each run. This distance counts clades present in one
tree but absent from the other. Constant clade indicators have undefined ESS;
we do not interpret that as unlimited sampling information.

This is an unusually clean, small synthetic dataset with a rooting constraint.
The results and times cannot be extrapolated to a 90 × 250 matrix, missing
data, conflicting characters, different priors, or unconstrained rooting.

## Fixed-state numerical reference

The two `snapshot-*.json` files restore the final saved states, recompute the
posterior, and export node heights, branch rates, parameters, category rates
and weights, state patterns, unconditioned pattern likelihoods, the Mkv
correction, and individual prior contributions.

| Seed | Mkv log likelihood | Log prior | Log posterior |
| --- | ---: | ---: | ---: |
| 20261025 | -198.61115564792456 | 89.17329973057747 | -109.43785591734710 |
| 20261026 | -200.89301622575100 | 96.34161516003118 | -104.55140106571983 |

An independent Python pruning implementation reconstructed these likelihoods
with total absolute errors of 1.76 × 10⁻¹² and 1.11 × 10⁻¹². Maximum
per-pattern errors were 8.17 × 10⁻¹⁴ and 1.31 × 10⁻¹². The restored posteriors
also agree with the last trace samples within 10⁻⁹. These exported states
are intended for later Rust equality tests; the current Rust engine does not
yet implement the complete model.

## Reproduce

Use a JDK 17 and extract these packages into separate directories under a
local `packages/` directory, with `version.xml` and `lib/` in each package:

- [BEAST.base and BEAST.app 2.7.8](https://github.com/CompEvol/beast2/releases/tag/v2.7.8)
- [MM 1.2.1](https://github.com/CompEvol/morph-models/releases/download/v1.2.0/MM.addon.v1.2.1.zip)
- [ORC 1.2.1](https://github.com/jordandouglas/ORC/releases/download/v1.2.1/ORC.addon.v1.2.1.zip)
- [BEASTLabs 2.0.3](https://github.com/BEAST2-Dev/BEASTLabs/releases/download/v2.0.0/BEASTlabs.package.v2.0.3.zip)

Use `launcher.jar` from the BEAST 2.7.7 distribution; the 2.7.8 release
updates the base and app packages. `metadata.json` records JAR hashes,
settings, output hashes, and timings. The runner uses the headless MCMC
entry point, so JavaFX is not needed. The Java helper belongs to the reference
tooling and is not a dependency of the Rove crate.

From the repository root, using an output directory that does not yet exist:

```sh
python3 tools/beast/run_reference.py \
  --packages /path/to/packages \
  --launcher /path/to/launcher.jar \
  --xml fixtures/reference/20x50/analysis.xml \
  --truth fixtures/reference/20x50/truth.newick \
  --output /path/to/new-reference-run

python3 tools/beast/compare_runs.py \
  /path/to/new-reference-run/seed-20261025/analysis.log \
  /path/to/new-reference-run/seed-20261026/analysis.log

python3 tools/beast/check_snapshot.py \
  /path/to/new-reference-run/seed-20261025/snapshot.json \
  /path/to/new-reference-run/seed-20261026/snapshot.json

python3 -m unittest discover -s tools/beast -p 'test_*.py'
```

The runner checks versions, preserves the input XML, runs chains sequentially,
rejects incomplete output even when BEAST exits with status zero, and exports
diagnostics after the timed runs. Original traces, trees, checkpoints, and
console logs remain local; only compact results and checksums are versioned.
