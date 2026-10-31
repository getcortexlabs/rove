# Marginal likelihood by power posteriors

Issue [#24](https://github.com/getcortexlabs/rove/issues/24) adds
`rove marginal`, which estimates the log marginal likelihood of a model from
power posteriors with the stepping-stone and path-sampling estimators. This
report records the comparison with BEAST 2.7.8 and its MODEL_SELECTION
package under the [frozen protocol](../../../fixtures/validation/MARGINAL-LIKELIHOOD.md).

## Outcome

Both configurations pass every gate. The engines agree on the log marginal
likelihood within the Monte Carlo error of the estimates, and one
implementation recomputes each engine's reported value from its own logs.

| Configuration | Rove–BEAST pairs | Largest difference / combined SE | Differences, Rove − BEAST | Powers beyond tolerance |
| --- | ---: | ---: | --- | --- |
| Strict clock, Mkv | 9 of 9 agree | 1.79 | −0.05 to +0.78 | 1 of 32 in one pair, 0 elsewhere |
| Strict clock, Mk | 9 of 9 agree | 2.53 | −0.67 to +0.75 | 0 |

Values are log units. The gate accepts a pair when its stepping-stone
difference is at most four combined standard errors. Path-sampling pairs
agree the same way, with largest ratios of 1.92 and 2.53. Recomputed
stepping-stone values differ from Rove's `marginal.json` by at most 6e-14
and from the estimate PathSampler prints by at most 1.8e-8.

### Strict clock, Mkv

| Engine | Seed | Stepping stone | Path sampling | Lowest power ESS | Wall seconds |
| --- | ---: | ---: | ---: | ---: | ---: |
| BEAST | 24101 | −251.368 ± 0.274 | −251.628 ± 0.274 | 73 | 112 |
| BEAST | 24102 | −251.666 ± 0.263 | −251.787 ± 0.248 | 4 | 117 |
| BEAST | 24103 | −251.042 ± 0.272 | −251.243 ± 0.254 | 61 | 121 |
| Rove | 24001 | −251.022 ± 0.284 | −251.088 ± 0.267 | 25 | 52 |
| Rove | 24002 | −251.092 ± 0.288 | −251.426 ± 0.273 | 21 | 50 |
| Rove | 24003 | −250.881 ± 0.351 | −251.329 ± 0.308 | 56 | 46 |

### Strict clock, Mk

| Engine | Seed | Stepping stone | Path sampling | Lowest power ESS | Wall seconds |
| --- | ---: | ---: | ---: | ---: | ---: |
| BEAST | 24101 | −299.933 ± 0.214 | −300.136 ± 0.210 | 50 | 104 |
| BEAST | 24102 | −298.968 ± 0.227 | −299.239 ± 0.225 | 36 | 138 |
| BEAST | 24103 | −299.497 ± 0.208 | −299.815 ± 0.205 | 56 | 125 |
| Rove | 24001 | −299.637 ± 0.215 | −300.013 ± 0.208 | 60 | 48 |
| Rove | 24002 | −299.304 ± 0.209 | −299.525 ± 0.202 | 41 | 43 |
| Rove | 24003 | −299.186 ± 0.203 | −299.529 ± 0.205 | 21 | 46 |

Estimates and standard errors are the recomputed values; both engines'
reported values agree with them as stated above. Path sampling lies 0.07 to
0.45 below stepping stone in every run. That is the direction expected from
the trapezoidal rule on a concave mean log-likelihood with 32 powers, and the
reported standard errors do not include it.

The lowest ESS per run belongs to a power near zero in every case, where the
chains explore the prior and the likelihood varies widely between samples.
Between four and eight of the 32 powers per run have an ESS below 100 in
both engines. The per-power ESS, means and variances are in the JSON files.

## Settings

- Configurations: [`beast-mkv.xml`](../../strict-clock/2026-10-28/beast-mkv.xml)
  and [`beast-mk.xml`](../../strict-clock/2026-10-28/beast-mk.xml), the
  strict-clock Mkv and Mk models already matched between the engines, on the
  supplied 20 × 50 matrix with the T1–T19 constraint, fixed proposal steps and
  no tuning.
- Schedule: 32 powers, the Beta(0.3, 1) quantiles at `i / 31`, from the
  posterior down to the prior. 1,000,000 steps per power, the likelihood
  logged every 1,000 steps, 20% of each power's 1,001 samples discarded by
  count. BEAST additionally runs `preBurnin=200000` before its first step.
- BEAST: `modelselection.inference.PathSampler` from MODEL_SELECTION 1.6.2
  around the configuration's MCMC, `posterior2prior=true`, one thread, each
  step started as a separate `BeastMCMC` process on the pinned jars, with the
  previous step's state copied forward. Seeds 24101 to 24103.
- Rove: `rove marginal` with `--powers 32 --alpha 0.3 --steps 1000000
  --sample-every 1000 --burnin-percent 20` and the moves, weights and fixed
  steps read from the XML. Seeds 24001 to 24003.
- Analysis: `tools/beast/marginal_likelihood.py` reads both engines' logs and
  applies the stepping-stone and path-sampling formulas with the
  initial-positive-sequence ESS used by Rove's diagnostics.

No run was extended, dropped or retuned after the results.

## Analysis correction after the runs

An independent review of the code found that the Python ESS divided the
autocovariance at each lag by the sample count instead of by the count minus
the lag, Rove's convention. The script was corrected and re-applied to the
unchanged logs with `--recompute`, which keeps the recorded commands, wall
times and reported values. Standard errors moved by about 1%; no gate changed.
Each JSON records the hash of the script that ran the engines
(`script_sha256`) and of the script that produced the analysis
(`analysis_script_sha256`). The Mkv configuration ran under the committed
script of 1337cea; the Mk configuration started after the analysis functions
had been edited and ran under an intermediate version that differed only in
those functions and in the path sanitizer, not in how either engine was
configured or launched. Both JSONs carry the same final analysis hash.

## Provenance

- Rove source 1337cea; executable SHA-256
  `a8646b8edc6be838a2ce113a008757f0b813e735f5431fd23f8658f52671d854`,
  a native aarch64 release build with fat LTO and the NEON backend.
- BEAST.base and BEAST.app 2.7.8, BEASTLabs 2.0.3, MM 1.2.1, ORC 1.2.1 as in
  the earlier comparisons, plus MODEL_SELECTION 1.6.2 from
  `MODEL_SELECTION.addon.v1.6.2.zip`, SHA-256
  `797a4db8a8a9eb2a39438b84e33035cb31a0b72b1910a9c887d1ff9d5bf4fb01`.
  Temurin OpenJDK 17.0.15. Jar hashes are in the JSON files.
- Apple M4 Max, macOS 26.6.2. The runs took from 00:25 to 00:42 on
  31 October 2026 in UTC−3. Debug builds and tests ran on the same machine
  during part of the Mk runs. Wall times include process launch and, for
  BEAST, 32 JVM starts per run; they are reported for completeness and are
  not a timing comparison.
- Protocol SHA-256
  `2d18fd318bad65c5bab504dc04694e414dfb8d1a555f59fbc96a3ee5e5785af1`.

## Files

- `comparison-mkv.json` and `comparison-mk.json`: settings, commands, hashes,
  per-run estimates, per-power means, ESS and variances, every pairwise
  comparison and the gate results.
- `path-sampler-mkv.xml` and `path-sampler-mk.xml`: the generated PathSampler
  configurations, with the repository path replaced by `<repository>`.

Raw likelihood logs, traces and console output stay local.

## Reproduce

```sh
cargo build --locked --release --bin rove
python3 tools/beast/marginal_likelihood.py \
  --packages /path/to/packages --launcher /path/to/launcher.jar \
  --xml benchmarks/strict-clock/2026-10-28/beast-mkv.xml \
  --matrix fixtures/reference/20x50/matrix.nex \
  --rove target/release/rove --output /path/to/new-mkv-run
```

The package directory must hold MODEL_SELECTION 1.6.2 next to the 2.7.8
packages. Use `beast-mk.xml` for the Mk configuration, and `--recompute` with
an existing `--output` directory to reapply the analysis without rerunning
the engines.

This comparison establishes that two implementations of the same estimator
agree on one small matrix under two models. It does not establish
model-selection performance on empirical data, the adequacy of 32 powers for
larger matrices, or any speed difference between the engines.
