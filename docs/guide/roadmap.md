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
| Full-inference and cross-engine benchmarks | Pending, #8 |
| Mkv ascertainment correction | Done |
| Direct Mk pruning | Done |
| Minimal standalone MCMC | Done |
| NEON vectorisation | Done |
| Relaxed-clock reference model | Next, #16 |
| Complete inference validation | Planned, #12 |
| Evaluation of repeats within subtrees | Planned, #17 |
| Optional BEAST 2 JNI and BEAGLE C ABI adapters | Possible, #6 and #7 |

## Establish the baseline

The [core](./kernel) is checked against an independent scalar implementation. The [NEXUS reader](./nexus) supplies weighted observations without BEAST. The [reference experiment](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md) records exact inputs, model settings, two completed BEAST runs and fixed numerical states.

The [Apple M4 Max baseline](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-26/README.md) records 11 Rust workloads, separating setup, full evaluations, partial updates and proposal bookkeeping. Raw samples, allocation observations and variation across three processes are included. Full-inference and cross-engine benchmarks remain pending. The strict-clock comparison below records wall times for both engines on one small configuration, but it is not a controlled benchmark.

## Correctness before optimization

Mkv conditions the likelihood on the observation process for variable-only matrices. It reproduces both archived BEAST states, and its constant patterns stay separate from the observed pattern weights.

The direct Mk path replaces the generic matrix product for Mk models. On Apple M4 Max, the matrix path took 1.25 times as long for a full evaluation of the reference matrix, and 2.66 times as long with eight states ([report](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md)). The NEON backend then fused the Mk node update into one pass over blocks of eight patterns. On the same machine, the portable code took 1.48 times as long for a full evaluation of the reference matrix, 1.10 to 1.32 times as long on the 90 × 250 cases with 250 patterns and 1.08 to 1.11 times as long for a sampler run ([report](https://github.com/getcortexlabs/rove/blob/main/benchmarks/neon/2026-10-28/README.md)). The logarithm of rescaling, one scalar call per pattern and node, now takes about two fifths of the node-update time in a sampler run. The portable implementation remains the reference and fallback. Each optimization needs a before/after measurement, not just a count of arithmetic operations.

## Standalone inference

The [minimal sampler](./sampler) supports binary Mk or Mkv, a strict clock and the birth–death prior. With the same moves and fixed steps as BEAST 2.7.8, its posterior means and clade frequencies on the reference matrix met criteria fixed before the runs ([report](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md)). The relaxed-clock configuration of the reference analysis comes next. Validation proceeds from fixed-state likelihoods to complete runs on known matrices. Sampling quality, including ESS per time and tree diagnostics, counts alongside runtime.

The intended product is a native macOS application. Application UI work has not started. BEAST and BEAGLE adapters are optional compatibility paths, not dependencies of the Rust engine.

Follow the [repository issues](https://github.com/getcortexlabs/rove/issues) for acceptance criteria and results.
