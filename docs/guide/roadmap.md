---
description: "Where Rove stands: the likelihood core and its tests exist. The Mk fast path, NEON, benchmarks and standalone inference come next, in that order."
---

# Roadmap

Rove is early. There are no releases, no published crate and no dates. The order below is the order of the work, not a schedule.

| Step | State |
| --- | --- |
| Core kernel: Felsenstein pruning, rate categories, rescaling | Done |
| Reference implementation and numerical equality tests | Done |
| Mk closed-form fast path | Next |
| NEON vectorisation | Planned |
| Benchmarks on a 90 × 250 morphological matrix and on BEAGLE's `synthetictest` | Planned |
| Standalone inference engine | Planned |
| Optional BEAST 2 package (`LikelihoodCore` over JNI) and BEAGLE C ABI | Possible |

## Done: the core and its reference

`LikelihoodCore` computes tree likelihoods over a pattern-major layout, with rate categories, rescaling and double-buffered node state. `reference::Problem` is the plain implementation it is tested against. The [likelihood core](./kernel) page describes what it can do today.

## Next: the Mk fast path

`model::Mk` already produces the closed-form transition matrix. The kernel still multiplies it as a generic `k × k` matrix. The next step is a path that uses the closed form directly, followed by the Mkv ascertainment correction for matrices that contain only variable characters.

## Then: NEON, and numbers

The layout was chosen so the inner loop can be vectorised. NEON on Apple silicon comes first, AVX2 second, with the scalar code kept as the reference.

Benchmarks follow the vectorisation, not the other way round. Results will record the data, program versions, model settings, hardware, build settings and commands used, and will name the BEAST implementation they ran against. No speedup is claimed before it is measured.

## Standalone inference

The goal is an engine that runs an analysis on its own: tree proposals, MCMC, priors and clock models. Validation starts with likelihoods compared at fixed states against BEAST 2, then complete runs on the [synthetic control](./fixtures) and on real matrices. Sampling quality counts as much as runtime.

The intended product around that engine is a native macOS application. No application work has started.

## Possible: adapters

A BEAST 2 package exposing Rove as a `LikelihoodCore` over JNI, and a BEAGLE C ABI, are possible compatibility paths. They are not dependencies of the Rust engine and are not in progress.

## Follow along

Work is tracked in the [issues](https://github.com/getcortexlabs/rove/issues) of the repository.
