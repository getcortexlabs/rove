# Rust likelihood baseline: Apple M4 Max

The unchanged generic Mk kernel was measured on 11 workloads in three fresh,
sequential processes. The 90 × 250 simulated matrix took a median **258.326 µs**
for a full evaluation with cached branch matrices, and **26.983 µs** for a
deepest-tip branch update affecting seven internal nodes. These are kernel
operations, not MCMC steps or complete inference times.

The full-evaluation process medians for that matrix ranged from **236.198 to
299.991 µs**. Some controlled cases varied much more: binary/four-rate full
calculations ranged from 247.107 to 578.436 µs. This is an initial baseline on
an uncontrolled workstation, not evidence for small performance differences.
No comparisons with BEAST or BEAGLE runtime were made.

## Configuration and numerical check

- MacBook Pro, Apple M4 Max, 12 performance and 4 efficiency cores, 64 GB RAM;
  macOS 26.6.2, build 25G83; ARM64.
- Rust 1.99.1, LLVM 23.1.1; `target-cpu=native` resolves to `apple-m4`.
  Optimization level 3, fat LTO, one codegen unit, no incremental compilation,
  debug assertions or overflow checks.
- One thread, one process at a time, no CPU affinity or frequency controls.
  Background load was not controlled. The power report said AC power while
  the battery was discharging; thermal-status queries were unavailable.
- `Scaling::Always`; observed-data Mk likelihood only. No Mkv correction,
  priors, proposal acceptance calculation, topology moves or chain logging.
- Measured source: `d5dcb27` (`Add reproducible likelihood benchmarks`).
  Metadata records full hashes of source files, inputs, executable and raw runs.

The reference 20 × 50 case uses the archived BEAST 2.7.8 tree, branch rates and
four gamma categories for seed 20261025. It contains 23 observed patterns and
excludes the two zero-weight ascertainment patterns. Its unconditioned Mk
value is checked before timing; it is not the XML's Mkv value of −198.611.

Rove: `-433.28114529381804`; weighted BEAST pattern logs: `-433.28114529381799`; absolute difference: `5.68e-14`.

Every workload also passes the independent scalar reference and partial-update,
acceptance and rejection-restoration checks within an absolute `1e-9` tolerance.

The initial hardware queries through `sysctl` were denied by the environment.
The report's hardware supplement was collected after timing with
`system_profiler` and `rustc --print target-cpus`. The collector was then updated
to support that fallback and remove battery identifiers. These changes do not
alter the benchmark executable or measurements. Original local diagnostics
were retained; published metadata contains only the selected hardware fields.

## Main workloads

Values are microseconds per operation: **median of the three process medians**,
with their minimum–maximum in parentheses. Each process used 15 measured
batches per metric after 100 ms warm-up and calibration to at least 20 ms per
batch. Individual iterations within a batch are not independent samples.

| Operation | Reference 20 × 50, 23 patterns | Simulated 90 × 250, 250 patterns |
| --- | ---: | ---: |
| Parse in-memory NEXUS | 28.259 (26.328–39.686) | 418.857 (406.859–519.303) |
| Compress observations and build tip inputs | 15.912 (15.513–21.740) | 517.115 (497.222–625.875) |
| Allocate/populate/drop core | 10.389 (9.902–12.645) | 267.412 (264.670–337.572) |
| Full tree, cached matrices | 7.836 (6.115–8.304) | 258.326 (236.198–299.991) |
| Rebuild all matrices and evaluate | 8.217 (7.255–8.587) | 271.466 (239.960–272.256) |
| Deepest-tip branch update | 2.457 (2.125–2.698) | 26.983 (24.883–34.657) |
| Internal branch directly below root | 0.772 (0.605–0.917) | 10.337 (10.267–23.397) |
| Forced accepted tip proposal | 2.735 (2.155–2.987) | 25.211 (25.124–41.701) |
| Forced rejected tip proposal | 2.576 (2.170–2.959) | 25.558 (24.842–42.790) |
| Store buffer indices | 0.011 (0.010–0.012) | 0.008 (0.008–0.012) |
| Store and restore indices | 0.010 (0.009–0.012) | 0.013 (0.013–0.024) |

The reference tip update touches six internal nodes; the 90-taxon balanced
case touches seven. The internal-branch update touches only the root in both.
All these numerical paths still perform root integration and scaling recovery.
Setup timings include destruction of the temporary result and use warmed
allocator/cache state. They are not cold process-start or disk-read latency.

Do not subtract separately timed operations to estimate tiny component costs.
For example, acceptance, rejection and raw update medians were recorded at
different moments, and their ordering can change under the observed noise.
Bookkeeping timings also include loop/dispatch/black-box overhead.

## Controlled cases: full evaluations

All use uniform state frequencies, `Scaling::Always`, and branch distances
0.1. Four-category synthetic cases use rates `[0.25, 0.75, 1.25, 1.75]`, each
weighted 0.25; these are a controlled mixture, not fitted gamma rates.

| Case | Taxa × characters | Patterns / stride | States / categories | Shape | Full evaluation, µs (range) |
| --- | ---: | ---: | ---: | --- | ---: |
| `binary_one_rate` | 90 × 250 | 250 / 256 | 2 / 1 | balanced | 121.320 (114.031–125.415) |
| `binary_four_rates` | 90 × 250 | 250 / 256 | 2 / 4 | balanced | 327.786 (247.107–578.436) |
| `ternary_four_rates` | 90 × 250 | 250 / 256 | 3 / 4 | balanced | 439.492 (411.051–705.130) |
| `eight_states` | 90 × 250 | 250 / 256 | 8 / 4 | balanced | 2162.523 (1725.349–2280.570) |
| `missing_20_percent` | 90 × 250 | 250 / 256 | 2 / 4 | balanced | 326.222 (284.643–368.697) |
| `repeated_columns` | 90 × 250 | 25 / 32 | 2 / 4 | balanced | 42.896 (40.714–55.090) |
| `ladder` | 90 × 250 | 250 / 256 | 2 / 4 | ladder | 239.113 (234.075–300.730) |
| `small_unpadded` | 5 × 16 | 16 / 16 | 2 / 4 | balanced | 1.087 (1.073–1.292) |
| `small_padded` | 5 × 17 | 17 / 24 | 2 / 4 | balanced | 1.580 (1.513–1.891) |

The missing-data case has 4,462 missing cells out of 22,500 (19.83%). The repeated
case retains 250 characters through 25 patterns of weight ten. Those are distinct
workloads; the table does not estimate compression speedup by comparing different
data. Five-taxon cases enumerate 16 or 17 distinct columns to expose stride padding.

Tree shape changes which cached partials can be reused. In the ladder case,
changing the deepest tip requires 89 internal updates, rather than seven on
the balanced tree. Its tip-update measurement was 301.666 (232.269–354.680) µs.
This gives future optimizations a concrete workload where a local change still
requires work throughout the tree.

## Allocation observations

No allocation, reallocation or deallocation calls were observed in any of the
eight numerical/bookkeeping metrics, in any of the 11 cases or three processes.
This is one separately instrumented operation per metric and process, not a
trace of every timed iteration.

Setup remains allocation-heavy. The table records one operation; counters were
identical across the three processes for these rows. Bytes are total requested
allocation sizes plus new sizes requested by reallocations, not peak live memory
or RSS. Timing uses counting disabled, but the forwarding allocator's flag check
remains on allocation/deallocation calls.

| Workload | Operation | Allocations | Reallocations | Requested bytes |
| --- | --- | ---: | ---: | ---: |
| `reference_20x50` | `parse` | 180 | 163 | 136,080 |
| `reference_20x50` | `compress` | 80 | 90 | 32,404 |
| `reference_20x50` | `core_setup` | 247 | 0 | 151,716 |
| `morphology_90x250` | `parse` | 597 | 1023 | 2,318,105 |
| `morphology_90x250` | `compress` | 354 | 648 | 962,596 |
| `morphology_90x250` | `core_setup` | 1087 | 0 | 6,683,244 |

The current benchmark supports keeping setup separate and reusing the initialized
core. It does not measure cache misses or memory bandwidth, and it does not prove
that allocations dominate total inference time.

## Reproduce and inspect

From a clean checkout, with an output directory that does not exist:

```sh
python3 tools/benchmarks/prepare.py --check
cargo test --locked --test benchmark
python3 tools/benchmarks/run.py --output .local/new-benchmark-run
```

The [protocol](../../README.md) specifies all workload and metric definitions.
The runner records the current revision, so use measured commit `d5dcb27` to
recover the original harness exactly. Subsequent collector changes only improve
hardware metadata and privacy; the benchmark source and numerical core are unchanged.

Files retained here:

- `metadata.json`: environment, commands, source/input/executable hashes, power
  reports, collection notes and raw-file checksums.
- `run-1.tsv`, `run-2.tsv`, `run-3.tsv`: all 5,445 timed batch observations, with
  per-operation allocation counters measured separately.
- `summary.json`: per-process medians, their range, pooled batch p10/p90 and
  the workload configurations. Percentiles describe batch means, not call latencies.

Future matched runs must use the same workloads and settings. The next numerical
step is Mkv correctness (#14), followed by direct Mk pruning (#4) and NEON (#5).
End-to-end runtime, ESS per time, tree diagnostics and supported cross-engine
comparisons remain open in #8 until the required inference models exist.
