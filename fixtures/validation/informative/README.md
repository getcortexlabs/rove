# Informative-character conditioning

This constructed six-taxon, nine-character matrix validates `--conditioning informative`
(Mkparsinf) under equal-frequency, unordered Mk. It is a numerical fixture, not
an empirical dataset or a posterior benchmark. It includes two-, three- and
four-state partitions, duplicate observed columns, missing cells and an uncertain
state set. No monophyly constraints are imposed. Every completion of every
character has at least two states represented at least twice.

## Definition and input rules

For a complete character, let I mean that at least two distinct states each occur
in two or more taxa. The log likelihood of a partition is

```
sum_i weight_i * log P(observation_i) - character_count * log P(I).
```

`P(I)` is computed on the same tree, branch distances and category mixture as the
observations. Categories are integrated before conditioning. Each state-count
partition has its own correction. These are the informative-character selection
rules discussed by [Lewis (2001)](https://doi.org/10.1080/106351501753462876);
[RevBayes](https://revbayes.github.io/documentation/dnPhyloCTMC.html) also exposes
an `informative` coding choice.

The complement is larger than constant patterns plus one exceptional tip when
there are more than two states: `00123` is uninformative. Rove rejects a character
if any completion of its uncertain or missing cells is uninformative. For each
candidate repeated state A, tips that allow A can take it; the remaining tips
must be matched to distinct non-A states. A complete bipartite matching witnesses
an excluded completion. Thus the raw observed numerator contains only retained
completions. This is global conditioning across all taxa, not a correction for
each character's missing-data mask. Polymorphic sets retain the reader's existing
uncertainty interpretation.

## Recurrence

The production implementation does not enumerate excluded columns. For k states,
fix one distinguished state A and set m = k − 1. Count the event E_A in which
every non-A state occurs at most once. A subtree with r distinct non-A singleton
states has three partials:

- U[r]: sum over all r-element subsets, conditional on ancestral state A.
- V[r]: sum over subsets containing a fixed other ancestral state B.
- W[r]: sum over subsets excluding B, conditional on that same B.

Each quantity is a probability bounded by one. A tip starts with
`[U[0], V[0], W[0]] = [1, 0, 0]` and `[U[1], V[1], W[1]] = [0, 1, 0]`.
Write a uniform Mk branch transition as `P = stay * Identity + change * 1πᵀ`
and put `d = change / k`. After propagation along a branch:

```
U'[r] = stay * U[r] + d * (U[r] + m * (V[r] + W[r]))
V'[r] = stay * V[r] + d * (r/m * U[r] + r * (V[r] + W[r]))
W'[r] = stay * W[r] + d * ((m-r)/m * U[r] + (m-r) * (V[r] + W[r]))
```

Let D_M(a,b) be the probability that independent uniform subsets of sizes a and
b from M labels are disjoint. It is zero for a+b > M; otherwise it is the product
of `(M-a-i)/(M-i)` over i from zero to b−1 (the empty product is one).
For each pair of child counts a,b, add the following to the parent at r = a+b:

```
U[r] += U_left[a] * U_right[b] * D_m(a,b)
V[r] += V_left[a] * W_right[b] * D_(m-1)(a-1,b)  [a > 0]
      + W_left[a] * V_right[b] * D_(m-1)(a,b-1)  [b > 0]
W[r] += W_left[a] * W_right[b] * D_(m-1)(a,b)
```

At the root with n tips, the fixed-A event mass is
`p_r = (U[r] + m * (V[r] + W[r])) / k`. For r ≤ n−2, A repeats;
the k labelings are disjoint and contribute k*p_r. For r ≥ n−1, all tip states
are distinct; contribute p_r once, since those patterns belong to every E_A.
Mix these excluded masses over categories, then compute `log P(I)` with `log1p`.

Memory is O(nodes × categories × min(k,n)); a node combination takes at most
O(categories × min(k,n)²). Only affected ancestors are updated. Double buffers
restore rejected proposals; checkpoints rebuild this deterministic cache from
the saved tree and rates. Updates allocate no heap memory.

The recurrence relies on exchangeable state labels: it does not apply to ordered
or unequal-frequency models. It has no two-to-four-state implementation limit.
Bounded probabilities avoid overflow from combinatorial counts; underflow of
excluded events has negligible absolute effect on the retained probability.
The observed likelihood keeps its existing scaled pruning. If excluded mass is
within `32 * n * k * f64::EPSILON` of one, or is invalid, the correction is
undefined and the proposal is rejected rather than returning an unstable value.

## Independent checks

`tests/informative.rs` enumerates every complete pattern on two- to five-tip
trees with two, three, four and six states, for two shapes and three branch
scales, including a zero-rate category and unequal mixture weights. It sums
informative pattern probabilities from the independent scalar matrix-pruning
reference, without using this recurrence. Absolute retained-probability error
must be below `3e-13`. Complete-character classification is checked for two to
four states, and uncertain-cell classification against every possible completion.
Other checks cover zero and tiny branches, 90/500-tip trees, 32 states, incremental
updates, accepted/rejected moves, portable/NEON chains and checkpoint continuation.

`beast-likelihoods.json` records six fixed states from **BEAST 2.7.8 / MM 1.2.1**,
using the Java BeerLikelihoodCore, scaling always and ambiguities enabled. The
tree, heights, branch rates, category rates and per-partition results are in that
file. Cases cross branch scales 0.01, 1 and 10 with a strict single-category
clock or a lognormal relaxed clock and four gamma categories. For each partition,
`PartitionLikelihood.java` independently enumerates all excluded patterns and
appends them as zero-weight ascertainment columns in BEAST. There are 14, 129
and 916 excluded patterns for the two-, three- and four-state partitions.

On Apple ARM, the largest absolute log-likelihood difference, over partition
and total values on both Rove backends, was **1.151e-9**. Test tolerances are
`2e-8` per partition and `3e-8` for totals. This verifies fixed-state values,
not agreement of posterior distributions or inference speed.

`provenance.json` contains package versions, Java version and SHA-256 hashes of
the input, Java source, output and jars. Reproduce with Java 17 and the pinned
packages:

```sh
python3 tools/beast/partition_likelihood.py \
  --packages <packages> --launcher <launcher.jar> \
  --matrix fixtures/validation/informative/matrix.nex \
  --conditioning informative --output <new-directory>
cargo test --test informative -- --nocapture
cargo test --release --test informative
```

See the [cost measurement](../../../benchmarks/ascertainment/2026-10-30/README.md)
for correction-only timings and allocations, and the [user guide](../../../docs/guide/character-selection.md)
for choosing the model.
