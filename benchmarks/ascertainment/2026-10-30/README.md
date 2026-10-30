# Cost of informative-character conditioning

This measures the probability correction added in #20, separately from observed
character likelihoods and MCMC. On this Apple M4 Max, updating the correction
along one tip's ancestor path took about **0.96–2.08 µs**, depending on tree size
and state count. A full correction evaluation took **11.64–86.90 µs**. Updates
made no heap allocations. These are not whole-analysis timings or a comparison
with BEAST.

## Workload and method

`benches/informative.rs` constructs deterministic, approximately balanced binary
trees with 90 or 500 tips, two or four states, and four equally weighted rate
categories. Every branch/category distance starts at `0.01 * (category + 1)`.
Each update changes tip zero's first-category distance to
`0.002 + (iteration % 37) * 1e-6` and evaluates the corrected retention
probability, then restores the correction buffers. The full case recalculates
every internal node; the partial case recalculates only that tip's ancestors.
Both include buffer-index storage, the root/category sum and restoration.

Each case warms up for 1,000 updates, times nine batches of 1,000 updates and
measures allocations in a separate untimed batch of 1,000. Three sequential
processes ran after the local builds and tests finished. The table reports the
median of the three process medians, with their range. The JSON files retain all
27 batch means per case.

Hardware: MacBook Pro, Apple M4 Max (12 performance and 4 efficiency cores),
64 GB RAM, macOS 26.6.2. Rust 1.99.1 / LLVM 23.1.1, native aarch64 target,
release/bench profile, fat LTO, one codegen unit, no `target-cpu=native`
override. Background load, CPU affinity, clock frequency and thermal state were
not controlled. `provenance.json` records source/executable hashes and versions.

## Results

| Tips | States | Update | Internal nodes updated | Median µs | Process median range µs | Correction setup bytes |
| ---: | ---: | --- | ---: | ---: | ---: | ---: |
| 90 | 2 | Full | 89 | 11.636 | 11.573–11.649 | 74,640 |
| 90 | 2 | One tip | 7 | 0.962 | 0.953–0.964 | 74,640 |
| 90 | 4 | Full | 89 | 15.533 | 15.362–15.540 | 143,680 |
| 90 | 4 | One tip | 7 | 1.500 | 1.491–1.514 | 143,680 |
| 500 | 2 | Full | 499 | 65.787 | 65.751–65.831 | 415,760 |
| 500 | 2 | One tip | 9 | 1.279 | 1.276–1.291 | 415,760 |
| 500 | 4 | Full | 499 | 86.895 | 86.552–87.001 | 799,680 |
| 500 | 4 | One tip | 9 | 2.084 | 2.055–2.096 | 799,680 |

Setup bytes count allocations owned by the correction, including both buffer
sets, index arrays, disjoint-subset tables and scratch space. They exclude the
tree, observed likelihood core, matrix and benchmark harness. Every timed path
had zero allocations, reallocations and deallocations in its allocation pass.
`tests/allocations.rs` also checks 20,000 actual sampler steps with informative
two-, three- and four-state partitions under each clock preset.

This cost does not depend on the number of observed characters within a
partition. More state-count partitions each require their own correction.
Tree shape and the set of changed branches determine how much can be reused;
the one-tip result does not represent an average MCMC proposal. The observed
likelihood still uses the selected portable/NEON backend; this small recurrence
is shared by both.

## Validation and reproduction

The [fixture and derivation](../../../fixtures/validation/informative/README.md)
describe the probability recurrence and the independent checks. Complete
enumeration agrees with scalar matrix pruning; fixed-state log likelihoods
match BEAST 2.7.8 / MM 1.2.1 within `1.151e-9` in the recorded cases. Portable
and NEON trajectories agree, and pause/crash recovery preserves samples for one
and four independent chains. This work does not include a posterior-distribution
comparison or ESS-per-second measurement for Mkparsinf.

```sh
cargo bench --bench informative
cargo test --test informative --test allocations --test checkpoint
cargo test --release --test informative
```

Run the compiled benchmark in three separate processes to reproduce the
sampling scheme. Keep builds and other tests outside the measurement interval.
The test matrix is constructed for validation; these timings contain no
empirical input data.
