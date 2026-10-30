# Ordered-character likelihood checks

This constructed six-taxon, eight-character fixture reuses the observations in
`../multistate/matrix.nex`, marking characters 2, 3, 5 and 8 as ordered. It is a
numerical test, not a biological dataset or a convergence benchmark. The default
partitioning produces:

| Characters (one-based) | Model | States |
| --- | --- | --- |
| 1, 4, 6 | Unordered Mk | 2 |
| 7 | Unordered Mk | 3 |
| 2, 8 | Ordered | 3 |
| 3, 5 | Ordered | 4 |

Character 5 observes `0`, `2` and `3`. Its ordered model must retain intermediate
state `1`; removing it would change the process. Missing observations and an
uncertainty set are also present. Ordered alphabets use the declared prefix
through the highest observed or explicitly possible state. The reference exporter
receives the ordered character indices explicitly, independently of Rove's NEXUS
parser.

## Matrix exponential

The rate matrix has adjacent entries `k / (2(k - 1))`, zero nonadjacent entries
and a diagonal equal to minus each row's outgoing rate. Uniform stationary
frequencies give one expected change per unit distance.

`matrices.json` contains 36 independently computed matrix exponentials: 2, 3, 4
and 8 states, with distances 0, 1e-14, 1e-9, 0.001, 0.1, 1, 10, 10000 and 1000000.
`tools/beast/ordered_matrices.py` uses Python's standard-library Decimal with
160-digit precision, a dense Taylor series of Q and scaling and squaring. It
does not call Rove or use the production algorithm's positive uniformization.
Values are archived as decimal strings; the Rust test compares them after
conversion to f64, with relative tolerance `3e-13` plus `1e-300` absolute.

For three states, let `a = exp(-3t/4)` and `b = exp(-9t/4)`. The analytic matrix
has endpoint diagonals `1/3 + a/2 + b/6`, adjacent entries `(1-b)/3`, opposite
entries `1/3 - a/2 + b/6`, and centre diagonal `(1+2b)/3`. This is checked
separately, as are binary Mk equivalence and nonnegative, symmetric, stochastic
matrices up to 64 states at distances from `1e-300` through infinity.

## BEAST reference

`beast-likelihoods.json` contains twelve fixed-state evaluations from BEAST 2.7.8
and MM 1.2.1, using MM's `Ordinal` substitution model for ordered partitions and
`LewisMK` for unordered partitions. Both use equal stationary frequencies and
BEAST's normalization to one expected change per unit distance.

The cases cover unconditioned and variable-only likelihoods, strict and explicit
relaxed clocks, and tree-height multipliers of 0.01, 1 and 10. Relaxed-clock cases
use four midpoint gamma categories with shape 1. Partitions share the tree and
rate variation. Each Mkv partition adds one excluded constant pattern per state.
The Java exporter uses `BeerLikelihoodCore`, ambiguity masks and scaling.

The maximum absolute difference in log likelihood was `1.8184209693572484e-9`.
Tests require `2e-8` per partition and `3e-8` for the total, on portable and NEON
backend configurations. Ordered partitions currently use general matrix pruning
in either configuration; unordered partitions retain their Mk backend.

`tests/ordered.rs` also compares sampler states to `reference::Problem`, checks
accepted and rejected proposals, and requires identical continuation after a
checkpoint. The CLI checkpoint test covers pause, interruption, uncommitted
output tails and resumption for one and four chains. Allocation tests require
zero allocations during proposals after initialization for both clock models
and both supported conditioning modes. Ordered Mkparsinf is explicitly rejected.

Reproduce from the repository root, with Java 17 and the pinned BEAST packages
only for regenerating the external oracle:

```sh
python3 tools/beast/ordered_matrices.py --check
python3 tools/beast/partition_likelihood.py \
  --packages <package-directory> --launcher <launcher.jar> \
  --matrix fixtures/validation/ordered/matrix.nex \
  --conditioning comparison --ordered 2,3,5,8 --output <new-directory>
cargo test --test ordered -- --nocapture
cargo test --test allocations --test checkpoint --test cli
```

`provenance.json` records package versions, Java/platform details, source, matrix
and output hashes, and the reproduction command. The modified exporter also
reproduces every archived unordered partition contribution unchanged. These
checks establish fixed-state numerical agreement, not convergence or an ordered
performance claim. BEAST remains outside the standalone engine.
