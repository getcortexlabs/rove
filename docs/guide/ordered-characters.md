---
description: "Declare ordered morphological characters, preserve intermediate states, and mix ordered and unordered partitions in one analysis."
---

# Ordered characters

An ordered character changes through neighbouring states: `0 ↔ 1 ↔ 2`. A branch can still start at `0` and end at `2`, by passing through `1`. Ordering restricts each individual change, not the total change along a branch. Unordered Mk allows a direct change between any pair of states.

Rove reads this choice from NEXUS. Add one `ASSUMPTIONS` block **after** the `DATA` or `CHARACTERS` block:

```nexus
BEGIN ASSUMPTIONS;
  OPTIONS DEFTYPE=UNORD;
  TYPESET * character_order = ORD: 2-3 5 8;
END;
```

Here characters 2, 3, 5 and 8 are ordered; the rest are unordered. Indices start at one. The Mac app shows **Ordered** or **Unordered** beside each partition in the matrix and analysis review. There is no separate ordering editor yet.

## Syntax and state alphabets

- `DEFTYPE` accepts `ORD` or `UNORD`; the default is `UNORD`. It applies to characters omitted from the type set.
- One named `TYPESET` is supported, with an optional `*`. The sole type set is applied with or without the star. The standard form accepts individual indices, ranges, `.` for the last character, `ALL`, and strides such as `1-7\2`.
- `(STANDARD)` is optional. `(VECTOR)` instead requires one `ORD` or `UNORD` token per character, for example `TYPESET order (VECTOR) = ORD UNORD ORD;` for three characters.
- Multiple type sets, overlaps, named character sets, other transformation types, weights and exclusions are rejected. Unsupported assumptions are never silently discarded. See [Reading NEXUS](./nexus) for the rest of the import format.

The declared `SYMBOLS` order determines adjacency; descriptive state labels do not. Under `--partitioning observed`, ordered characters retain the declared alphabet from its first symbol through the highest observed or explicitly possible state. With `SYMBOLS="0123"`, observations `0`, `2`, `3` therefore require **four** ordered states, including the unobserved intermediate `1`. The same unordered observations form a three-state model.

Missing cells do not define possible states. A declared binary alphabet always keeps both states; otherwise an inferred alphabet with fewer than two states is rejected. If higher unobserved states are biologically possible, choose `--partitioning declared` to use the full alphabet. The Rust `compress_sets` API also accepts explicit alphabets; ordered alphabets must be contiguous and follow declared symbol order.

Partitions group by both state count and ordering. Compression never merges ordered and unordered characters, even when their observations match. They share the tree, clock and gamma shape. `rove inspect` and `run.json` record each partition's ordering, original character indices and state mapping.

## Model and limits

For `k` states, adjacent instantaneous rates are `k / (2(k − 1))`, other off-diagonal rates are zero, and each diagonal is minus its outgoing rate. Stationary frequencies are uniform. This normalizes the expected number of changes per character to one per unit branch distance; two states give the same model as uniform binary Mk.

The Rust engine computes transition matrices by positive uniformization with scaling and squaring, reusing its work buffers. Ordered partitions use the general matrix pruning path; there is no dedicated ordered NEON kernel yet. The direct Mk path remains available to unordered partitions.

Ordered characters support all-character likelihoods and variable-only Mkv conditioning, with strict or relaxed clocks and gamma categories. **Informative-only Mkparsinf is not supported for ordered characters** and is rejected before a run. Choose conditioning to match how the matrix was collected; changing it to bypass an error changes the statistical model. See [character selection](./character-selection).

Checkpoints preserve ordered matrices and cached distances. Resuming still requires the same engine build, input and settings; old-build checkpoints cannot be migrated by this change.

## Validation

The [ordered fixture](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/ordered) records 36 independent matrix exponentials computed with 160-digit arithmetic, an analytic three-state check, and 12 fixed-state comparisons against BEAST 2.7.8 / MM 1.2.1's `Ordinal` model. The maximum absolute log-likelihood difference was `1.82e-9`, across Mk/Mkv, both clocks and three tree scales. Tests also check stochastic matrices, intermediate states, sampler rollback, checkpoint continuation and allocations after setup.

These checks establish numerical agreement at the tested states. They do not establish posterior convergence or a speedup for ordered analyses. BEAST is only used to generate the validation reference; running Rove needs no BEAST installation.
