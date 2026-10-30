# Subtree site-repeat evaluation

**Decision: retain the prototype behind `experimental-repeats`, disabled by
default. Do not enable it in the native app.** Packing representatives into the
existing Mk kernels helps some portable full-tree evaluations, but copying and
index maintenance outweigh those savings in the measured NEON workloads and
in complete sampling of the 500-taxon matrix. This closes the evaluation in
[#17](https://github.com/getcortexlabs/rove/issues/17), not a production speedup.

This is an adaptation of an existing method: Kobert, Stamatakis and Flouri,
[Efficient Detection of Repeating Sites to Accelerate Phylogenetic Likelihood
Calculations](https://doi.org/10.1093/sysbio/syw075), Systematic Biology 66(2),
205–217 (2017). There is no novelty claim. The paper discusses both reuse of
repeat likelihoods and compact storage. This prototype retains dense partial
buffers; the results do not establish the performance of a compact
implementation or of other implementations of site repeats.

## What was evaluated

Two globally different characters can have the same observations below a
particular node. For that subtree, their partial likelihoods are identical.
Tip classes compare all state partials exactly. Internal classes are pairs of
child class IDs. An open-addressed scratch table checks full keys, including
tip vectors on hash collisions; hashes never establish equality by themselves.

Each partition owns its core and index. States, category layout and model are
fixed within that core. Branch and category-rate changes do not change the
classes. Topology updates track child node IDs and index versions, rebuilding
only on affected paths. A unique-pattern child makes its parent unique without
hashing. Rejection restores index selectors with partial-buffer selectors;
checkpoint resume reconstructs indices before loading the saved live buffers.

Representatives are gathered into contiguous scratch arrays, evaluated with
the same portable or NEON Mk operation order, rescaled with the same binary
exponents, and expanded into the existing buffers. All scratch memory is
preallocated. The final selective policy allocates an index only at 64 or more
core patterns, and packs a node only if its padded stride is at most half the
ordinary stride. These are experimental cutoffs chosen after a preliminary
run, not universally optimal thresholds.

The [preliminary raw runs](./pilot/) preserve the less selective policy, its
patch and fingerprints. It indexed small cores too and packed whenever one
padded block could be saved. In its single-process pilot, the 20×50 Mkv NEON
full evaluation went from 3.053 to 5.847 µs, and the 90×250 NEON evaluation from
109.115 to 142.320 µs. Those observations motivated the selective policy;
they are not pooled with the repeated measurements below.

## Measurement conditions

- Apple M4 Max, 12 performance and 4 efficiency cores, 64 GB RAM, macOS 26.6.2.
- Rust 1.99.1, LLVM 23.1.1, release profile, fat LTO, one codegen unit,
  default target CPU. No `target-cpu=native` override.
- `dense`: this implementation with the Cargo feature off. `gated`: the same
  source with `--features experimental-repeats`, including its counters.
- Three fresh sequential processes per variant. Variant and backend order
  alternate in the kernel measurements; sampler jobs use a recorded shuffle.
- Kernel entries are the median of three process medians. Each process has
  five calibrated batches of at least 5 ms after 10 ms warmup. Iterations
  within a batch are not independent samples.
- No concurrent build or test jobs were started during timings. Other machine
  activity was not controlled. Power reports, load averages, exact source and
  executable hashes, raw timings and process variation are retained in the
  metadata. The power report named AC power while showing the battery
  discharging; thermal status was unavailable. Small differences are not
  evidence of a reliable improvement.

The 18 kernel workloads cover the selected 20×50 matrix (23 observed patterns,
25 with Mkv constants), 90×250 morphology, 2/3/8 states, one/four categories,
missing data, repeated columns, padded/unpadded layouts and a ladder tree.
Six added cases change 10% or 30% of cells or replace 20% with missing values
on the 20×50 and 90×250 inputs, using seed 2026103017. Percentages are sampling
probabilities. The degraded small cases have 49–50 patterns and therefore stay
dense under the selective policy. Existing globally repeated columns are
compressed before testing subtree repeats.

`core_setup` includes allocation and tip-class construction. `setup_first`
also includes the first traversal and internal-index construction.
Both dispose of the newly constructed core within the measured operation.
`full_cached` traverses the tree with indices already available. `tip_update`
changes a branch and its ancestral path. The two topology metrics exchange
the first and last tips, update the union of their ancestral paths, score,
and either retain or reject the result. They measure index maintenance for
that move, not all possible tree proposals. Whole sampler runs include the
actual proposal mixture.

## Kernel results

Times below are µs per operation. Speedup is dense time divided by prototype
time; below 1 means the prototype is slower. The
[complete table](./kernel-summary.tsv) and [raw batches](./kernel/) include all
216 workload/backend/metric combinations, including slowdowns.

| Workload | Backend | Operation | Dense | Prototype | Speedup |
| --- | --- | --- | ---: | ---: | ---: |
| 20×50 Mkv | NEON | Full | 3.120 | 3.133 | 0.996× |
| 90×250 | Portable | Full | 159.508 | 136.670 | 1.167× |
| 90×250 | NEON | Full | 110.746 | 121.856 | 0.909× |
| 90×250 | NEON | Setup + first evaluation | 370.260 | 524.177 | 0.706× |
| 90×250 | NEON | Tip branch update | 9.478 | 10.238 | 0.926× |
| 90×250 | NEON | Accept tip exchange | 15.602 | 22.827 | 0.683× |
| 90×250 | NEON | Reject tip exchange | 15.619 | 23.006 | 0.679× |
| 90×250, 30% noise | Portable | Full | 158.366 | 135.554 | 1.168× |
| 90×250, 30% noise | NEON | Full | 111.557 | 124.036 | 0.899× |
| 90×250, 20% missing | NEON | Full | 112.047 | 132.676 | 0.845× |
| 90×250, eight states | NEON | Full | 415.997 | 495.120 | 0.840× |
| 90×250, ladder | NEON | Full | 108.975 | 114.419 | 0.952× |

The 90×250 morphology traversal packs 74 of 89 internal nodes and eliminates
78.2% of their combined padded pattern lanes. Nevertheless, its NEON time
increases by 10.0%. Fewer arithmetic operations are not sufficient: full
partial buffers still have to be written, both children are gathered into
scratch arrays, and index bookkeeping is added. The higher topology cost is
consistent with rebuilding indices. These are explanations supported by the
operation structure and counters, not a hardware-counter attribution of every
cycle.

That core allocates 810,934 additional heap bytes for indices and scratch
capacity. The eight-state case adds 958,390 bytes. These counts exclude the
allocator's bookkeeping and existing dense buffers. There are 88 hash-index
builds on the first 90×250 traversal; subsequent branch-only/full cached
traversals perform none. Identity shortcuts are not counted as hash builds.
Per-operation counters before and after timing are included in the raw JSON.
All measured post-setup operations make zero allocation or reallocation calls.
This prototype saves computation at some nodes, not partial-buffer memory.

A separate [default-build check](./default-check.json) compares the shared Mk
kernel extraction with production commit `1b2e5a8`, using the same large-case
settings and seeds in twelve shuffled runs. All six before/after pairs have
identical traces and trees. NEON medians are 6.688 s before and 6.706 s after
(+0.27%); portable medians are 8.428 s and 8.049 s. This check found no material
default-path regression; it is not a speedup claim or pooled with the feature
comparison. Metadata `source_commit` names the parent commit of the experiment;
individual source hashes describe the modified files used for the builds.

## Complete sampler runs

Each variant/backend ran three seeds, 2026103017–2026103019: 36 runs total.
Both small cases use two million steps and sample every 1,000; the 500×759
matrix uses 20,000 steps and samples every 100. All use the reference model
with four gamma categories and a relaxed lognormal clock. The small cases
constrain T1–T19 to be monophyletic. The large case is unconstrained and uses
180 binary, 140 ternary and 439 four-state characters with separate Mkv
corrections. Input hashes and settings are in [sampler.json](./sampler.json).
Thiago's original matrix and sampled trees remain local.

| Case | Backend | Dense median, s | Prototype median, s | Change in time |
| --- | --- | ---: | ---: | ---: |
| 20×50 Mk | Portable | 3.479 | 3.581 | +2.9% |
| 20×50 Mk | NEON | 2.754 | 2.785 | +1.1% |
| 20×50 Mkv | Portable | 3.718 | 3.832 | +3.1% |
| 20×50 Mkv | NEON | 2.986 | 3.024 | +1.3% |
| 500×759 Mkv | Portable | 8.675 | 9.441 | +8.8% |
| 500×759 Mkv | NEON | 7.308 | 8.155 | +11.6% |

After removing command/provenance header lines, trace and tree files match
byte for byte across all four variant/backend combinations in all nine
case/seed groups. Thus the same samples, ESS values and clade frequencies are
being compared here. Minimum ESS is 51.1–84.2 for small Mk, 44.2–62.5 for small
Mkv, and 2.44–2.49 for the large case after 10% burn-in. These short runs do
not establish convergence. In particular, a seven-second large run is not a
completed biological analysis.

For the large case, median minimum ESS per second falls from 0.334 to 0.305
with NEON and from 0.281 to 0.263 with the portable backend. These are medians
of per-run ESS/time, so their ratios need not equal ratios of median times.
Peak process RSS, measured by `wait4` over the entire process including final
checkpoint writing, rises from 523.7 to 551.2 MiB with NEON and from 525.1 to
551.3 MiB with the portable backend. RSS is not a measurement of live kernel
storage alone. Individual large runs vary substantially; all are retained.

## Correctness and scope of the decision

The benchmark checks every workload on both available backends against the
independent scalar `reference::Problem`, including accepted and rejected tip
exchanges. Tests compare every partial and pattern log likelihood with dense
updates for 2, 3, 4 and 8 states, one/four categories, unequal frequencies,
missing and uncertain observations, changed branch rates, changing roots,
and scaling on/off. Non-NaN values must have identical bits. Additional tests
cover changed class membership with an unchanged class count, a 1,024-tip
ladder requiring cumulative scaling, and exact checkpoint continuation with
two 128-pattern partitions under Mk and Mkv. Sampler steps with active repeat
indices are also checked for zero allocations.

Keep the prototype and its tests for reproducibility, but reject enabling this
packing approach by default. A future attempt would need to reduce data
movement, potentially with compact per-node partials and kernels that consume
those layouts directly. That would require new measurements and validation;
adjusting a cutoff alone did not make the present approach worthwhile on the
target NEON workloads. No BEAST run or M1 measurement was part of this study.

## Reproduction

Build two CLI binaries and the same harness from this checkout, copying each
artifact before rebuilding with the other feature set:

```sh
cargo build --release --locked
cargo bench --locked --bench repeats --no-run --message-format=json
cargo build --release --locked --features experimental-repeats
cargo bench --locked --features experimental-repeats --bench repeats --no-run --message-format=json
```

Use the `compiler-artifact.executable` path from each bench build. Name the
copies `rove-dense`, `bench-dense`, `rove-gated`, and `bench-gated`, then run:

```sh
python3 tools/benchmarks/repeats.py --binaries /path/to/copies \
  --output /path/to/new-kernel-results --phase kernel
python3 tools/benchmarks/repeats.py --binaries /path/to/copies \
  --output /path/to/new-sampler-results --phase sampler \
  --large-matrix /path/to/500_zilla.nex
cargo test --features experimental-repeats
cargo bench --features experimental-repeats --bench repeats -- --check
```

The sampler driver requires Unix `wait4` and an ARM build exposing both
backends. The validation harness also runs on x86_64 with its portable backend.
The private large matrix is optional for kernel measurements; it is not
redistributed here.
