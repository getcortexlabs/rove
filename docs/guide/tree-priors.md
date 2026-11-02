---
description: "Which tree priors Rove supports, and the next addition selected after reviewing BEAST and RevBayes."
---

# Choosing a tree prior

The tree prior describes the branching process before the characters are
considered. It is different from the prior on clock sigma (variation among
branches) and the gamma shape prior (variation among characters).

## Decision · 2 November 2026

Keep the current birth–death default and FBD dating. Select **Yule as the next
optional prior for contemporaneous species**, without replacing the default.
This is a development decision from the review, not a claim that Yule fits
Thiago's datasets better. Yule is not implemented in Rove yet; implementation is tracked in [#96](https://github.com/getcortexlabs/rove/issues/96).

| Analysis | Choice | Rove status / limit |
| --- | --- | --- |
| Species treated as sampled at one time | Birth–death | Available; all tips at zero, complete sampling, constant rates |
| Same setting, deliberately assuming no extinction | Yule | Next addition; a restricted comparison model |
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

## What the Yule implementation must establish

The first version will use the complete-sampling, tip-count-conditioned
convention of BEAST 2.7.8 `YuleModel`: no root/origin conditioning and no fossil
tips or node calibrations. With n tips, birth rate λ and internal heights h,
its unscaled log density is `(n−1) log(λ) − λ (sum(h) + root_height)`.
This follows by setting extinction to zero in our current birth–death density;
see [Gernhard (2008)](https://doi.org/10.1016/j.jtbi.2008.04.005).

The implementation issue requires fixed-state checks against pinned BEAST,
independent synthetic prior sampling, removal of the extinction parameter
and its proposals, explicit birth-rate prior settings, versioned saved setups,
checkpoints, trace/model records and EN/PT-BR controls. Preserve the existing
proper Uniform(0,1000) birth-rate prior as the reference comparison in unit-clock
analyses; document its scale and permit deliberate configuration. Audit density
constants and constraints before allowing marginal-likelihood comparisons.

Before interpreting results, compare independent chains, prior-only behavior
and sensitivity of clade support and heights to the assumptions. Faster sampling
or larger support alone does not make one model scientifically preferable.

## ORC is a separate decision

Rove already has explicit lognormal branch rates and optional joint moves.
It does not implement the entire ORC proposal suite. The
[XML audit](https://github.com/getcortexlabs/rove/blob/main/fixtures/reference/20x50/orc-audit.md)
records the exact versions, supported/partial/missing components and tests for
future schedules. Keep the current schedule while #48 and #72 evaluate weights.
The next isolated proposal experiment is [NER #97](https://github.com/getcortexlabs/rove/issues/97): changing nearby branches and
rates together. Its benefit for morphology still needs measurement.
