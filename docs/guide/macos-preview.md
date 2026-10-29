---
description: "Build and explore Rove's first native Mac workspace: import a matrix, inspect its characters and prepare an analysis."
---

# The Mac preview

Rove has a first native macOS workspace built with SwiftUI and AppKit. It
imports matrices through the Rust engine and prepares analysis settings.
Execution and progress monitoring inside the window are not implemented yet.

![The native Rove workspace](/images/macos-preview.png)

## Build and open

On macOS 14 or newer, with Xcode supporting Swift 6 and Rust installed, run
these commands from the repository root:

```sh
scripts/build-macos.sh
open build/macos/Rove.app
```

This builds a local application with an ad-hoc signature. It is not a
notarized release or an installer. The app currently uses English and a dark
appearance inspired by Cortex Labs, with reduced-motion support.

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

## Prepare an analysis

Choose a strict clock or the [relaxed-clock reference preset](./reference-model),
Mk or Mkv, chain length, sampling interval and seed. Review these choices before
saving. Priors keep their engine defaults, and no group is constrained
automatically. Chain length alone does not establish convergence.

**Save setup** writes a `.rove.json` record with the original matrix path,
SHA-256 and settings. It does not copy the matrix or start a run. Reopening
these records in the app is not implemented yet.

**Copy terminal command** prepares a command for the bundled Rust engine.
Pasting it into Terminal starts a real analysis using the original matrix at
execution time. The command uses a new output folder under `~/Rove Runs`.
The [sampling guide](./sampler) explains those output files and diagnostics.

## Inspect from the command line

```sh
target/release/rove inspect fixtures/reference/20x50/matrix.nex
```

The command emits versioned JSON with counts, a bounded cell preview,
the input hash and separate sampling/Mkv eligibility errors. Add
`--gaps missing` only when that interpretation is appropriate. Reading a
matrix does not write any analysis outputs.
