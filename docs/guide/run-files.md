# Your run folder

Open **What are these files?** beside the folder in Results or History for an offline guide. A typical single-chain run looks like this:

```text
analysis-<id>/
  setup.rove.json
  session.json
  events.jsonl
  engine.stderr.log
  results/
    run.json
    trace.log
    trees.nexus
    checkpoint.json
    checkpoint-<step>.bin
```

| File | Use |
| --- | --- |
| `setup.rove.json` | Requested settings, matrix path and fingerprint. Reopen to prepare a new run. |
| `session.json` | App session, status and execution details. |
| `events.jsonl` | Progress events, one JSON object per line. |
| `engine.stderr.log` | Engine messages for troubleshooting; this is not the numerical trace. |
| `dating.json` | Fossil-dating inputs, only when enabled. |
| `results/run.json` | Engine settings, build/input fingerprints and completion record. |
| `results/trace.log` | Sampled numerical values. Open in Rove Diagnostics or Tracer. |
| `results/trees.nexus` | Raw sampled trees, including the initial state. |
| `results/checkpoint.json` and `checkpoint-<step>.bin` | Resume index and binary state. Keep the referenced binary and associated run outputs. |

With multiple chains, each chain has its own files under `results/chain-01/`, `chain-02/`, etc. The group record stays at `results/run.json`.

## From samples to a tree

Try the bundled public 20 × 50 example. After a run, inspect **Diagnostics** and compare independent runs. Then choose **Summarize trees**, decide how much initial sampling to discard (burn-in), and open the resulting `summaries/summary-<id>/mcc.tree` in Rove or FigTree. This is an annotated representative tree. `samples.trees` contains raw samples; `summary.json` records the choices and support calculation. Neither summarization nor finishing the requested steps proves convergence.

`.nexus` and `.trees` can both name a NEXUS tree sample. Changing the extension does not convert it or calculate node support.

## Reports created later

Optional `diagnostics/`, `summaries/`, `comparisons/`, `predictions/` and `marginal/` folders appear after you run the corresponding tools. Diagnostics include `diagnostics.json`. A [predictive check](./posterior-prediction) includes `predictive.json`, `replicates.jsonl`, `characters.tsv` and simulated `matrices/`. Keep reports with their source run. Imported tree samples use `tree-source.json` to record their original path and fingerprint.

## Keep or share

Keep the whole folder, the unchanged original matrix and the same Rove build to resume. The app does **not** copy the input matrix or engine into the run folder. A setup or report JSON cannot substitute for the binary checkpoint. If the analysis used a selected subset, keep the exact matrix recorded in its setup.

For a tree figure, share `mcc.tree` and summary choices. For a sampling review, include the trace and raw trees. For reproducibility, include the full context and model settings. Check paths and sensitive data before sharing. Partial/interrupted runs may contain incomplete outputs; review their status before interpreting them.
