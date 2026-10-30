# Expanded benchmarks on Apple M4 Max

This experiment measures the current Rove engine against BEAST 2.7.8 on three full-model workloads and against the BEAST Java and BEAGLE CPU kernels on twelve numerical workloads. It adds measurement tools; it does not change the sampler or its default proposals.

The [protocol](../PROTOCOL.md) was committed in `cc72ba5` before production timing. The machine is an Apple M4 Max (12 performance and 4 efficiency cores, 64 GB), running macOS 26.6.2. Runs were sequential, with one likelihood thread and no concurrent builds, tests or diagnostics. Neither process was pinned to a core. JVM service and compilation threads were left at their defaults.

Rove used Rust 1.99.1, LLVM 23.1.1, ordinary release settings (O3, fat LTO, one codegen unit, no target-cpu override or experimental features). BEAST used Temurin 17.0.15+6, BEAST.base/app 2.7.8, MM 1.2.1, ORC 1.2.1 and BEASTLabs 2.0.3. BEAGLE was built from tag v4.0.1 (`bf35ad4`), with Apple Clang 21.0.0, effective O3, JNI and CPU vector support, and no CUDA/OpenCL. Its version string reports `4.0.1 (PRE-RELEASE)`.

## Full inference

Both engines use uniform unordered Mk, four shared midpoint gamma categories, an uncorrelated lognormal relaxed clock, and the same birth–death and parameter priors. State partitions, ambiguity masks and missing-data treatment match. Only the small Mkv case constrains T1–T19 to be monophyletic. Rove keeps its default classical proposals; BEAST keeps the supplied ORC/Bactrian schedule. Thus these are matched probability models, not identical MCMC trajectories or work per proposal.

The 90 × 250 matrix is generated under binary Mk with constant columns retained. The 500 × 759 input was supplied privately by Thiago; its biological provenance and the suitability of these model choices are not established by this benchmark. No private alignment, taxon labels, XML, trees or clade maps are included.

| Case | States: characters / observed patterns | Requested steps per chain | Trace/tree interval |
| --- | --- | ---: | ---: |
| 20 × 50, Mkv | 2: 50 / 23 | 10,000,000 | 1,000 |
| 90 × 250, Mk | 2: 250 / 250 | 1,000,000 | 100 |
| 500 × 759, partitioned Mk | 2: 180 / 180; 3: 140 / 140; 4: 439 / 439 | 10,000 | 10 |

The Mkv kernel additionally evaluates two constant patterns with zero observed weight. Four fresh processes per engine use seeds 20261201–20261204, alternating engine order. Timings include process startup, input, JVM warm-up, adaptation, trace/tree output and checkpoints. Checkpoint and screen intervals are 100,000 steps. The file formats, checkpoint sizes and additional BEAST log columns differ. Final operator counts show N proposals in Rove and N+1 in BEAST for a requested chain length N; the extra BEAST proposal is at most 0.01% of the requested budget and is included in wall time.

| Case | Rove median seconds [min, max] | BEAST median seconds [min, max] | Median paired BEAST/Rove time |
| --- | ---: | ---: | ---: |
| 20 × 50, Mkv | 17.562 [17.302, 19.260] | 58.688 [57.868, 60.305] | 3.35× |
| 90 × 250, Mk | 14.519 [14.460, 14.635] | 57.529 [57.263, 58.341] | 3.96× |
| 500 × 759, partitioned Mk | 3.649 [3.507, 3.982] | 112.719 [106.288, 113.043] | 30.00× |

**These are fixed-step time ratios. They are not ratios of time to a converged posterior.** Per-seed timings, standard deviations, output fingerprints and final operator counts are in [inference.json](inference.json) and [metadata.json](metadata.json). The large case stops at 10,000 steps, around Rove’s adaptation delay, and is explicitly a bounded scaling run.

## Sampling quality

Both engines’ traces and trees were analyzed with the same BEAST diagnostic helper after discarding floor(10% of samples). Parameter diagnostics use rank-normalized split R-hat including folded ranks. The gates require R-hat ≤ 1.01, ESS ≥ 100 in every chain and ≥ 400 in total, plus agreement of means within four combined Monte Carlo standard errors. Between-chain uncertainty is included. ESS per second divides by the whole process time.

| Case | Rove maximum parameter R-hat | BEAST maximum parameter R-hat | All parameter gates | All sampling gates |
| --- | ---: | ---: | --- | --- |
| 20 × 50, Mkv | 1.006 | 1.018 | not met | not met |
| 90 × 250, Mk | 1.044 | 1.019 | not met | not met |
| 500 × 759, partitioned Mk | 2.500 | 19.845 | not met | not met |

None of these fixed budgets meets all sampling gates. This experiment therefore establishes no winner in time to a reliable answer. ESS/time below is descriptive, especially for the larger nonstationary runs; it cannot justify a biological conclusion or a speedup at equal posterior accuracy. The earlier [six-case validation](../../inference/2026-10-29/README.md) used its own convergence protocol and longer extension for Mkv.

### 20 × 50, Mkv

| Statistic | Rove median ESS/s | BEAST median ESS/s | Rove / BEAST | Rove / BEAST minimum chain ESS |
| --- | ---: | ---: | ---: | ---: |
| posterior | 23.331 | 1.242 | 18.78× | 242.5 / 35.0 |
| likelihood | 345.984 | 112.535 | 3.07× | 5367.9 / 3688.8 |
| prior | 22.529 | 1.226 | 18.37× | 235.9 / 34.6 |
| gamma_shape | 17.344 | 4.950 | 3.50× | 297.1 / 279.2 |
| clock_sigma | 24.979 | 60.154 | 0.42× | 326.4 / 3443.3 |
| tree_height | 78.905 | 5.040 | 15.66× | 1040.7 / 139.5 |

Parameter gates not met:

- rove: none.
- beast: posterior, prior, birth_death, relative_death_rate, tree_height, tree_length.

Clades compared: 17; largest absolute pooled-frequency difference: 0.0000555. 0 exceed 0.05, and 0 exceed the 0.0125 Monte Carlo error limit in either engine. The imposed ingroup, where present, is excluded.

| Engine | Distinct topologies per chain | RF R-hat | RF ESS per chain |
| --- | --- | ---: | --- |
| rove | 1, 2, 1, 2 | 1.000 | undefined, 9001.0, undefined, 9001.0 |
| beast | 1, 2, 1, 2 | 1.000 | undefined, 9001.0, undefined, 9001.0 |

### 90 × 250, Mk

| Statistic | Rove median ESS/s | BEAST median ESS/s | Rove / BEAST | Rove / BEAST minimum chain ESS |
| --- | ---: | ---: | ---: | ---: |
| posterior | 1.711 | 1.307 | 1.31× | 15.8 / 50.1 |
| likelihood | 4.237 | 2.741 | 1.55× | 45.2 / 96.6 |
| prior | 1.372 | 0.946 | 1.45× | 12.6 / 44.6 |
| gamma_shape | 5.836 | 1.796 | 3.25× | 69.4 / 96.7 |
| clock_sigma | 1.462 | 0.934 | 1.57× | 13.8 / 42.0 |
| tree_height | 6.151 | 2.731 | 2.25× | 57.6 / 92.3 |

Parameter gates not met:

- rove: posterior, likelihood, prior, birth_death, tree_height, tree_length, gamma_shape, clock_sigma, gamma_prior, clock_rate_prior, clock_sigma_prior, mean_branch_rate.
- beast: posterior, likelihood, prior, birth_death, tree_height, tree_length, gamma_shape, clock_sigma, gamma_prior, clock_rate_prior, clock_sigma_prior, mean_branch_rate.

Clades compared: 128; largest absolute pooled-frequency difference: 0.079. 2 exceed 0.05, and 10 exceed the 0.0125 Monte Carlo error limit in either engine. The imposed ingroup, where present, is excluded.

| Engine | Distinct topologies per chain | RF R-hat | RF ESS per chain |
| --- | --- | ---: | --- |
| rove | 2259, 2147, 2089, 2168 | 1.002 | 369.0, 332.1, 619.4, 580.5 |
| beast | 2293, 2222, 2182, 2257 | 1.004 | 434.4, 378.3, 546.3, 1200.0 |

### 500 × 759, partitioned Mk

| Statistic | Rove median ESS/s | BEAST median ESS/s | Rove / BEAST | Rove / BEAST minimum chain ESS |
| --- | ---: | ---: | ---: | ---: |
| posterior | 0.762 | 0.024 | 31.09× | 2.7 / 2.7 |
| likelihood | 0.762 | 0.024 | 31.45× | 2.7 / 2.7 |
| prior | 8.207 | 0.023 | 363.55× | 9.7 / 2.4 |
| gamma_shape | 1.554 | 0.045 | 34.68× | 2.6 / 2.5 |
| clock_sigma | 1.331 | 0.027 | 50.14× | 3.6 / 2.9 |
| tree_height | 0.730 | 0.039 | 18.58× | 2.6 / 2.8 |

Parameter gates not met:

- rove: posterior, likelihood, prior, birth_death, birth_difference_rate, relative_death_rate, tree_height, tree_length, gamma_shape, clock_sigma, gamma_prior, clock_rate_prior, clock_sigma_prior, mean_branch_rate.
- beast: posterior, likelihood, prior, birth_death, birth_difference_rate, relative_death_rate, tree_height, tree_length, gamma_shape, clock_sigma, gamma_prior, clock_rate_prior, clock_sigma_prior, mean_branch_rate.

Clades compared: 17494; largest absolute pooled-frequency difference: 0.411. 5768 exceed 0.05, and 15612 exceed the 0.0125 Monte Carlo error limit in either engine. The imposed ingroup, where present, is excluded.

| Engine | Distinct topologies per chain | RF R-hat | RF ESS per chain |
| --- | --- | ---: | --- |
| rove | 564, 561, 532, 557 | 6.287 | undefined, undefined, 44.7, undefined |
| beast | 668, 622, 659, 669 | 1.189 | undefined, 9.4, 4.5, undefined |

RF is measured against the same untimed pilot tree in each case, chosen before production. That reference is not a true tree. Constant tree indicators provide no mixing diagnosis. Clade uncertainty uses both twenty within-chain batches and variation across chains; observed frequencies are not certified posterior supports when convergence fails.

## Kernels

Each implementation receives identical tip partials, branch distances, category mixtures and pattern weights exported by the Rust harness. Before timing, initial and two changed states must agree within 1e-8 log units. Timings cover branch updates, pruning, category integration, scaling and pattern-log reduction as appropriate. No MCMC moves or priors enter these measurements.

Rove uses the direct Mk path and NEON in the table; raw results also include its generic matrix path and portable backend. BEAST uses BeerLikelihoodCore. BEAGLE uses its JNI CPU implementation, double precision, synchronous execution and no worker threads. The actual flags include `VECTOR_SSE`: on ARM this release translates those operations through `sse2neon.h`. The bundled JNI wrapper exposes the flags but not the native implementation name. JNI calls and transfers are included; this is not a native-C-API or GPU benchmark.

The scaling implementations differ: Rove caches power-of-two subtree exponents, Beer uses threshold-based scaling, and the BEAGLE adapter writes changed-node scalers and accumulates all internal scalers. Both adapters generate Mk transition matrices analytically. The comparison is of equivalent scaled probabilities through these adapters, not identical instructions or a language-only experiment.

| Workload | Full cached Rove µs | BEAST Java µs | BEAGLE JNI µs | Java/Rove | BEAGLE/Rove |
| --- | ---: | ---: | ---: | ---: | ---: |
| reference_20x50 | 2.284 | 14.439 | 7.669 | 6.32× | 3.36× |
| reference_20x50_mkv | 3.040 | 14.422 | 8.228 | 4.74× | 2.71× |
| morphology_90x250 | 109.042 | 622.017 | 427.095 | 5.70× | 3.92× |
| binary_one_rate | 40.249 | 195.042 | 170.359 | 4.85× | 4.23× |
| binary_four_rates | 108.359 | 651.747 | 434.902 | 6.01× | 4.01× |
| ternary_four_rates | 148.729 | 971.714 | 587.165 | 6.53× | 3.95× |
| eight_states | 411.052 | 4339.042 | 2035.047 | 10.56× | 4.95× |
| missing_20_percent | 109.540 | 605.736 | 396.763 | 5.53× | 3.62× |
| repeated_columns | 14.371 | 62.418 | 41.800 | 4.34× | 2.91× |
| ladder | 107.139 | 621.629 | 377.970 | 5.80× | 3.53× |
| small_unpadded | 0.393 | 2.085 | 1.795 | 5.30× | 4.56× |
| small_padded | 0.537 | 2.222 | 1.875 | 4.13× | 3.49× |

| Workload | Tip update Rove µs | BEAST Java µs | BEAGLE JNI µs |
| --- | ---: | ---: | ---: |
| reference_20x50 | 0.797 | 4.940 | 3.212 |
| reference_20x50_mkv | 1.040 | 5.289 | 3.482 |
| morphology_90x250 | 9.374 | 78.608 | 43.158 |
| binary_one_rate | 3.830 | 44.302 | 17.066 |
| binary_four_rates | 9.392 | 78.613 | 37.036 |
| ternary_four_rates | 12.615 | 115.257 | 52.048 |
| eight_states | 34.256 | 370.257 | 154.229 |
| missing_20_percent | 9.306 | 78.722 | 36.629 |
| repeated_columns | 1.158 | 7.905 | 4.462 |
| ladder | 107.417 | 622.826 | 378.874 |
| small_unpadded | 0.342 | 1.704 | 1.683 |
| small_padded | 0.452 | 1.797 | 1.760 |

Values are medians of three process medians. Each metric warms for 100 ms in Rust and 500 ms in Java/JNI, then uses nine calibrated batches of at least 10 ms. Implementation order reverses in the middle repetition. This finite warm-up does not guarantee that JIT compilation has ceased. [kernel-summary.json](kernel-summary.json) contains all four numerical operations, process-median ranges, batch p10/p90, and Rove parse/compression/setup, accepted/rejected proposals and allocation counts. [kernel/](kernel/) retains every timing batch and the shared numerical inputs in `input.json.gz` (gzip, preserving the exact exported JSON bytes). Adapter setup observations are single cold constructions, not a comparable warmed setup benchmark. Java/native allocations were not profiled.

## Numerical checks and compatibility

Pilot and final BEAST states were checked on both Rove backends. Maximum frozen-category per-partition log-likelihood difference: 2.62e-10. Maximum total log-likelihood difference with independently computed gamma categories: 9.47e-06. All declared tolerances passed; [fixed-states.json](fixed-states.json) records the components without exposing private trees.

The forced full-analysis path through BeagleTreeLikelihood failed during initialization because the pinned MM LewisMK implementation returns no eigendecomposition. BEAST exited with status zero despite that error, so the runner validates complete trace states instead of trusting the exit code. [beagle-compatibility.json](beagle-compatibility.json) records the failed configuration. Supplying transition matrices directly permits the kernel comparison. This failure does not establish that every BEAST implementation of the same Mk model is incompatible with BEAGLE; alternative model implementations were not benchmarked.

The diagnostic helper was corrected after timing to decode quoted Newick names and NEXUS space/underscore aliases consistently. It rejects ambiguous aliases. The RF reference trees and all 120 recorded output files remained unchanged ([integrity check](integrity.json)); the corrected helper hash is recorded in the metadata. This affects post-processing, not measured runtime.

## Reproduce and interpret

Use the [protocol commands](../PROTOCOL.md#commands). [metadata.json](metadata.json) records commands, build settings, toolchains, package/binary/input hashes, hardware and power state. `$REPO`, `$RUN` and `$HOME` replace local paths in the public record. [beagle-build.json](beagle-build.json) records the upstream tag, effective flags and local build. The private 500-taxon case requires the matching input fingerprint; the public cases and kernel inputs are available in this repository.

The current driver can reproduce the public cases without the private input: omit `--zilla` during preparation. Use `--case reference` or `--case morphology90` to select one. The measured experiment supplied `--zilla` and ran all three cases. Use a new report directory rather than the already archived path in the original protocol, for example:

```sh
python3 tools/benchmarks/report_expanded.py .local/expanded --output .local/expanded-report
```

This is one M4 Max, one BEAST 2 configuration, and one BEAGLE CPU adapter. It says nothing measured about BEAST X, GPUs, M1 Air or M1 Pro. The full-chain ratios include different proposal schedules, initialization and output costs. Kernel ratios do not predict posterior ESS per second. Larger converged comparisons and measurements on the M1 machines remain separate experiments.

## Validation

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, ten benchmark-helper tests, twenty-three reference-helper tests, the Java diagnostic self-test and `npm run docs:build` passed. Every numerical adapter check and all pilot/final-state checks passed. The standalone engine has no new runtime dependency.
