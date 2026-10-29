# Native macOS preview

A SwiftUI and AppKit workspace for importing a morphological NEXUS matrix,
inspecting its observations and preparing an analysis. Requires macOS 14 or
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

The window does not run or monitor MCMC, reopen setup files, edit the matrix,
or display inferred trees. There are no invented runs, progress or results.
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
needed to import a matrix; only the explicitly clicked guide and Cortex links
open a browser.

Files are limited to 32 MB in the app. Statistics cover the full imported
matrix; the visual preview contains at most 200 taxa and 120 characters.
The window is resizable down to 1080 × 700, with scrollable content.

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
