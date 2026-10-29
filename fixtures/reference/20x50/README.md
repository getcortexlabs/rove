# Selected 20 × 50 reference

These are the supplied synthetic morphological data and BEAST 2 configuration
selected for the reference experiment. They are separate from the generated
control in `fixtures/synthetic`.

- `matrix.nex`, `legend.txt`, and `analysis.xml` preserve the supplied bytes,
  including the original comments and identifiers. `manifest.json` records
  their source filenames and SHA-256 hashes.
- `truth.newick` extracts the known topology from the NEXUS comment. It is
  used to assess the sampled trees, not to initialize the analysis.

There are 20 taxa and 50 binary characters, with no missing or constant
observed characters. The observations contain 23 unique column patterns:
nine with weight three, nine with weight two, and five with weight one.
The XML appends constant-zero and constant-one columns for Mkv ascertainment;
they have zero observation weight and bring the likelihood workspace to 25
patterns. The observed character count remains 50.

The XML configures Mk + Mkv, four gamma categories, a lognormal relaxed clock,
and the Gernhard birth-death tree prior. It requires BEAST.base 2.7.8,
MM 1.2.1, ORC 1.2.1, and BEASTLabs 2.0.3. BEAST.app 2.7.8 supplies the CLI
and analysis tools.

T1–T19 are constrained to be monophyletic. Their posterior support is imposed;
the other 17 reference clades are assessed separately. No calendar-time
calibration is present.

See the [reference report](../../../benchmarks/beast-2.7.8/2026-10-25/README.md)
for results, commands, limitations, and exported numerical states.

## Standalone Rove configuration

After `cargo build --release --locked`, run:

```sh
sh fixtures/reference/20x50/rove.sh runs/reference-1 1
```

This runs ten million steps with `--model reference`, samples every thousand
steps and constrains T1–T19. The output directory must not exist. The model
uses the XML's four gamma midpoint categories, explicit lognormal branch
rates and priors. The [model guide](../../../docs/guide/reference-model.md)
accounts for the parameters and for proposal differences from ORC. This
command does not use BEAST; complete posterior validation is tracked in #12.
