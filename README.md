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

Early. The likelihood core, morphological NEXUS reader, pattern compression
and numerical tests are implemented; standalone inference and integrations are
not. Work is tracked in the
[issues](https://github.com/getcortexlabs/rove/issues).

## Why

- Small state spaces and short morphological alignments are worth measuring
  as their own workload, including the cost of setup and partial updates.
- Mk has a closed form, `P(t) = e*I + (1-e)*pi`, which permits an O(k) child
  update per pattern instead of a full O(k^2) matrix multiplication. The
  current crate builds these matrices; the direct pruning path is planned.
- ARM-specific vectorisation and reuse of repeated calculations are candidate
  optimisations. Their value needs to be established with matched benchmarks.

The first [BEAST 2.7.8 reference experiment](benchmarks/beast-2.7.8/2026-10-25/README.md)
records the exact model, runtime, sampling diagnostics, and fixed numerical
states for a selected 20 x 50 dataset. It used BEAST's Java likelihood core.
No cross-engine speedup has been measured.

The [Rust baseline on Apple M4 Max](benchmarks/rove/2026-10-26/README.md)
measures 11 workloads, separating setup, full evaluations, partial updates and
proposal bookkeeping. It includes raw samples and the variation between runs.

## Design

- Pattern-major layout: one contiguous array per state per rate category, so
  the inner loop vectorises across site patterns, not states. Works for k=2
  as well as k=61.
- Model-aware fast paths, starting with Mk and Mkv (ascertainment correction).
- NEON first, AVX2 second. Scalar fallback always available and used as the
  reference in tests.
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
3. Rust kernel baseline recorded; implement and validate Mkv correction next.
4. Add direct Mk pruning and NEON, measuring each change.
5. Build standalone MCMC, then the relaxed-clock reference model.
6. Validate complete inference and evaluate subtree repeats.
7. Consider optional BEAST 2 JNI and BEAGLE C ABI adapters.

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
