# Sampler phase occupancy on Apple M4 Max

The numerical kernel accounts for about 68% of phase observations in the
90-taxon reference preset and 82% in the 500-taxon reference preset. Within
these instrumented profiles, eliminating node updates and rescaling entirely
would cap the gain within the sampled execution window at about 3.14× and
5.50×, respectively. These are conditional bounds, not measured or promised
production speedups.

The [protocol](../PROTOCOL.md) and measurement code were committed in
`ee592836b5234d7af562e265ff5170438063cb3d` before production. This experiment
adds measurement tools; it changes no proposal defaults or application UI.

## Method and workloads

An Apple M4 Max (12 performance and 4 efficiency cores, 64 GB) ran macOS
26.6.2. Rust 1.99.1 / LLVM 23.1.1 built both binaries with O3, fat LTO, one
codegen unit, default target features and the NEON backend. Only the optional
`profile-sampler` build contains phase guards and an observer thread. Default
builds contain neither. The 90 × 250 public matrix uses Mk; the private
500 × 759 matrix uses Mkv and observed-state partitions of 180 binary, 140
ternary and 439 four-state characters. Every observed character is a distinct
pattern in these inputs.

`strict` is the delivered strict-clock, one-category preset. `reference` adds
four gamma categories with estimated shape and a lognormal relaxed clock.
The table compares these presets, not the isolated cost of a relaxed clock.
Both keep the default moves and tuning, gaps as missing, and no constraints
or dating. There is no BEAST timing in this experiment.

Two repetitions use seeds 2026103181 and 2026103182. Each pairs normal and
instrumented execution of the same seed; workload and binary order reverse
in repetition two. All eight pairs produced identical trace values, sampled
trees **and operator statistics**. Runs cover 2 million proposals for 90 taxa
and 300,000 for 500 taxa, logging every 1,000 and checkpointing every 100,000.

Phase observations start after 100,000 proposals. Final operator records
confirm that tuning was active. An observer sleeps nominally 1 ms between
reads of the current nested phase. Each observation belongs to exactly one
phase. There were 12,509–48,626 observations per run; the largest wake-up gap
was 4.52 ms. These are wall-clock phase observations, **not native CPU stack
samples**. Descheduling and instrumentation can affect attribution.

## Where the time goes

Percentages are the arithmetic mean of the two repetition fractions, with
equal weight per repetition. Raw counts and each repetition remain in
[results.json](results.json).

| Phase | 90 strict | 90 reference | 500 strict | 500 reference |
| --- | ---: | ---: | ---: | ---: |
| Node partials, including the fused maximum search | 25.79% | 46.65% | 40.30% | 58.57% |
| Rescaling and subtree exponent accumulation | 26.20% | 21.53% | 29.37% | 23.27% |
| Root likelihood and ascertainment reduction | 15.17% | 6.81% | 2.47% | 1.37% |
| Likelihood update: traversal, dirty paths, branch distances | 17.20% | 12.82% | 22.33% | 12.36% |
| Proposal construction and state bookkeeping | 4.23% | 2.65% | 1.34% | 0.70% |
| Priors | 6.88% | 6.25% | 2.42% | 1.81% |
| Random generation | 0.06% | 0.04% | 0.01% | 0.01% |
| Trace/tree output and checkpoints | 4.39% | 3.18% | 1.71% | 1.89% |
| Other | 0.08% | 0.06% | 0.05% | 0.03% |

The raw `tree_and_bookkeeping` category includes rate proposals and gamma
category construction as well as tree proposals. Random generation covers
the generator and normal/exponential transforms; tiny surrounding arithmetic
can remain in the enclosing phase. Low counts do not establish zero cost.

The rescaling phase includes extracting binary exponents, multiplying partials
by the scale, and accumulating child exponents. Finding maxima is already fused
into the NEON node update, so that part belongs to node partials. Replacing the
old rescaling logarithm did not eliminate memory passes or exponent bookkeeping.

| Preset | Kernel fraction, repetitions 1 / 2 | Ceiling from mean fraction | Normal wall seconds, repetitions 1 / 2 | Steps/s from mean wall time |
| --- | ---: | ---: | ---: | ---: |
| 90 strict | 52.41% / 51.58% | 2.08× | 18.99 / 16.67 | 112,167 |
| 90 reference | 68.07% / 68.29% | 3.14× | 34.53 / 32.88 | 59,340 |
| 500 strict | 69.50% / 69.84% | 3.30× | 47.48 / 37.61 | 7,052 |
| 500 reference | 81.97% / 81.69% | 5.50× | 99.34 / 96.16 | 3,069 |

Here `kernel = node partials + rescaling` and `ceiling = 1 / (1 - kernel)`.
Root reduction and tree traversal stay outside that hypothetical improvement.
The fractions exclude startup and the first 100,000 proposals; the ceiling
applies to the observed window. Applying it to an entire production run would
require that window to represent the rest of execution.
Steps/s uses normal-build whole-process time, including initialization,
adaptation, output and checkpoints. It describes these budgets and trajectories;
it is not time to a reliable posterior sample.

## Instrumentation and environmental variation

| Preset | Instrumented / normal wall time, repetitions 1 / 2 |
| --- | ---: |
| 90 strict | 0.988 / 1.005 |
| 90 reference | 0.971 / 0.988 |
| 500 strict | 0.926 / 1.008 |
| 500 reference | 0.973 / 0.995 |

These paired ratios include code-layout, scheduling and frequency variation;
they do not isolate the cost of each atomic store or demonstrate an advantage
from instrumentation. No workload triggered the predeclared median >10%
overhead rule. Native sampling could not attach to the process in this environment, and
the Instruments tracing service could not initialize. A native CPU profile
remains useful before fine-grained kernel changes; this profile supplies coarse phase attribution with explicit limits.

The machine was on AC power and charging. Observed one-minute load averages
ranged from 7.81 to 13.59; it was not a dedicated, CPU-pinned benchmark host.
Thermal queries were unavailable in the sandbox, recorded as errors rather
than interpreted as evidence of no throttling. No local builds, tests or other
Rove experiments ran concurrently with these timings.

An earlier attempt was interrupted when Android emulators, OrbStack and Chrome
were competing for the machine and load averages exceeded 400. The user closed
those workloads, then this entire profiling suite was restarted in a fresh
directory using the same frozen binaries and protocol. No pair from that
interrupted attempt enters these tables; all partial outputs remain local.

## Cost from the terminal inference states

The [supplement protocol](../TERMINAL-STATES.md) was committed in `5800321`
before these measurements, after the 90-taxon diagnosis was known and while
the 500-taxon chains were still running. Its harness ran from `1362ed7`, with
unchanged engine sources, Cargo files and compiler relative to the original
experiment. It selected chains 1 and 2 by index, copied their 20M / 5M states,
and continued each for 1M / 100,000 proposals with the saved RNG and tuning.
These draws do not enter the convergence study or extend its caps.

All four pairs round-tripped the original checkpoints exactly and produced
matching final full-state hashes between normal and instrumented builds.
The original checkpoints were unchanged. The timed loop contains only
`Sampler::step`: parsing, restoration, serialization and trace/tree/checkpoint
output are excluded. Whole-process time is reported separately. Normal runs
first for chain 1 and instrumented first for chain 2.

| Source state | Proposals in window | Normal sampling seconds | Normal process seconds | Sampling steps/s | Instrumented / normal sampling time | Phase samples |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 90 taxa, 20M, chain 1 | 1,000,000 | 16.938 | 17.236 | 59,037 | 1.002 | 11,276 |
| 90 taxa, 20M, chain 2 | 1,000,000 | 16.454 | 16.485 | 60,774 | 1.004 | 10,959 |
| 500 taxa, 5M, chain 1 | 100,000 | 31.807 | 32.512 | 3,144 | 0.987 | 20,838 |
| 500 taxa, 5M, chain 2 | 100,000 | 29.665 | 30.350 | 3,371 | 0.998 | 19,606 |

Phase fractions below average the two selected states equally, as in the main
report. These are single-chain continuation windows, not four-chain timings.

| Phase | 90 taxa, from 20M | 500 taxa, from 5M |
| --- | ---: | ---: |
| Node partials | 48.51% | 60.57% |
| Rescaling | 22.46% | 23.13% |
| Root likelihood | 6.81% | 1.28% |
| Likelihood update | 12.93% | 12.27% |
| Proposal construction and bookkeeping | 2.63% | 0.77% |
| Priors | 6.58% | 1.97% |
| Random generation | 0.07% | 0.01% |
| Output | 0.00% | 0.00% |
| Other | 0.02% | 0.01% |

Node partials plus rescaling account for **70.97% / 83.70%** of these late
observations. The same phases dominate after the full inference budgets.
Since these windows omit output and start from different states, their
fractions cannot isolate the effect of chain age or replace the full CLI
rates above. Eliminating this kernel would imply bounds of
3.44× / 6.13× for these **output-free sampled windows only**.
The 500-taxon states still fail the convergence criteria; profiling them is
not evidence of successful posterior exploration.

There were 10,959–20,838 phase observations per window; the largest observer
gap was 4.28 ms. One-minute load ranged from 11.95 to 15.14, on AC power;
thermal status remained unavailable. No builds, tests or inference studies
ran alongside these measurements. The small paired time differences include
environmental and instrumentation effects, as in the main experiment.
[terminal-states.json](terminal-states.json) retains commands, build/source/input/
state hashes, both timings, power/load, raw counts and fractions. Private
checkpoints and job files remain local.

## Consequences and reproduction

Node arithmetic is the largest measured phase in the 500-taxon relaxed case.
Rescaling is still substantial in every workload, and traversal/branch-update
work is especially visible in the strict presets. These measurements justify
testing memory-pass and changed-path improvements; they do not establish that
a particular optimization will improve ESS per second. Sampling quality is
assessed separately in the [four-chain study](../../convergence/2026-10-31/README.md).

Follow the [protocol commands](../PROTOCOL.md#commands). The completed profile
used `.local/sampler-study-profile-repeat`, copied from the prepared binary set;
the interrupted attempt stayed in `.local/sampler-study`. The report records
commands, input/protocol/binary hashes, per-run power/load, operator statistics
and raw phase counts. `$RUN`, `$REPO`, `$PUBLIC_MATRIX` and `$PRIVATE_MATRIX`
replace local paths. Private labels, alignments, trees and checkpoints are
not published. No extrapolation to M1 Air, M1 Pro, GPUs or another BEAST engine
follows from these measurements.
