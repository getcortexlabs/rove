# Native macOS preview

A SwiftUI and AppKit workspace for importing a morphological NEXUS matrix,
inspecting its observations and running an analysis. Requires macOS 14 or
newer, Xcode with Swift 6, and the repository's Rust toolchain.

From the repository root:

```sh
scripts/build-macos.sh
open build/macos/Rove.app
```

Pass `release` to build an optimized Swift executable. The Rust helper is
always built in release mode. The script creates a local, ad-hoc-signed app;
it does not create a notarized distribution or install anything. All generated
files stay in `build/` and `apps/macos/.build/`. In an environment that already
sandboxes build processes and cannot start SwiftPM's nested sandbox, set
`ROVE_NESTED_BUILD_SANDBOX=1`; the outer environment's restrictions still apply.

## Behavior

- Open a matrix with Command-O or drop one file onto the welcome screen.
- The bundled `rove inspect` process reads the file using the existing Rust
  parser. Swift does not implement a second NEXUS parser. Import runs away
  from the main thread, and a stale import cannot replace a newer workspace.
- The preview preserves state sets, missing cells and gaps. Colors follow
  declared symbols, not hard-coded binary digits. Parsing a matrix does not
  mean the sampler supports it: unsupported sampling inputs remain viewable
  with an explanation, and analysis preparation is blocked.
- Gaps are rejected for sampling until the user explicitly treats them as
  missing. Mkv remains unavailable for columns that may be constant.
- Select the strict or relaxed-clock preset, Mk/Mkv, chain length, sampling
  interval and seed. Priors are the engine's defaults; no monophyly constraint
  is applied automatically. The complete settings are reviewed before export.
- Save a `.rove.json` record or copy a shell-quoted command for the bundled
  CLI. A saved setup references the original matrix and records its SHA-256;
  it does not copy the data. Copied commands read the file when executed.

The window runs the bundled Rust sampler with live progress and cancellation.
It does not edit matrices or resume chains. Saved MCC summaries open in a native tree explorer. Completion
means the requested steps finished and output metadata was verified, not convergence.
The setup format is a preview format, not a configuration accepted by the CLI.
The app supports English and Brazilian Portuguese, with a dark appearance.
Open Settings from the sidebar or Command-comma to follow the Mac's language
or choose one explicitly. Changes apply immediately without resetting the
matrix or analysis settings. Language and visual-comfort preferences persist
in UserDefaults; scientific exports and CLI arguments retain their format.
System accessibility preferences take priority over the app's motion and
transparency controls. Native system dialogs follow the Mac's language;
original engine diagnostics remain available under Technical details.
No account, network service, Java or BEAST is
needed to import or analyze a matrix; only the explicitly clicked guide and Cortex links
open a browser.

Files are limited to 32 MB in the app. Statistics cover the full imported
matrix; the visual preview contains at most 200 taxa and 120 characters.
The window is resizable down to 1080 × 700, with scrollable content.

## Execution and saved setups

Review the configuration and press Start analysis (Command-Return). In Model,
Outgroup optionally selects one terminal from a matrix with at least three taxa.
The app passes a monophyly constraint for all remaining terminals to the existing
Rust sampler. Every taxon stays in the analysis. T20 is never selected automatically;
multiple outgroups are not supported by this control.

Additional groups remain explicit, nested or disjoint, with at least two known
terminals each. The editor stages changes until Apply group; Cancel leaves the
previous group untouched. Existing groups can be edited with the pencil button.
Groups incompatible with the outgroup block review and execution. An identical
manual ingroup is deduplicated in the applied constraints, but retained as a manual
choice if the outgroup is later cleared.

Review, running results and History show the outgroup, applied groups and retained
taxa. Imposed support is not independent evidence. Changing the setup affects a
new run, never reroots saved trees. The outgroup is a tip, not the ancestral root;
branch rotation or vertical ordering in a viewer does not determine rooting.

New setup records use format 2: `taxa` stores the complete terminal list,
`outgroup` is optional, `monophyly` retains manual groups, and `appliedMonophyly`
records the derived constraints sent through `--monophyly`. Validation checks
these fields agree, then verifies the terminal list against the fingerprint-checked
matrix on reopen. Engine records are matched against the applied constraints.
Format 1 setups and sessions remain readable with their original groups, without
inferring an outgroup or inventing a missing terminal list. Older apps reject format
2 rather than silently dropping rooting. The outer session format remains 1.

Each run gets a unique directory under `~/Rove Runs` or a selected parent folder:
`setup.rove.json`, `session.json`, `events.jsonl`, `engine.stderr.log` and
`results/{trace.log,trees.nexus,run.json}`. The app never overwrites a run.
A session starts as `partial`; only a valid terminal event, successful exit
and matching engine metadata establish `completed`. Failed and cancelled runs
retain their partial files. Abrupt termination can leave incomplete files.

Open Setup (Command-Shift-O) restores a `.rove.json` after checking its version,
settings, referenced matrix and SHA-256. Missing or changed input blocks reopening.
The app and engine check the fingerprint again before a run. Reopening a setup
prepares a new run; it does not resume a previous chain.

The main actor only receives validated progress. The process worker drains stdout
and writes stderr separately; malformed events, unexpected exits and invalid output
metadata fail the run. Analysis, tree-summary and diagnostic workers each wait on
a dedicated thread and return through a continuation. Blocking pipe reads and
process waits do not occupy Swift's cooperative executor, keeping cancellation
and other tasks schedulable on machines with fewer cores.
Cancellation closes the engine's stdin pipe. The engine
polls EOF between steps, finishes its writers and records `cancelled` with exit 130.
Closing the main window or quitting asks whether to stop, then waits for the owned
process. EOF also stops sampling if the app dies. A stalled child gets SIGTERM after
two seconds and SIGKILL two seconds later; this fallback may leave incomplete files.

## History

History (Command-Shift-H) lists recorded runs newest first, with search by
matrix or seed and an outcome filter. Details retain the model, explicit groups,
input fingerprint, requested/completed steps, elapsed time and file locations.
Use this setup passes the recorded draft through the existing fingerprint-checked
review flow; it neither runs automatically nor resumes a chain. Browsing preserves
the current workspace and owned process. Return with Command-Shift-R; an active
analysis also remains reachable from the sidebar.

`RunHistory` remembers output roots and discovered run paths in UserDefaults
(`historyFolders`, `historyRuns`). It discovers immediate child app-run folders
in `~/Rove Runs` and explicitly selected output locations. Add run folder registers
an individual app-run directory. No recursive home-directory search, database or
network service is involved. The session and engine records remain the source of
truth; the index only stores locations. Tests inject isolated defaults and roots.

Scanning runs off the main actor and rejects stale scan results. It reads bounded
metadata (2 MiB per record), never trace or tree contents. Missing locations remain
visible as unavailable. A completed session also needs matching engine metadata
and nonempty trace/tree files; otherwise it reports incomplete files. A `partial`
session remains interrupted even when an engine file claims completion. History
cannot establish convergence and never deletes any run files.

Session format 1 now optionally records `startedAt`, `finishedAt`, `elapsedSeconds`
and `completedSteps`. Older records fall back to recorded engine timing and steps,
or show an unavailable value. The app creates its session record before input
validation so early cancellation and failed starts remain discoverable; the engine
still checks the input hash before creating scientific output. If the run folder
itself cannot be written, History reports its unavailability.

## CLI event protocol (version 1)

`rove sample ... --events-json` emits one JSON object per stdout line and flushes
it immediately. Human diagnostics remain on stderr. `--print-every N` controls
progress frequency without changing the random trajectory or scientific samples.

- `started`, `progress`, `completed`, `cancelled`: `schema_version`, `event`,
  `step`, `total_steps`, `elapsed_seconds`, `input_sha256`, `posterior`,
  `likelihood`, `prior`. Posterior values are log densities, not support or ESS.
- `failed`: `schema_version`, `event`, `message`; it may appear before `started`.
- `completed` follows closed outputs and the written `run.json`; exit status is 0.
  `cancelled` exits 130. Other failures exit 2. A crash may emit no terminal event.
- `--expect-input-sha256 HASH` checks the exact bytes parsed before creating output.
- `--cancel-on-stdin-close` opts into owner-pipe cancellation. Keep stdin open while
  running; EOF requests cancellation. Ordinary terminal runs do not enable this flag.

Consumers must reject unknown protocol versions, truncated/malformed events,
backward steps or elapsed time, a mismatched fingerprint, events after termination,
and an exit without a terminal event. Event lines are capped at 64 KiB in the app.
The `run.json` record adds `status` and `completed_steps`; requested steps remain
under `settings.steps`. The inspector includes full `taxon_names` independently
of its bounded cell preview so group selection covers the whole matrix.

## Checks

```sh
cargo build --locked --release --bin rove
swift test --package-path apps/macos
```

The tests invoke the actual Rust reader on the reference and missing-data
fixtures, check import failure/reset behavior, validate sampling settings,
round-trip draft metadata and execute copied commands against an argument
recorder to check shell quoting. Preference tests cover language resolution,
persistence, localized formatting and analysis preservation during switching.
Run tests also compare app and CLI samples with the same binary and seed, reopen
setups, reject changed input, cancel a live child, and handle malformed events,
launch failures, output collisions and stale progress. History tests cover real
completed/cancelled/failed runs, cancellation before launch, legacy metadata,
missing and corrupt files, persistent custom locations, duplicate paths and
configuration reuse while preserving an active run. Outgroup tests run both clock
presets and inspect every saved root split, including STATE_0; they also cover staged
group edits, conflicting selections, format 1 compatibility, format 2 integrity,
reset/import behavior and reuse through History.
Set `ROVE_TEST_ENGINE` to test another Rust
executable; otherwise tests use `target/release/rove`.

On a local Mac, `PreviewTests` renders the actual SwiftUI/AppKit views in
normal and compact windows, plus Settings, in both languages to `.local/ui/i18n/`.
It skips rendering when `CI` is
set because CI does not provide an interactive desktop. These images support
visual review; they are not pixel-equality assertions. Debug app builds also
accept `--snapshot <path>` with optional `--example`, `--model`, `--review`
and `--compact`, then exit after capturing their content view.

## Assets

- The display typeface and playful direction follow [Cortex Labs](https://getcortexlabs.com/).
  The app uses graphite, lavender and ice blue, with native Liquid Glass on
  macOS 26 and a frosted-material fallback on earlier systems.
- `Resources/RoveExplorer.png` is a generated, transparent raster illustration
  of a fictional fossil explorer. SwiftUI animates its hover and click responses;
  it is not a scientific reconstruction or a live 3D scene. The existing Rove
  emblem is used unchanged.
- `Resources/en.lproj/Localizable.strings` and `Resources/pt-BR.lproj/Localizable.strings`
  contain interface translations. Keep keys and format arguments synchronized.
- `Resources/BricolageGrotesque.ttf` is the unmodified variable font from
  [Google Fonts](https://github.com/google/fonts/tree/main/ofl/bricolagegrotesque),
  licensed under the bundled `OFL.txt`. Body text uses the system font.
- `Resources/Example.nex` is a copy of `fixtures/reference/20x50/matrix.nex`,
  with the same provenance and 20 taxa, 50 observed characters and 23 patterns.
  Keep it synchronized with that fixture. It is not the separate generated
  control and is not a posterior result.

## Posterior tree summaries

Completed Run and History entries open `TreeSummaryView`. The model dispatches
`TreeSummaryProcess` off the main actor; it invokes the bundled `rove summarize`
with explicit burn-in and height policy. Closing its stdin cancels the child,
including when the owner exits. Original session/setup/engine files are never
rewritten. New UUID directories under `summaries` contain the raw `.trees` copy,
annotated `mcc.tree` and versioned `summary.json`; there is no Java dependency.

Summary records and required output files are checked when reopening the panel.
Incomplete directories are ignored. The native process rechecks that the source
run is completed; the CLI independently validates source metadata, sampling
intervals and tree content. Errors retain engine details without interpreting
summary support as convergence. The [guide](../../docs/guide/macos-preview.md#summarize-posterior-trees)
documents supported inputs, formulas, disk costs and remaining statistical limits.

## Annotated-tree explorer

Saved summary cards in completed results and History open a separate window.
`rove inspect-tree <summary-directory>` verifies the summary record, tree SHA-256,
translation, constraints and heights, then emits a version 1 display document.
It accepts one rooted Rove MCC tree, not raw `STATE_` samples or arbitrary external
files. Files are bounded at 16 MiB; metadata at 2 MiB; trees at 4,096 taxa and
512 nesting levels. Optional annotations remain optional in the display document.

`TreeExplorer` reads and lays out the document off the main actor. Display revisions
prevent an older layout from replacing a newer collapse/rotation. `TreeLayout`
keeps height-based horizontal positions independent of display order. `TreeCanvas`
uses a magnifying AppKit scroll view and culls offscreen node labels. Search can
reveal a tip through collapsed ancestors. The inspector and arrow/Space keys
provide navigation; no display operation changes the scientific tree or reroots it.

`TreeFigure` shares geometry between the canvas and SVG. Export includes the whole
display arrangement, scale, summary settings and constraint legend. File writes
reject scientific source paths, including symlink aliases. SVG generation happens
on a worker task. Raw sample browsing, external tree import, PNG and HPD intervals
are not implemented. See the [guide](../../docs/guide/macos-preview.md#explore-an-annotated-tree)
for the distinction between clade-conditional ranges and common ancestor heights.

Tests cover known topology and support, collapse/rotation invariants, provenance
mismatches, missing metadata, outgroup context, search through collapsed clades,
XML escaping and protected-file exports. A 4,096-tip synthetic document exercises
background decoding, layout and SVG generation; its elapsed time is printed by
`largeLayoutAndExportRemainBounded`, without a hardware-dependent pass threshold.
Native preview captures cover both languages and compact windows.

## Trace diagnostics

Completed results and History entries with `results/trace.log` open a separate
Diagnostics window. Cancelled and partial runs may be inspected, with their
status visible. `TraceDiagnosticsProcess` runs the bundled `rove diagnose`
off the main actor and cancels through stdin EOF. The Rust `diagnostics` module
owns parsing, moments, quantiles, autocovariance ESS and display point selection;
Swift decodes and validates the versioned report rather than implementing another
estimator. No Java or Tracer runtime is needed.

Each successful calculation saves a new `diagnostics/trace-*/diagnostics.json`
with source SHA-256, burn-in, retained state range and methods. Reopening restores
the latest successful percentage, then calculates from the current trace.
Applying burn-in never changes raw data. Older results keep their applied
percentage visible while a new calculation runs or fails. The tree-summary
button passes that same applied percentage through the existing summary workflow.

Swift Charts receives at most 1,028 representative points per parameter; Rust
statistics use every retained sample. A native table, search and sample stepper
provide keyboard access. Whole-run and retained-only plots share the same
statistics; the latter adjusts the display range after burn-in. Plot coordinates
are normalized internally for extreme finite values, while axes and accessibility
controls expose original states and values.

ESS is unavailable for constant or very short traces. Reaching the estimator's
lag limit is marked, and the interval is explicitly equal-tailed, not HPD.
The view never infers cross-run or tree convergence from scalar ESS. See the
[guide](../../docs/guide/macos-preview.md#inspect-trace-diagnostics) and
[validation report](../../benchmarks/diagnostics/2026-10-29/README.md) for methods,
input bounds, reference comparisons and measured cost.
