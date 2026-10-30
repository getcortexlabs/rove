# Generated benchmark matrix

`morphology_90x250.nex` has 90 taxa and 250 binary characters simulated under
equal-frequency Mk. It uses a recursively balanced tree, branch distance 0.1,
one rate and SplitMix64 seed 20261026. All characters are retained, including
constants; no ascertainment filtering or posterior-support selection is used.

Regenerate with `python3 tools/benchmarks/prepare.py` from the repository root.
Use `--check` to verify that committed inputs match the generator and archived
BEAST reference state. See the [benchmark protocol](../../benchmarks/README.md)
for evaluation settings, controlled synthetic cases and interpretation.

## Distributed Mac benchmark

`mac-v1.json` is the fixed protocol for `rove benchmark`, also bundled as
`Resources/BenchmarkProtocol.json` in the Mac app. Keep those bytes identical;
changing the workload requires a new protocol ID and matching upload validation.
The command embeds the synthetic matrix directly, so the shipped executable needs
no external file, BEAST, Java or toolchain. It emits JSON lines: `started`,
`warming`, six `measurement` events, then `completed`. With
`--cancel-on-stdin-close`, owner exit or stdin closure produces `cancelled` and
exit code 130. Cancelled tests never emit `completed`.

The preset is `Settings::reference()` with `Conditioning::None` and the native
backend. Each observation constructs fresh samplers with seeds 20261030 + chain
index, warms each for 5,000 proposals, synchronizes workers, then measures 50,000
proposals per chain. Three repetitions for one chain precede three for four
chains. Timing excludes parsing, setup, warm-up and output writing, and includes
worker scheduling/join overhead. The output records the binary fingerprint,
protocol fingerprint, backend, optimization and experimental-repeat flags.

This is a fixed-work sampler measurement, not an inference/convergence benchmark.
The native UI stores individual observations and offers explicit sharing, as
explained in the [Mac benchmark guide](../../docs/guide/mac-benchmark.md).
