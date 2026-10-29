# Relaxed-clock proposal comparison

This experiment follows the fixed [proposal evaluation protocol](../../../fixtures/validation/SAMPLING-EFFICIENCY.md)
for [#27](https://github.com/getcortexlabs/rove/issues/27). The probability
model, likelihood kernel, constraints and inference criteria are unchanged.

## Outcome

The joint candidate passes the unchanged scientific gates in all six cases,
against both current classical Rove and the archived BEAST 2.7.8 ORC runs.
It is **not promoted to the default**. The new moves remain explicitly
selectable through `--weight`; the native app keeps the classical schedule.

The predeclared primary score is **6.626**, with a 90% whole-chain bootstrap
interval of **5.894–7.321**. This is an aggregate ESS/second ratio, not an
elapsed-time speedup: posterior-density efficiency improves 9.42–13.19 times
in the five Mk cases but is 0.94 of classical in Mkv. Clock-sigma efficiency
improves 8.81–11.11 times across the six cases.

The regression gate fails. Gamma-shape median ESS/second is 0.689 of classical
in control and 0.661 in missing; birth–death log-density efficiency is 0.632
in Mkv. The gamma intervals are wide and include one. Four chains cannot
resolve every tradeoff precisely, but the fixed decision rule uses these
medians, and we do not change it after seeing the results.

Against the **recorded** BEAST ORC observations, the candidate's median
ESS/second is higher for every continuous statistic in every case, with the
smallest ratio 1.646. These are six small constructed matrices on one M4 Max,
not a general comparison with BEAST, BEAST X or a BEAGLE backend. BEAST timings
come from the earlier archive; it was not rerun alongside this experiment.

[TABLES.md](TABLES.md) reports every parameter, runtime ranges, scientific
gates, topology counts, autocorrelation and the regression intervals.
[final/efficiency.json](final/efficiency.json) records the decision and all
bootstrap intervals. Each case's comparison and summaries preserve per-chain
ESS/second, R-hat, means, standard deviations, 95% HPD intervals, clade
frequencies and batch-based Monte Carlo uncertainty.

The largest continuous mean discrepancy is 2.953 combined Monte Carlo
standard errors; the largest unconstrained clade-frequency difference is
0.00605. The largest continuous R-hat is 1.00398 and the lowest chain ESS is
262.6. Mkv retains very few distinct topologies and some constant tree
indicators: those mixing diagnostics remain marked limited. Passing the gates
is not proof that all tree space was explored. T1–T19 remains an imposed clade.

## Exploratory selection

The pilot used reference-mk, reference-mkv and missing, two seeds per schedule,
two million steps per chain and 10% burn-in. Five schedules ran sequentially
on an Apple M4 Max. The score is the geometric mean across cases of the
smaller of the posterior and clock-sigma median ESS/second ratios against
classical Rove. These short runs select a candidate; they do not establish
posterior concordance or a final speedup.

| Candidate | Pilot score |
| --- | ---: |
| sigma-frequency | 1.005 |
| time-rates | 1.510 |
| sigma-rates | 6.437 |
| combined | 7.397 |

`combined` is selected without further weight adjustment. Its overrides are
`node_height=18`, `node_time_rates=12`, `root_scale=1`, `root_time_rates=2`,
`clock_sigma_scale=1`, `clock_sigma_rates=2`. All other weights keep their
classical values; the total selection weight is unchanged.

[pilot.json](pilot.json) retains every parameter of every pilot chain,
including regressions. [pilot-metadata.json](pilot-metadata.json) records
commands, timings, hashes, model settings and operator counts. The pilot
used an uncommitted implementation: its source-file hashes are recorded,
and final evaluation requires a clean commit. No default is changed on the
basis of these pilot observations.

The joint time/rate and standardized-rate proposals implement established
ideas from [Douglas, Zhang and Bouckaert (2021)](https://doi.org/10.1371/journal.pcbi.1008322).
Their independently derived transformations and Hastings factors are documented
in the [reference model guide](../../../docs/guide/reference-model.md#joint-proposals).
This is not an implementation of ORC's entire adaptive mixture.

## Proposal profile

The [profiles](profiles/metadata.json) measure classical and combined schedules
in all six cases, seed 27001, after 2M warm-up steps: 500,000 timed complete
steps and a separate 100,000-step allocation pass. Each move records calls,
acceptance, rejection, invalid proposals, final tuning, elapsed nanoseconds,
squared parameter jumps and accepted changes of canonical clades. Counting
clades and allocations is outside the step timing. These are descriptive
profiles, with per-step clock overhead, not end-to-end throughput estimates.

On reference-mk, classical node-height moves take 31.7% of measured step time
and individual branch-rate scalers 20.8%. The sigma-only scaler takes just
0.6%. Increasing its selection frequency alone gives a pilot score of 1.005;
the joint sigma/rate move is the useful candidate in that pilot.

In the final proposal profile, sigma-only and joint sigma/rate moves both
accept about 23% of proposals. The latter costs about 5,534 ns per call versus
322 ns for the classical sigma scaler, but its mean squared log-sigma jump
per call is 0.0615 versus 0.00727. Similar acceptance rates do not imply similar
movement. The final reference-mk chains reduce median sigma autocorrelation
time from 19.08 to 1.59 logged samples. These observations motivate the joint
move; they do not isolate the causal effect of each component of the combined
schedule.

There were **zero allocations, reallocations or allocated bytes** inside the
100,000 measured proposal steps in every profile, including all new moves.
This excludes setup, logging, diagnostics and the profiler's own bookkeeping.
Canonical-clade changes over the 500,000 timed steps were:

| Case | Classical | Combined |
| --- | ---: | ---: |
| reference-mk | 2,599 | 2,926 |
| reference-mkv | 0 | 3 |
| control | 2,668 | 2,500 |
| homoplasy | 4,102 | 4,108 |
| missing | 11,009 | 11,464 |
| reduced | 5,985 | 6,132 |

The profiles show no uniform topology-movement gain. The proposal changes
principally address correlated continuous parameters.

## Provenance and reproduction

The timed engine is the clean commit `a2483aa`. Its SHA-256 is
`cd85f3f520f88ada0bc60663ad1e89911f1affd5134a6963a83d990def6777b9`.
Rust 1.99.1 targets aarch64-apple-darwin; release uses fat LTO, one codegen unit
and no `target-cpu` override. NEON is the native backend. The machine is an
Apple M4 Max with 16 CPU cores (12 performance, 4 efficiency) and 64 GB RAM.
The desktop remained in use; cores were not pinned. No other experiment
chains or validation computations ran alongside the timed chains. Load and
power observations are recorded, not interpreted as a controlled thermal
experiment.

The 48 runs took place sequentially from 20:31 to 20:54 UTC on 29 October
2026, alternating schedule order by seed. Four seeds per schedule ran 10M
steps in each Mk case and 40M in Mkv, with one sample every 1,000 steps and
10% burn-in. Diagnostics followed the timed runs. The development environment
restarted during Mkv; the independent experiment process and its child chain
continued, and no run was repeated or replaced.

Every current classical chain reproduces the corresponding archived Rove
scientific trace and trees exactly after removing provenance headers. The
earlier archive used `target-cpu=native`; this comparison rebuilds both Rove
schedules with the same current flags. A separate [100,000-step check](classical-identity.json)
also reproduces the pre-change `e91df87` executable's samples.

[final/archive.json](final/archive.json) includes source, binary and input
hashes, full commands, settings, final tuning and operator counts, reference
metadata and the nested ORC operator weights. BEAST uses 2.7.8, MM 1.2.1,
ORC 1.2.1 and BEASTLabs 2.0.3 with Java 17 and its Java likelihood core.
The analysis-helper hashes are recorded separately. Profile source hashes
match the frozen engine; only documentation, analysis scripts and result files
were being edited during profiling. Raw traces and trees remain local; their
hashes are in the archive.

From the repository root, using a release binary built from `a2483aa`:

```sh
cargo build --locked --release --bin rove
python3 tools/sampling/experiment.py pilot \
  --hardware benchmarks/tree-summary/2026-10-29/hardware.json \
  --output .local/sampling-pilot
python3 tools/sampling/experiment.py final \
  --hardware benchmarks/tree-summary/2026-10-29/hardware.json \
  --pilot .local/sampling-pilot/pilot.json --output .local/sampling-final
```

The final runner requires a clean checkout. Reproducing the recorded candidate
specifically uses `--pilot benchmarks/sampling/2026-10-29/pilot.json`, rather
than a new pilot selection. Hardware JSON must describe the machine used.

```sh
python3 tools/sampling/report.py --runs .local/sampling-final \
  --reference-root .local/issue12 \
  --packages .local/beast-reference-20261025/packages \
  --launcher .local/beast-reference-20261025/tools/launcher.jar \
  --java /Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home/bin/java \
  --javac /Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home/bin/javac \
  --output .local/sampling-report
cargo bench --locked --bench proposals -- \
  fixtures/validation/reference-mk/matrix.nex none 27001
cargo bench --locked --bench proposals -- \
  fixtures/validation/reference-mk/matrix.nex none 27001 \
  node_height=18 node_time_rates=12 root_scale=1 root_time_rates=2 \
  clock_sigma_scale=1 clock_sigma_rates=2
```

Repeat the profile commands with the six fixture paths and `variable` for
reference-mkv. The report command requires the original local BEAST archive;
its creation commands and public summaries are recorded in the
[previous inference report](../../inference/2026-10-29/README.md). Java is an
optional validation dependency and is never used by the standalone Rove engine.

To try the evaluated candidate, append these overrides to a
`rove sample ... --model reference` command:

```sh
--weight node_height=18 --weight node_time_rates=12 \
--weight root_scale=1 --weight root_time_rates=2 \
--weight clock_sigma_scale=1 --weight clock_sigma_rates=2
```

No changes to the selected weights were made after the final results. A later
schedule study needs fresh validation, including the gamma and Mkv regressions.
