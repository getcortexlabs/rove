---
description: "Rove's implementation order: establish correctness and measured baselines, then optimize the kernel and build standalone inference."
---

# Roadmap

Rove is early. There are no releases, no published crate and no dates. This is the implementation order, not a schedule.

| Step | State |
| --- | --- |
| Scalar likelihood core and numerical reference tests | Done |
| CI on ARM64, x86_64 and the minimum Rust version | Done |
| Reproducible BEAST 2.7.8 reference analysis | Done |
| Morphological NEXUS and observed-pattern compression | Done |
| Rust kernel benchmark baseline | Done, #8 |
| Full-inference and cross-engine benchmarks | 20/90/500-taxon runs and kernel adapters measured; sampling limits reported, #8 |
| Mkv ascertainment correction | Done |
| Direct Mk pruning | Done |
| Minimal standalone MCMC | Done |
| NEON vectorisation | Done |
| Relaxed-clock reference model | Done, #16 |
| Complete inference validation | Done for six 20-taxon cases, #12 |
| Relaxed-clock sampling efficiency | Evaluated; joint moves opt-in, classical default retained, #27 |
| Native Mac import and setup preview | Implemented, #28 |
| In-app execution and progress | Implemented, #29 |
| Run history and configuration reuse | Implemented, #30 |
| Explicit outgroup selection and applied constraints | Implemented, #31 |
| Posterior MCC tree summary and annotated export | Implemented, #32 |
| Native annotated-tree explorer and SVG figures | Implemented, #33 |
| Native trace diagnostics and burn-in inspection | Implemented, #34 |
| Parallel chains and convergence reports | Implemented, #23 |
| Power-of-two rescaling | Implemented, #18 |
| Checkpoints and resumed runs | Implemented in CLI and native History, #22 |
| Partitions by number of states | Implemented, #19 |
| Conditioning on parsimony-informative characters | Implemented, #20 |
| [Ordered characters](./ordered-characters) | Implemented, #21 |
| Marginal likelihoods for model comparison | Planned, #24 |
| Fossilized birth–death prior with tip dates | Planned, #25 |
| [Posterior predictive checks](./posterior-prediction) | Implemented in the CLI, #26 |
| Evaluation of repeats within subtrees | Evaluated; experimental, off by default, #17 |
| Optional BEAST 2 JNI and BEAGLE C ABI adapters | Possible, #6 and #7 |

## Establish the baseline

The [core](./kernel) is checked against an independent scalar implementation. The [NEXUS reader](./nexus) supplies weighted observations without BEAST. The [reference experiment](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md) records exact inputs, model settings, two completed BEAST runs and fixed numerical states.

The [Apple M4 Max baseline](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-26/README.md) records 11 Rust workloads, separating setup, full evaluations, partial updates and proposal bookkeeping. Raw samples, allocation observations and variation across three processes are included. A [full-model comparison](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md) now records wall time and ESS per second on six 20-taxon cases; the [expanded report](https://github.com/getcortexlabs/rove/blob/main/benchmarks/expanded/2026-10-30/README.md) adds 90- and 500-taxon runs and BEAST/BEAGLE kernel comparisons, with convergence limits recorded. The strict-clock comparison below records wall times for both engines on one small configuration, but it is not a controlled benchmark.

## Correctness before optimization

Mkv conditions the likelihood on the observation process for variable-only matrices. It reproduces both archived BEAST states, and its constant patterns stay separate from the observed pattern weights.

The direct Mk path and the NEON backend are implemented and measured in the [Mk](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md) and [NEON](https://github.com/getcortexlabs/rove/blob/main/benchmarks/neon/2026-10-28/README.md) reports. Power-of-two rescaling now removes per-node logarithms and caches subtree exponent totals. Its [comparison](https://github.com/getcortexlabs/rove/blob/main/benchmarks/scaling/2026-10-30/README.md) separates the two changes on both backends, includes sampler runs and reports slower cases. The portable implementation remains the reference and fallback.

## Standalone inference

The first [minimal sampler](./sampler) supported binary Mk or Mkv, a strict clock and the birth–death prior. With the same moves and fixed steps as BEAST 2.7.8, its posterior means and clade frequencies on the reference matrix met criteria fixed before the runs ([report](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md)). The [reference model](./reference-model), with gamma categories and the lognormal relaxed clock, now reproduces both archived states component by component. Its posterior summaries and clade frequencies met the predeclared criteria in six constructed 20-taxon cases ([report](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md)). Validation proceeds from fixed-state likelihoods to complete runs on known matrices. Sampling quality, including ESS per time and tree diagnostics, counts alongside runtime.

With the proposal evaluation recorded (#27), the planned features follow what morphological analyses need: marginal likelihoods for model comparison (#24) and the fossilized birth–death prior with tip dates (#25). RevBayes documents each of these for morphological data and can serve as a second reference where BEAST has no matching feature.

The [native Mac preview](./macos-preview) imports and displays matrices and prepares analysis settings. It runs the bundled Rust engine with live progress, cancellation and fingerprint-checked setup reopening. BEAST and BEAGLE adapters are optional compatibility paths, not dependencies of the Rust engine.

Follow the [repository issues](https://github.com/getcortexlabs/rove/issues) for acceptance criteria and results.

State-count partitioning (#19) is implemented in the engine, CLI, native app and checkpoints. See [NEXUS partitions](./nexus#partitions-by-state-count) for the model rules and fixed-state validation.

Informative-character conditioning (#20) is implemented in the engine, CLI, native app and checkpoints. See [character selection](./character-selection) for its input rules and validation.

Posterior predictive simulation and paired morphology statistics are available through `rove predict`. See [posterior predictive checks](./posterior-prediction) for the observation rules, interpretation and limits.
