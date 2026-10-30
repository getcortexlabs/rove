---
description: "Rove is an ARM-first phylogenetic inference project in Rust. Learn why it starts with a likelihood kernel for morphological data, what exists today and what does not."
---

# What is Rove?

Rove is a phylogenetic inference project written in Rust. It starts with a likelihood kernel for morphological data, designed for ARM CPUs first. The goal is standalone inference: running an analysis from a character matrix to a sample of trees without depending on BEAST.

The name comes from exploring, wandering and discovering paths.

::: info Project status
Early. The likelihood core with its direct Mk path and a NEON kernel for it, the Mkv correction, the NEXUS reader, pattern compression, numerical tests and a minimal standalone sampler are implemented. A BEAST 2.7.8 reference experiment is recorded. The relaxed-clock [reference model](./reference-model) is implemented and checked at fixed states. Posterior comparisons cover six constructed 20-taxon cases. The [expanded benchmark](https://github.com/getcortexlabs/rove/blob/main/benchmarks/expanded/2026-10-30/README.md) covers 20, 90 and 500 taxa and compares BEAST/BEAGLE kernel adapters, with runtime and sampling quality reported separately. The larger runs did not meet the convergence criteria. An AVX2 path remains future work. Work is tracked in the [issues](https://github.com/getcortexlabs/rove/issues).
:::

## Why another kernel

Small state spaces and short morphological alignments are worth measuring as their own workload.

- Mk has a closed form, `P(t) = e·I + (1 − e)·π`, which permits an O(k) child update per pattern instead of a full O(k²) matrix multiplication. Rove's Mk core prunes with it directly.
- ARM-specific vectorisation and reuse of repeated calculations are candidate optimisations. Their value needs matched benchmarks, including setup and partial-update costs.
- The recorded BEAST 2.7.8 reference used its Java likelihood core. That observation does not establish the backend used by every BEAST version, installation or model.

Rove starts from the data instead: a layout that vectorises across site patterns, and fast paths for the models morphologists use. The [design](./design) page goes through the decisions.

## What exists today

The engine is one Rust crate with no runtime dependencies. A separate Swift package
provides the [native Mac preview](./macos-preview) for matrix import, reviewed settings, execution and live progress.

- `LikelihoodCore`: Felsenstein pruning over a pattern-major layout, with rate categories, optional per-node rescaling and store/restore of node state for a sampler. `LikelihoodCore::mk` prunes with the Mk closed form; `LikelihoodCore::new` multiplies a transition matrix per branch.
- `Backend`: portable code for any target and, on aarch64, a NEON kernel for the Mk node update that gives the same results. See [Backends](./kernel#backends).
- `model::Mk`: the Mk model with its closed-form transition matrix, for uniform or custom state frequencies, which are validated.
- `ascertainment::VariableCharacters`: the Mkv correction for matrices of variable characters, checked against two fixed BEAST 2.7.8 states.
- `ascertainment::InformativeProbability`: cached Mkparsinf conditioning for uniform unordered Mk, checked by complete pattern enumeration and BEAST 2.7.8. See [character selection](./character-selection).
- `reference::Problem`: a plain recursive implementation kept as the reference in tests.
- `rates::GammaRates`, `rates::RelaxedClock`: [Mk + gamma, lognormal](./reference-model).
- `mcmc::Sampler` and the `rove sample` command: a Metropolis–Hastings sampler for binary and multistate characters under partitioned Mk or Mkv, a strict or lognormal relaxed clock and a birth–death tree prior, with monophyly constraints. See [Sampling trees](./sampler).
- `nexus::parse` and `data::Matrix::compress`: morphological input, preserved observations, explicit partitions and weighted site patterns. See [Reading NEXUS](./nexus).
- Property tests that compare both kinds of core against the reference on random trees with 2 to 10 tips, 2 to 6 states and 1 to 4 rate categories, including branch lengths from 10⁻⁹ to 100 and sequences of accepted and rejected changes.
- A synthetic positive-control matrix of 20 taxa and 50 binary characters, with the script that generates it and degraded variants.

The [likelihood core](./kernel) page shows how to call it. The [synthetic matrices](./fixtures) page describes the control.

## What does not exist yet

- User-defined partition models, tip dates, calibrations and additional tree priors.
- An AVX2 code path. Targets other than aarch64 run the portable code.
- Production BEAST 2 or BEAGLE adapters. Benchmark adapters are separate tools.

Nothing on this list is documented as available anywhere on this site. The [roadmap](./roadmap) gives the order.

## BEAST, as a reference

BEAST is not a dependency. It is the reference for validation and benchmarking: likelihoods are compared at fixed states first, then complete inference runs. BEAST 2 and BEAST X are different code bases, and results for one do not establish results for the other, so comparisons name the implementation and version they ran against.
