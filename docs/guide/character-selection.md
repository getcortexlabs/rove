---
description: "Choose Mk, Mkv or Mkparsinf to match how morphological characters were collected."
---

# Character selection

The correction must match which characters could enter the matrix. It is not a speed or quality setting.

| Characters retained during collection | CLI option | Mac app |
| --- | --- | --- |
| Constant and variable | `--conditioning none` | All characters · Mk |
| Variable only | `--conditioning variable` (default) | Variable only · Mkv |
| Parsimony-informative only | `--conditioning informative` | Informative only · Mkparsinf |

A complete character is parsimony-informative when at least two different states each occur in two or more taxa. `0011` qualifies; `0001` and `00123` do not. With several states, excluding just constants and single exceptional tips is insufficient.

Mkparsinf is available with both clock presets, state-count partitions, independent chains and checkpoints. Each partition uses uniform, unordered Mk. Rove divides its observed likelihood by the probability that a character is informative on the same tree and branch rates, after mixing rate categories. It uses cached subtree probabilities instead of enumerating excluded columns. This recurrence does not apply to ordered or unequal-frequency models.

Missing and uncertain cells are accepted only if every allowed completion is informative: `0011?` qualifies, `001??` does not. This conditions on the complete character across all taxa, not a separate event for each missing-data mask. `rove inspect` reports `informative_error`; the app blocks incompatible analyses and the CLI identifies the original character number. Rove never removes characters automatically. Extremely short trees whose retained probability cannot be resolved in double precision are rejected.

In the app, open **Model → Which characters did you include?** and choose **Informative only · Mkparsinf** only if it matches your collection process. The bundled 20 × 50 example includes uninformative characters; keep Mkv for that example.

For a small functionality check:

```sh
target/release/rove sample fixtures/validation/informative/matrix.nex \
  --conditioning informative --model reference \
  --output runs/informative --seed 42 --steps 10000 --sample-every 100
```

This short run checks the workflow, not convergence. [Validation](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/informative) includes complete pattern enumeration, missing-data checks, checkpoint restoration and fixed-state agreement with BEAST 2.7.8. It does not establish posterior agreement or an inference speedup.
