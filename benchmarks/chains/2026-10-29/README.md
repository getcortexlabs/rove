# Independent-chain execution and diagnostics

Rove now runs independent chains on separate Rust threads and compares their
samples. The Mac app offers one, two or four chains, keeps group history and
opens diagnostics and tree summaries for a selected member. This implements
[#23](https://github.com/getcortexlabs/rove/issues/23). Classical proposals remain
the default; this experiment does not change the sampling distribution or
repeat the BEAST performance comparison.

## Throughput on Apple silicon

The measured engine is commit `9689b36de7cdd3cc7095fa1fecc65bc1f03a1e3a`, release
profile with fat LTO and one codegen unit, using the native NEON backend.
Executable SHA-256:
`d7a11624ae710badf7d99cfaa41124de8e11ac4ae7a59e52091a729f057c9c91`.

Hardware: Apple M4 Max, **12 performance cores and 4 efficiency cores**, 64 GB,
macOS 26.6.2. The hardware report identifies the processor layout as
`proc 16:12:4:0`; sandboxed `sysctl` queries were unavailable. macOS scheduled
all workers at their default QoS. There was no core affinity, and the experiment
does not attribute an individual worker's time to either core class. Counts
12 and 16 expose the change in throughput around the performance-core count,
without assuming which cores were actually used.

Each chain ran 2,000,000 steps on `fixtures/validation/reference-mk/matrix.nex`
(20 taxa, 50 characters), with the reference relaxed clock, four gamma
categories, Mk conditioning, T1–T19 monophyly, and one sample every 1,000 steps.
The first seed was 2026102900; subsequent chains received successive seeds.
Three repetitions of each count ran in a fixed shuffled order after a 500,000-step
warm-up. No build or other Rove experiment ran during the timed series. This was
a working Mac with other applications present, not an isolated benchmark host.

| Chains | Median total seconds | Total range | Median sampling seconds | Aggregate throughput vs. one chain |
| --- | ---: | ---: | ---: | ---: |
| 1 | 3.699 | 3.546–4.162 | 3.691 | 1.00× |
| 2 | 3.543 | 3.515–3.569 | 3.468 | 2.09× |
| 4 | 4.056 | 3.695–4.185 | 3.905 | 3.65× |
| 8 | 4.675 | 4.623–12.657 | 4.389 | 6.33× |
| 12 | 5.856 | 5.488–6.649 | 5.416 | 7.58× |
| 16 | 7.968 | 7.293–9.163 | 7.356 | 7.43× |

Total time includes process startup, sampling, output, and the final multi-chain
comparison. The sampling interval includes worker setup, writes and joins;
it excludes the final comparison. The single-chain command produces its usual
outputs without a cross-chain comparison. The speedup column compares **total
steps per wall second**, not latency of one chain or time to convergence.
The two-chain result slightly exceeds ideal linear scaling relative to the
single-chain median; ordinary timing and seed-dependent variation are not a
claim of superlinear hardware scaling.

The eight-chain range includes a 12.657-second repetition. One-minute load
averages ranged from 5.19 to 25.16. All observations are retained in
[throughput.json](throughput.json), including CPU time, peak RSS, commands,
source hashes, load averages and hardware details. Three repetitions on this
small matrix do not establish an optimal chain count for other data or Macs.

[Sampling quality](sampling-quality.json) records per-chain ESS and scalar
R-hat, retained draws and the largest unconstrained clade-frequency difference
for one repetition at each multi-chain count. These short throughput runs are
not convergence-certified analyses. Automatic stopping was disabled.

## Correctness and reference agreement

- The CLI integration test replays each parallel member as a standalone run,
  including seed wraparound at `u64::MAX`, and compares every trace and tree
  sample. The stopping test verifies the standalone prefix at the actual
  stopping step and checks that its trace remains readable by diagnostics.
- All **129 timed chain outputs** reproduce the same trace and tree content
  whenever their seed occurs again, across thread counts and repetitions.
  [identity.json](identity.json) records hashes for the 16 distinct seeds.
- Native diagnostics were compared with the four archived Rove chains in each
  of the six #12 cases: **24 chains, 14 statistics per chain**. The maximum
  R-hat absolute difference from `tools/beast/compare_runs.py` is 1.47e-12;
  the maximum ESS relative difference from the archived Java/BEAST-convention
  summaries is 1.32e-9. Every observed clade frequency matches exactly, including
  clades absent from an individual chain. [archive-validation.json](archive-validation.json)
  records the source hashes and per-case errors.
- Those historical `run.json` files predate status and completed-step fields.
  Validation adds those fields to temporary metadata copies only, then lets
  the current CLI verify complete trace coverage. Scientific files are read
  unchanged through symlinks; original metadata hashes are recorded.
- Rank-normalized split R-hat follows the maximum of rank and folded-rank
  estimates described by [Vehtari et al.](https://arxiv.org/abs/1903.08008).
  Ties use average ranks and an odd middle draw is omitted. Fixed oracle tests
  include ties, scale and location differences, trends and constant chains.
  Constants and undefined/infinite diagnostics cannot satisfy the stopping rule.

The optional stopping rule requires every traced scalar's ESS to reach its
threshold in every chain and R-hat to stay below its threshold, with at least
1,000 retained draws and two consecutive successful checks. Lag-limited ESS
estimates block stopping. Workers check at the same saved state; termination
reason, completed steps and the checks are recorded. Defaults are 200 ESS,
1.01 R-hat, 10% burn-in and a check every 1,000 sampling intervals.
These conditions do not certify topology convergence. Clades imposed by the
setup are marked constrained, and constant clade indicators remain explicitly
limited evidence. No pooled trace or pooled MCC tree is generated.

## Reproduce

```sh
cargo build --release --locked
python3 tools/chains/benchmark.py --steps 2000000 --output .local/parallel-timing
python3 tools/chains/validate.py --archive .local/issue12/production \
  --output .local/parallel-validation.json
python3 tools/chains/report.py --timings .local/parallel-timing/report.json \
  --validation .local/parallel-validation.json --output .local/parallel-report
python3 tools/chains/rhat_fixture.py
```

The archive-validation step needs the local #12 raw archive; public summaries
alone cannot regenerate its traces. The other commands use committed fixtures.
The Rust toolchain and timed arguments are in `throughput.json`; `$REPO` denotes
the repository root in published commands.

Validation also covers malformed progress, worker failure, group cancellation,
legacy setup loading, history reopening and missing member files. The native
suite runs actual Rust processes for selected-chain diagnostics, tree summary
and an early diagnostic stop. Hosted view captures were inspected in English
and Portuguese. The implementation passed Rust tests, Clippy, formatting, native
tests and the documentation build; CI runs these checks on the supported targets.
