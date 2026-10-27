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
| Direct Mk pruning | Next, #4 |
| NEON vectorisation | Planned, #5 |
| Minimal standalone MCMC | Planned, #15 |
| Relaxed-clock reference model | Planned, #16 |
| Complete inference validation | Planned, #12 |
| Evaluation of repeats within subtrees | Planned, #17 |
| Optional BEAST 2 JNI and BEAGLE C ABI adapters | Possible, #6 and #7 |

## Establish the baseline

The [core](./kernel) is checked against an independent scalar implementation. The [NEXUS reader](./nexus) supplies weighted observations without BEAST. The [reference experiment](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md) records exact inputs, model settings, two completed BEAST runs and fixed numerical states.

The [Apple M4 Max baseline](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-26/README.md) records 11 Rust workloads, separating setup, full evaluations, partial updates and proposal bookkeeping. Raw samples, allocation observations and variation across three processes are included. Full-inference and cross-engine comparisons remain pending; no cross-engine speedup has been measured.

## Correctness before optimization

Mkv conditions the likelihood on the observation process for variable-only matrices. It reproduces both archived BEAST states, and its constant patterns stay separate from the observed pattern weights.

Next, the direct Mk path can replace generic matrix multiplication, followed by NEON on Apple silicon. The scalar implementation remains the reference and fallback. Each optimization needs a before/after measurement, not just a count of arithmetic operations.

## Standalone inference

A minimal sampler comes before the full relaxed-clock and birth–death configuration. Validation proceeds from fixed-state likelihoods to complete runs on known matrices. Sampling quality, including ESS per time and tree diagnostics, counts alongside runtime.

The intended product is a native macOS application. Application UI work has not started. BEAST and BEAGLE adapters are optional compatibility paths, not dependencies of the Rust engine.

Follow the [repository issues](https://github.com/getcortexlabs/rove/issues) for acceptance criteria and results.
