# Multistate likelihood checks

This constructed six-taxon, eight-character matrix exercises three state-count
partitions: three binary characters, four ternary characters and one four-state
character. It includes nonconsecutive symbols, repeated columns after recoding,
missing data, an uncertainty set and unquoted hyphenated names. It is a numerical
test fixture, not a biological dataset or a convergence benchmark.

`beast-likelihoods.json` contains twelve fixed-state evaluations from BEAST 2.7.8
and MM 1.2.1: Mk and Mkv, strict and explicit relaxed clocks, and tree-height
multipliers of 0.01, 1 and 10. The relaxed-clock cases use four midpoint gamma
categories with shape 1. All partitions share the tree and rate variation.
Each Mkv partition adds one excluded constant pattern for each of its states.
The Java exporter uses `BeerLikelihoodCore`, ambiguity masks and scaling.

`tests/partitions.rs` compares each contribution and their total with Rove.
The largest observed absolute difference in log likelihood was
`1.3307612789503764e-9`; the test tolerances are `2e-8` per partition and `3e-8`
for the sum. It also compares changing states against `reference::Problem`,
checks single-partition sums, and exercises rejection, checkpoint restoration
and identical portable/NEON trajectories. `tests/allocations.rs` checks that
partitioned proposals allocate no memory after setup.

State counts include distinct observed symbols and explicitly allowed states
inside uncertainty sets; missing values and gaps do not add states. This follows
the `calcNumberOfStates` convention in MM 1.2.1's
`BeautiMorphModelAlignmentProvider`. Rove preserves a declared binary alphabet
even for constant or missing-only characters; a larger alphabet needs explicit
model states when a column has fewer than two observed/possible states. State
labels in `CHARSTATELABELS` remain descriptive in Rove. Ordered transitions are
outside this check.

Reproduce from the repository root with Java 17 and the pinned packages:

```sh
python3 tools/beast/partition_likelihood.py \
  --packages <package-directory> --launcher <launcher.jar> \
  --output <new-directory>
cargo test --test partitions -- --nocapture
```

The exporter writes the reference values, console output and a provenance
record with package, source, matrix and output hashes. Only the values and
provenance are archived here. No BEAST code is linked into the Rove engine.
