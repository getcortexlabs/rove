# Metropolis-coupled chains

Issue #71 asks whether four-temperature ensembles improve time to useful
posterior samples on the 90 × 250 and private 500 × 759 workloads. Freeze this
protocol, implementation and measurement tools before pilot or production
sampling. Preserve all seeds, failed diagnostics and temperature candidates.

## Target and implementation

Slot i targets `prior(x) * likelihood(x)^beta[i]`, with beta[0] = 1. Only the
cold slot writes posterior samples. Every 1,000 steps, select one adjacent
pair uniformly; accept a state exchange with log ratio
`(beta[i] - beta[j]) * (logL[j] - logL[i])`. The priors cancel. Slots retain
their RNGs and proposal adaptation; scientific state and likelihood buffers
move together. Fixed ladders use `beta[i] = 1 / (1 + i * delta)` for i = 0..3.
Count attempted/accepted swaps per neighbour and complete cold–hot–cold
journeys per state label. Swaps alone do not establish mixing.

Pin BEAST 2 CoupledMCMC 1.2.2 from its
[release archive](https://github.com/CompEvol/CoupledMCMC/releases/tag/v1.2.0).
Its bundled source and example expose `heatLikelihoodOnly=true`; use that
explicitly, `chains=4`, `deltaTemperature=0.1`, `optimise=false`,
`neighbourSwapping=true`, `resampleEvery=1000`, and `logHeatedChains=false`.
The package defaults to heating the whole posterior, which is a different
heated target. Record archive, JAR and generated XML hashes, package versions,
Java version, exact commands and seeds. Use BEAST base/app 2.7.8, MM 1.2.1,
ORC 1.2.1 and BEASTLabs 2.0.3. This is an independent posterior reference;
different operators and scheduling preclude attributing timing to Rust alone.

## Small-fixture validation

Run reference-Mk, reference-Mkv, control, homoplasy, missing and reduced from
`fixtures/validation`. Preserve each target, T1–T19 monophyly, reference preset
and default proposal schedule. Four independent runs per arm: ordinary Rove,
four-slot Rove (delta 0.1), and BEAST CoupledMCMC (settings above). Every run
has 10M steps, except reference-Mkv at 40M; log every 1,000 steps. For case
index k in the order above, use seeds `710000 + 100*k + 10*j`, j = 0..3.
Internal slot seeds must not overlap across ensembles. Matching base seed
numbers across algorithms do not mean matched random trajectories.

Execute arms sequentially, with all four independent runs within an arm
concurrent. Rotate arm order by case index. Measure whole-group wall time,
including initialization, adaptation, synchronization, logging and checkpoints.
Rove emits one progress event per logged sample in both arms.
No compilation, tests or diagnostics may compete with timed inference.
Discard floor(10% of draws), including state zero. Use the existing independent
Java trace/tree reader after timing. For every common sampled scalar require
ESS ≥ 100 per chain, summed ESS ≥ 400, rank/folded R-hat ≤ 1.01 and agreement
of pooled means within four combined MCSEs. Pooled MCSE uses the larger of
within-chain and between-chain mean variance. Missing, constant or lag-limited
scalar diagnostics prevent a pass. For every unconstrained clade with pooled
frequency ≥ .01 in either arm, require frequency difference ≤ .05 and each
arm's pooled MCSE ≤ .0125 using 20 contiguous batches and the between-chain
safeguard. Report rooted RF/indicator limitations, not fabricated ESS values.
Compare Rove coupled to both ordinary Rove and BEAST coupled. No extensions,
dropped seeds or changed proposal weights after examining results.

## Large-workload ladder pilot

Use the same models and inputs as the frozen #49 protocol: reference preset,
observed-state partitions, gaps missing, no constraints/dating, Mk for 90 taxa
and Mkv for 500. The private input SHA-256 is
`fd3ed2bb5b35e9814cae9b033f1c78fc473a0a01b846fde2eddf495992f1482a`.
Compare delta .01, .05 and .2. For each candidate run two independent ensembles
concurrently: 1M steps for 90 taxa and 250,000 for 500, log every 1,000/500.
Base seeds are 711000 + 100*k + 10*j for case k=0,1 and replicate j=0,1.
Pilot seeds are excluded from production. Selection uses the entire pilot,
including adaptation: choose the largest delta for which both replicates have
at least one complete round trip, at least 50 attempts on each edge and at
least .10 acceptance on every edge. If none qualify, choose the candidate
with the greatest minimum round-trip total across replicates, breaking ties
by the smallest delta. Report all candidates and the selection limitation:
short transient swap statistics cannot establish a useful temperature ladder.

## Large-workload production

For each case compare four ordinary cold chains, three independent four-slot
ensembles (12 workers), and four independent four-slot ensembles (16 workers).
Use the pilot-selected ladder unchanged. On this M4 Max there are 12
performance and 4 efficiency cores. Workers are not pinned; record hardware,
power and load. No extrapolation to M1 Air, M1 Pro or another Mac.

Each cold slot gets the same #49 cap: 20M steps for 90 taxa (log 1,000,
diagnose every 2M) and 5M for 500 (log 500, diagnose every 1M). Checkpoint every
100,000 steps. For case k=0,1, arm a=0,1,2 and member j=0..n-1 use base seed
`712000 + 1000*k + 100*a + 10*j`. Execute case 90 arms cold/three/four, then
case 500 arms four/three/cold. Complete every cap; prefix diagnostics identify
the second of two consecutive passes, not a timed automatic stop. Record
elapsed group wall time at every prefix as well as final group wall time.

Apply all #49 scalar, rooted-RF and clade gates without changing thresholds.
Also require summed within-chain scalar/RF ESS ≥ 400 for the three-ensemble
arm. Explicitly distinguish three independent cold samples streams from the
original four-stream protocol. The initial-tree RF anchor can saturate for
500 taxa; retain that failure and report distinct topologies and clade checks.

Primary comparison: four ensembles vs four ordinary chains, time to sustained
pass. A winner requires the small-fixture agreement gates plus a shorter time
to the same large-workload criteria. If either arm fails within budget, report
the censored outcome; do not substitute an ESS-only convergence claim. Report
every scalar's ESS/s, the minimum over scalar/chain pairs, R-hat, topology
operator attempts/acceptance, per-edge swaps and round trips. Compare the
three-ensemble resource budget separately. One group per arm is not a timing
replication; small differences cannot establish a general speed advantage.

## Reproducibility and release decision

Use one clean, frozen release executable for Rove arms: O3, fat LTO, one codegen
unit, default CPU target/NEON, no experimental features. Record Git revision,
protocol/binary/input hashes, compiler, OS, power/load, output hashes, commands,
statuses and all measurements. Historical #49 results are context only.
Keep private alignment, taxa, trees, clade keys and checkpoints in `.local`;
publish only allow-listed aggregates. Retain aborted runs and deviations.

Expose the app option only after measurement and only if all small-fixture
agreement gates pass. Keep the ordinary sampler as the default. If validation
fails, leave coupling experimental in the CLI, report why the app option is
withheld, and track the unmet scientific gate explicitly rather than implying
that the feature is ready. A valid but slower result is still a study result.
