# Sampler phase occupancy

Commit this protocol and measurement code before production. macOS `sample`
could not attach in the local sandbox; Instruments failed to launch through
DTServiceHub. Rather than present the old native stack profile as current,
this experiment uses an explicitly instrumented, optional benchmark build.

## Workloads

Use the same inputs, conditioning, partitioning and default tuning as the
[independent-chain budget study](../convergence/PROTOCOL.md). Profile both
delivered CLI presets on each matrix:

- `strict`: strict clock, one rate category, no gamma or branch-rate parameters.
- `reference`: lognormal relaxed clock and four estimated gamma categories.

This compares delivered presets, **not** the isolated cost of relaxing a
clock with every other setting held fixed. No dating or constraints are used.

Run 2,000,000 steps on 90 taxa and 300,000 on 500 taxa. Log every 1,000 steps,
checkpoint every 100,000. Use seeds 2026103181 and 2026103182 in two repetitions.
In the first repetition, run 90 strict, 90 reference, 500 strict, 500 reference;
reverse that order in the second. Each case has a normal and instrumented run
with the **same** seed; run normal first in repetition one and instrumented
first in repetition two. Require identical trace values and sampled trees.

Build both binaries with ordinary release optimizations (O3, fat LTO, one
codegen unit, default target), NEON, with only `--features profile-sampler`
different. Record binary, source and input hashes. Run sequentially, without
builds, tests or independent-chain experiments competing for the machine.
Capture power/load before and after. No CPU pinning or thermal stabilization
is imposed, so paired timings include ordinary run-to-run variability.

## Measurement and attribution

The instrumented binary supports only one fresh sample chain. A separate
observer sleeps nominally 1 ms, then reads the current phase from a relaxed
atomic. Start counting after the first 100,000 proposals, beyond the default
10,000-tunable-proposal adaptation delay for these schedules. Verify changed
tuning values in the final operator record. Tuning continues during the
measurement: this is not a stationary or converged-run profile.

Nested phase guards assign every observation once, to the innermost active
phase. Report counts and fractions for node partials, rescaling, root
likelihood, likelihood update/traversal/transition work, tree moves and state
bookkeeping, priors, random number generation, output/checkpoints, and other.
The NEON maximum search is fused with node arithmetic and remains in node
partials; rescaling includes exponent extraction, scaling partials and adding
child exponents. Output includes serialization/checkpoints, while final
metadata formatting and uncategorized CLI work can fall into other.

These are **wall-clock phase observations**, not native stack samples or CPU
instruction attribution. Descheduling can leave the last phase visible.
The observer's wake-up interval is approximate; record sample count, first
and last observation and maximum gap. Require at least 2,000 observations per
run, publish both repetitions, and report the variation. A rare phase with
zero observations is unresolved, not evidence that it costs zero.

Phase stores and the observer perturb execution. Report the instrumented/
normal whole-process wall ratio for every pair and do not subtract it from
phase counts or claim unbiased production fractions. If the median overhead
exceeds 10% in a workload, treat its percentages as coarse attribution and
state that a native profile outside the sandbox is still needed for precise
optimization decisions. Verify that instrumentation does not alter either
trace values or tree samples. The default build has no profiling code.

Define the kernel fraction as node partials plus rescaling, excluding root
reduction and tree traversal. Compute the conditional Amdahl ceiling
`1 / (1 - kernel_fraction)` for making those phases free while everything else
is fixed. It is a bound **within the instrumented profile**, not a promised
production speedup. Report normal-build whole-process steps/s separately.

## Commands

Use the binaries prepared by the convergence protocol, then:

```sh
python3 tools/benchmarks/sampler_study.py profile --output .local/sampler-study
python3 tools/benchmarks/sampler_study.py publish-profile --output .local/sampler-study \
  --report benchmarks/profile/2026-10-31
```

Raw files remain local. Publish aggregate counts, fractions, operator metrics,
commands, hashes and hardware only. Update the design guide in all four
languages with the measured picture and its instrumentation limitations.
Do not claim speedups over BEAST or over an earlier, differently configured
profile. This experiment changes no sampler defaults or application UI.
