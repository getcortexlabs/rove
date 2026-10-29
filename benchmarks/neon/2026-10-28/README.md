# NEON kernel: Apple M4 Max

The portable backend and the NEON backend were measured in the same four
processes, each metric on both back to back. A full evaluation of the 20 × 50
reference on the Mk path took a median **4.733 µs** with the portable code and
**3.206 µs** with the NEON kernel: the portable code took **1.48** times as
long, and 1.47 to 1.49 times in each process. Among the 90 × 250 cases with
250 patterns, the ratio was **1.10** for binary characters with one rate
category, and between **1.26** and **1.32** with four categories, three or
eight states, missing cells or a ladder tree; it was **1.43** with tenfold
repeated columns, which leave 25 patterns. Ten-million-step runs of the
sampler on the reference matrix took **1.08** to **1.11** times as long with
the portable code, and both backends wrote the same traces and trees.

## What was measured

In the linked binary, LLVM already vectorises the portable loops of the
kernel, two lanes per register and four registers per iteration. Rewriting
those loops one by one with NEON intrinsics was slower than the compiler's
version in a development run, and was dropped. The NEON backend instead runs
the Mk node update as one pass over blocks of eight patterns: the weighted sums
of each child stay in registers instead of being written to and reread from an
accumulator for every state, and when the core rescales, the kernel also keeps
each pattern's running maximum, so rescaling does not reread the results to
find it. The matrix path, the logarithms of rescaling and the root computation
run the same portable code on both backends.

Per pattern, the kernel performs the same floating-point operations in the same
order as the portable code, without fused multiply–add. Every value that is not
NaN is therefore identical; the tests in `tests/equality.rs`, `tests/sampler.rs`
and `tests/cli.rs` require it.

## Configuration

- MacBook Pro (Mac16,6), Apple M4 Max, 12 performance and 4 efficiency cores,
  64 GB RAM; macOS 26.6.2, build 25G83; on AC power.
- Rust 1.99.1, LLVM 23.1.1. The kernel harness was built by
  `tools/benchmarks/run.py` with `target-cpu=native` (`apple-m4`), optimization
  level 3, fat LTO and one codegen unit. The sampler runs used
  `cargo build --release --locked`, with the same settings and the default
  target CPU.
- Measured source: `0dfdfb2` (`Add a NEON kernel for the Mk node update`).
  `metadata.json` and `sampler.json` record the hashes of the sources, inputs,
  executables and raw runs.
- One thread and one process at a time. Background load was not controlled:
  load averages over 1, 5 and 15 minutes were 27.7, 23.4 and 19.4 before the
  kernel runs and 10.4, 9.8 and 11.6 after them; 11.1, 10.1 and 11.6 before the
  sampler runs and 8.4, 9.3 and 11.0 after.
- `Scaling::Always`, as the sampler uses it. Each metric runs the four variants
  (matrix and Mk paths, portable and NEON backends) back to back in the same
  process, in reverse order on alternate metrics and alternate processes.

Before timing, every case was compared on both paths and both backends with the
independent scalar reference, and the harness required both backends to return
the same baseline log likelihood, bit for bit. Both give
`-433.28114529381804` for the unconditioned reference and
`-198.61115564792343` for the Mkv case, the values of the
[direct Mk report](../../rove/2026-10-28/README.md).

## Full evaluations on the Mk path

Medians over the four processes. The ratio is the portable time over the NEON
time, process by process; the range covers the four processes.

| Case | Patterns | Stride | States | Categories | Portable | NEON | Ratio | Range |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `reference_20x50` | 23 | 24 | 2 | 4 | 4.733 µs | 3.206 µs | 1.48 | 1.47–1.49 |
| `reference_20x50_mkv` | 23 + 2 | 32 | 2 | 4 | 5.713 µs | 3.987 µs | 1.43 | 1.43–1.44 |
| `morphology_90x250` | 250 | 256 | 2 | 4 | 199.104 µs | 156.726 µs | 1.28 | 1.27–1.29 |
| `binary_four_rates` | 250 | 256 | 2 | 4 | 202.486 µs | 157.633 µs | 1.28 | 1.27–1.29 |
| `binary_one_rate` | 250 | 256 | 2 | 1 | 102.394 µs | 91.989 µs | 1.10 | 1.07–1.13 |
| `ternary_four_rates` | 250 | 256 | 3 | 4 | 262.784 µs | 199.172 µs | 1.32 | 1.30–1.35 |
| `eight_states` | 250 | 256 | 8 | 4 | 560.189 µs | 443.398 µs | 1.26 | 1.26–1.27 |
| `missing_20_percent` | 250 | 256 | 2 | 4 | 203.702 µs | 160.407 µs | 1.27 | 1.23–1.32 |
| `repeated_columns` | 25 | 32 | 2 | 4 | 28.021 µs | 19.675 µs | 1.43 | 1.42–1.44 |
| `ladder` | 250 | 256 | 2 | 4 | 197.389 µs | 151.467 µs | 1.31 | 1.23–1.33 |
| `small_unpadded` | 16 | 16 | 2 | 4 | 0.859 µs | 0.507 µs | 1.68 | 1.67–1.70 |
| `small_padded` | 17 | 24 | 2 | 4 | 0.997 µs | 0.652 µs | 1.52 | 1.50–1.55 |

The binary case with one rate category gains least: each node update does the
least arithmetic per pattern there, so the logarithm of rescaling, which both
backends compute the same way, takes the largest share.

## Partial updates and proposals

Median ratios of the portable time over the NEON time on the Mk path.

| Case | `full_with_branches` | `tip_update` | `internal_update` | `accept_proposal` | `reject_proposal` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `reference_20x50` | 1.40 | 1.40 | 1.17 | 1.39 | 1.39 |
| `reference_20x50_mkv` | 1.38 | 1.36 | 1.16 | 1.36 | 1.35 |
| `morphology_90x250` | 1.29 | 1.19 | 1.04 | 1.18 | 1.18 |
| `binary_four_rates` | 1.28 | 1.18 | 1.05 | 1.18 | 1.18 |
| `binary_one_rate` | 1.11 | 1.07 | 1.02 | 1.06 | 1.06 |
| `ternary_four_rates` | 1.33 | 1.24 | 1.08 | 1.23 | 1.22 |
| `eight_states` | 1.25 | 1.20 | 1.09 | 1.20 | 1.20 |
| `missing_20_percent` | 1.28 | 1.19 | 1.04 | 1.18 | 1.19 |
| `repeated_columns` | 1.39 | 1.29 | 1.09 | 1.27 | 1.27 |
| `ladder` | 1.27 | 1.28 | 1.05 | 1.31 | 1.30 |
| `small_unpadded` | 1.52 | 1.59 | 1.33 | 1.60 | 1.57 |
| `small_padded` | 1.42 | 1.47 | 1.28 | 1.46 | 1.46 |

`internal_update` recomputes one node, the root, before the root computation,
which runs the same code on both backends, so it gains least.

## Controls

The matrix path runs the same code on both backends. Its median ratios for the
evaluation and update metrics were between 0.977 and 1.018, and single
processes between 0.969 and 1.032. Parsing, compression, core setup and the
store operations do not depend on the backend; their medians were between
0.966 and 1.029. None of the evaluation, update or proposal operations
allocated on either backend.

## The sampler

`tools/benchmarks/sampler_backends.py` ran `rove sample` on the two
configurations of the [strict-clock comparison](../../strict-clock/2026-10-28/README.md),
with their constraint and fixed steps, for 10⁷ steps and seeds 1 to 4. The two
backends of a seed ran back to back, the first alternating between seeds.

| Configuration | Portable | NEON | Ratio |
| --- | ---: | ---: | ---: |
| Mkv | 11.69–11.96 s | 10.74–10.81 s | 1.09–1.11 |
| Mk | 11.36–11.44 s | 10.37–10.48 s | 1.08–1.10 |

Times are each chain's own wall time from `run.json`. In all eight pairs the
trace without its `#` lines and the trees without their command line had the
same SHA-256. The script wrote the `results` field of `sampler.json`; its other
fields, the build, hashes, load and order, were recorded in the same session.

A profile of the Mkv chain with the NEON backend, taken with macOS `sample` at
1 ms intervals over 8 s, shows where the remaining time goes, as shares of the
sampled time:

| Part | Share |
| --- | ---: |
| Node updates, `calculate_partials` | 33% |
| of which the logarithm of rescaling | 14% |
| Root likelihood, `Likelihood::evaluate` | 17% |
| of which adding the log scalers of all 39 nodes per pattern | 10% |
| Tree traversal for the likelihood update and the constraint | 17% |
| Birth–death prior | 7% |
| Copying the tree and the buffer indices before each step | 3% |

In this run the logarithm of rescaling takes about two fifths of the
node-update time. NEON does not change it; rescaling by powers of two could
replace it with exponent arithmetic, which has not been tried.

## Notes

The library's assembly before linking is not vectorised: with fat LTO, LLVM
vectorises at link time. Inspect the linked binary, not an `--emit=asm` of the
library, to see which loops run as NEON.

On this compiler, `f64::max` in the portable max pass compiles to two `fminnm`
and one `fmaxnm` per pair of values, which quiets a signalling NaN; the kernel
uses a bare `fmaxnm`. Part of its gain in rescaled updates therefore comes from
the cheaper maximum rather than from fusion. The results agree because the
operands, products and earlier maxima, are never signalling NaNs.

## Reproduce

The measured source is `0dfdfb2`. The commit that adds this report changes
only documentation and adds `tools/benchmarks/sampler_backends.py`, so the
commands work from a clean checkout of it, with output directories that do not
exist yet:

```sh
python3 tools/benchmarks/run.py --repetitions 4 --output /path/to/kernel-run

cargo build --release --locked
python3 tools/benchmarks/sampler_backends.py --rove target/release/rove \
  --repository . --output /path/to/sampler-run --seeds 1 2 3 4
```

## Files

- `run-1.tsv` to `run-4.tsv`: raw batches and allocation counts of each process.
- `summary.json`: medians, process medians and batch percentiles, keyed by case,
  path and backend.
- `comparison.json`: per-process ratios of the matrix path over the Mk path on
  each backend, and of the portable backend over NEON on each path.
- `metadata.json`: hardware, compiler, flags, load, power and hashes.
- `sampler.json`: the sampler runs, their times and the hashes of their samples.

## Limitations

These numbers describe one machine with uncontrolled background load, the
`Scaling::Always` setting and the cases above; they do not carry over to other
chips, to x86_64, which runs the portable code, or to other models. The sampler
times come from one small matrix, and its core has one rate category, the
setting with the smallest kernel gain above. No cross-engine comparison is
implied.
