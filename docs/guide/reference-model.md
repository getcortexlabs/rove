---
description: "The standalone model of the supplied BEAST XML: gamma categories, explicit lognormal branch rates, priors and numerical validation."
---

# The reference model

Aligned DNA in NEXUS and FASTA is also supported under [JC69](./dna)

`rove sample --model reference` runs the probability model of the supplied
20 × 50 XML without BEAST. It uses the same binary Mk model, Mkv conditioning
by default, birth–death prior and parameter conventions. The default
`--model strict` keeps the earlier one-category, strict-clock configuration.
Both presets accept `--conditioning none` for unconditioned Mk.

## Rates and priors

| Component | Reference preset |
| --- | --- |
| Character rates | Four gamma midpoint quantiles, each with weight 1/4, normalized to mean one |
| Gamma shape | Estimated; Exponential(mean=1), starting at 1; parameter bounds [0.001, 1000] |
| Branch rates | One explicit rate per non-root branch; independent LogNormal(μ=−σ²/2, σ), starting at 0.5; positive rates |
| Clock standard deviation σ | Standard deviation of log rates; Gamma(shape=5, **scale**=0.2), starting at 0.2 |
| Mutation and clock multipliers | Both fixed at 1 |
| Birth–death parameters | λ−μ ~ Uniform(0,1000), μ/λ ~ Uniform(0,1), starting at 1 and 0.5 |
| Tree prior | Gernhard, `unscaled`, all lineages sampled, conditioned on tip count, not root height |
| Monophyly | Supplied explicitly through `--monophyly`; the reference command constrains T1–T19 |

The gamma quantiles are at 1/8, 3/8, 5/8 and 7/8, not the means of four
probability bins. The shape bounds also apply when sampling the prior; as
in BEAST, its logged density does not add a truncation normalizer. Sigma is
the standard deviation of **log rates**, not the mean or the standard
deviation of rates in real space. The mean of the lognormal distribution
is one; the realized branch rates are not normalized to mean one.

The rate vector has `2 × taxa − 2` entries. A non-root node uses its index;
if the last node is not the root, it uses the root's index. This is BEAST's
explicit-rate mapping, including when a subtree slide changes the root.
The root itself has no branch rate. A branch's genetic distance in category
`c` is `height difference × branch rate × category rate[c]`.

BEAST clock initialization overrides the XML's branch-rate lower bound of
1e-100 with zero. Rove therefore accepts every positive finite rate; zero
has zero lognormal density.

## Run the supplied matrix

From the repository root:

```sh
cargo build --release --locked
sh fixtures/reference/20x50/rove.sh runs/reference-1 1
```

The script runs ten million steps, samples every thousand steps and applies
the T1–T19 constraint. The output directory must not exist. It is equivalent
to the command in [Sampling trees](./sampler), with `--model reference` added.
The constraint's support of one is imposed, not recovered from the data.

## Proposals and output

The nine classical moves in the sampling guide keep their weights. Three
scale moves are added: `gamma_shape_scale` (weight 0.1), `clock_sigma_scale`
(3) and `branch_rate_scale` (20), which chooses one rate uniformly. Each
starts with factor 0.5 and includes the scale move's Hastings ratio. They
use the same tuning schedule as the other scale moves.

The default schedule does not use ORC's adaptive operator mixtures, Bactrian
kernels, epoch or tree-stretch moves. Rove also omits the XML's
mutation-rate proposal, which has no effect in one dimension. Matching the
probability model does not establish equal mixing or ESS per second.

The trace adds `gamma_shape`, `gamma_prior`, `clock_sigma`, `clock_rate_prior`,
`clock_sigma_prior` and `mean_branch_rate` (the unweighted arithmetic mean).
Trees keep time lengths and annotate each branch with `[&rate=...]`; multiply
length by rate for its distance before the character-rate multiplier. Heights
are relative to the fixed clock multiplier, with no calendar-time calibration.
`run.json` records the preset, priors, initial values, rate mapping and final
rate parameters. It is not a checkpoint for resuming a chain.

## Joint proposals

Three additional moves are available through `--weight`; their default weight
is zero. They require the relaxed clock. They change the proposals, leaving
the likelihood, priors and parameter coordinates intact.

- `node_time_rates` draws a non-root internal height uniformly between its
  older child and parent. Each incident rate becomes `rate × old duration /
  new duration`, preserving branch distance. The log Hastings factor is the
  sum of `ln(old duration / new duration)` over its three branches.
- `root_time_rates` scales the root height by `f`, drawn uniformly from
  `[s, 1/s]` (initial `s = 0.75`), and compensates its two branch rates.
  Its log Hastings factor adds `−ln(f)` to their duration terms.
- `clock_sigma_rates` scales sigma by the same kind of multiplier (initial
  `s = 0.5`) while keeping each standardized log rate
  `z = (ln(rate) + σ²/2) / σ` fixed. With `d` branch rates, its log Hastings
  factor is `(d − 1) ln(f) + Σ ln(new rate / old rate)`.

These factors include the proposal's change of volume, not an extra prior.
The sigma transformation has rate Jacobian `f^d × Π(new rate / old rate)`;
the uniform multiplier contributes `1/f`. The inverse uses `1/f` and the old
height. Invalid values are rejected before mutation. Changed distances still
go through the existing cache update and rejection path.

The tests check inverse maps, numerical Jacobians, independently known target
distributions, root-index changes, exact restoration and backend agreement.
The time/rate and standardized-rate ideas are established relaxed-clock
methods, described by [Douglas, Zhang and Bouckaert (2021)](https://doi.org/10.1371/journal.pcbi.1008322);
this implementation does not reproduce the entire ORC package. The
[evaluation protocol](https://github.com/getcortexlabs/rove/blob/main/fixtures/validation/SAMPLING-EFFICIENCY.md)
separates candidate selection from four-chain validation and defines the
conditions for changing the default schedule.

## Validation

`tests/rates.rs` reconstructs both archived BEAST 2.7.8 states from their
heights, gamma shape, sigma and explicit branch rates. The largest absolute
log-likelihood difference is below 8e-10; every prior component agrees within
2e-12. The category rates are calculated by Rove, not imported from BEAST.

Rove independently inverts the gamma CDF using [its positive series](https://dlmf.nist.gov/8.7.E1)
and bracketed Newton steps in log space. BEAST uses its legacy AS91
approximation. The largest category-rate difference over the archived
20-shape grid is below 1.4e-8; this accounts for the looser likelihood
tolerance than in tests that import category rates. Exponential and Erlang
closed forms provide separate numerical checks.

Other tests check the hyperpriors by sampling without data, rate mapping at
different roots, cached versus fresh evaluations after every step, restoration
after rejection, identical samples on both backends and no allocations during
the sampled steps.

Posterior summaries and clade frequencies met predeclared criteria in six constructed
20-taxon cases, including homoplasy, missing data and reduced signal. The
[report](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md)
records every batch, including a longer Mkv batch required by the ESS gate, runtime and
ESS per second. Rove completed the five unconditioned Mk cases in less time, but BEAST
delivered more ESS per second for log-posterior density and clock sigma with the
classical Rove schedule.

The subsequent [joint-proposal evaluation](https://github.com/getcortexlabs/rove/blob/main/benchmarks/sampling/2026-10-29/README.md)
passes the same scientific gates in all six cases. Its aggregate posterior/sigma
ESS-per-second score is 6.63 times classical Rove (90% bootstrap interval
5.89–7.32), but gamma-shape efficiency falls to 0.69 in control and 0.66 in missing,
and birth–death density efficiency falls to 0.63 in Mkv. The fixed regression
criterion therefore rejects promotion: **classical remains the default**,
including in the native app. The three joint moves are opt-in through `--weight`;
the report gives the evaluated weights. Every continuous median ESS/second exceeds
the recorded BEAST ORC value in these cases, but those BEAST timings are archived,
not fresh observations, and the results do not establish general performance.

Six additional fixed states cover those cases. With imported category rates, likelihood
differences were at most 1.14e-13; generating the categories independently increased the
maximum to 8.16e-8, from the same quantile approximation difference described above.

## Gamma proposal frequency

A [follow-up study](https://github.com/getcortexlabs/rove/blob/main/benchmarks/gamma-mixing/2026-10-30/README.md) found that gamma shape received only about one proposal per 1,100–1,200 steps in the constructed four-tip dated workload. Raising `gamma_shape_scale` weight from 0.1 to 3 increased gamma ESS per group wall second by about 22–24 times, with the same proposal and prior. Other parameters became the bottleneck, and an efficiency regression prevented promotion under the frozen criteria. **Weight 0.1 remains the default**, including in the Mac app. The tested setting is available through `--weight gamma_shape_scale=3`; inspect all parameters and tree diagnostics before choosing it. This is not a 22–24-fold speedup of the full analysis or a measurement on M1 hardware.

## Configuring the sigma prior

In the Mac app, open **Clock sigma prior** below the relaxed-clock option.
**Shape · Alpha** and **Scale · Beta** default to 5 and 0.2. Sigma is still
estimated; these controls describe its prior, not a fixed sigma value.
The Gamma mean is shape × scale and its variance is shape × scale².
Changing them changes the analysis assumptions, not the character gamma model
or the proposal step sizes. Beta is explicitly a scale, never a rate.

The CLI accepts `--clock-sigma-shape 2.5 --clock-sigma-scale 0.4` with
`--model reference`, for both `sample` and `marginal`. Values must be positive,
finite and have a representable log normalizer. Strict-clock runs reject these
flags. Checkpoints, prior-only runs, prediction and saved model comparisons
preserve the chosen prior; chain comparisons require matching priors.
`run.json` includes `clock_sigma_prior_parameters` with numeric `shape` and
`scale`, alongside the readable description. Legacy records use 5 and 0.2.
Custom app setups use format 6 so older apps cannot silently ignore the choice.
See [Tree priors](./tree-priors) for the separate decision about tree models.

## Optional joint topology/rate exchange

`--weight narrow_exchange_rates=10` enables an experimental proposal alongside the existing moves. It has zero weight by default and no tuning parameter. In the native app, use **Advanced sampling → Try joint topology/rate exchange**; its initial weight is 10 and remains editable. The choice is preserved in setup format 9, review, history, run records and checkpoints. Only an undated relaxed clock is currently supported; dated and strict-clock use is rejected before sampling. This is one independently implemented proposal, not the full BEAST ORC suite. No morphology speed or mixing benefit has been established; comparative measurements in [#97](https://github.com/getcortexlabs/rove/issues/97) remain deferred.

Let `G` be an internal node with an internal child. Choose `G` uniformly from the `C(T)` such nodes, take its older child `P` and younger child `U`, and choose either child `X` of `P` with probability 1/2. Exchange `X` and `U`, keeping heights and the root unchanged. Preserve the incoming distance of each reattached subtree with:

```text
rX' = rX (hP − hX) / (hG − hX)
rU' = rU (hG − hU) / (hP − hU)
J   = [(hP − hX) / (hG − hX)] [(hG − hU) / (hP − hU)]
log Hastings = log C(T) − log C(T') + log J
```

Positive finite durations/rates and `hP > hU` are required. Boundary/overflow proposals fail before mutation. The reverse chooses the same `G` and `P` and exchanges `U` back with `X`; its probability is `1 / (2 C(T'))`, compared with `1 / (2 C(T))` forward. The continuous map leaves heights unchanged, so the Jacobian is the product of the two rate scale factors. Rates remain attached to node-indexed incoming branches, including the special slot when the root is not the last node. Constraints and likelihood/prior rejection use the normal exact restoration path. Preserving the reattached distances does not preserve the likelihood: the topology changes.

Tests cover the inverse, full numerical Jacobian, enumerated forward/reverse choices, known uniform-topology/independent-lognormal targets, constraint rejection, fresh versus cached likelihood, different root indices, exact checkpoint replay, native cancellation/resumption and identical portable/ARM samples. These correctness checks do not replace posterior and performance studies. Related joint-proposal methods are described by [Douglas, Zhang and Bouckaert (2021)](https://doi.org/10.1371/journal.pcbi.1008322).

## Optional Yule tree prior

Choose **Tree prior → Yule · pure birth** in Setup, or pass
`--tree-prior yule --birth-rate-upper 1000` to `sample` or `marginal`.
Birth–death remains the default; fossil dating keeps FBD. Yule is a
complete-sampling species-tree prior for contemporaneous terminals at zero,
without extinction. It rejects fossil dating and does not supply node
calibrations or a population coalescent. Extant species, fossil terminals and
within-species population samples require different biological assumptions.

Only the birth rate λ is estimated, with a proper Uniform(0,U) prior whose
log density is −log U; U must be positive and finite. The initial λ is
min(1,U/2). Tree heights use expected changes on the fixed unit-clock scale,
so λ is per unit of those heights, not automatically per million years.
U=1000 is a reference starting value, not a recommendation for every matrix.
Inspect prior-only runs and repeat with scientifically defensible bounds.
There is no extinction coordinate, proposal or trace statistic under Yule.
The `birth_rate_scale` proposal supports the usual weight and tuning controls.

The BEAST 2.7.8 unscaled convention is
`(n−1) log λ − λ (sum of internal heights + root height)`.
It conditions on tip count, not root height or an origin. The root contributes
twice. For a labeled ranked tree, independent backwards waits Exp(kλ) and
uniform choices among k(k−1)/2 pairs give a normalized density multiplier
`2^(n−1)/(n−1)!`. This parameter-independent coefficient is the same for the
current birth–death convention at the same n. Power-posterior estimates use
Z(1)/Z(0), so a constant prior multiplier cancels; the resulting evidence is
relative to the normalized prior on the supported domain. Monophyly restricts
that domain. Keep the matrix, taxa and constraints consistent when comparing
evidence, and compare the full model including U and all other priors.

The selected model and bound appear in Review, History and model comparisons,
and persist in format-10 setups, run records, checkpoints, predictions and
marginal estimates. Earlier apps reject these setups. Convergence diagnostics
never combine Yule with birth–death or different birth-rate bounds. Exact
checkpoint resume still requires the original executable.

Validation includes 24 synthetic fixed states from pinned BEAST.base 2.7.8
(`fixtures/validation/yule`), the zero-extinction birth–death limit, independent
prior simulation of waiting times, heights and topology frequencies with and
without monophyly, and a power-posterior quadrature check. These are correctness
checks, not comparative timing or mixing claims.
