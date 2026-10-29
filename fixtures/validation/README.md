# Full-model validation inputs

The [protocol](PROTOCOL.md) defines the cases and criteria before the production
comparison. `manifest.json` records input hashes, matrix counts and generation
commands. Every case contains a matrix, its known construction topology and a
BEAST XML derived from the supplied reference. The generated degradation legends
record individual cell flips, the missing-cell count and removed characters.

`snapshot.json` is a fixed state from a 1,000-step BEAST smoke run, seed 17.
`snapshot-provenance.json` identifies the packages, exporter and command. These
short states test likelihoods and priors; they are not posterior estimates.
`tools/benchmarks/prepare.py` generates `tests/support/inference_states.rs`,
and `tests/inference.rs` checks each state with imported and independently
generated gamma categories before complete runs are compared.

Install the pinned packages as described in the
[reference report](../../benchmarks/beast-2.7.8/2026-10-25/README.md). From a
clean checkout, run each case in its own new output directory:

```sh
python3 tools/beast/inference.py --case reference-mkv \
  --packages /path/to/packages --launcher /path/to/launcher.jar \
  --output /path/to/new-reference-mkv-run
python3 tools/beast/compare_inference.py /path/to/new-reference-mkv-run
```

Repeat with `reference-mk`, `control`, `homoplasy`, `missing` and `reduced`.
The driver builds Rove with `target-cpu=native`, optimization level 3, fat LTO,
one codegen unit, no incremental compilation, debug assertions or overflow
checks. It records the exact build settings and executable hash. Java uses
256 MB initial and 2 GB maximum heap, an 8 MB stack, one likelihood thread and
the Java core. Timed chains run sequentially, alternating engine order. Summary
and fixed-state exports run afterward, outside those timings.

`metadata.json` records every command, input/output hash and timing;
`comparison.json` contains the criteria outcomes, Monte Carlo errors, convergence
diagnostics, clade comparisons and ESS per second. `summaries.json` retains the
per-chain summaries and clade batches. Raw traces, annotated trees, checkpoint
files and diagnostic logs stay in the chosen run directory.

To regenerate matrix/XML inputs in a new directory inside the checkout, use
`python3 tools/beast/prepare_inference.py --output .local/new-validation-inputs`.
The generator does not replace archived snapshots or the protocol. NEXUS comments
include the generation command, so their byte hashes change with the output path;
the observed sequences do not. `--smoke` on the run driver reproduces a short
fixed-state export rather than a production comparison.
