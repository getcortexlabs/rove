---
description: "Build and explore Rove's first native Mac workspace: import a matrix, inspect its characters and prepare an analysis."
---

# The Mac preview

Rove has a first native macOS workspace built with SwiftUI and AppKit. It
imports matrices, prepares settings and runs the bundled Rust engine with
live progress and cancellation. BEAST and Java are not required.

![The native Rove workspace](/images/macos-preview.png)

## Build and open

On macOS 14 or newer, with Xcode supporting Swift 6 and Rust installed, run
these commands from the repository root:

```sh
scripts/build-macos.sh
open build/macos/Rove.app
```

This builds a local application with an ad-hoc signature. It is not a
notarized release or an installer. The app has a dark appearance inspired by
Cortex Labs and supports English and Brazilian Portuguese.

## Language and settings

Open **Settings** from the sidebar or press **Command-comma**. Follow your Mac's
language or select English or Brazilian Portuguese. The change is immediate
and preserves your open matrix and analysis settings. Preferences are saved
on this Mac. You can also reduce animations and sidebar transparency; system
accessibility preferences always take priority. System dialogs follow the
Mac's language, and original engine diagnostics remain under **Technical details**.

## Explore a matrix

Open a NEXUS file with **Command-O**, drop one onto the welcome screen, or
choose **Try an example**. The example is the supplied 20-terminal,
50-character reference matrix.

The matrix view shows taxa, observed characters, compressed patterns and
missing cells. Click a cell to see its taxon, character and observation.
Statistics cover the full matrix; the scrollable preview shows up to 200 taxa
and 120 characters. The app accepts files up to 32 MB.

Import support and sampling support are separate. A readable multistate matrix
can be viewed, but the current sampler accepts only binary characters.
Unsupported inputs show an explanation. Gaps need an explicit decision to
treat them as missing; potentially constant characters cannot use Mkv.

## Prepare and run an analysis

Choose a strict clock or the [relaxed-clock reference preset](./reference-model),
Mk or Mkv, chain length, sampling interval and seed. Priors keep their engine
defaults. In Model, **Add group** selects terminals that must form a clade.
Groups must be nested or disjoint. Nothing is constrained automatically,
including the example's ingroup. Support imposed by a constraint is not
independent recovery from the data.

In **Outgroup**, optionally select one terminal such as T20. All terminals remain
in the analysis: choosing T20 requires T1–T19 to form a clade, with T20 on the
other side of the root split. The outgroup is a terminal, not the ancestral root.
This control supports one outgroup, and starts at **Not selected**.

**Additional groups** can be added or edited with the pencil button. Checking
terminals prepares a group; **Apply group** commits it and **Cancel** discards
pending edits. Unchecked terminals remain in the analysis. Conflicts between a
group and the outgroup block execution. Clearing the outgroup preserves any
manually applied groups, so check **Applied groups** in Review.

Review lists the root split, applied groups and every retained terminal. The same
information is available under **Applied analysis setup** during a run and in
History. Save/reopen and **Use this setup** preserve the choice; a new matrix or
**New analysis** starts without an outgroup. Older setups retain their original
groups without guessing which terminal was intended as outgroup. New format 2
setups require the updated app. Changing these settings prepares a new analysis;
it does not reroot an existing result. A constrained group's support is imposed.

![Explicit outgroup selection in the Mac workspace](/images/macos-outgroup.png)

Review the settings and groups, choose an output location, and press **Start
analysis** or **Command-Return**. The bundled Rust engine runs locally while
the window shows actual completed steps and elapsed time. Settings remain
available during a run; editing its scientific configuration is locked.
There is no estimated completion time or invented ESS or posterior support.

Each run gets a new folder under `~/Rove Runs` by default. **Open folder** shows
the saved setup, session record, progress events and engine log. The `results`
subfolder contains `trace.log`, `trees.nexus` and `run.json`. Once the run ends,
the Trace, Trees and Run record buttons reveal those files in Finder.
**Completed** means the requested steps finished and output metadata was checked.
It does not establish convergence. Compare independent runs and assess sampling
and tree diagnostics before interpreting results; see the [sampling guide](./sampler).

**Cancel analysis** asks before stopping. Partial files remain clearly marked;
they are not a completed analysis or a checkpoint. Closing the main window or
quitting while running asks whether to stop and waits for the engine to exit.
If a process must be forcibly stopped, its partial files may be incomplete.
Resuming a chain is not available yet.

## Save and reopen a setup

**Save setup** writes a `.rove.json` with the original matrix path, SHA-256,
settings and explicit groups. It does not copy the matrix or start a run.
**Open Setup** (**Command-Shift-O**) restores the configuration after checking
that the original matrix still exists and has the same fingerprint. Missing
or changed input is reported before execution. Review the restored settings
before starting a new run. A setup is not a checkpoint.

**Copy terminal command** prepares a command for the bundled Rust engine and
a new output folder under `~/Rove Runs`. It reads the matrix at execution time.
The same binary, input, settings, groups and seed produce the same scientific
samples through the app and CLI; the app's progress reporting does not change
sampling.

For automation, `rove sample ... --events-json` emits versioned JSON lines with
`started`, `progress`, `completed`, `cancelled` or `failed` events.
`--expect-input-sha256 HASH` rejects changed input before creating outputs;
`--cancel-on-stdin-close` enables cancellation when an owner closes its pipe.
The app uses these options. The [protocol and process notes](https://github.com/getcortexlabs/rove/blob/main/apps/macos/README.md#cli-event-protocol-version-1)
define fields, validation and exit codes.

## Browse past runs

![Rove History with completed, cancelled and failed runs](/images/macos-history.png)

Open **History** in the sidebar or press **Command-Shift-H**. Runs are listed
newest first, with the matrix, date, recorded outcome, elapsed time, model and
seed. Search by matrix name or seed, or filter by status. Select a run to see
its completed/requested steps, settings, explicit groups and original input
fingerprint. The file buttons reveal available outputs in Finder.

Existing app runs in `~/Rove Runs` are discovered automatically. Other output
locations selected in the app are remembered on this Mac. **Add run folder**
adds an existing app run saved elsewhere; choose the folder containing
`session.json` or `setup.rove.json`. **Refresh** checks the files again. The app
reads small metadata files, not the trace or tree contents, and keeps this work
away from the interface thread.

Missing folders, unreadable records and incomplete output are identified.
A run without a confirmed finish is marked **Interrupted**, even if some
output exists. Older records remain readable; unavailable measurements display
an em dash rather than an estimate. New attempts save a session before checking
input so an early cancellation or failed start remains visible; no scientific
output is produced when the input fingerprint fails.

**Use this setup** checks the original matrix and opens the saved configuration
for review. It does not start an analysis automatically or resume a chain.
While a run is active, you can browse History and return through **Current
analysis**; reusing a setup stays disabled until it finishes or is cancelled.
**Back to workspace** (**Command-Shift-R**) preserves the matrix and settings
already open. History does not delete output files or establish convergence.

## Inspect from the command line

```sh
target/release/rove inspect fixtures/reference/20x50/matrix.nex
```

The command emits versioned JSON with counts, a bounded cell preview,
the input hash and separate sampling/Mkv eligibility errors. Add
`--gaps missing` only when that interpretation is appropriate. Reading a
matrix does not write any analysis outputs.

## Summarize posterior trees

![Posterior tree summary in Rove](/images/macos-summary.png)

After a completed run, choose **Summarize trees** in the result or its History
entry. Choose the burn-in percentage and node heights, then **Create tree
summary**. Saved summaries reappear when this panel is reopened. Processing
runs outside the interface thread and can be cancelled.

Burn-in discards the first `floor(total samples × percentage / 100)` trees.
`STATE_0` counts as a sample. The initial 10% is an editable suggestion, not a
convergence assessment; choose it after inspecting the trace. The panel reports
discarded and retained counts. Partial, cancelled and interrupted runs cannot
be summarized through the app.

MCC (maximum clade credibility) selects a **sampled topology**, not a
majority-rule consensus. Rove scores each retained tree by the sum of log clade
frequencies over internal nodes. Clades are sorted before summation; the first
retained candidate wins exact ties. `posterior` is the fraction of retained
trees containing a clade. Groups imposed by recorded constraints carry
`constrained=true`; their support is not independent recovery from data.

Choose **Keep selected tree** to preserve the selected sample's heights, or
**Common ancestor mean** to use the mean height of each group's most recent
common ancestor across **every** retained tree. Branch lengths are differences
between parent and child heights in the source time units. Rates remain separate
multipliers; no calendar calibration or rate-scaled distances are inferred.
Height/rate means and observed ranges are conditional on the clade existing;
`CAheight_mean` and `CAheight_range` use all retained trees. Ranges are minima
and maxima, **not HPD or credible intervals**. Median heights and HPD intervals
are not implemented.

Each summary creates a new folder under the run's `summaries` directory:

- `mcc.tree`: one annotated, rooted NEXUS tree for FigTree. Select `posterior`
  in FigTree's Node Labels to display support.
- `samples.trees`: a byte-for-byte copy of all original samples, including
  burn-in, for tools expecting the `.trees` extension.
- `summary.json`: source and executable hashes, source run metadata, burn-in,
  retained count, chosen state, method, heights policy and Rove version.

The buttons reveal these files in Finder. Original `results/trees.nexus` and
other run outputs remain unchanged. Each summary needs disk space for another
copy of the raw tree file. A summary and high node support do not establish
convergence; compare independent runs and sampling diagnostics.

The standalone Rust command uses the same calculation, without Java or BEAST:

```sh
target/release/rove summarize /path/to/results/trees.nexus \
  --output /path/to/new-summary --burnin-percent 25 --heights ca
```

Use `--burnin-samples N` instead of a percentage, or `--heights keep`. The output
directory must not exist. Extensions do not change the input interpretation.
Without an adjacent `run.json`, completion is recorded as **unverified**;
a syntactically complete file alone cannot prove a run finished. With a run
record, the command verifies completion, sample count and every sampling interval.

This reader supports Rove's rooted binary, ultrametric trees with contemporaneous
tips, a translation table and increasing `STATE_` identifiers. Mixed taxa,
missing final `End;`, partial rate annotations and unsupported tree shapes are
rejected. Fossil tip dates, polytomies and arbitrary external NEXUS blocks are
not supported. Limits are 4,096 taxa, 512 nesting levels, 2 MiB per statement and
one million distinct clades or a 128 MiB estimate for clade keys, whichever comes
first. That estimate is not a hard process-memory cap. Four streaming passes
retain clade counts and target statistics, not all parsed trees.

[Validation and Apple silicon measurements](https://github.com/getcortexlabs/rove/blob/main/benchmarks/tree-summary/2026-10-29/README.md)
record comparisons with pinned TreeAnnotator and external NEXUS readers.
