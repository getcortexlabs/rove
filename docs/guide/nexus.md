---
description: "Read morphological NEXUS matrices in Rust, preserve observation semantics, and compress repeated columns within explicit model partitions."
---

# Reading morphological NEXUS

`nexus::parse` reads a UTF-8 string into a `data::Matrix`. It preserves taxon order, the declared state alphabet, character and state labels, and every observed cell. It has no runtime dependencies and does not invoke BEAST.

Try the supplied reference matrix from the repository root:

```sh
cargo run --release --example read_nexus -- fixtures/reference/20x50/matrix.nex
```

```text
20 taxa, 50 characters, 2 declared states
23 observed patterns (50 total weight)
```

This example treats all characters as one partition and rejects gaps and polymorphism when preparing likelihood inputs. It imports data; it does not run inference.

## Supported syntax

Invalid or unsupported input returns `nexus::Error` with a one-based line, column and explanation.

| Feature | Support |
| --- | --- |
| Blocks | One `DATA` or `CHARACTERS` block, optionally preceded by one `TAXA` block. `TREES` blocks are skipped; trees are not imported. Other blocks are rejected. |
| Dimensions | Positive `NTAX` and `NCHAR`. A preceding `TAXA` block may supply `NTAX`. `NEWTAXA` requires `NTAX`. All declared taxa must occur exactly once in the matrix. |
| Format | `DATATYPE=STANDARD`, `SYMBOLS`, `MISSING`, `GAP`, `RESPECTCASE`, `LABELS`, `NOTOKENS`, and `INTERLEAVE` or `INTERLEAVE=YES/NO`. |
| Alphabet | At least two valid ASCII symbols, one character per state. Defaults to `01`. Declared order is preserved, including unobserved states. |
| Sequential rows | Named rows, with optional whitespace between cells and line wrapping. |
| Interleaved rows | Every page repeats every taxon label in the same order. Each row occupies one line, with equal character counts within a page. Page widths may differ. Blank lines are optional. Unlabeled continuation pages are unsupported. |
| Names | Single-quoted names, doubled apostrophes and unquoted names. Underscores become spaces. Matching and duplicate detection ignore ASCII case; returned names keep their spelling. |
| Labels | `CHARLABELS` with exactly `NCHAR` names; indexed `STATELABELS` and `CHARSTATELABELS`. Descriptive state labels do not reduce the declared alphabet. |
| Comments | Nested square-bracket comments, including comments inside words. LF, CRLF and CR line endings are accepted. |

With `TAXLABELS`, returned rows follow the declared order even if matrix rows appear in another order. Otherwise, the first matrix page defines the order.

Defaults are case-insensitive states, missing symbol `?`, no gap symbol, and sequential rows. Thus `-` is not a gap unless declared with `GAP=-`. State, missing and gap symbols must be distinct under the chosen case policy.

Molecular or continuous datatypes, `TOKENS`, `TRANSPOSE`, `NOLABELS`, `EQUATE`, `MATCHCHAR`, state ranges, frequencies/counts, `ELIMINATE`, multiple matrices and unsupported commands are rejected. `SETS`, `ASSUMPTIONS` and application-specific blocks are also rejected: their partitions, weights, ordering or exclusions must not disappear on import. XML parsing is outside this API.

## Observations and tip likelihoods

The distinction between braces and parentheses follows the [NEXUS specification](https://doi.org/10.1093/sysbio/46.4.590). For a declared alphabet `012`:

| Input | Stored cell | Tip likelihood vector |
| --- | --- | --- |
| `1` | `Cell::State(1)` | `[0, 1, 0]` |
| `{01}` | `Cell::Uncertain([0, 1])` | `[1, 1, 0]` |
| `(01)` | `Cell::Polymorphic([0, 1])` | Rejected at likelihood conversion |
| `?` | `Cell::Missing` | `[1, 1, 1]` |
| Declared gap | `Cell::Gap` | Rejected, or `[1, 1, 1]` with explicit `GapTreatment::Missing` |

Uncertainty means one of the listed states; polymorphism records multiple states present. The current Mk observation model does not represent polymorphism. Import retains it for inspection, but compression returns an error identifying the taxon and character. There is no automatic conversion to uncertainty. Gaps likewise require an explicit choice because morphological gaps can represent inapplicability.

Uncertainty vectors are likelihood masks, not probability distributions: `{01}` contributes one for each allowed state, without division by two. State sets are sorted and deduplicated on import.

## Compressing repeated columns

Partition ranges are zero-based, contiguous, nonempty, and must cover the matrix once in order. Pass separate ranges whenever characters use different models or parameters. Every partition retains the full declared alphabet; the reader never infers per-character state counts or ordered models from the observations.

```rust
use rove::{data::GapTreatment, nexus};

let input = std::fs::read_to_string("fixtures/reference/20x50/matrix.nex")?;
let data = nexus::parse(&input)?;
let sites = 0..data.character_count();
let partitions = data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)?;
let patterns = &partitions[0];

assert_eq!(patterns.weights().len(), 23);
assert_eq!(patterns.weights().iter().sum::<usize>(), 50);
```

`Patterns::weights` holds exact integer multiplicities. `site_to_pattern` maps each original character within the partition to a pattern, numbered by first occurrence. `tip_partials` contains pattern-major values for each taxon, ready for `LikelihoodCore::set_tip_partials`. Convert integer weights to `f64` when calling the core's `log_likelihood`.

Compression compares complete cell observations, including their kind. Missing and uncertain cells remain separate patterns even when their likelihood masks match. Identical columns across partitions also remain separate. This preserves the original observations through the matrix and mapping, at the cost of occasionally retaining redundant calculations.

Only observed columns are compressed. Constant and missing-only columns remain present, and no artificial ascertainment patterns are added. The [Mkv correction](./kernel#conditioning-on-variable-characters) appends its constant patterns separately and rejects characters that could be constant.

Tests read both [20 × 50 fixtures](./fixtures), reconstruct their columns from the pattern mapping, and compare compressed and uncompressed likelihoods on generated matrices with uncertain and missing data.
