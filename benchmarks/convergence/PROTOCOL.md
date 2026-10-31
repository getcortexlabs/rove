# Independent-chain budgets on 90 and 500 taxa

Freeze this protocol and the measurement code in Git before production runs.
This is an experiment with the current sampler, not a proposal optimization.
The earlier seed-42 desktop run is observational context only; it cannot enter
these independent chains or determine a passing threshold after measurement.

## Targets and budget

| Case | Input | Conditioning | Steps per chain | Logged every | Diagnostic prefixes |
| --- | --- | --- | ---: | ---: | ---: |
| morphology90 | `fixtures/benchmarks/morphology_90x250.nex` | Mk | 20,000,000 | 1,000 | every 2,000,000 |
| zilla | Private 500 × 759 matrix supplied by Rafael and Thiago | Mkv | 5,000,000 | 500 | every 1,000,000 |

Use four independent concurrent chains, seeds 2026103101–2026103104 for
morphology90 and 2026103111–2026103114 for zilla. Complete and diagnose the
90-taxon case before starting zilla, whether or not the first case passes.
Do not extend or replace a selected chain after inspecting its results.
The cap is the stopping rule for execution. All chains run to that cap;
the prefix rule below identifies the first sustained diagnostic pass within
it. Thus this study does **not** time the app's automatic stopping feature.

Both cases use the reference preset: unordered uniform Mk state frequencies,
four midpoint gamma categories with estimated shape, independent lognormal
branch rates with estimated sigma and distribution mean one, fixed clock
multiplier one, birth–death tree prior and the existing parameter priors.
Use observed-state partitions, gaps as missing, no dating, no monophyly
constraints, the default classical move weights and tuning. In particular,
gamma weight remains 0.1; do not select a newer schedule from these results.
The 500-taxon Mkv target differs from #8's Mk target. Its private input SHA-256
is `fd3ed2bb5b35e9814cae9b033f1c78fc473a0a01b846fde2eddf495992f1482a`.
This is a computational workload, not a validation of its biological model.

Checkpoints are written every 100,000 steps. Progress events are written at
each diagnostic prefix. Use the ordinary release binary, NEON, one likelihood
thread per chain, O3, fat LTO, one codegen unit, default CPU target, and no
experimental features. Record whole-group wall time, per-chain wall time,
power/load before and after, hardware/OS/toolchain, commands and input,
binary, protocol, trace and tree hashes. Do not run builds, tests or the
separate profiling experiment concurrently with inference.

## Prefix diagnostics

After completion, analyze every declared prefix, including the cap. Discard
floor(10% of that prefix's draws), counting state zero. Use the existing Rust
ESS estimator (paired-positive autocovariance, lag < 2,000) and rank-normalized
split R-hat with folded ranks. Require at least 1,000 retained draws, ESS ≥ 100
in **every** chain for every traced scalar, and R-hat ≤ 1.01. Four chains then
give at least 400 summed within-chain ESS; this is not pooled bulk/tail ESS.
Unavailable, constant or lag-limited diagnostics fail rather than disappear.

For topology, compare each retained tree's nontrivial rooted clades to the
state-zero tree of chain 1, a reference chosen by this rule before sampling.
Report distinct topologies per chain and the rooted Robinson–Foulds distance
series' ESS and R-hat, with the same gates as the scalar statistics. A constant
RF series fails; a common distance alone does not prove tree-space mixing.

For every clade with pooled frequency ≥ 0.01, require the largest minus smallest
frequency among the four chains ≤ 0.05 and each chain's estimated MCSE ≤ 0.0125.
MCSE is the standard error across 20 contiguous equal-size batches; report
the omitted remainder and count clades constant within every chain. Constant
clades do not independently certify mixing. Batch errors depend on adequately
long batches; all these diagnostics remain evidence, not proof of convergence.

A prefix passes only when **all** scalar, RF and clade criteria pass. The first
sustained pass is the second of two adjacent passing prefixes; an intervening
failure resets the count. Report every check and every parameter, not just the
best chain or minimum ESS. If no sustained pass occurs, report “did not meet
the criteria within the budget.” Do not infer that convergence is impossible.

Use operator attempts, acceptance/failure rates and the separate #68 profile
to identify follow-up hypotheses. Acceptance alone cannot establish which
move causes poor mixing. ESS/time from failing chains is descriptive only.
No BEAST run is timed here and no comparative performance claim follows.

## Commands and publication

```sh
CARGO_HOME="$PWD/.local/cargo-summary" CARGO_NET_OFFLINE=true \
  python3 tools/benchmarks/sampler_study.py prepare --output .local/sampler-study \
  --zilla "$ZILLA_MATRIX"
python3 tools/benchmarks/sampler_study.py budget --output .local/sampler-study
python3 tools/benchmarks/sampler_study.py publish-budget --output .local/sampler-study \
  --report benchmarks/convergence/2026-10-31
```

The runner refuses changed inputs, binaries, protocols or tracked source.
Preparation records the frozen source commit. Raw data stay in the ignored
local directory; only allow-listed aggregates are exported. Never publish the
500-taxon alignment, labels, trees, XML, clade membership maps or checkpoints.
Retain failures and report deviations. No extrapolation to another Mac model.
