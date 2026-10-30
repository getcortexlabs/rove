# Scaling reference

`exact.json` contains 36 unscaled pruning evaluations computed with Python's
standard-library `decimal` module at 60 significant digits. Regenerate with
`python3 tools/benchmarks/scaling_reference.py`, or verify with `--check`.
No BEAST installation or third-party Python package is needed.

The 32 small cases have seven tips, 2/3/4/8 states, one/four rate categories,
missing cells and branch distances of 1e-9, 0.001, 1 or 100. Four further
cases use balanced and ladder trees with 2,048 tips, two/four states and four
categories. Their unscaled double-precision likelihoods underflow.

Each case evaluates three observed patterns and one constant pattern per
state. Tips, joins and pattern order are defined in the generator and
independently constructed by `tests/scaling.rs`. The generator multiplies
full transition matrices and never rescales partials. It interprets the
binary64 frequencies, Mk normalization and branch-distance/rate products
as exact inputs before evaluating the matrix exponentials in Decimal.
The stored logarithms retain all 60 digits.

The tests compare every pattern with the decimal values and both backends
with each other, then check Mk and Mkv totals. The small cases are also
checked against `reference::Problem`. Mkv divides by the probability of a
variable character; at 1e-9 branch distances that probability is tiny, so
rounding in the constant-pattern probabilities is amplified. The test bounds
this separately using `72 * f64::EPSILON / P(variable)`, with a floor of three
times the pattern-log tolerance. This is a conditioning limitation, not an
underflow fix. The reference engine tests use much tighter tolerances at
the archived BEAST states in `tests/ascertainment.rs`.

Setting `ROVE_SCALING_REPORT` to a local output path records the values and
errors from `tests/scaling.rs`. `ROVE_BEAST_SCALING_REPORT` does the same for
the archived BEAST states, with and without Mkv conditioning. These reports
allow a direct before/after comparison without retaining the old scaling
implementation in the production engine.
