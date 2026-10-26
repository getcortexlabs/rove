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

Early. The likelihood core and numerical tests are implemented; standalone
inference and integrations are not. Work is tracked in the
[issues](https://github.com/getcortexlabs/rove/issues).

## Why

- BEAST 2 never uses BEAGLE for the Mk model: `LewisMK` returns a null eigen
  decomposition, the BEAGLE path throws, and the Java core takes over. Every
  morphological analysis in BEAST 2 runs in pure Java today.
- BEAGLE's ARM support is SSE translated through `sse2neon`. The BEAGLE 4.1
  preprint reports about 1.3x over scalar code, dropping to 1.1x at 8 threads.
- The CPU kernels vectorise across states and are specialised for 4 states.
  Binary and ternary characters, the bulk of morphological matrices, take the
  generic path with padding.
- Mk has a closed form, `P(t) = e*I + (1-e)*pi`, so `P*L` costs O(k) per
  pattern instead of O(k^2). A kernel that knows the model can use it; a
  generic eigen-decomposition kernel cannot.

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

## Roadmap

1. Core kernel: Felsenstein pruning, gamma rate categories, rescaling.
2. Reference implementation and numerical equality tests.
3. Mk closed-form fast path.
4. NEON vectorisation.
5. Benchmarks on a 90 x 250 morphological matrix and on BEAGLE's `synthetictest`.
6. Standalone inference engine.
7. Optional BEAST 2 package (`LikelihoodCore` over JNI) and BEAGLE C ABI.

## Website

The documentation site at [rove.getcortexlabs.com](https://rove.getcortexlabs.com/) uses VitePress. It documents what the crate does today; planned work is on its roadmap page.

```sh
npm ci
npm run docs:dev
```

Build with `npm run docs:build`; preview the build with `npm run docs:preview`.

## License

MIT or Apache-2.0, at your option.
