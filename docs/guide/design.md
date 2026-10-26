---
description: "How Rove lays out likelihood data, why the kernel is model-aware, what the scalar reference is for, and how buffers and rescaling work."
---

# Design

Rove is one Rust crate. It computes the likelihood of a rooted binary tree under the Mk model for a matrix of discrete characters. The decisions below are the ones already visible in the code. The SIMD and fast-path work they prepare for is on the [roadmap](./roadmap).

## Pattern-major layout

A likelihood kernel spends its time multiplying transition matrices into partial likelihoods. BEAGLE's CPU kernels vectorise across states and are specialised for four states, which suits nucleotides. Morphological matrices are mostly binary and ternary characters, so those kernels take the generic path with padding.

Rove stores partials the other way round. For each rate category and each state there is one contiguous run of values, one per site pattern, padded to a multiple of eight lanes. The inner loop runs across patterns, so it has the same shape for two states as for sixty-one.

`Layout` describes that arrangement: the number of states, rate categories and patterns, and the padded stride. `offset(category, state)` gives the start of one run.

## Model-aware fast paths

Mk has a closed form. The transition matrix for a branch is `P(t) = e·I + (1 − e)·π`, where `e` depends only on the branch length and `π` holds the state frequencies. Multiplying that into a vector costs O(k) per pattern instead of O(k²).

Today `model::Mk` produces the full matrix and the kernel multiplies it generically. The closed form is tested against Lewis' formula. Baseline measurements and the Mkv ascertainment correction precede the direct Mk path on the [roadmap](./roadmap).

## A scalar reference, always

`reference::Problem` is a plain recursive implementation of Felsenstein's pruning algorithm. It is slow and simple on purpose. The equality tests build random trees and compare `LikelihoodCore` against it, with tip states and with tip partials, with and without rescaling, to a relative tolerance of 1e-9.

Every optimised path added later has to pass the same tests. The reference stays.

## Double buffers and rescaling

Each node owns two buffers for partials and two for transition matrices. Before a proposal overwrites a node, `set_node_partials_for_update` or `set_node_matrix_for_update` flips that node to its other buffer, so the previous values survive. `store` records which buffers are current. `restore` brings that set back by swapping, `unstore` by copying. Accepting a move costs nothing. This is what an MCMC sampler needs to reject a move cheaply.

With `Scaling::Always`, each internal node's partials are divided by their per-pattern maximum after they are computed, and the logarithm of that maximum is kept per node. The log likelihood adds the scalers back. Without scaling, deep trees underflow.

## NEON first, AVX2 second

The layout exists so that the inner loop can be vectorised. The plan is NEON on Apple silicon first and AVX2 second, with the scalar code kept as the reference and fallback. Neither exists yet; the current kernel is scalar Rust.

## One crate

Rove has no dependency on BEAST or BEAGLE. BEAST is a validation and benchmarking reference. A BEAGLE C ABI and a BEAST 2 `LikelihoodCore` adapter over JNI are possible compatibility paths. Neither is required by the Rust core, and neither is being built now.

## Non-goals

Competing with BEAGLE's CUDA backend on large nucleotide alignments. Use the GPU for that.
