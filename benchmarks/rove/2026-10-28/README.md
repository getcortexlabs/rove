# Direct Mk pruning: Apple M4 Max

Both pruning paths were measured in the same three processes, each metric on
the matrix path and the Mk path back to back. A full evaluation of the selected
20 × 50 reference took a median **5.963 µs** with matrix products and
**4.780 µs** with the Mk closed form. In every process the matrix path took
**1.25** times as long. The ratio was 1.11 for the binary 90 × 250 case with
four rate categories, 1.35 for three states and 2.66 for eight. With one rate
category, the binary case showed no difference.

Parsing and compression do the same work on both paths. Their ratios stayed
between 0.97 and 1.03, which bounds the noise of this paired comparison. These
are kernel operations, not MCMC steps or complete inference times. No BEAST or
BEAGLE runtime was measured.

## Configuration and numerical check

- MacBook Pro (Mac16,6), Apple M4 Max, 12 performance and 4 efficiency cores,
  64 GB RAM; macOS 26.6.2, build 25G83; ARM64.
- Rust 1.99.1, LLVM 23.1.1; `target-cpu=native` resolves to `apple-m4`.
  Optimization level 3, fat LTO, one codegen unit, no incremental compilation,
  debug assertions or overflow checks.
- One thread, one process at a time, no CPU affinity or frequency controls.
  Background load was not controlled. Load averages over 1, 5 and 15 minutes
  were 23.0, 24.2 and 38.5 before the runs and 16.3, 23.4 and 32.4 after. The
  power report said AC power, with the battery charging.
- `Scaling::Always`. Mk likelihood of the observed patterns; the
  `reference_20x50_mkv` case also applies the Mkv correction. No priors,
  proposal generation, topology moves or chain logging.
- Measured source: `a5c0c9a` (`Prune Mk branches with the closed form`).
  Metadata records full hashes of source files, inputs, executable and raw runs.

Before timing, every case was compared on both paths with the independent
scalar reference. The check covered partial updates, forced acceptance,
rejection restoration and full branch rebuilds, within an absolute `1e-9`.
Both paths give `-433.28114529381804` for the unconditioned reference.
For the Mkv case they give `-198.61115564792343`, against `-198.61115564792456`
in the XML's archived state.

## Full evaluations

Values are microseconds per operation, the median of the three process medians.
The ratio divides the matrix path's median by the Mk path's, process by process;
the table gives the median of the three ratios and their range. Each process
measured 15 batches per metric and path, after 100 ms of warm-up and
calibration to at least 20 ms per batch.

| Case | Patterns / stride | States / categories | Matrices, µs | Mk, µs | Ratio (range) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `reference_20x50` | 23 / 24 | 2 / 4 | 5.963 | 4.780 | 1.25 (1.25–1.25) |
| `reference_20x50_mkv` | 23 + 2 / 32 | 2 / 4 | 7.100 | 5.715 | 1.24 (1.24–1.25) |
| `morphology_90x250` | 250 / 256 | 2 / 4 | 230.285 | 203.568 | 1.12 (1.11–1.14) |
| `binary_one_rate` | 250 / 256 | 2 / 1 | 108.991 | 108.012 | 1.00 (0.98–1.04) |
| `binary_four_rates` | 250 / 256 | 2 / 4 | 231.203 | 210.612 | 1.11 (1.08–1.11) |
| `ternary_four_rates` | 250 / 256 | 3 / 4 | 371.113 | 270.727 | 1.35 (1.35–1.39) |
| `eight_states` | 250 / 256 | 8 / 4 | 1,540.661 | 575.818 | 2.66 (2.65–2.68) |
| `missing_20_percent` | 250 / 256 | 2 / 4 | 229.817 | 206.712 | 1.10 (1.10–1.13) |
| `repeated_columns` | 25 / 32 | 2 / 4 | 35.880 | 28.790 | 1.25 (1.22–1.25) |
| `ladder` | 250 / 256 | 2 / 4 | 225.865 | 201.866 | 1.12 (1.12–1.13) |
| `small_unpadded` | 16 / 16 | 2 / 4 | 1.066 | 0.865 | 1.23 (1.22–1.24) |
| `small_padded` | 17 / 24 | 2 / 4 | 1.292 | 1.012 | 1.28 (1.28–1.29) |

`23 + 2` counts the observed patterns and the two constant patterns of the
Mkv correction.

## Updates and proposals

Time ratios of the matrix path over the Mk path, as above:

| Case | Rebuild all branches | Deepest tip | Below the root | Accepted proposal | Rejected proposal |
| --- | ---: | ---: | ---: | ---: | ---: |
| `reference_20x50` | 1.38 (1.38–1.39) | 1.26 (1.26–1.26) | 1.19 (1.19–1.20) | 1.28 (1.27–1.28) | 1.27 (1.27–1.27) |
| `reference_20x50_mkv` | 1.35 (1.32–1.35) | 1.25 (1.23–1.25) | 1.19 (1.18–1.19) | 1.25 (1.24–1.25) | 1.25 (1.24–1.26) |
| `morphology_90x250` | 1.15 (1.14–1.16) | 1.10 (1.08–1.10) | 1.04 (1.04–1.04) | 1.09 (1.09–1.10) | 1.09 (1.08–1.12) |
| `binary_one_rate` | 1.05 (1.03–1.07) | 1.03 (1.03–1.05) | 1.01 (1.00–1.03) | 1.03 (1.03–1.04) | 1.04 (1.04–1.04) |
| `binary_four_rates` | 1.15 (1.13–1.16) | 1.10 (1.10–1.11) | 1.03 (1.02–1.04) | 1.11 (1.10–1.12) | 1.11 (1.07–1.13) |
| `ternary_four_rates` | 1.39 (1.36–1.42) | 1.29 (1.28–1.29) | 1.11 (1.10–1.12) | 1.29 (1.28–1.29) | 1.30 (1.28–1.31) |
| `eight_states` | 2.73 (2.71–2.75) | 2.47 (2.46–2.49) | 1.71 (1.71–1.72) | 2.46 (2.21–2.46) | 2.47 (2.43–2.48) |
| `missing_20_percent` | 1.14 (1.12–1.17) | 1.10 (1.10–1.11) | 1.04 (1.03–1.04) | 1.09 (1.08–1.10) | 1.10 (1.10–1.11) |
| `repeated_columns` | 1.32 (1.30–1.33) | 1.18 (1.14–1.19) | 1.08 (1.03–1.08) | 1.18 (1.14–1.19) | 1.19 (1.15–1.20) |
| `ladder` | 1.14 (1.12–1.14) | 1.14 (1.12–1.15) | 1.04 (1.04–1.05) | 1.10 (1.10–1.14) | 1.14 (1.14–1.17) |
| `small_unpadded` | 1.34 (1.33–1.36) | 1.25 (1.23–1.25) | 1.22 (1.20–1.23) | 1.23 (1.23–1.26) | 1.24 (1.23–1.25) |
| `small_padded` | 1.38 (1.36–1.42) | 1.28 (1.28–1.29) | 1.25 (1.25–1.28) | 1.31 (1.30–1.31) | 1.30 (1.28–1.31) |

On the Mk path, the reference took 5.259 µs to rebuild every branch and
evaluate, 1.670 µs to update the deepest tip, which touches six internal nodes,
0.507 µs for the branch below the root and 1.677 µs for a rejected proposal.

Storing and restoring buffer indices is the same code on both paths. Those
ratios stayed between 0.89 and 1.13, which reflects the resolution of
operations that take 5 to 15 ns.

## Setup and allocations

Setup ratios were between 0.97 and 1.09. An Mk core makes one more allocation,
its copy of the model, and requests fewer bytes, since each branch holds three
values per rate category instead of k². For the reference that is 149,236
requested bytes against 151,716; for eight states, 24,281,036 against
24,979,788. No allocation, reallocation or deallocation was observed in any
numerical or bookkeeping metric on either path.

## Reading the ratios

With two states, the two paths perform about the same number of floating-point
operations per pattern and rate category: four products per child through the
matrix, against a weighted sum and one combination per state through the closed
form. The Mk path still makes fewer passes over the child partials and the
accumulators. With eight states, a parent update takes 136 multiplications per
pattern and category through matrices and about 40 through the closed form.

Cases with few patterns gained more than the binary cases with 250 patterns
and four rate categories: 1.23 to 1.28 for 16 to 25 patterns, against 1.10 to
1.12.

Every node is rescaled the same way on both paths, with one logarithm per
pattern. In a side run with `Scaling::None` in `benches/support/workload.rs`,
which is not part of the files here, full evaluations on the matrix path took
3.832 µs for the reference, 126.168 µs for `binary_four_rates` and 32.624 µs
for `binary_one_rate`, against 5.963, 231.203 and 108.991 µs above. Without
rescaling, the ratios were 1.51, 1.28 and 1.23. Rescaling is a large share of
every evaluation in this kernel, and the one-rate case shows it most. No other
rescaling strategy is implemented, so this is not a measurement of one.

Absolute times differ from the [2026-10-26 baseline](../2026-10-26/README.md),
which recorded 7.836 µs for the same reference evaluation. The executable,
power state and background load differ between the two runs. Compare the paths
within this report, not across reports.

## Reproduce and inspect

From a clean checkout at or after `a5c0c9a`, with an output directory that does
not exist:

```sh
python3 tools/benchmarks/prepare.py --check
cargo test --locked --test benchmark
cargo bench --locked --bench likelihood -- --check
python3 tools/benchmarks/run.py --output .local/new-benchmark-run
```

The [protocol](../../README.md) defines the workloads, metrics and the order of
the two paths.

Files retained here:

- `metadata.json`: environment, commands, source/input/executable hashes, power
  reports, load averages and raw-file checksums.
- `run-1.tsv`, `run-2.tsv`, `run-3.tsv`: all 11,880 timed batch observations,
  with per-operation allocation counters measured separately.
- `summary.json`: per-process medians, their range, pooled batch p10/p90 and
  the configuration of each case and path.
- `comparison.json`: the per-process ratios of the two paths for every case and
  metric, with their median and range.

End-to-end runtime, ESS per time, tree diagnostics and supported cross-engine
comparisons remain open in #8 until the minimal sampler (#15) and the inference
models exist.
