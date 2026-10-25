# happypath

happypath is a phylogenetic likelihood kernel written in Rust. It computes tree
likelihoods for Bayesian and maximum-likelihood inference, the loop that eats
85% of a BEAST run.

It is built for the cases BEAGLE serves worst: ARM CPUs, small state spaces
such as morphological data, and small alignments. It ships as a BEAST 2 package
and as a drop-in implementation of the BEAGLE C API, so existing analyses run
unchanged.

## Status

Early. Nothing to run yet. Work is tracked in the
[issues](https://github.com/rafaelaugustos/happypath/issues).

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
- One crate. The BEAGLE ABI is a feature flag that builds a `cdylib`. The BEAST
  2 package lives in `beast/` and calls the same crate over JNI.

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
6. BEAST 2 package (`LikelihoodCore` over JNI).
7. BEAGLE C ABI drop-in.

## License

MIT or Apache-2.0, at your option.
