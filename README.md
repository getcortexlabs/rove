<p align="center">
  <img src="docs/public/rove-logo.png" alt="Rove logo" width="180" height="180" />
</p>

<h1 align="center">Rove</h1>

<p align="center">Explore evolutionary paths.</p>

<p align="center"><a href="https://rove.getcortexlabs.com/">Website</a> · <a href="https://github.com/getcortexlabs/rove">GitHub</a></p>

Rove is an ARM-first phylogenetic inference project written in Rust, starting
with a likelihood kernel for morphological data. The name comes from exploring,
wandering, and discovering paths.

The goal is standalone inference. The current crate computes tree likelihoods
without depending on BEAST. A complete inference engine is still future work;
BEAST is a reference for validation and benchmarking. Optional BEAST 2 and BEAGLE
integration remains a possible compatibility path.

## Status

Early. The likelihood core with its direct Mk path and a NEON kernel for it,
the Mkv correction, the morphological NEXUS reader, pattern compression,
numerical tests and a minimal standalone sampler are implemented. The relaxed-clock model of the reference
analysis and the integrations are not. Work is tracked in the
[issues](https://github.com/getcortexlabs/rove/issues).

## Why

- Small state spaces and short morphological alignments are worth measuring
  as their own workload, including the cost of setup and partial updates.
- Mk has a closed form, `P(t) = e*I + (1-e)*pi`, which permits an O(k) child
  update per pattern instead of a full O(k^2) matrix multiplication. Rove's
  Mk core prunes with it directly.
- ARM-specific vectorisation and reuse of repeated calculations are candidate
  optimisations. Their value needs to be established with matched benchmarks.

The first [BEAST 2.7.8 reference experiment](benchmarks/beast-2.7.8/2026-10-25/README.md)
records the exact model, runtime, sampling diagnostics, and fixed numerical
states for a selected 20 x 50 dataset. It used BEAST's Java likelihood core.
No controlled cross-engine benchmark has been run yet.

The [Rust baseline on Apple M4 Max](benchmarks/rove/2026-10-26/README.md)
measures 11 workloads, separating setup, full evaluations, partial updates and
proposal bookkeeping. It includes raw samples and the variation between runs.
The [direct Mk comparison](benchmarks/rove/2026-10-28/README.md) measures both
pruning paths in the same processes: the matrix path took 1.25 times as long as
the closed form for a full evaluation of the reference dataset, and 2.66 times
as long with eight states.

With the [Mkv correction](docs/guide/kernel.md#conditioning-on-variable-characters),
Rove reproduces the likelihood of both archived BEAST states for the reference
dataset, pattern by pattern.

## Design

- Pattern-major layout: one contiguous array per state per rate category, so
  the inner loop vectorises across site patterns, not states. Works for k=2
  as well as k=61.
- Model-aware fast paths. Mk prunes through its closed form; other models can
  pass full transition matrices.
- NEON first, AVX2 second. On aarch64 a NEON kernel runs the Mk node update;
  the portable code runs everywhere else, is its reference and gives the same
  results. There is no AVX2 path yet.
- One crate today. Optional BEAGLE C ABI and BEAST 2 JNI adapters are planned;
  neither is required by the Rust core.

## Non-goals

Competing with BEAGLE's CUDA backend on large nucleotide alignments. Use the
GPU for that.

## Fixtures

`fixtures/synthetic/control_20x50.nex` is a positive control: 20 taxa, 50 binary
characters, no homoplasy, no missing data, known tree. `tools/synth_matrix.py`
regenerates it and produces degraded variants. See `fixtures/synthetic/README.md`.

The separately supplied [reference dataset](fixtures/reference/20x50/README.md)
includes its BEAST 2 XML and provenance. Do not substitute one fixture for the
other when reproducing a result.

## Roadmap

1. Establish the scalar core, tests, CI and BEAST reference. Done.
2. Read morphological NEXUS and compress observed patterns. Done.
3. Record the Rust kernel baseline and add the Mkv correction. Done.
4. Add direct Mk pruning, measuring the change. Done.
5. Build a minimal standalone MCMC. Done.
6. Add NEON where profiles of complete runs show kernel time.
7. Add the relaxed-clock reference model, validate complete inference and
   evaluate subtree repeats.
8. Consider optional BEAST 2 JNI and BEAGLE C ABI adapters.

## Sampling trees

```sh
cargo run --release --bin rove -- sample fixtures/reference/20x50/matrix.nex \
  --output runs/seed-1 --seed 1 --steps 10000000 --sample-every 1000 \
  --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
```

The first sampler supports one partition of binary characters under Mk or Mkv,
a strict clock with rate 1 and the birth–death tree prior of the reference
analysis, with monophyly constraints. The [sampling guide](docs/guide/sampler.md)
describes the model, the output files and the checks. With the same moves and
fixed steps as BEAST 2.7.8, its posterior means and clade frequencies on the
reference matrix met criteria fixed before the runs
([report](benchmarks/strict-clock/2026-10-28/README.md)).

## Reading a matrix

```sh
cargo run --release --example read_nexus -- fixtures/reference/20x50/matrix.nex
```

The reference matrix produces 23 observed patterns with a total weight of 50.
The [NEXUS guide](docs/guide/nexus.md) defines the supported syntax, gap policy,
uncertainty and polymorphism handling, and partition-aware compression.

## Website

The documentation site at [rove.getcortexlabs.com](https://rove.getcortexlabs.com/) uses VitePress, in English, Portuguese, Spanish and Chinese. It documents what the crate does today; planned work is on its roadmap page.

```sh
npm ci
npm run docs:dev
```

Build with `npm run docs:build`; preview the build with `npm run docs:preview`.

## License

MIT or Apache-2.0, at your option.
