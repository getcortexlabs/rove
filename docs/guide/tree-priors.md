---
description: "Supported tree priors, their assumptions, and the limits of each choice."
---

# Choosing a tree prior

The tree prior describes the branching process before the characters are
considered. It is different from the prior on clock sigma (variation among
branches) and the gamma shape prior (variation among characters).

## Available choices · 5 November 2026

**Yule is available as an optional prior for contemporaneous species.** Birth–death remains the default and fossil dating uses FBD. Choose the model from the study's sampling and biological assumptions; adding Yule does not establish that it fits Thiago's datasets better. See the [implemented model and controls](./reference-model).

| Analysis | Choice | Rove status / limit |
| --- | --- | --- |
| Species treated as sampled at one time | Birth–death | Available; all tips at zero, complete sampling, constant rates |
| Same setting, deliberately assuming no extinction | Yule | Available; pure birth, without extinction |
| Fossil tips with ages | Fossilized birth–death (FBD) | Available; fixed or estimated process, uncertain tip ages; terminal fossils only |
| Fossils that may be direct ancestors | FBD with sampled ancestors | Separate work in #52 |
| Samples within a population | Coalescent / demographic skyline | Deferred; not a substitute for species diversification |
| Morphology without a clock or dating | Uniform topology with a branch-length prior | Useful alternative to assess separately; requires a different tree parameterization |
| Diversification or fossil recovery changing over time | Birth–death/FBD skyline | Deferred until there is a defined study design and enough information to estimate extra rates |

[BEAST's Yule guidance](https://beast2-dev.github.io/hmc/hmc/YuleModel/index/)
limits this model to contemporaneous tips. The
[RevBayes morphology tutorial](https://revbayes.github.io/tutorials/morph_tree/V2)
shows a no-clock alternative. Its
[FBD tutorial](https://revbayes.github.io/tutorials/fbd_simple/)
models diversification and fossil recovery jointly.

A fossil without an age does not become a living species. Entering every tip
at zero is a modeling assumption, not a solution to missing ages. Sampling
coverage, extinction and the intended time scale need to match the analysis.
No universal best prior follows from the matrix dimensions or a high ESS.

Our birth–death density uses the unscaled Gernhard convention, conditioned on tip count rather than root/origin. FBD uses origin conditioning, no survival/present-sampling conditioning and removal probability zero. Fixed-process and estimated-process restrictions have different recorded density conventions; see [Fossil tip dating](./tip-dating#what-the-prior-assumes) before model comparison.

## Yule implementation and validation

The implementation uses the complete-sampling, tip-count-conditioned
convention of BEAST 2.7.8 `YuleModel`: no root/origin conditioning and no fossil
tips or node calibrations. With n tips, birth rate λ and internal heights h,
its unscaled log density is `(n−1) log(λ) − λ (sum(h) + root_height)`.
This follows by setting extinction to zero in our current birth–death density;
see [Gernhard (2008)](https://doi.org/10.1016/j.jtbi.2008.04.005).

Validation includes 24 fixed states from pinned BEAST 2.7.8, independent prior simulations, the zero-extinction birth–death limit, power-posterior quadrature and exact checkpoint replay. The extinction parameter and its proposals are absent. The app and CLI preserve Yule settings through setup, Review/History, prediction and model comparison.

The birth-rate prior is a proper Uniform(0,U), with configurable positive U. The reference bound U=1000 is in unit-clock height units, not a universal biological recommendation. Compare prior-only behavior and defensible alternative bounds. Marginal comparisons require the same data, constraints and compatible normalization conventions; see [model details](./reference-model).

Before interpreting results, compare independent chains, prior-only behavior
and sensitivity of clade support and heights to the assumptions. Faster sampling
or larger support alone does not make one model scientifically preferable.

## ORC is a separate decision

Rove already has explicit lognormal branch rates and optional joint moves.
It does not implement the entire ORC proposal suite. The
[XML audit](https://github.com/getcortexlabs/rove/blob/main/fixtures/reference/20x50/orc-audit.md)
records the exact versions, supported/partial/missing components and tests for
future schedules. Keep the current schedule while #48 and #72 evaluate weights.
The joint topology/rate exchange in [#97](https://github.com/getcortexlabs/rove/issues/97) is implemented as an opt-in experiment for undated relaxed-clock analyses. Its default weight is zero. Comparative benefits for morphology remain unmeasured; [#128](https://github.com/getcortexlabs/rove/issues/128) records partial ORC coverage and the remaining operators and validation.
