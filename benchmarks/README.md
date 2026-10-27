# Likelihood benchmarks

The Rust harness measures the current generic Mk pruning implementation. It
does not run MCMC, compute priors or apply Mkv ascertainment correction. BEAST
is not needed to execute it. No cross-engine performance comparison is implied.

The first [Apple M4 Max baseline](rove/2026-10-26/README.md) includes all raw
samples, timing variability, allocation observations and reproduction commands.

## Run

Use a clean checkout with Rust, Cargo and Python 3.9 or newer. The driver builds
once, validates every numerical path, then runs three fresh processes in
sequence. Choose an output directory that does not exist:

```sh
cargo test --locked --test benchmark
cargo bench --locked --bench likelihood -- --check
python3 tools/benchmarks/prepare.py --check
python3 tools/benchmarks/run.py --output .local/benchmark-run
```

`run.py` sets `target-cpu=native`, optimization level 3, fat LTO, one codegen
unit, no incremental compilation, and no debug assertions or overflow checks.
The build targets the compiler's host triple. It records the exact compiler,
flags, source commit, source hashes and executable hash. The core has no
explicit SIMD path yet; LLVM can still auto-vectorize ordinary Rust loops.

The executable can also run a single case for development:

```sh
cargo bench --locked --bench likelihood -- \
  --case morphology_90x250 --samples 15 --sample-ms 20 --warmup-ms 100
```

That command uses the caller's build environment. Use the Python driver for
the recorded configuration, rather than mixing results from different flags.

## What is measured

All numerical operations use `Scaling::Always`, one thread and precompressed
observations. No file I/O, result formatting or logging occurs inside a timed
operation. Loading the input text is outside the parse timing.
Setup is repeated with warmed allocator/cache state; it does not measure cold
process startup or disk-read latency.

| Metric | Work per operation |
| --- | --- |
| `parse` | Parse the in-memory NEXUS text; drop the resulting matrix. |
| `compress` | Compress an already parsed matrix as one explicit partition; build tip partials and drop the result. |
| `core_setup` | Allocate the core and scratch matrix; populate tips and all branch matrices; drop them. Does not prune. |
| `full_cached` | Recalculate every internal node and integrate the root, using existing branch matrices. |
| `full_with_matrices` | Alternate all branch distances between 90% and 110% of their baseline; rebuild matrices and perform a full evaluation. |
| `tip_update` | Change a deepest tip's branch distance, rebuild its matrices, update its ancestors and integrate the root. |
| `internal_update` | Change an internal branch immediately below the root; rebuild its matrices, update the root and integrate. Descendant partials are reused. |
| `accept_proposal` | Store buffer indices, switch the affected buffers, perform a deepest-tip update and retain it. |
| `reject_proposal` | The same proposal, followed by restoring the saved buffer indices. |
| `store` | Copy current partial and matrix buffer indices into the stored arrays. |
| `store_restore` | Store indices and restore them, without calculating likelihoods. |

Accepted and rejected outcomes are forced to isolate costs. They contain no
proposal generation, Hastings ratio, prior evaluation, acceptance test, tree
topology changes or chain logging. They are not MCMC iterations. Branch
distances are independent kernel inputs; the updates are not clock operators.

Each metric gets a separate initialized core. Timing includes the harness loop,
dispatch and `black_box` barriers; no overhead is subtracted. Very small
bookkeeping measurements should be read with that limitation in mind.

## Workloads and correctness

- `reference_20x50` imports the selected matrix and the frozen tree, branch
  rates and four gamma categories from the BEAST 2.7.8 state with seed
  20261025. It uses 23 observed patterns. The weighted, **unconditioned** Mk
  log likelihood is checked against the exported pattern likelihoods within
  an absolute tolerance of `1e-9`. The two zero-weight ascertainment patterns
  are excluded. This value differs from the XML's Mkv likelihood.
- `morphology_90x250` imports the [generated matrix](../fixtures/benchmarks/morphology_90x250.nex).
  It was simulated under binary Mk on a balanced tree, distance 0.1 per branch,
  one rate, and SplitMix64 seed 20261026. Constant characters were retained.
  The benchmark evaluates it with four equally weighted rates
  `[0.25, 0.75, 1.25, 1.75]`, a controlled mixture rather than fitted gamma rates.
- The controlled 90 × 250 cases use reproducible independent states: binary
  with one or four rates, ternary, eight states, approximately 20% missing
  cells, tenfold repeated columns, and a ladder tree. They are arithmetic
  workloads, not inferred biological datasets. Random draws use SplitMix64
  seed 20261026; the source specifies the mapping to states and missing cells.
- Two five-taxon cases enumerate 16 or 17 unique binary columns, exercising
  strides of 16 and 24. They expose the effect of padded pattern storage.

Generated cases use uniform state frequencies. Four-rate cases use the mixture
above; single-rate cases use rate 1. Every generated branch has distance 0.1.
Tree shapes are fixed, not optimized. Counts distinguish original characters,
observed patterns, padded stride and the number of dirty internal nodes.

Before timing, every case is compared with the independent scalar reference.
Partial updates, accepted proposals, rejection restoration and full matrix
updates are also checked. CI runs those checks, verifies generated fixtures,
and tests report aggregation; CI timings are not benchmark results.

`tools/benchmarks/prepare.py` regenerates the simulated NEXUS and the numeric
Rust fixtures derived from the archived BEAST JSON: the benchmark state and
both final states used by the Mkv tests. The generated Rust files are data,
with node indices remapped into postorder. `--check` detects stale copies.

## Timing and allocations

Defaults: 100 ms of warm-up per metric, calibration to a batch lasting at least
20 ms, then 15 timed batches at the calibrated iteration count. Repeat in three
fresh processes. Warm-up and calibration are excluded. Durations come from
`Instant`; each batch records its elapsed nanoseconds and iteration count.

Each sample is a **batch mean per operation**, not a single call's latency.
The summary reports the median of each process's sample medians, the range of
those process medians, and pooled batch p10/p90. It does not treat the millions
of inner-loop iterations as independent samples or claim confidence intervals.
Calibration batches may exceed the requested duration because sizes double.

A benchmark-only allocator forwards to Rust's system allocator. One untimed
operation per metric counts allocation, reallocation and deallocation calls,
plus requested bytes. Requested bytes sum allocation sizes and new reallocation
sizes; they are not live memory, peak RSS or cache traffic. Counts are observed
allocator calls, not a guarantee about optimizations the compiler may perform.

Counting is disabled during timings, but the forwarding allocator still checks
an atomic flag on allocation/deallocation. That overhead remains in setup
measurements. No allocator instrumentation is installed in the Rove library.
The only new unsafe code is the forwarding implementation required by Rust's
`GlobalAlloc` API, with arguments and return values passed through unchanged.

The driver records hardware, OS, available power/thermal reports, commands and
hashes. It does not pin CPU cores, fix clock frequencies or suppress background
work. Results describe the recorded machine and configuration. JVM warm-up and
BEAGLE thread settings do not apply to these Rust-only measurements. A future
cross-engine comparison must match supported calculations on the same hardware.

## Outputs

`metadata.json` records provenance and settings. `run-*.tsv` retains the raw
batches and allocation observations; `summary.json` contains derived summaries.
`build.log` is a local diagnostic. Version a report with the compact result
files, keeping machine identifiers and unrelated local logs out of the commit.

Full-inference runtime, ESS per time and tree diagnostics remain pending the
standalone samplers in #15 and #16. Issue #8 stays open for those measurements
and equivalent cross-engine comparisons.
