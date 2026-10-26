---
description: "Rove is an ARM-first phylogenetic inference project in Rust. Learn why it starts with a likelihood kernel for morphological data, what exists today and what does not."
---

# What is Rove?

Rove is a phylogenetic inference project written in Rust. It starts with a likelihood kernel for morphological data, designed for ARM CPUs first. The goal is standalone inference: running an analysis from a character matrix to a sample of trees without depending on BEAST.

The name comes from exploring, wandering and discovering paths.

::: info Project status
Early. The likelihood core and its numerical tests are implemented. Standalone inference, SIMD code paths, benchmarks and integrations are not. Work is tracked in the [issues](https://github.com/getcortexlabs/rove/issues).
:::

## Why another kernel

Morphological analyses are a poor fit for the kernels that exist today.

- BEAST 2 never uses BEAGLE for the Mk model. `LewisMK` returns a null eigen decomposition, the BEAGLE path throws, and the Java core takes over. Every morphological analysis in BEAST 2 runs in pure Java.
- BEAGLE's ARM support is SSE translated through `sse2neon`. The BEAGLE 4.1 preprint reports about 1.3x over scalar code, dropping to 1.1x at 8 threads.
- BEAGLE's CPU kernels vectorise across states and are specialised for four states. Binary and ternary characters, the bulk of morphological matrices, take the generic path with padding.
- Mk has a closed form, `P(t) = e·I + (1 − e)·π`, so `P·L` costs O(k) per pattern instead of O(k²). A kernel that knows the model can use it; a generic eigen-decomposition kernel cannot.

Rove starts from the data instead: a layout that vectorises across site patterns, and fast paths for the models morphologists use. The [design](./design) page goes through the decisions.

## What exists today

The repository is one Rust crate with no runtime dependencies.

- `LikelihoodCore`: Felsenstein pruning over a pattern-major layout, with rate categories, optional per-node rescaling and store/restore of node state for a sampler.
- `model::Mk`: the Mk model with its closed-form transition matrix, for uniform or custom state frequencies.
- `reference::Problem`: a plain recursive implementation kept as the reference in tests.
- Property tests that compare the core against the reference on random trees with 2 to 10 tips, 2 to 6 states and 1 to 4 rate categories.
- A synthetic positive-control matrix of 20 taxa and 50 binary characters, with the script that generates it and degraded variants.

The [likelihood core](./kernel) page shows how to call it. The [synthetic matrices](./fixtures) page describes the control.

## What does not exist yet

- Standalone inference: tree proposals, MCMC, priors and clock models.
- The Mk closed-form fast path inside the kernel, and the Mkv ascertainment correction.
- NEON or AVX2 code paths. The current kernel is scalar Rust; the layout is prepared for vectorisation.
- Benchmarks or measured comparisons with BEAST or BEAGLE.
- NEXUS parsing, a user interface, and the optional BEAST 2 or BEAGLE adapters.

Nothing on this list is documented as available anywhere on this site. The [roadmap](./roadmap) gives the order.

## BEAST, as a reference

BEAST is not a dependency. It is the reference for validation and benchmarking: likelihoods are compared at fixed states first, then complete inference runs. BEAST 2 and BEAST X are different code bases, and results for one do not establish results for the other, so comparisons name the implementation and version they ran against.
