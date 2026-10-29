---
description: "How Rove lays out likelihood data, why the kernel is model-aware, what the scalar reference is for, and how buffers and rescaling work."
---

# Design

Rove is one Rust crate. It computes the likelihood of a rooted binary tree under the Mk model for a matrix of discrete characters. The decisions below are the ones already visible in the code. The SIMD work they prepare for has started with the [NEON backend](./kernel#backends); the rest is on the [roadmap](./roadmap).

## Pattern-major layout

A likelihood kernel spends its time multiplying transition matrices into partial likelihoods. BEAGLE's CPU kernels vectorise across states and are specialised for four states, which suits nucleotides. Morphological matrices are mostly binary and ternary characters, so those kernels take the generic path with padding.

Rove stores partials the other way round. For each rate category and each state there is one contiguous run of values, one per site pattern, padded to a multiple of eight lanes. The inner loop runs across patterns, so it has the same shape for two states as for sixty-one.

`Layout` describes that arrangement: the number of states, rate categories and patterns, and the padded stride. `offset(category, state)` gives the start of one run.

## Model-aware fast paths

Mk has a closed form. The transition matrix for a branch is `P(t) = e·I + (1 − e)·π`, where `e` depends only on the branch length and `π` holds the state frequencies. Multiplying that into a vector costs O(k) per pattern instead of O(k²).

An Mk core, built with `LikelihoodCore::mk`, uses that form directly. Each branch holds a distance, `e` and `1 − e` per rate category instead of a matrix, and a child update computes one frequency-weighted sum per pattern for all states. The matrix core stays for models without such a form, and the tests compare both with the reference.

On an Apple M4 Max, the [direct Mk comparison](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md) measured both cores in the same processes. For a full evaluation of the 20 × 50 reference, the matrix core took 1.25 times as long as the Mk core; for a binary 90 × 250 matrix, 1.11 times; with eight states, 2.66 times. With two states both paths do about the same arithmetic, and the Mk path makes fewer passes over memory.

## A scalar reference, always

`reference::Problem` is a plain recursive implementation of Felsenstein's pruning algorithm. It is slow and simple on purpose. The equality tests build random trees and compare `LikelihoodCore` against it, with tip states and with tip partials, with and without rescaling, to a relative tolerance of 1e-9. `variable_log_likelihood` applies the Mkv correction in the same direct way.

Every optimised path added later has to pass the same tests. The reference stays.

## Double buffers and rescaling

Each node owns two buffers for partials and two for its branch: transition matrices, or Mk distances. Before a proposal overwrites a node, `set_node_partials_for_update` or `set_node_matrix_for_update` flips that node to its other buffer, so the previous values survive. `store` records which buffers are current. `restore` brings that set back by swapping, `unstore` by copying. Accepting a move costs nothing. This is what an MCMC sampler needs to reject a move cheaply.

With `Scaling::Always`, each internal node's partials are divided by their per-pattern maximum after they are computed, and the logarithm of that maximum is kept per node. The log likelihood adds the scalers back. Without scaling, deep trees underflow.

## Constant patterns in the same core

The Mkv correction needs the probability of each constant pattern under the same tree, rates and scaling as the observations. Rove appends those patterns to the core's tip inputs instead of computing them separately, so partial updates, rescaling and store/restore cover them unchanged. For the 20 × 50 reference, the two extra patterns raise the padded stride from 24 to 32, so each node update covers a third more lanes.

## A sampler on the same buffers

The [sampler](./sampler) keeps the tree and its parameters outside the core. Before each step it saves them and stores the core's buffer indices. After a proposal it compares the new tree with the saved one, gives each changed branch a new distance in its other buffer, and recomputes only the ancestors of those branches. A rejected step swaps the indices back. The proposals reproduce BEAST 2's operators and Hastings ratios, so a comparison with BEAST can use the same moves and weights.

The reference preset adds gamma shape, clock sigma and explicit branch rates to the stored state. A change of sigma leaves the likelihood untouched; a change of shape updates category distances throughout the tree. Its proposals differ from ORC, as the [reference-model guide](./reference-model#proposals-and-output) records.

## NEON first, AVX2 second

The layout exists so that the inner loops can be vectorised across patterns. In release builds the compiler already vectorises the portable loops, one loop at a time: a node update zeroes two accumulators, adds each state's contribution to them, combines them with the children's partials, and rescaling rereads the results to find each pattern's maximum. The [NEON backend](./kernel#backends) runs the Mk node update as one pass over blocks of eight patterns instead, with the sums and the maxima in registers. Per pattern it performs the same operations in the same order, so it reproduces the portable code's results bit for bit except for the payload of a NaN, and the tests require it. On Apple M4 Max, the portable code took 1.48 times as long for a full evaluation of the reference matrix ([report](https://github.com/getcortexlabs/rove/blob/main/benchmarks/neon/2026-10-28/README.md)). The logarithm of rescaling remains a scalar call per pattern and node. There is no AVX2 path.

## One crate

Rove has no dependency on BEAST or BEAGLE. BEAST is a validation and benchmarking reference. A BEAGLE C ABI and a BEAST 2 `LikelihoodCore` adapter over JNI are possible compatibility paths. Neither is required by the Rust core, and neither is being built now.

## Non-goals

Competing with BEAGLE's CUDA backend on large nucleotide alignments. Use the GPU for that.
