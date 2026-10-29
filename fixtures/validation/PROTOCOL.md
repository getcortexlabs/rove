# Full-model inference comparison

This protocol is fixed before examining the production runs. Short, 1,000-step
smoke runs use seed 17 only to check file formats and fixed-state calculations;
they do not contribute posterior summaries or timing claims.

## Cases and models

All six cases use binary Mk, four normalized gamma midpoint categories, an
explicit lognormal relaxed clock, and the birth–death model and priors of the
supplied BEAST 2.7.8 XML. T1–T19 are constrained in both engines. Its support
is reported separately and never counted as independent recovery.

| Case | Input | Conditioning |
| --- | --- | --- |
| reference-mkv | Supplied 20 × 50 matrix | Mkv |
| reference-mk | Same supplied matrix | Unconditioned Mk |
| control | Existing generated 20 × 50 control, distinct known topology | Mk |
| homoplasy | Generated control with ten cell flips, seed 7 | Mk |
| missing | Generated control with missing-cell probability 0.2, seed 1 | Mk |
| reduced | Generated control without the T1–T4 synapomorphies, 47 characters | Mk |

The latter four follow the examples already recorded in `fixtures/synthetic`.
Their exact generation commands, modifications, character/pattern counts and
hashes are archived. Missing cells may hide the only observed state change in
a column. Using unconditioned Mk for all four controls avoids silently dropping
these columns or applying an unsupported ascertainment scheme. These are
constructed test inputs, not draws from the fitted model, and they are not a
calibration study of Bayesian coverage. No minimum clade support is required.

BEAST keeps the supplied ORC adaptive mixtures, Bactrian, epoch and tree-stretch
proposals. Rove uses its twelve classic tree/parameter moves and scale moves,
with its existing square-root tuning schedule. Both tune during execution;
their proposal distributions and starting trees differ. Initial scalar
parameters, priors, constraints and observations agree. Extra BEAST loggers
record prior components, tree length and the sum of explicit branch rates.
The arithmetic mean of those rates is compared with Rove's `mean_branch_rate`,
not with BEAST's branch-length-weighted `RateStatistic.mean`.

## Runs and numerical gate

Fixed-state agreement must pass first: both archived reference states and one
new BEAST smoke-run state per case, evaluated by standalone Rove. Test each
prior component and the likelihood separately. With the same imported gamma
categories, likelihoods must agree within 1e-9; with independently generated
categories, likelihood and posterior use 2e-7, categories use 3e-8, and prior
components use 1e-10. The smoke states exposed amplification of BEAST's legacy
quantile approximation: the largest likelihood difference was 8.16e-8 when
categories were generated and 1.14e-13 when imported. This was diagnosed and
the tolerances frozen before any production posterior comparison.

Run four independent chains per engine and case, seeds 20261110–20261113,
10 million steps each, sampling every 1,000 steps. Discard the first 10% of
samples. Equal seed labels do not imply equal random numbers or trajectories.
All chains run sequentially; alternate which engine runs first for each seed.
Use a release build from a clean commit. Record executable and input hashes,
compiler/JVM/package versions, hardware, flags, power/load observations,
commands and completion checks. BEAST uses its Java likelihood core, one
thread and a fresh JVM; Rove uses its default backend. Include startup,
JVM warm-up, adaptation and logging in process wall time, excluding subsequent
diagnostics. No other benchmark chains run concurrently.

## Criteria

- Compare posterior, likelihood, total prior, birth–death density, both
  birth–death parameters, tree height/length, gamma shape, clock sigma, the
  three rate-related prior components and the arithmetic branch-rate mean.
- For each engine, pool equal-length chains by averaging chain means. Estimate
  the Monte Carlo variance of that mean as the larger of `sum(sd²/ESS)/m²`
  and the variance of the chain means divided by `m`, where `m=4`. ESS uses
  the same BEAST estimator in both engines. Require the engine means to differ
  by at most four combined Monte Carlo standard errors.
- Check rank-normalized split R-hat, including folded ranks, for each continuous
  statistic. Require R-hat ≤ 1.01, each chain's ESS ≥ 100, and total ESS ≥ 400.
  These are diagnostic gates, not proof of convergence or universal cutoffs.
  Report all failures and null diagnostics; never count undefined ESS as passing.
- For every unconstrained clade with pooled frequency ≥ 0.01 in either engine,
  require the absolute frequency difference ≤ 0.05. Report Monte Carlo uncertainty
  using 20 contiguous equal-size batches per chain, with a between-chain variance
  safeguard as above. Report discarded remainder samples for this batch estimate.
  Require each engine's pooled clade MCSE ≤ 0.0125. A constant indicator has no
  measurable mixing ESS; a frequency of zero or one is not a convergence test.
- Report reference-clade support, distinct sampled topologies, reference-topology
  frequency, and rooted Robinson–Foulds distance to the known tree. For nonconstant
  tree diagnostics, compare means by four combined MCSE and report R-hat and ESS.
  Apply the convergence gates when every chain varies. If any chain is constant,
  mark the mixing diagnostic as limited rather than fabricating an ESS or counting
  it as a pass. Report constant diagnostics without interpreting them as evidence
  of topology exploration; the mean and clade-agreement checks still apply.
- Report posterior standard deviations and 95% HPD intervals, chain variability,
  wall times and ESS per second for every compared continuous statistic. Timing
  ratios apply only to these data, models, operators, hardware and logging settings.

If a case fails a convergence/precision gate, run a new four-chain batch of
40 million steps in both engines with seeds 20261210–20261213. If needed, use
160 million steps with seeds 20261310–20261313. Keep all earlier results and
report the extension; do not pool different run lengths. If well-resolved means
or clades disagree, investigate the model/numerics/proposals before rerunning.
No thresholds are changed after observing a failure. Remaining failures must be
reported as unresolved rather than treating a completed process as validation.

The R-hat method follows [Vehtari et al. (2021)](https://doi.org/10.1214/20-BA1221).
Full-inference efficiency contributes to #8; it does not establish BEAGLE,
BEAST X, large-matrix or cross-hardware performance.
