# ORC workflow audit

Reviewed on 2 November 2026 for #89–#91 against Rove `78b74c5` and the changes
on this branch. This is a source/configuration audit, not a new timing study.
Thiago supplied the XML; the conclusions below are our engineering review,
not a claim that he has reviewed this document.

## Input and probability model

The supplied `matriz_teste.xml` has SHA-256
`cc8d64d6eba916232271136dca56bd74fec2a0aaded7c1d0f52fff34d5902e79`.
The pinned reference is [analysis.xml](./analysis.xml). Required versions:
BEAST.base 2.7.8, MM 1.2.1, ORC 1.2.1 and BEASTLabs 2.0.3.

The XML uses binary Mk with variable-character conditioning, four gamma
categories, and `UCRelaxedClockModel` with 38 explicit branch rates for 20
tips. The independent lognormal distribution has real-space mean 1; rates
start at 0.5 and sigma at 0.2. The clock multiplier is 1, without realized-mean
normalization. The tree prior is `BirthDeathGernhard08Model`, not BICEPS:
operator IDs containing “BICEPS” do not select a tree prior.

The sigma prior is Gamma(alpha=5, beta=0.2). Inspection of the pinned
`BEAST.base.src.jar` confirms that `Gamma.modeInput` defaults to `ShapeScale`.
Thus its mean is 1 and variance 0.2. The character gamma shape instead has
Exponential(mean=1). Rove's new shape/scale controls affect only sigma.

## Proposal mapping

Weights below are the XML's outer scheduling weights. Nested operators have
initial weight 1; adaptation changes their mixture. They are not independent
outer proposals with those same weights.

| XML component | Rove status | Difference / next action |
| --- | --- | --- |
| Explicit lognormal rates, sigma prior, character gamma | Supported | Same parameter coordinates; #89 exposes sigma shape/scale |
| Adaptive sigma mixture (3): UcldScaler, Bactrian random walk/scale, SampleFromPrior | Partial | `clock_sigma_scale` and optional `clock_sigma_rates`; no adaptive mixture, Bactrian kernels or prior draws |
| Adaptive root mixture (1): SimpleDistance, SmallPulley | Partial | Optional `root_time_rates` preserves incident distances, with its own scale proposal; no SmallPulley |
| Adaptive internal-rate mixture (20): InConstantDistance, Bactrian random walk/scale, SampleFromPrior | Partial | `branch_rate_scale` and optional `node_time_rates`; the latter draws uniformly between child and parent, not the XML's window proposal |
| Adaptive NER mixture (10): MetaNER and NER_dAE_dBE_dCE | Missing | Plain narrow exchange exists, but no joint topology/rate NER proposal |
| EpochFlex (2 + 2), TreeStretch (2) | Missing | Whole-tree scaling is not equivalent to these moves |
| Bactrian root scale (3), node (30), subtree slide (15) | Partial | Corresponding Rove move families use different proposal kernels |
| Narrow exchange (15), wide exchange (3), Wilson–Balding (3) | Supported families | Do not assume identical random proposals or acceptance histories |
| Bactrian birth/death scales (3 each) | Partial | Rove uses ordinary scale proposals |
| Character gamma scale (0.1) | Supported | Its low frequency is already under investigation in #48 |
| Mutation-rate delta exchange (2) | Omitted | One mutation-rate dimension leaves no exchange to make |

Rove's three joint proposals were independently implemented in
[`src/mcmc/coupled.rs`](../../../src/mcmc/coupled.rs). Their change of variables
and Jacobians are tested. Similar conserved quantities do not establish
equivalence to the ORC kernels or full ORC compatibility. They remain opt-in.

## Decision and acceptance gates

Keep the clock label “Relaxed clock + gamma”. Do not add an “ORC” switch that
implies we implement the whole suite. Keep the current default schedule.
Use #48 for schedule comparisons and #72 for adaptive selection weights;
step-size tuning already present in Rove is a separate mechanism.

The first additional ORC proposal experiment should be a joint narrow
exchange/rate move (NER), tracked in [#97](https://github.com/getcortexlabs/rove/issues/97). It targets topology mixing, a gap the existing
time/rate moves cannot address. This is a hypothesis to test, not a promise
that it will outperform ordinary exchange. Defer SmallPulley, Bactrian,
EpochFlex, TreeStretch and prior-draw proposals until measurements identify
a relevant bottleneck. Do not launch all of them as one implementation.

Before adopting a proposal or default schedule:

1. Test the forward/reverse transformation, support, Hastings ratio and
   Jacobian numerically, including root-index changes and monophyly.
2. Test prior-only stationary distributions and small-tree topology
   frequencies; reject proposals that alter the target distribution.
3. Compare independent runs under the same priors, matrix, constraints,
   burn-in rule and logging interval. Compare parameter distributions and
   split/clade frequencies with Monte Carlo uncertainty, not matching seeds.
4. Measure minimum relevant ESS/second, topology agreement, wall time to the
   predeclared diagnostic target, and peak memory. Include small, 90 × 250,
   and 500-terminal matrices, strict/relaxed clocks where applicable and
   dated cases. Report chains that hit the cap without meeting the target.
5. On the same quiet Mac, compare the baseline, each candidate, and pinned
   BEAST+ORC. Include warm-up/adaptation in elapsed time; record build,
   hardware, power settings and seeds. Preserve checkpoint determinism and
   test both native and portable likelihood backends.

The [ORC paper](https://doi.org/10.1371/journal.pcbi.1008322) motivates
separating parameterization, proposal kernels and adaptive mixtures. Its
reported improvements are not Rove benchmarks. The
[BEAST performance guide](https://beast.community/measuring-beast-performance.html)
is a useful timing reference; our decision criterion also includes mixing.
The [site-repeat paper](https://doi.org/10.1093/sysbio/syw075) addresses redundant
likelihood work. It does not justify a particular tree prior or an MCMC
schedule. No performance measurements were run for this audit.

## Sigma-prior validation

`tests/support/sigma_priors.json` contains 25 fixed densities exported by
`tools/beast/export_rate_model.py`, with Java and JAR provenance. It spans
shape below, at and above 1, five scales/shapes and sigma from 0.01 to 20.
The Rust comparison tolerance is 3e-12 in log density. Defaults retain the
previous arithmetic bit for bit. Prior-only tests check two different Gamma
means/variances, and checkpoint tests cover custom settings and exact replay.

The runtime remains Rust; BEAST/Java are used only to generate reference data.
