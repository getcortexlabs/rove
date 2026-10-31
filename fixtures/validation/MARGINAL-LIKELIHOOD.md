# Marginal likelihood by power posteriors

Issue #24 adds `rove marginal`: a schedule of likelihood exponents from zero
to one, one chain per exponent, and the stepping-stone and path-sampling
estimates of the marginal likelihood with Monte Carlo standard errors. This
protocol is frozen before the comparison runs; its SHA-256 is recorded with
them.

## Estimators

The schedule follows BEAST 2's PathSampler: with `n` powers and shape `α`,
`β_i = (i / (n − 1))^(1/α)`, the quantiles of Beta(α, 1). Each power's chain
continues from the previous power's final state, from the posterior down to
the prior, and keeps its operator tuning.

Stepping stone (Xie et al. 2011) sums `log E_{β_{k−1}}[L^{β_k − β_{k−1}}]`
over adjacent powers, estimated from the samples at the lower power with the
largest log-likelihood factored out. Its standard error adds the delta-method
variances of the stone logarithms, each from the ESS of that stone's weights,
treating stones as independent. Path sampling (Lartillot and Philippe 2006)
applies the trapezoidal rule to the mean log-likelihood along the schedule;
its standard error adds the variances of those means. The trapezoid's
discretization error is not estimated, so the two estimates can differ by
more than their standard errors suggest.

## Checks before the comparison

- A conjugate Gaussian model with exact iid samples recovers
  `−½ log(1 + τ)` for both estimators, and about 95% of forty replicates fall
  within two reported standard errors.
- A three-taxon tree with fixed topology and root height and one free
  internal height has a one-dimensional posterior. The midpoint rule over the
  engine's own fixed-state densities gives the evidence; the stepping-stone
  estimate from 16 powers must agree within four standard errors, and path
  sampling within four standard errors plus 0.05.
- A chain at power zero reproduces the prior-sampling chain step by step
  while still evaluating the likelihood; powers outside [0, 1] are rejected;
  checkpoints record the live power.

## Comparison with BEAST 2.7.8

Configurations: the two strict-clock files already matched between the
engines, `benchmarks/strict-clock/2026-10-28/beast-mkv.xml` and
`beast-mk.xml`, on the supplied 20 × 50 matrix with the T1–T19 constraint,
fixed proposal steps and no tuning. BEAST runs MODEL_SELECTION 1.6.2's
PathSampler with `nrOfSteps=32`, `alpha=0.3`, `chainLength=1000000`,
`burnInPercentage=20`, `preBurnin=200000`, `posterior2prior=true`, one
thread, and the same packages as the earlier comparisons plus
MODEL_SELECTION. Rove runs `rove marginal` with `--powers 32 --alpha 0.3
--steps 1000000 --sample-every 1000 --burnin-percent 20` and the moves,
weights and fixed steps read from the XML by `strict_clock.py`. Three BEAST
seeds, 24101 to 24103, and three Rove seeds, 24001 to 24003, per
configuration. No run is extended, dropped or retuned after the results.

Both engines log the untempered likelihood at every 1,000th step of every
power. `tools/beast/marginal_likelihood.py` recomputes the stepping-stone and
path-sampling estimates from those logs with the burn-in above, using the
same formulas and the same ESS method as Rove, so the two engines are
compared by one implementation applied to both.

Decision, per configuration:

- Consistency: the recomputed stepping-stone value from Rove's traces equals
  `marginal.json` within 1e-6, and the recomputed value from BEAST's logs
  equals the estimate PathSampler prints within 1e-6.
- Agreement: every Rove–BEAST pair of stepping-stone estimates differs by at
  most four combined standard errors, `4 √(SE_Rove² + SE_BEAST²)`.
- Per power: report how many of the 32 mean log-likelihoods differ by more
  than four combined standard errors between the engines; more than two is a
  reported concern, not a gate.
- Report path sampling for both engines the same way, the difference between
  the two estimators, wall times and the per-power ESS. Wall time includes
  process launch and, for BEAST, 32 JVM starts; it is not a speed comparison.

A failed agreement gate keeps #24 open with the recorded results. Passing it
establishes agreement between implementations of the same estimator on one
small matrix, not model-selection performance on empirical data.
