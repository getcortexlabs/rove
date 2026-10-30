# State-count partitions: regression and throughput checks

The sampler now evaluates separate Mk/Mkv partitions on a shared tree, clock and
gamma distribution. This report checks the cost on the existing binary case
and exercises a larger matrix that the previous sampler could not analyze.
It does not compare inference speed against BEAST.

All timings use an Apple M4 Max, 12 performance and 4 efficiency cores, 64 GB
RAM, the repository release profile with fat LTO and one codegen unit, and no
`target-cpu` override. The OS schedules threads without affinity. This is a
working Mac with varying load, not an isolated benchmark host. All repetitions,
including outliers, remain in [measurements.json](measurements.json).

## Binary regression

The baseline is the pre-change release executable from `9ff49c3`. The case is
`fixtures/validation/reference-mk/matrix.nex`: 20 taxa, 50 characters, Mk,
four gamma categories, explicit lognormal branch rates and T1–T19 constrained
to be monophyletic. Each chain runs 2 million steps, samples every 1,000 and
uses seeds starting at 2026102900. Three repetitions run in shuffled order.

| Chains | Previous median | Partitioned sampler | Difference |
| --- | ---: | ---: | ---: |
| 1 | 3.3247 s | 3.3328 s | +0.24% |
| 4 | 4.1181 s | 3.8701 s | −6.02% |

These wall times include startup and durable initial/final checkpoints; groups
also include their final comparison. No periodic checkpoint is due in these
short runs. The four-chain difference is not evidence of a general speedup.

The scientific trace rows and trees match exactly in all 15 before/after chain
pairs. The single-chain trace has posterior ESS 62.27 and minimum scalar ESS
48.60 after 10% burn-in, or approximately 14.58 minimum ESS/s using the new
median wall time. This is a regression check on fixed work, not a new
convergence validation. Identical samples preserve the original diagnostics.

```sh
python3 tools/checkpoint/benchmark.py \
  --before /path/to/pre-change/rove --after target/release/rove \
  --output .local/partitions-binary
```

## Supplied 500 × 759 matrix

The local matrix has 500 taxa and 759 characters: 180 with two states, 140 with
three and 439 with four. Each partition has as many observed patterns as
characters. It contains 11,107 missing cells, 1,209 uncertainty sets and no gaps.
Unquoted hyphenated names now import. Input and scientific output files remain
local; the input fingerprint and aggregate measurements are recorded here.

One chain runs 20,000 steps with seed 42, samples every 100, uses Mkv and the
reference clock/gamma preset, and imposes no monophyly constraints. Three
repetitions per backend run in an order shuffled with Python seed 19.

| Backend | Median total wall time |
| --- | ---: |
| Portable | 11.9525 s |
| NEON | 10.3743 s |

One portable run took 20.8676 s; the other two took 11.9525 and 11.4671 s.
Load averages vary substantially. The median ratio is about 1.15, limited to
this short workload on this machine. There is no extrapolation to M1 hardware
or long chains. All six runs produce identical scientific samples.

These samples are still in the starting phase: after 10% burn-in, posterior
ESS is 3.01 and the minimum scalar ESS is 2.49. A quick completed run is not a
usable posterior. Long-run multistate inference agreement remains separate
from the fixed-state numerical checks delivered here.

```sh
target/release/rove sample /path/to/500_zilla.nex \
  --output <new-directory> --seed 42 --steps 20000 --sample-every 100 \
  --model reference --backend neon
# Repeat with --backend portable and distinct output directories.
```

## Correctness and application checks

- Twelve fixed states match BEAST 2.7.8 / MM 1.2.1, partition by partition and
  in total; maximum absolute log-likelihood difference `1.3307612789503764e-9`.
  The [fixture](../../../fixtures/validation/multistate/README.md) includes the
  exporter, reference values and provenance.
- Independent reference calculations and the sum of single-partition
  likelihoods agree during changing tree/rate states. The hot proposal loop
  allocates no memory after setup.
- Binary and multistate CLI runs preserve all samples after pause, process
  termination and restoration of one or four chains. Partition definitions
  and caches are part of the checkpoint.
- Native tests cover import, saved policy, legacy setup decoding, actual engine
  runs and direct-CLI equality. The matrix and review views display partition
  counts in English and Brazilian Portuguese.

The measurements record executable and source hashes, toolchain, raw timings,
load averages and scalar ESS. They establish numerical and restart correctness
for the tested cases, not broader biological convergence or BEAST superiority.
