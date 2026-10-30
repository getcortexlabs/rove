# Expanded likelihood and inference benchmark

This protocol is committed before production timings. Preparation uses seed
81991 and 1,000 BEAST steps only to check inputs and arithmetic. A separate
20,000-step 90 × 250 pilot sizes the workload. Neither pilot enters the timing
or posterior comparisons. Do not extend selected chains after seeing results.

## Inference

| Case | Observations | Conditioning | Steps per chain | Log interval |
| --- | --- | --- | ---: | ---: |
| reference | Thiago's public 20 × 50 fixture | Mkv; T1–T19 monophyletic | 10,000,000 | 1,000 |
| morphology90 | Reproducible binary 90 × 250 fixture | Mk; no constraint | 1,000,000 | 100 |
| zilla | Private 500 × 759 input supplied by Thiago | Mk; no constraint | 10,000 | 10 |

Use seeds 20261201–20261204 in each engine, alternating BEAST/Rove order.
Matching seeds do not imply matching trajectories. Execute one process at a
time, with one likelihood thread, on the same machine. No concurrent builds,
tests or diagnostics during measurement. Record AC/battery state, load,
hardware, OS, toolchains, binaries, input and package fingerprints.

Use Rove's current default operators and BEAST 2.7.8 with MM 1.2.1, ORC 1.2.1,
BEASTLabs 2.0.3 and BEAST.app 2.7.8. This measures the delivered algorithms:
proposal schedules and adaptation are different. Both targets have uniform
unordered Mk state frequencies, four discretized gamma categories, a shared
gamma shape, an uncorrelated lognormal relaxed clock, and the same birth–death
and parameter priors as `fixtures/validation/reference-mkv/beast.xml`.
Keep the clock mean fixed at one. Use observed-state partitions and interpret
gaps as missing in both engines; export ambiguity masks from Rove to BEAST.
Report characters and compressed patterns separately, by partition.

Rove uses an ordinary release build (opt-level 3, fat LTO, one codegen unit,
no `target-cpu` override, no experimental features) and its native backend.
BEAST uses Java 17, `-Xms256m -Xmx2g -Xss8m`, `-Djava.only=true`, and scaling
enabled. Neither is pinned to a CPU core. Trace and tree intervals match;
checkpoint interval is 100,000 steps and screen interval is 100,000. File
formats, initial trees, additional BEAST log columns, and checkpoint sizes
differ. Time process launch through exit, including input, initialization,
JVM warm-up, adaptation, logging and checkpoints. Do not subtract startup.
Keep raw files locally and record hashes. Reject truncated output even when
the process exits successfully.

Before timing, compare every partition at a BEAST pilot state on both Rove
backends. Frozen gamma categories must agree within 1e-7 log units per
partition. Independently computed gamma categories must give total likelihood
and posterior differences below 2e-5; other prior components below 1e-8.
Repeat the checks at the final BEAST states after timing. No state from the
private matrix is published.

Discard floor(10% of samples), including the initial sample in the count.
Use the existing BEAST `ReferenceDiagnostics` for both engines' means,
standard deviations, ESS and tree summaries, and rank-normalized split R-hat
(including folded ranks) from `compare_runs.py`. Normalize the BEAST sum of
branch rates by 2N−2 before comparing it to Rove's arithmetic mean.

For each continuous statistic, report four within-chain ESS values, ESS per
whole-process second, pooled means with within/between-chain Monte Carlo
error, and R-hat. Keep the previous gates: R-hat ≤ 1.01, each ESS ≥ 100,
total ESS ≥ 400, and mean differences ≤ four combined Monte Carlo standard
errors. A fixed step count is a throughput measurement, not a guarantee of
convergence. ESS/time from nonstationary chains is descriptive only and must
not establish a winner in time to a reliable answer.

Report distinct topologies, rooted RF distance to a common reference,
RF ESS and R-hat, and pooled clade-frequency differences with 20-batch and
between-chain uncertainty. The RF reference is the pilot's final tree,
chosen before production, not a known true tree. Compare clades occurring
with pooled frequency above 0.01 in either engine; require absolute difference
≤ 0.05 and each engine's estimated MCSE ≤ 0.0125. Exclude the imposed ingroup
constraint from evidence of recovery. Constant indicators do not prove mixing.

## Kernels

Export the existing 12 numerical workloads from `benches/likelihood.rs`.
Retain the public reference Mk/Mkv cases, 90 × 250, and controlled cases
varying states, categories, missing values, repeated patterns, balanced and
ladder trees, and SIMD padding boundaries. All implementations receive the
same tip partials, tree, branch distances, category rates and weights.

Compare four operations: cached full pruning, full pruning with changed
branch lengths, one terminal branch update, and one internal branch update.
The latter two recompute precisely the affected ancestor path. Lengths
alternate between 1.1 and 0.9 times their initial value. Validate the initial
state and both changed states to an absolute tolerance of 1e-8 before timing.

Rove runs both generic matrices and its direct Mk path, portable and NEON.
The Java adapter uses BEAST's BeerLikelihoodCore (BeerLikelihoodCore4 for
four states). BEAGLE 4.0.1 uses its JNI CPU adapter, double precision,
synchronous execution, no worker threads, and a preference for its vector
implementation. Require and record the actual implementation and flags;
never silently accept a Java fallback. BEAGLE's `VECTOR_SSE` label also names
its ARM translation in this release; do not describe it as x86 execution.

All calculate scaled likelihoods, but scaling algorithms differ: Rove uses
power-of-two scaling with cached subtree exponents, Beer uses its enabled
threshold scaling, and BEAGLE explicitly writes each changed node's log
scale and accumulates all internal scale factors. These are equivalent
probabilities, not identical arithmetic or instruction counts. Matrix
generation uses the same Mk closed form in both adapters. JNI calls, buffer
transfer, scale accumulation and retrieval of pattern logs count in BEAGLE
timings. These are adapter-level CPU results, not a claim about every BEAGLE
integration, native C API, GPU or model.

Run three fresh processes per implementation, reverse implementation order
in the middle repetition, and reverse Rove's internal order there. Warm each
Rove metric for 100 ms and each Java/JNI metric for 500 ms; calibrate batch
size to at least 10 ms, then collect nine batches. JVM warm-up is finite,
not an assertion that every JIT compilation has finished. Publish all batch
times, per-process medians, their range, and p10/p90 across batches.

Keep Rove's existing parse, compression, setup, accepted/rejected proposals,
store/restore and allocation measurements in the same run. Adapter setup is
recorded separately as a single cold construction per metric, not a comparable
setup benchmark. Java/native allocations are not profiled; do not infer zero
allocations for them. No MCMC or posterior conclusions follow from kernel ratios.

## BEAGLE compatibility and publication

The pinned MM model returns no eigendecomposition. Forcing
`BeagleTreeLikelihood` in the full XML fails during initialization, before
sampling. Record this unsupported configuration; use direct transition
matrices in the standalone kernel adapter instead. It does not make BEAGLE a
Rove dependency and does not justify changing the target to claim an end-to-end
BEAGLE result. There is no BEAST X baseline here.

Publish aggregate results, commands, versions, hashes and public kernel
inputs. Keep the private matrix, sequences, taxon labels, XML, tree samples
and derived clade maps local. The 500-taxon case is a bounded scaling exercise,
not a biological analysis or a validated Mk choice for every character.
Do not extrapolate this machine's results to M1 Air or M1 Pro.

## Commands

Install the pinned BEAST packages using the existing reference setup. Build
BEAGLE v4.0.1 locally with CMake, JNI and CPU/SSE enabled, CUDA/OpenCL disabled.
Record the effective compiler flags: upstream overrides `CMAKE_BUILD_TYPE`
to `RelWithDebInfo`. Use `-DCMAKE_CXX_FLAGS_RELWITHDEBINFO="-O3 -g -DNDEBUG"`
so its final optimization level is O3 as well. Set paths to the local
installations and private input:

```sh
python3 tools/benchmarks/expanded.py prepare --output .local/expanded \
  --packages "$BEAST_PACKAGES" --launcher "$BEAST_LAUNCHER" \
  --java-home "$JAVA_17_HOME" --beagle-lib "$BEAGLE_LIB" --zilla "$ZILLA_MATRIX"
python3 tools/benchmarks/expanded.py measure --output .local/expanded
python3 tools/benchmarks/expanded.py diagnose --output .local/expanded
python3 tools/benchmarks/report_expanded.py .local/expanded --output benchmarks/expanded/2026-10-30
```

Preparation builds and preserves binaries. Measurement requires a clean
checkout and unchanged protocol, binaries and input fingerprints. Use a new
output directory for a new experiment. Diagnostics run only after timings.
