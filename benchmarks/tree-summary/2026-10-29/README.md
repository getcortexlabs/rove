# Posterior tree summary validation

Rove's MCC topology, posterior supports, retained-tree heights and common
ancestor mean heights matched the pinned TreeAnnotator in six comparisons:
a four-taxon example, an exact topology tie, and a 20-taxon relaxed-clock run,
each with `keep` and `ca`. Posterior differences were zero. The largest height
difference was 4.45e-16 on the toy input and 6.29e-18 on the reference input;
the largest branch-rate mean difference was 2.78e-14. These are checks of
post-processing calculations, not convergence or inference-efficiency results.

## Inputs and external readers

The [four-taxon fixture](../../../fixtures/summary/four-taxa.trees) has four
samples. With 25% burn-in, the remaining topologies are AB|CD, AB|CD and AC|BD.
The chosen topology is AB|CD, support for AB and CD is 2/3, log credibility is
`2 × ln(2/3)`, conditional AB height is 1.5 and its common ancestor mean is
8/3. Mean root height is 4. Keeping heights retains the first selected sample's
AB height of 1 and root height of 3. With 0% burn-in the two topologies tie;
both implementations choose the first sample, AC|BD.

The reference input was generated from the supplied
[20 × 50 matrix](../../../fixtures/reference/20x50/matrix.nex):

```sh
cargo build --locked --release
target/release/rove sample fixtures/reference/20x50/matrix.nex \
  --output .local/summary-validation/chain --seed 20261029 \
  --steps 1000000 --sample-every 100 --model reference --conditioning variable \
  --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
```

This uses the engine's reference preset: binary Mkv, four gamma categories,
lognormal branch rates and the birth–death prior with default parameters.
T1–T19 are explicitly constrained. The 10,001 trees include state zero;
25% discards 2,500 and retains 7,501. We make no convergence claim for this
short chain. Input and executable hashes are in `validation.json`.

Validation used the BEAST.app and BEAST.base **2.7.8 packages** from the pinned
reference environment. The TreeAnnotator banner still says 2.7.7; the package
versions and jar hashes identify the tested implementation. Java was the
bundled Zulu 17.0.5 runtime with JavaFX. FigTree **1.4.4** came from its
[official release](https://github.com/rambaut/figtree/releases/tag/v1.4.4).

Every exported tree was imported through **FigTree's bundled JEBL
NexusImporter**, preserving rooting, terminal names, topology, height-derived
branch lengths and numerical posterior/rate annotations. A second import
through **BEAST.base's NexusParser** reproduced the same node values within
floating-point tolerance. The reference CA tree also rendered successfully through FigTree's own SVG and PDF
export commands (`figtree-reference.svg` and `.pdf`). These were headless import/render
checks, not automated interaction with the FigTree GUI. The archived `.tree` files can be
opened directly in FigTree; select `posterior` under Node Labels. The PNG
export command produced a blank image under this Java runtime, so only the
working vector exports are archived.

```sh
java -Djava.awt.headless=true -jar "$figtree_jar" -graphic SVG \
  -width 1000 -height 700 \
  benchmarks/tree-summary/2026-10-29/reference-ca-rove.tree \
  .local/summary-validation/figtree-reference.svg
```

`*-rove.tree`, `*-beast.tree` and the importer TSV files retain the small
annotated outputs. `validation.json` records per-field maximum differences.
Acceptance was matching node/clade sets and `rel_tol=1e-9, abs_tol=1e-12` for
heights, posterior probabilities and mean rates. Rove reports observed ranges;
TreeAnnotator also writes HPD intervals and medians, which Rove does not claim
to reproduce. Branch units remain source time units with separate rate
multipliers.

To reproduce, supply the pinned jars and their package directory:

```sh
python3 tools/tree-summary/validate.py \
  --java '/Applications/BEAST 2.7.7/jre/zulu-17.jre/Contents/Home/bin/java' \
  --classpath "$beast_classpath" --packages "$beast_packages" \
  --figtree "$figtree_jar" --input .local/summary-validation/chain/trees.nexus \
  --output .local/summary-validation/comparison-new
```

`beast_classpath` includes the pinned `launcher.jar`, `BEAST.base.jar` and
`BEAST.app.jar` (plus the reference environment's BEASTLabs, MM and ORC jars).
`beast_packages` is the package directory containing their `version.xml`
files. These validation dependencies are not bundled with or called by Rove.

## Apple silicon measurements

Apple M4 Max (16 CPU cores: 12 performance, 4 efficiency), 64 GB RAM,
macOS 26.6.2, native ARM64, Rust 1.99.1. Release defaults, no `RUSTFLAGS`.
Three separate processes per input, 25% burn-in and CA heights. Wall time
includes snapshot copying, SHA-256, four parsing passes and output writing.
Peak RSS is the per-child `ru_maxrss` returned by `wait4`, in bytes on macOS;
it is not macOS physical footprint. `time -l` could not collect its metrics
under the execution sandbox, so the script records the supported resource
usage call directly. Page cache was not flushed.

| Input | Bytes | Trees | Retained | Distinct retained clades | Median seconds | Peak RSS range |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Original reference run | 19,540,196 | 10,001 | 7,501 | 20 | 0.544 | 2.89–3.03 MiB |
| Ten repetitions | 195,493,833 | 100,010 | 75,008 | 102 | 5.368 | 2.81–3.03 MiB |

The larger input repeats the original samples ten times with increasing state
identifiers. It exercises streaming and disk costs, **not** independent MCMC
sampling. Burn-in on the repeated file retains some early topologies from
later repetitions, hence more distinct clades. Neither case stresses the
maximum distinct-clade budget. Memory can grow with taxon count and distinct
clades; these measurements do not establish a universal memory bound or a
speedup over TreeAnnotator. Raw trials, hashes and hardware are in
`measurements.json`.

```sh
python3 tools/tree-summary/measure.py \
  --input .local/summary-validation/chain/trees.nexus \
  --hardware benchmarks/tree-summary/2026-10-29/hardware.json \
  --output .local/summary-validation/measurement-new
```

Replace the hardware JSON with the current host's chip, model, core count and
RAM when measuring elsewhere. Each trial preserves its summary and full raw
copy, so allow several times the input size in free disk space.
