---
description: "Build and explore Rove's first native Mac workspace: import a matrix, inspect its characters and prepare an analysis."
---

# The Mac preview

Rove has a first native macOS workspace built with SwiftUI and AppKit. It
imports matrices, prepares settings and runs the bundled Rust engine with
live progress and cancellation. BEAST and Java are not required.

![The native Rove workspace](/images/macos-preview.png)

## Beta access and your first visit

The native beta opens with **I have an invitation** and **Join the beta list**.
Use an approved email to request an eight-digit code; it expires in ten minutes.
Joining the list also verifies your email, but only records your interest. It does
not grant access. Invitation approval is managed by the project team.

After access is confirmed, a letter from Rafael thanks Thiago J. Silva before
the introduction. Choosing **Continue** marks it as read on this Mac; signing out
or updating will not show it again. It follows the app language. Existing users
see the letter once without repeating their completed tour.

A four-part introduction then covers the workspace, matrices,
outgroups and results. Skip it, explore the example, or choose your own matrix;
no analysis starts automatically. Reopen it with **Help → Take the Rove tour**.
**Help → What's new in Rove** shows versioned highlights and previous notes.
New notes appear once per documented update, when the workspace is idle. The
first tour does not immediately stack another announcement. Reopened tours and
release notes preserve an existing matrix and its settings.

Access is stored in macOS Keychain. Rove checks it online at launch and every six
hours while active. After a successful check, access works offline for up to seven
days. An expired or revoked invitation blocks new analyses and checkpoint resumes;
a run already in progress can finish. Sign out through the Help menu.

Cloudflare handles beta access and Resend delivers verification codes. The service
stores your email, invitation status, confirmed interest, interface language and
access timestamps. Matrices, trees and analysis files stay on your Mac. The waiting
list is not a newsletter subscription. Contact [Cortex Labs](https://getcortexlabs.com/)
to request removal from the list. The standalone Rust CLI does not require beta
access or these network services.


## Build and open

Each [release](https://github.com/getcortexlabs/rove/releases) attaches `Rove-<version>.dmg` for Apple silicon, signed with a Developer ID certificate and notarized by Apple: open the image, drag Rove to Applications and launch it. The app targets macOS 14 or newer.

To build it yourself, install Xcode 26.3 or newer and Rust on a Mac supported by that Xcode version, then run
these commands from the repository root:

```sh
scripts/build-macos.sh
open build/macos/Rove.app
```

This builds a local application with an ad-hoc signature. `scripts/package-macos.sh` builds the release configuration and wraps it in a disk image; with `CODESIGN_ID` and `NOTARY_PROFILE` set, it signs and notarizes the app and the image the way releases are built. The app has a dark appearance inspired by
Cortex Labs and supports English and Brazilian Portuguese.

## Language and settings

Open **Settings** from the sidebar or press **Command-comma**. Follow your Mac's
language or select English or Brazilian Portuguese. The change is immediate
and preserves your open matrix and analysis settings. Preferences are saved
on this Mac. You can also reduce animations and sidebar transparency; system
accessibility preferences always take priority. System dialogs follow the
Mac's language, and original engine diagnostics remain under **Technical details**.

**Rove → About Rove** opens the project story, app version and credits for
Rafael Augusto (software and design) and Thiago J. Silva (biology and research).
The window follows the app language and includes links to the website, field
guide and source repository.

## Explore a matrix

Open a NEXUS file with **Command-O**, drop one onto the welcome screen, or
choose **Try an example**. The example is the supplied 20-terminal,
50-character reference matrix.

The matrix view shows taxa, observed characters, compressed patterns and
missing cells. Click a cell to see its taxon, character and observation.
Statistics cover the full matrix; the scrollable preview shows up to 200 taxa
and 120 characters. The app accepts files up to 32 MB.

Binary and multistate matrices can be sampled. **State partitions** groups
characters by observed/possible states, or explicitly uses the declared alphabet.
The matrix and review views show the partition counts.
Unsupported inputs show an explanation. Gaps need an explicit decision to
treat them as missing; potentially constant characters cannot use Mkv.

## Prepare and run an analysis

Choose a strict clock or the [relaxed-clock reference preset](./reference-model),
Mk, Mkv or [Mkparsinf](./character-selection), chain length, sampling interval and seed. Priors keep their engine
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

**Pause analysis** saves at the next sampled step and stops the engine. For a group, all chains pause together. In **History**, select the run and choose **Resume analysis**; the card shows the saved step. The same matrix, settings and Rove executable are required. Keep the original run folder.

**Cancel analysis** asks before stopping and retains partial output. Closing the app also stops its engine. If interruption occurs, History offers recovery when a checkpoint exists; steps after the last save are repeated. Old runs without checkpoints cannot be resumed. A saved setup remains a configuration for a new run, separate from resuming an existing one.

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

**Compare with other runs** appears on a completed run. It lists the other
completed runs in History with the same matrix, model, constraints and sampling
interval and seeds that do not overlap, including groups. Choose runs and a
burn-in, and the app runs `rove compare` on their chain folders. The report is
the one shown for a chain group, with one column per chain, and is saved in the
run's `comparisons` folder for later. Agreement between runs is not convergence
of either.

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
discarded and retained counts. The burn-in can also be given as a number of
samples, as with `--burnin-samples`. Partial, cancelled and interrupted runs
cannot be summarized through the app.

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

**Trees from another program.** **File → Summarize Tree File…** opens a NEXUS tree sample written by Rove, BEAST 2 or another program, asks for the burn-in and node heights, and saves the summary in a folder named after the file under the runs folder. The folder's `tree-source.json` records the file's path and SHA-256, and History lists it as a tree file with its summaries and the explorer. No outgroup is known, since Rove did not run the sample here, and the summary records completion as unverified unless the file sits beside a Rove run record. The welcome screen and the sidebar offer the same entry, and a tree sample chosen or dropped in the matrix import opens the summary sheet directly.

The standalone Rust command uses the same calculation, without Java or BEAST:

```sh
target/release/rove summarize /path/to/results/trees.nexus \
  --output /path/to/new-summary --burnin-percent 25 --heights ca
```

Use `--burnin-samples N` instead of a percentage, or `--heights keep`. The output
directory must not exist. Tree files written by BEAST 2, which carry a
zero-length root stem and a rate on the root, are accepted; the root rate is
dropped. Extensions do not change the input interpretation.
Without an adjacent `run.json`, completion is recorded as **unverified**;
a syntactically complete file alone cannot prove a run finished. With a run
record, the command verifies completion, sample count and every sampling interval.

This reader supports Rove's rooted binary time trees, with zero-age tips or explicit fossil
height annotations, a translation table and increasing `STATE_` identifiers. Mixed taxa,
missing final `End;`, partial rate annotations and unsupported tree shapes are
rejected. See [fossil tip dating](./tip-dating) for dated outputs. Polytomies and arbitrary external NEXUS blocks are
not supported. Limits are 4,096 taxa, 512 nesting levels, 2 MiB per statement and
one million distinct clades or a 128 MiB estimate for clade keys, whichever comes
first. That estimate is not a hard process-memory cap. Four streaming passes
retain clade counts and target statistics, not all parsed trees.

[Validation and Apple silicon measurements](https://github.com/getcortexlabs/rove/blob/main/benchmarks/tree-summary/2026-10-29/README.md)
record comparisons with pinned TreeAnnotator and external NEXUS readers.

## Explore an annotated tree

![Native MCC tree explorer](/images/macos-tree.png)

In a completed result or History entry, open **Summarize trees**, then choose
**View tree** on a saved summary. A separate native window opens the MCC result;
it does not browse raw samples or treat `STATE_0` as a final result. The header
identifies the source run and retained/discarded counts. **About this summary**
shows the burn-in, height policy, selected source sample and imposed groups.

Click a node or tip to inspect support, branch length, heights and available
rate ranges. Expand **Descendant taxa** to navigate its children or select a tip.
Missing annotations display **Unavailable**. Diamonds mark constrained groups;
their support is not independent evidence. A verified outgroup is identified
as a tip, not the root node. Vertical tip order does not determine rooting.

Scroll or drag to pan, pinch or use the zoom buttons to magnify, and choose
**Fit width** to restore 100%. Taller trees remain scrollable so labels stay
readable. Search finds taxa inside collapsed clades and expands their ancestors
when selected. **Rotate**, **Collapse clade** and **Reset display** affect only
the drawing. With the canvas focused, use Up/Down for visible nodes, Left for
the parent, Right for a child and Space to collapse/expand. The inspector also
provides native navigation buttons. Pointing at a node or tip highlights its
path to the root without changing the selection. These operations do not animate.

**Support** toggles labels; short branches and crowded rows omit labels to avoid
overlap, while the inspector retains every available value. Long tip labels
are abbreviated in the drawing and shown in full in search and the inspector.
Scale and branches use source time units. Rate ranges are separate multipliers;
observed ranges are not HPD intervals.

**Export SVG** saves the whole displayed arrangement, including rotations and
collapsed clades, with a scale, legend and summary provenance. Zoom and scrolling
choose the viewport, not a crop of the exported figure. SVG is a vector image;
the original `mcc.tree`, raw trees and run files remain unchanged. No PNG export,
external tree import or rerooting is included in this viewer.

The bundled `rove inspect-tree /path/to/summary-directory` validates the saved
record and annotated-tree fingerprint before emitting versioned display JSON.
Reading, layout, search and SVG generation run outside the interface thread.
The viewer accepts one Rove MCC tree, up to 16 MiB, 4,096 taxa and 512 nesting
levels; raw sample reader limits above remain unchanged. A summary still does
not establish convergence.

## Inspect trace diagnostics

![Trace diagnostics with 25% burn-in in the native Mac workspace](/images/macos-diagnostics.png)

Choose **Diagnostics** from a finished result or a History entry with a trace.
The native window lists every recorded parameter, with posterior, likelihood
and prior first. Select a row to see its mean, standard deviation, ESS, 95%
equal-tailed interval and trace. `posterior` is a **log density**, not a node's
posterior support.

Set **Burn-in (%)** and press **Apply**. Counts and the first retained state
refer to the applied calculation; pending edits do not relabel older results.
The rule matches tree summarization: discard `floor(N × percent / 100)` samples,
including `STATE_0` in a standard recorded run. **Summarize trees with this
burn-in** opens the summary panel with the applied percentage. Both readers
check recorded sample states and intervals against the run metadata.

**Whole run** includes the discarded beginning in gray and marks the burn-in
boundary. **After burn-in** fits the plot to retained points without changing
statistics. The sample control below the plot supports keyboard inspection of
the plotted state and value. The table, search and controls use native keyboard
navigation; no plot transition is animated. The display keeps at most 1,028
points per parameter, preserving the first, minimum, maximum and last values
in each bucket and the burn-in boundary. **Calculations use every retained
sample**, never the reduced plot.

ESS estimates information equivalent to independent samples, not the number
of MCMC steps. Rove uses the BEAST 2.7.8 paired-positive autocovariance convention,
with lags below 2,000; it is not rank-normalized bulk or tail ESS. A constant
series has unavailable ESS; fewer than four retained values also leave ESS
unavailable. Positive correlation reaching the evaluated lag limit is flagged.
This single-chain panel does not trigger a convergence badge or a stopping rule. Optional multi-chain stopping is configured separately in the exploration plan.

The interval uses linearly interpolated 2.5th and 97.5th percentiles (R type 7).
It describes the retained distribution, **not HPD** or the Monte Carlo error of
the mean. One value has no interval, and very short traces are identified.
Scalar mixing, agreement between independent runs and topology convergence
remain distinct: R-hat and clade comparisons are not calculated in this view.

Each successful calculation saves a new `diagnostics/trace-*/diagnostics.json`
with the source hash, burn-in, state range, methods, statistics and plot points.
**Calculation record** reveals it in Finder. Reopening remembers the last
successful percentage and recalculates from the current trace; saved reports
and original run files remain unchanged. Parsing and statistics run outside
the interface thread and can be cancelled while other analyses continue.

Partial or unverified runs are marked. Non-finite values, inconsistent columns,
irregular states and an unfinished final line are rejected. A recorded completed
run whose trace is missing samples is rejected too. Without `run.json`, a
syntactically valid trace cannot establish completion. Limits are 128 MiB,
one million rows, 128 parameters and eight million numeric cells. The limits
bound the workload, not total process memory.

The standalone command uses the same Rust implementation:

```sh
target/release/rove diagnose /path/to/results/trace.log --burnin-percent 25 \
  --output /path/to/new-diagnostics
```

Omit `--output` to emit JSON, or use `--burnin-samples N` instead of a percentage.
An output directory must be new. No Java or Tracer runtime is required.
[Validation and measurements](https://github.com/getcortexlabs/rove/blob/main/benchmarks/diagnostics/2026-10-29/README.md)
cover synthetic traces and a pinned BEAST reference.

## Comparing independent chains

![Comparison of four short independent chains](/images/macos-chains.png)

In **Model → The exploration plan**, choose **1, 2 or 4 chains**. The step count applies to each chain. Multiple chains run at the same time with distinct, recorded seeds; progress shows each chain separately. Cancellation closes the whole group.

Completed groups and their History details include **Do the chains agree?**, with R-hat and ESS for every traced statistic and a clade-frequency comparison. Select a chain to open its diagnostics, summarize its trees or inspect its folder. Saved setups preserve the chain count; older setups remain single-chain. Outputs stay separate so one chain's burn-in is never mistaken for another's continuation.

**Stop when diagnostic thresholds are met** is optional and off by default. It uses 10% burn-in, ESS ≥ 200 per chain, R-hat ≤ 1.01 for all traced scalar statistics, at least 1,000 retained samples per chain and two consecutive checks. The actual stopping step and reason are saved. These are scalar diagnostic criteria, not a guarantee of topology convergence; inspect the clades and traces. See [independent chains](./sampler#independent-chains) for the CLI options and limits.

**Advanced sampling**, collapsed below the exploration plan, exposes the remaining `rove sample` options. **Sample from the priors only** runs without the characters. With automatic stopping on, the minimum ESS, maximum R-hat and check interval can replace the engine's defaults of 200, 1.01 and its own schedule. The checkpoint interval can be set in steps instead of the default of about every 30 seconds. The proposal schedule offers the classical default, the two schedules evaluated in the [relaxed-clock](https://github.com/getcortexlabs/rove/blob/main/benchmarks/sampling/2026-10-29/README.md) and [gamma-frequency](https://github.com/getcortexlabs/rove/blob/main/benchmarks/gamma-mixing/2026-10-30/README.md) reports, and custom weights and starting step sizes per move, with step-size tuning that can be turned off. Each option is validated before the engine starts, recorded in the run record and in History, and saved in setups as format 5, which earlier app versions refuse rather than run without it. Neither evaluated schedule is recommended; the reports record what each one gained and lost.

## Posterior predictive checks

Completed runs in History include **Can the model reproduce the matrix?**, which runs [`rove predict`](./posterior-prediction) on the run with a chosen burn-in, replicate count and seed; a group is checked one chain at a time. Progress and cancellation work as for the other tools, and a cancelled check leaves no files. Each saved check lists, for parsimony length, the consistency and retention indices, pattern surprise and informative characters, the valid pairs, the lower and upper tail fractions with their 95% Wilson intervals, and the ties, exactly as `predictive.json` records them. Checks are saved in the run's `predictions` folder together with the replicate matrices. Small tail fractions mark a discrepancy to investigate; the app applies no threshold.

## Model comparison

The same details include **Which model explains the matrix better?**, which runs [`rove marginal`](./model-comparison) for the run's model and constraints with a chosen number of powers, alpha, steps per power, burn-in and seed. The run's sampling interval, proposal weights and step sizes are reused; chains, checkpoints and automatic stopping are not available, and no estimate is offered for estimated FBD parameters. Progress shows the current power and step; a cancelled estimate keeps its partial traces in the run folder but is not listed. Each saved estimate shows the stepping-stone and path-sampling values with their standard errors and the per-power table from `marginal.json`. When other completed runs of the same matrix, constraints and gap policy have saved estimates, the panel lists them with the difference from this run's newest estimate and a standard error that combines both in quadrature; these two derived numbers are the only ones the app computes. The guide's caveats apply: the estimate includes the priors, Mk, Mkv and Mkparsinf describe different observation processes, and dated models compare only with identical dating settings.

## Benchmark this Mac

[Benchmark this Mac](./mac-benchmark)
