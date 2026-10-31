# Profiling at the convergence-study budgets

This supplement measures computational phase occupancy from the terminal
states of the [four-chain study](../convergence/PROTOCOL.md). Commit this text
and its tools before these measurements. The 90-taxon diagnostic outcome was
already available when this supplement was written; the 500-taxon runs were
still in progress. The original convergence criteria and caps do not change.

Select chains **1 and 2 by index** from each workload, irrespective of their
diagnostics: states 20M for 90 taxa and 5M for 500 taxa. Profile only copies
in a separate local directory. Neither their continuations nor their outputs
enter the convergence assessment, which remains capped at the original steps.

For each saved state, run the same short continuation in a normal and an
instrumented build of `examples/profile_checkpoint.rs`. Use 1M proposals on
90 taxa and 100,000 on 500 taxa. Normal runs first for chain 1; instrumented
runs first for chain 2. Process the 90-taxon pair before the 500-taxon pair.
The saved RNG and adaptive tuning state continue without reinitialization.

The driver requires unchanged engine sources, Cargo files and Rust compiler
relative to the frozen source experiment. It verifies the source executable,
input and state fingerprints. The harness additionally requires a bit-exact
checkpoint round trip before advancing and matching final full-state hashes
between normal and instrumented continuations. Verify the original checkpoint
hash after each pair. The normal CLI's executable-fingerprint restart rule
is unchanged; this separate benchmark deliberately compares the two builds.

This window measures `Sampler::step` and excludes setup, input parsing,
checkpoint decoding and final serialization. It also excludes trace/tree
logging and checkpoint output **inside** the loop. Report sampling time and
whole-process time separately; do not substitute these rates or fractions
for the full CLI measurements. The observer starts after state restoration,
so the original 100,000-step phase-sampling threshold has already been passed.
Require at least 2,000 observations per instrumented continuation and retain
both states. These are wall-clock phase observations with the same limits as
the [main profile](PROTOCOL.md), not native CPU stack samples.

Do not run this experiment, builds or tests concurrently with the original
budget study. Record commands, binaries, source-state and protocol hashes,
power/load and paired sampling-time ratios. Keep private checkpoints local;
publish aggregate counts and final-state hashes only. No inference draws are
added, no seeds are selected by performance and no convergence claim is made
from these short windows.

```sh
CARGO_HOME="$PWD/.local/cargo-summary" CARGO_NET_OFFLINE=true \
  python3 tools/benchmarks/profile_terminal_states.py \
  --study .local/sampler-study --output .local/sampler-terminal-profile
```
