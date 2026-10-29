# Trace diagnostics validation

Rove's within-chain ESS matched the BEAST 2.7.8 reference on seven synthetic
columns and fifteen columns from a recorded BEAST run. The largest normalized
error was `3.49e-14` for the synthetic input and `5.47e-11` for the recorded run.
Means and population standard deviations were checked against both BEAST and
Python; equal-tailed intervals were checked against Python's inclusive quantiles.
These checks validate post-processing, not convergence or inference speed.

## Statistical conventions

Burn-in discards `floor(N * percent / 100)` rows, including state zero when
present. The CLI also accepts an explicit sample count. All statistics use the
retained rows, independently of the bounded display points.

For retained values with mean `m`, the lag covariance is
`gamma(k) = sum((x[i] - m) * (x[i+k] - m)) / (N-k)`. ESS starts with
`I = gamma(0)` and adds `2 * (gamma(1) + gamma(2))`, then the pairs `(3, 4)`,
`(5, 6)`, and so on, stopping before the first nonpositive pair. Lags are less
than `min(N, 2000)`. The result is `N * gamma(0) / I`, at most `N`. This is the
[BEAST 2.7.8 convention](https://github.com/CompEvol/beast2/blob/v2.7.8/src/beast/base/inference/util/ESS.java),
not rank-normalized bulk or tail ESS. In particular, it does not report ESS
above `N` for antithetic samples.

Rove centers and scales the data before a zero-padded FFT, avoiding cancellation
from subtracting large raw moments. A direct covariance calculation checks the
FFT on power-of-two and irregular lengths. Reaching the evaluated lag limit
without a nonpositive pair is reported; positive correlation beyond that limit
can make ESS optimistic. Constant series and nonconstant series shorter than
four values have unavailable ESS, with a reason rather than a numeric substitute.

Standard deviation uses denominator `N`. The 95% interval uses the 2.5th and
97.5th percentiles with linear interpolation at `(N-1)*p`, the
[R type 7 convention](https://stat.ethz.ch/R-manual/R-devel/library/stats/html/quantile.html).
It is an interval for the retained distribution, not HPD or Monte Carlo error
of the mean. One retained value has no interval. The reference JSON records
BEAST HPD bounds separately; they are not expected to equal these quantiles.
Python's [inclusive quantiles](https://docs.python.org/3.9/library/statistics.html#statistics.quantiles)
provide an independent interval reference.

## Inputs and reproduction

[`synthetic.tsv`](../../../fixtures/diagnostics/synthetic.tsv) has 2,049 rows,
spaced by 100 states, generated with Python seed `34033`: Gaussian white noise,
AR(1) processes with coefficients `0.9` and `-0.8`, a linear trend, a constant,
an isolated spike and a skewed distribution. A 25% burn-in leaves 1,537 rows.
The positive AR(1) estimate is also checked against the asymptotic prediction
`N * (1-phi) / (1+phi)` with a finite-sample tolerance. Analytical moments,
quantiles, large offsets, extreme finite magnitudes and cancellation have
separate Rust tests.

The real input is `runs/seed-20261025/analysis.log` from the
[BEAST reference experiment](../../beast-2.7.8/2026-10-25/README.md),
using the supplied 20-taxon, 50-character matrix and
reference XML. It has 10,001 rows; 25% burn-in retains 7,501. Its SHA-256 is
recorded in [`reference-run.json`](reference-run.json). Validation does not
depend on interpreting this chain as converged.

Both comparisons use BEAST.base and BEAST.app **2.7.8**, Java 17.0.15 and Python
3.9.6. Package versions, jar hashes and the test helper's hash are saved beside
the results. The Java helper is test-only; no Java, Tracer or BEAST code is
linked into the application. From the repository root, with the reference
packages already installed:

```sh
cargo build --locked --release
python3 tools/diagnostics/validate.py \
  --packages .local/beast-reference-20261025/packages \
  --launcher .local/beast-reference-20261025/tools/launcher.jar \
  --output fixtures/diagnostics/expected.json
python3 tools/diagnostics/validate.py \
  --packages .local/beast-reference-20261025/packages \
  --launcher .local/beast-reference-20261025/tools/launcher.jar \
  --trace .local/beast-reference-20261025/runs/seed-20261025/analysis.log \
  --output benchmarks/diagnostics/2026-10-29/reference-run.json
```

Both commands default to 25% burn-in. Use `--java` and `--javac` if Java 17 is
not on PATH. Normal `cargo test` compares the committed fixture and expected
statistics without running Java. Errors are `abs(actual-reference) / max(1,
abs(reference))`; these are absolute errors near zero, not relative percentages.

The parser rejects non-finite values, irregular sample states, duplicate columns,
missing cells and unfinished final lines. Completed run metadata must match
the trace's state range and sampling interval. Cancelled/partial runs and traces
without metadata remain explicitly marked. The CLI bounds inputs at 128 MiB,
one million rows, 128 numeric parameters and eight million numeric cells.
Those limits are workload limits, not a process RSS guarantee.

## Measured cost

Apple M4 Max, 16 CPU cores, 64 GB RAM, arm64 macOS 26.6.2. Rust 1.99.1, release
build, fat LTO, one codegen unit, no `target-cpu` override. Each input repeats
the synthetic values to exercise parsing, full retained statistics and JSON
serialization; these repeated values are not an inference experiment.

| Rows × parameters | Input bytes | Median wall time | Largest child peak RSS |
| --- | ---: | ---: | ---: |
| 10,001 × 7 | 1,275,473 | 14.7 ms | 13.4 MiB |
| 100,001 × 7 | 12,852,653 | 125.7 ms | 36.8 MiB |
| 250,001 × 7 | 32,297,851 | 288.1 ms | 77.5 MiB |

Three separate CLI processes per input, stdout discarded. `wait4` records
per-child peak RSS, converted to bytes for the host platform. Input creation
is outside the timed region; files may already be cached. These figures exclude
SwiftUI rendering and do not compare speed with BEAST. Full observations,
hardware, compiler, input hashes and binary hash are in [`timings.json`](timings.json).

```sh
python3 tools/diagnostics/measure.py \
  --hardware benchmarks/tree-summary/2026-10-29/hardware.json \
  --work .local/diagnostics-measurement \
  --output benchmarks/diagnostics/2026-10-29/timings.json
```

Use a hardware record for the machine actually being measured. Native tests
exercise cancellation, restoring the last applied burn-in, preserving raw
files, matching tree-summary states and navigating the workspace while both
diagnostics and an analysis run. Captures cover English and Brazilian Portuguese
at regular and compact sizes. One-run ESS, cross-run agreement and topological
convergence remain distinct; R-hat, cross-chain clade comparisons and stopping
rules belong to #23 and are not calculated here.
