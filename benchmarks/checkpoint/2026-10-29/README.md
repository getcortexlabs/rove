# Checkpoint overhead and restart validation

Rove now saves lossless sampler checkpoints, pauses independent chains together,
and resumes their output files. The default save interval is approximately 30
seconds, at a common sampled state. `--checkpoint-every` selects a fixed step
interval instead. Initial and final states are also saved.

## Throughput

The comparison uses the bundled engine from `5b90373` and the new release engine,
on an Apple M4 Max with 12 performance and 4 efficiency cores and 64 GB RAM.
Both builds use the repository release profile: fat LTO, one codegen unit and no
`target-cpu=native` override. Threads use the OS scheduler without core affinity.
The case is the repository's 20 × 50 `reference-mk` matrix, lognormal clock and
four gamma categories, with T1–T19 constrained. Each chain runs 2 million steps,
logging every 1,000, with seeds starting at 2026102900. Three repetitions of each
combination run in a shuffled order.

| Chains | Previous median wall time | With checkpoints | Additional time |
| --- | ---: | ---: | ---: |
| 1 | 3.216 s | 3.286 s | 2.2% |
| 4 | 3.429 s | 3.749 s | 9.3% |

These are short runs on a working Mac, not an isolated machine. They include
startup, initial/final durable saves and, for groups, the final comparison. They
finish before a timed periodic save becomes due. The first new four-chain run
took 4.879 s; it remains in the measurements. These figures do not establish the
cost on M1 hardware or on longer analyses.

The trace rows and sampled trees were identical across both engines and all
repetitions after excluding command and executable comments. This preserves the
samples and their diagnostics; it is not new evidence of biological convergence.
[measurements.json](measurements.json) records timings, load averages, source and
executable hashes, toolchain, matrix fingerprint and output fingerprints.

```sh
python3 tools/checkpoint/benchmark.py \
  --before /path/to/previous/rove \
  --after target/release/rove \
  --output .local/checkpoint-overhead
```

## Restart checks

- Sampler tests compare every proposal after restoration, including accepted and
  rejected moves, strict and relaxed clocks, prior-only sampling, joint moves,
  adaptive tuning, posterior components, tree state and live cache bytes.
- CLI tests pause and kill processes, append an uncommitted file tail, and resume
  one or four chains. Scientific samples, final parameters and move counts match
  uninterrupted runs.
- A saved diagnostic history produces the same automatic stopping decision.
- Changed input, executable, settings and damaged state files are refused.
- An active process cannot be resumed concurrently. A deliberately failed atomic
  manifest write leaves the preceding checkpoint usable.
- A four-chain run was killed after its default timed save became visible at
  31.2 seconds (state 16,743,000). Resume restored all four chains at that state
  and preserved the saved trace/tree prefixes.
- Native tests pause, reopen History and resume single and four-chain runs. They
  also recover an interrupted session and preserve its record after rejecting a
  changed matrix.

Checkpoints bind to the original executable, input, settings and directory.
Their purpose is exact restart within that environment; they are not a portable
exchange format between Rove versions. The engine retains the current committed
checkpoint. A failed write may leave an unreferenced temporary or state file,
which is never treated as a completed checkpoint.
