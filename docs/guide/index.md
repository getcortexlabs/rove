---
description: "Rove is an ARM-first phylogenetic inference project in Rust. Learn why it starts with a likelihood kernel for morphological data, what exists today and what does not."
---

# What is Rove?

Rove is a phylogenetic inference project written in Rust. It starts with a likelihood kernel for morphological data, designed for ARM CPUs first. The goal is standalone inference: running an analysis from a character matrix to a sample of trees without depending on BEAST.

The name comes from exploring, wandering and discovering paths.

::: info Project status
Early. The likelihood core, the Mkv correction, the NEXUS reader, pattern compression and numerical tests are implemented. A BEAST 2.7.8 reference experiment is recorded. Standalone inference, SIMD code paths and cross-engine performance comparisons remain future work. Work is tracked in the [issues](https://github.com/getcortexlabs/rove/issues).
:::

## Why another kernel

Small state spaces and short morphological alignments are worth measuring as their own workload.

- Mk has a closed form, `P(t) = e·I + (1 − e)·π`, which permits an O(k) child update per pattern instead of a full O(k²) matrix multiplication. The current crate builds these matrices; the direct pruning path is planned.
- ARM-specific vectorisation and reuse of repeated calculations are candidate optimisations. Their value needs matched benchmarks, including setup and partial-update costs.
- The recorded BEAST 2.7.8 reference used its Java likelihood core. That observation does not establish the backend used by every BEAST version, installation or model.

Rove starts from the data instead: a layout that vectorises across site patterns, and fast paths for the models morphologists use. The [design](./design) page goes through the decisions.

## What exists today

The repository is one Rust crate with no runtime dependencies.

- `LikelihoodCore`: Felsenstein pruning over a pattern-major layout, with rate categories, optional per-node rescaling and store/restore of node state for a sampler.
- `model::Mk`: the Mk model with its closed-form transition matrix, for uniform or custom state frequencies.
- `ascertainment::VariableCharacters`: the Mkv correction for matrices of variable characters, checked against two fixed BEAST 2.7.8 states.
- `reference::Problem`: a plain recursive implementation kept as the reference in tests.
- `nexus::parse` and `data::Matrix::compress`: morphological input, preserved observations, explicit partitions and weighted site patterns. See [Reading NEXUS](./nexus).
- Property tests that compare the core against the reference on random trees with 2 to 10 tips, 2 to 6 states and 1 to 4 rate categories.
- A synthetic positive-control matrix of 20 taxa and 50 binary characters, with the script that generates it and degraded variants.

The [likelihood core](./kernel) page shows how to call it. The [synthetic matrices](./fixtures) page describes the control.

## What does not exist yet

- Standalone inference: tree proposals, MCMC, priors and clock models.
- The Mk closed-form fast path inside the kernel.
- NEON or AVX2 code paths. The current kernel is scalar Rust; the layout is prepared for vectorisation.
- Cross-engine performance comparisons with BEAST or BEAGLE.
- A user interface and the optional BEAST 2 or BEAGLE adapters.

Nothing on this list is documented as available anywhere on this site. The [roadmap](./roadmap) gives the order.

## BEAST, as a reference

BEAST is not a dependency. It is the reference for validation and benchmarking: likelihoods are compared at fixed states first, then complete inference runs. BEAST 2 and BEAST X are different code bases, and results for one do not establish results for the other, so comparisons name the implementation and version they ran against.
