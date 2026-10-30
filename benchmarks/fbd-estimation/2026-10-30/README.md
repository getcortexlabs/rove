# FBD estimation on a constructed four-tip matrix

Rove can now estimate any subset of the five FBD process parameters while keeping
the others fixed. The new parameter moves pass analytic, numerical and pinned
BEAST 2 density checks. On this tiny workload, adding them modestly increased
runtime per fixed step count and reduced sampling efficiency. The relaxed-clock
preset also exposes poor gamma-shape mixing with only eight characters, including
when the FBD process is fixed. These results are not a speed comparison with
BEAST or an empirical fossil analysis.

## Workload and protocol

- Apple M4 Max, 16 CPU cores (12 performance, 4 efficiency), 64 GB;
  macOS 26.6.2, native aarch64 Rust release, fat LTO, one codegen unit, NEON backend.
  Full compiler information and engine/source hashes are in the JSON records.
- Constructed `fixtures/validation/dating/matrix.nex`: four tips, eight binary
  observed characters, four compressed patterns, Mkv. Two tips are extant,
  one fossil is fixed at 12 Ma, and another has age evidence uniform on [5,9] Ma.
- Fixed settings: `fixtures/validation/dating/dating.json`. Estimated settings:
  `fixtures/validation/fbd-estimation/dating.json`, with all five process priors.
  Both estimate the global clock and the interval age. No clades are constrained.
  The configurations define different posterior distributions.
- Strict preset: one character-rate category, strict clock. Reference preset:
  four gamma categories and lognormal relative branch rates, using existing
  default priors and proposal adaptation. Process-move initial weights are 3 each.
- Three repetitions of each configuration, one million steps per chain, logging
  every 100, four independent chains concurrently. Groups run sequentially, with
  workload order reversed in the middle repetition; no separate warmup.
  Group seeds are 41, 51, 61; child seeds are in the records.
- External wall time includes process launch, sampling, trace/tree IO, checkpoints
  and the final chain comparison. It excludes this report's Python postprocessing.
  ESS and rank-normalized split/folded R-hat use 10% sample-count burn-in (9,001
  retained samples per chain). Reported minimum ESS is across every sampled scalar
  and chain; verified fixed values are excluded. ESS/s is that minimum divided by
  the **four-chain group wall time**, not the sum of chains' ESS.

## One-million-step runs

Ranges cover all three repetitions; wall time is median [minimum–maximum].

| Process | Clock preset | Group seconds | Minimum ESS range | Minimum ESS/s range | Worst R-hat | Largest clade difference |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Fixed | Strict | 1.156 [1.035–1.165] | 872–1187 | 754–1054 | 1.0012 | 0.0183 |
| Estimated | Strict | 1.238 [1.235–1.398] | 341–391 | 276–280 | 1.0025 | 0.0142 |
| Fixed | Relaxed + gamma | 1.687 [1.460–1.771] | 15–23 | 10–14 | 1.0572 | 0.0270 |
| Estimated | Relaxed + gamma | 1.811 [1.634–1.869] | 7–16 | 4–9 | 1.0961 | 0.0519 |

The strict runs meet the common scalar checks ESS > 200 and R-hat < 1.01 in
this experiment. The relaxed runs do not. Gamma shape (and its algebraically
related prior column) has the lowest ESS in every relaxed run. Among the five
estimated FBD quantities alone, minimum ESS is 215 and maximum R-hat is 1.0069
in those short relaxed runs. That does not rescue the full model's diagnostics.

Clade differences are absolute differences between the highest and lowest chain
frequencies. They assess the recorded clades, not complete topology convergence;
`topology_convergence_assessed` remains false. Scalar agreement does not prove
that every relevant tree region was explored.

## Longer diagnostic follow-up

Because the short relaxed runs mixed poorly, both relaxed configurations were
run for ten million steps per chain, four chains, seed 41, otherwise unchanged.
There is one group per configuration, not a repeated timing benchmark.

| Process | Group seconds | Minimum ESS | Minimum ESS/s | Worst R-hat | Largest clade difference |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fixed | 14.209 | 150 | 10.57 | 1.0049 | 0.0051 |
| Estimated | 17.443 | 119 | 6.81 | 1.0037 | 0.0124 |

R-hat and clade agreement improve, but gamma shape still falls below ESS 200.
These are **not declared converged runs**. The estimated FBD quantities themselves
have minimum ESS 2,976 and maximum R-hat 1.0002. This separates successful process
sampling on the constructed target from the remaining gamma mixing limitation.
No operator or prior was changed to improve this report's numbers. Further
investigation is tracked in [#42](https://github.com/getcortexlabs/rove/issues/42).

The estimated strict runs show within-chain Pearson correlations of 0.79–0.83
between origin and root height, −0.45 to −0.38 between birth rate and origin,
and −0.30 to −0.19 between global clock and root height. Clock versus interval
age is weak here (−0.028 to 0.009), which does not establish general independence.
The longer relaxed run retains origin/root correlation around 0.81. These
tradeoffs are why free parameters need explicit proper priors and sensitivity
analysis. Neither a mean estimate nor high ESS establishes biological
identifiability. No empirical calibration or parameter-recovery claim is made.

## Reproduce and inspect

`report.json` and `extended.json` retain every scalar summary, clade frequency,
correlation, seed, input/source/binary hash and measured runtime. Raw traces and
trees stay local; their hashes are recorded. The original script completed all
runs but failed while querying hardware through sandbox-denied `sysctl`. The
reports were recovered from those outputs and wall times printed to 0.001 s;
no measurement was omitted. The script now uses `system_profiler` and saves each
result as it completes. Hardware records exclude machine identifiers.

```sh
cargo build --release --bin rove
python3 tools/benchmarks/fbd_estimation.py --output .local/fbd-cost
python3 tools/benchmarks/fbd_estimation.py --output .local/fbd-cost-extended \
  --steps 10000000 --repetitions 1 --presets reference
```

The process-estimation [model and reference checks](../../../fixtures/validation/fbd-estimation/README.md)
define the observation conditioning and priors. Runtime varies by machine and
load. These measurements do not predict M1 times, larger matrices, or the cost
needed for a specified scientific precision.
