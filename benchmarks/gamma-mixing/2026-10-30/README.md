# Gamma-shape proposal frequency

Issue [#42](https://github.com/getcortexlabs/rove/issues/42) investigates slow
gamma-shape mixing in the four-tip fossil-dating example. Giving the existing
proposal a larger selection weight improves gamma ESS per wall time. The
candidate remains opt-in; the recorded decision applies to the complete
sampler, including parameters that become slower.

## Cause and candidate

The [preceding FBD study](../../fbd-estimation/2026-10-30/README.md) exposed a
low gamma ESS with both fixed and estimated process parameters. Inspection of
its 10M-step traces found 8,098–9,219 gamma proposals per chain, of which
roughly 23–24% were accepted. Proposal-size adaptation had reached its target;
it does not change how often an operator is selected. The inherited gamma
weight of 0.1 gave this parameter about one opportunity every 1,100–1,200
steps. See [baseline-inspection.json](baseline-inspection.json).

The gamma marginal is broad, reaching close to its lower bound and into the
exponential tail. Its linear correlations with clock rate, tree height and
clock sigma were weak in these traces. This does not rule out nonlinear
dependence. The frequency experiment supports infrequent updates as a major
limitation on this workload; it does not establish a universal cause of slow
gamma mixing.

The candidate changes only `gamma_shape_scale` weight from 0.1 to 3. The
proposal remains `shape' = shape × f`, with `f` uniform on `[s, 1/s]` and
log-Hastings correction `−log(f)`. The Exponential(mean=1) prior, support
`[0.001, 1000]`, four midpoint gamma categories, clock model, FBD observation
convention and other operator weights are unchanged. With fixed tuning, a
state-independent mixture of invariant MH kernels preserves the target;
the existing proposal-size adaptation is reused.

Selection weight and proposal size are separate controls, as described in
the [BEAST 2 operator documentation](https://beast2-dev.github.io/hmc/hmc/Standard/Operators/).
Gamma updates require a full likelihood refresh, so their frequency also
changes runtime. More accepted gamma proposals do not imply an equal gain
for the whole analysis.

## Efficiency and decision

The table reports baseline → candidate. Time is the median group wall time;
minimum ESS is the lowest across all sampled scalars and chains, excluding
only values verified as fixed by the model. Efficiency ratios compare the
medians of per-chain ESS divided by group wall time.

| Case | Group seconds | Minimum scalar ESS | Gamma ESS/time ratio | Lowest scalar ESS/time ratio |
| --- | ---: | ---: | ---: | --- |
| Dated, fixed FBD | 9.32 → 10.33 | 151 → 2,775 | 21.93 | mean branch rate: 0.797 |
| Dated, estimated FBD | 10.79 → 11.57 | 131 → 2,349 | 23.86 | mean branch rate: 0.866 |
| Reference Mk | 26.27 → 30.59 | 224 → 277 | 10.36 | clock sigma prior: 0.712 |
| Reference Mkv | 109.47 → 126.25 | 850 → 1,188 | 19.86 | clock sigma prior: 0.751 |
| Control | 25.75 → 30.14 | 281 → 378 | 9.05 | clock rate prior: 0.865 |
| Homoplasy | 36.25 → 36.38 | 307 → 245 | 12.72 | clock sigma prior: 0.801 |
| Missing | 41.36 → 46.91 | 310 → 353 | 10.58 | relative death rate: 0.821 |
| Reduced | 27.91 → 28.89 | 263 → 303 | 10.26 | mean branch rate: 0.835 |

**Keep the candidate opt-in.** The frozen rule requires every sampled scalar's
median efficiency ratio to be at least 0.8. Reference Mk and Mkv miss this
gate, and the fixed-FBD case narrowly misses it at 0.7969. The protocol's
dated-only fallback therefore fails too. These point estimates have Monte
Carlo and timing uncertainty; the near-boundary miss does not prove a true
20% loss. It also does not justify rounding the result up after seeing it.

In the final dated runs, the candidate makes about 238,000–271,000 gamma
attempts per 10M steps, versus 8,035–9,424 with the baseline. Both accept
roughly 23–24%. The candidate exposes clock/tree mixing as the next limitation.
Homoplasy also shows why the gamma gain is not an end-to-end speedup: its worst
scalar ESS falls despite gamma ESS/time improving more than twelvefold.

## Posterior agreement and validation

All eight cases pass the frozen scientific checks. These compare two proposal
schedules for the same model, not the biological adequacy of that model.

| Case | Largest absolute mean z-score | Largest scalar R-hat | Largest clade-frequency difference | Largest clade MCSE |
| --- | ---: | ---: | ---: | ---: |
| dated-fixed | 1.589 | 1.00564 | 0.00374 | 0.00249 |
| dated-estimated | 1.973 | 1.00306 | 0.00329 | 0.00237 |
| reference-mk | 1.224 | 1.00567 | 0.00258 | 0.00170 |
| reference-mkv | 1.404 | 1.00124 | 0.00001 | 0.00001 |
| control | 1.411 | 1.00323 | 0.00283 | 0.00153 |
| homoplasy | 0.900 | 1.00214 | 0.00478 | 0.00235 |
| missing | 1.004 | 1.00264 | 0.00555 | 0.00302 |
| reduced | 1.113 | 1.00432 | 0.00453 | 0.00357 |

Thresholds are mean z-score ≤ 4, R-hat ≤ 1.01, clade difference ≤ 0.05
and clade MCSE ≤ 0.0125, with per-chain ESS and lag-limit checks as specified
in the protocol. Tree-distance and topology-indicator checks also pass where
defined. Constant indicators remain explicitly marked as limited evidence
of tree mixing; passing scalar checks is not proof of convergence.

[report.json](report.json) contains every scalar, clade, gamma-tail diagnostic,
correlation, operator count, source hash and timing. The [reader cross-check](reader-check.json)
confirms identical retained clade frequencies from Rove and the independent
BEAST reader in all 80 chains, including the normalized metadata copies.

Validation also passed 28 optimized Rust tests covering fixed-state likelihoods,
known hyperpriors, FBD distributions, rejected-state restoration, allocation-free
steps and exact checkpoint continuation. The 37 Python analysis-helper tests
include negative controls for shifted means/clades, uncertain clades, unavailable
ESS and efficiency regressions. The four-language VitePress build passes.

## Protocol and provenance

The [frozen protocol](../../../fixtures/validation/GAMMA-MIXING.md) specifies
seeds, lengths, diagnostics and default-promotion gates. The exploratory
[pilot](pilot.json) tried weights 0.1, 0.5, 1 and 3 in the two dated examples,
then 0.1 and 3 on reference Mk, reference Mkv and missing-data matrices. One
early seed group overlapped three chains with its predecessor; it is retained
and marked as exploratory, not counted as independent replication. The final
seeds are disjoint from every pilot seed.

The dated examples contain four tips, eight binary characters and four
compressed patterns, with Mkv conditioning. The six reference cases have 20
tips and 47–50 characters, compressing to 22–50 patterns as recorded in the
[fixture manifest](../../../fixtures/validation/manifest.json).

Final runs use one frozen executable from source `35f444a`, with explicit
weight overrides for both schedules. The machine is an Apple M4 Max with
16 CPU cores (12 performance, 4 efficiency), 64 GB RAM and macOS 26.6.2.
Rust 1.99.1 release builds use fat LTO, one codegen unit and the NEON backend,
without a `target-cpu` override. The desktop remained in use; cores were not
pinned. No other experiment, build or validation computation ran alongside
the timed groups.

Each group runs four independent chains concurrently. Groups run sequentially
with alternating schedule order. Dated cases have two groups per schedule;
the six undated cases have one group per schedule. Each chain runs 10M steps,
except reference Mkv at 40M. Samples are saved every 1,000 steps and the first
10% are discarded by sample count. The same seeds are reused between
schedules, without assuming corresponding MCMC states.

The 80 final chains total 1.04 billion steps. Timing ran from 21:53 to 22:04
on 30 October 2026 in UTC−3 (00:53–01:04 UTC on 31 October).

Timing covers the complete group process: startup, adaptation, trace/tree
writes, checkpoints and the final Rove comparison. ESS/time divides each
chain's ESS by its group's elapsed time. Four chains sharing a wall time are
not four independent timing observations. These numbers are neither
single-chain latencies nor estimates for M1 machines.

Posterior concordance uses the existing independent BEAST 2 validation helper
after timing. Java reads completed Rove output; it does not run the sampler.
The dated four-tip anchor is only a reference for tree-distance diagnostics,
not a known generating topology. The undated T1–T19 constraint is imposed and
excluded from recovery claims. Raw traces, trees and console logs stay local;
the result archive records their hashes, complete commands and operator counts.

The first independent read exposed an export-compatibility defect, now tracked
in [#43](https://github.com/getcortexlabs/rove/issues/43). Dated relaxed-clock
trees emit `[&height=h][&rate=r]`; BEAST 2.7.8 requires a single node metadata
block. For validation only, the report script writes copies using
`[&height=h,rate=r]`. It preserves both numeric strings, labels, branch lengths,
topologies and sample order. The original files remain intact. The archive
records both hashes and the replacement counts. This analysis adaptation
does not change the frozen sampling protocol or its decision thresholds; it
does not fix the application's writer.

## Reproduce

From the repository root, build the recorded source in an isolated checkout
and preserve that executable for both schedules:

```sh
cargo build --locked --release --bin rove
python3 tools/benchmarks/gamma_mixing.py \
  --binary target/release/rove --output .local/gamma-frequency-final
python3 tools/benchmarks/report_gamma_mixing.py \
  --runs .local/gamma-frequency-final \
  --packages .local/beast-reference-20261025/packages \
  --launcher .local/beast-reference-20261025/tools/launcher.jar \
  --java /Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home/bin/java \
  --javac /Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home/bin/javac \
  --output .local/gamma-frequency-report
```

The helper uses the same optional BEAST 2.7.8 validation packages as the
[previous inference study](../../inference/2026-10-29/README.md). Its source,
JARs, Java version, Rust source and inputs are fingerprinted in the archive.

To try the tested candidate on a dated analysis, append
`--weight gamma_shape_scale=3` to a `rove sample ... --model reference`
command. Omitting the override retains weight 0.1, including in the Mac app.
Assess every sampled parameter and the trees across independent chains.
Higher ESS does not make weakly informative data identify the model, and
scalar diagnostics alone do not prove convergence.
