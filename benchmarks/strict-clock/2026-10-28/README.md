# Strict-clock comparison with BEAST 2.7.8

Rove's sampler and BEAST 2.7.8 sampled the same strict-clock model of the
20 × 50 reference matrix, with the same moves, weights and fixed step sizes,
four chains each. `beast-mkv.xml` conditions the likelihood on variable
characters, as the reference analysis does. `beast-mk.xml` is the same file
without that correction and with its own step sizes. In both configurations,
every posterior mean and every clade frequency met the agreement criteria
that [`tools/beast/strict_clock.py`](../../../tools/beast/strict_clock.py)
fixed before the runs.

## Model and settings

- **Data.** `fixtures/reference/20x50/matrix.nex`, SHA-256 `cb9f8a79…1eab`:
  20 taxa and 50 binary characters in 23 patterns, with no missing data. Both
  XML files embed the same 50 columns and append a constant-zero and a
  constant-one column, which only the Mkv configuration uses.
- **Model.** Mk with two states, equal frequencies and one rate category. A
  strict clock with the rate fixed at 1, so node heights are in expected
  changes per character. The birth–death prior of Gernhard (2008), BEAST's
  `BirthDeathGernhard08Model` with `type="unscaled"`, with
  λ − μ ~ Uniform(0, 1000) and μ/λ ~ Uniform(0, 1), starting at 1 and 0.5.
  T1–T19 must form a clade. BEAST starts from its coalescent `RandomTree` and
  Rove from its own random tree.
- **Chains.** 10⁷ steps, one sample every 1,000 steps, and the first 10% of
  the samples discarded, which leaves 9,001 per chain. Both engines used
  seeds 20261028 to 20261031. Their random number generators differ, so equal
  seeds do not give equal chains.
- **Moves.** Moves that the reference XML also has keep its weights; the tree
  scale, which it lacks, has weight 3. The reference XML uses BEAST's
  Bactrian kernels for node heights, the subtree slide and the scale moves,
  and epoch and tree-stretch operators where this file has a tree scale. Rove
  has none of these, so both engines here use the classic operators. Every
  step is fixed: `optimise="false"` in BEAST and `--no-tuning` in Rove. The
  values come from one Rove run of each configuration with tuning on (seed
  100, 10⁷ steps), rounded to two significant figures. Those pilot runs are
  not part of the comparison.

| BEAST operator | Rove move | Weight | Step, Mkv | Step, Mk |
| --- | --- | ---: | ---: | ---: |
| `Uniform` | `node_height` | 30 | – | – |
| `SubtreeSlide` | `subtree_slide` | 15 | 0.00076 | 0.05 |
| `Exchange`, narrow | `narrow_exchange` | 15 | – | – |
| `Exchange`, wide | `wide_exchange` | 3 | – | – |
| `WilsonBalding` | `wilson_balding` | 3 | – | – |
| `ScaleOperator`, root only | `root_scale` | 3 | 0.75 | 0.72 |
| `ScaleOperator`, tree | `tree_scale` | 3 | 0.24 | 0.46 |
| `ScaleOperator`, λ − μ | `birth_difference_scale` | 3 | 0.28 | 0.25 |
| `ScaleOperator`, μ/λ | `relative_death_scale` | 3 | 0.28 | 0.18 |

The criteria were committed with both XML files in `a4206c7`, before any of
these chains ran. For each statistic, an engine's pooled mean is the mean of
its four chain means, and its Monte Carlo standard error is √(Σ sd²/ESS)/4,
with BEAST's ESS estimator. Two means agree when they differ by at most four
combined standard errors. Two clade frequencies agree when the pooled values
differ by at most 0.05; this applies to every clade with frequency 0.01 or
more in either engine.

## Fixed states

At the final state of each of the eight BEAST chains, Rove's log likelihood
is within 10⁻¹¹ under Mk and 10⁻¹⁰ under Mkv of an evaluation of the same
double inputs with 60 significant digits. Its log likelihood and posterior
are within 10⁻¹⁰ of BEAST's, and its tree prior and prior within 10⁻¹². The
largest likelihood differences from BEAST were 8.5 × 10⁻¹⁴ under Mk and
5.0 × 10⁻¹² under Mkv; from the 60-digit values, 8.5 × 10⁻¹⁴ and
4.3 × 10⁻¹². The Mkv trees are so short that a character is constant with
probability 0.97 to 0.997. The correction takes the logarithm of one minus
that probability, which amplifies rounding by up to 390 times; the looser Mkv
bound allows for that. `tests/sampler.rs` checks these states, which
`tools/benchmarks/prepare.py` reads from `snapshot-*.json`.

## Results

### Mkv

| Statistic | Rove | BEAST | z | ESS per chain, Rove | ESS per chain, BEAST |
| --- | --- | --- | ---: | ---: | ---: |
| `posterior` | −91.08 ± 0.50 | −90.88 ± 0.44 | −0.30 | 148–1329 | 277–819 |
| `likelihood` | −202.968 ± 0.014 | −202.976 ± 0.014 | 0.41 | 8099–8954 | 7573–8123 |
| `prior` | 111.89 ± 0.50 | 112.10 ± 0.43 | −0.32 | 143–1303 | 274–807 |
| `birth_death` | 118.79 ± 0.50 | 119.00 ± 0.43 | −0.32 | 143–1303 | 274–807 |
| λ − μ (`birth_difference_rate`) | 600.3 ± 1.9 | 596.8 ± 1.8 | 1.36 | 4056–5052 | 4462–5624 |
| μ/λ (`relative_death_rate`) | 0.6448 ± 0.0036 | 0.6476 ± 0.0032 | −0.59 | 623–2138 | 1306–1876 |
| `tree_height` | 0.003095 ± 0.000029 | 0.003119 ± 0.000032 | −0.54 | 1882–3163 | 1774–2410 |
| `tree_length` | 0.01994 ± 0.00019 | 0.02010 ± 0.00021 | −0.56 | 1823–3141 | 1768–2415 |

Means are pooled over four chains, ± one Monte Carlo standard error. The 18
clades of the reference tree below the root had pooled frequencies above
0.9999 in both engines, and no other clade reached 0.01. One of them, T1–T19,
is imposed by the constraint. In each engine, one of the 36,004 retained
trees had another topology, both in chains of seed 20261031 but at different
states, which `metadata.json` lists. Rank-normalized split R-hat, the larger
of the bulk and folded values (Vehtari et al. 2021), was at most 1.0007.

### Mk

| Statistic | Rove | BEAST | z | ESS per chain, Rove | ESS per chain, BEAST |
| --- | --- | --- | ---: | ---: | ---: |
| `posterior` | −220.076 ± 0.024 | −220.049 ± 0.023 | −0.82 | 7962–9001 | 8264–9001 |
| `likelihood` | −247.522 ± 0.016 | −247.510 ± 0.016 | −0.50 | 8381–9001 | 8495–9001 |
| `prior` | 27.446 ± 0.016 | 27.462 ± 0.016 | −0.70 | 8374–8843 | 8635–9001 |
| `birth_death` | 34.354 ± 0.016 | 34.370 ± 0.016 | −0.70 | 8374–8843 | 8635–9001 |
| λ − μ (`birth_difference_rate`) | 11.651 ± 0.040 | 11.730 ± 0.038 | −1.42 | 3947–4303 | 4085–4922 |
| μ/λ (`relative_death_rate`) | 0.5170 ± 0.0020 | 0.5127 ± 0.0019 | 1.58 | 3719–4531 | 4102–4789 |
| `tree_height` | 0.18959 ± 0.00020 | 0.18921 ± 0.00019 | 1.39 | 7500–8251 | 7883–8552 |
| `tree_length` | 1.19308 ± 0.00098 | 1.19150 ± 0.00097 | 1.15 | 8339–8737 | 8455–9001 |

ESS is capped at the 9,001 retained samples. Twenty-two clades reached 0.01
in at least one engine; the largest difference in pooled frequency was
0.002. In every chain of both engines, the reference topology had a
posterior frequency between 0.820 and 0.832. The retained samples of each
chain held 86 to 100 distinct topologies, and the mean rooted
Robinson–Foulds distance to the reference tree was 0.35 to 0.38. R-hat was at
most 1.0007.

### Acceptance

With the same steps, each move has the same acceptance rate in both engines
if they sample the same distribution. Rates over the four chains of each
configuration:

| Move | Mkv, Rove | Mkv, BEAST | Mk, Rove | Mk, BEAST |
| --- | ---: | ---: | ---: | ---: |
| `node_height` | 0.4415 | 0.4415 | 0.4667 | 0.4669 |
| `subtree_slide` | 0.2190 | 0.2205 | 0.2371 | 0.2371 |
| `narrow_exchange` | 0.000046 | 0.000054 | 0.0256 | 0.0256 |
| `wide_exchange` | 0.000011 | 0.0000097 | 0.00077 | 0.00079 |
| `wilson_balding` | 0.000019 | 0.000016 | 0.0028 | 0.0028 |
| `root_scale` | 0.2302 | 0.2292 | 0.2354 | 0.2354 |
| `tree_scale` | 0.2371 | 0.2364 | 0.2430 | 0.2422 |
| `birth_difference_scale` | 0.2337 | 0.2348 | 0.2283 | 0.2281 |
| `relative_death_scale` | 0.2341 | 0.2342 | 0.2264 | 0.2273 |

Acceptance rates were not part of the criteria. Against the spread of four
chains, the Mkv root scale differed by z = 3.5, with every Rove chain above
every BEAST chain, and the narrow exchange by z = −2.3. To check, 32 further
Mkv chains per engine ran: eight with the procedure above (seeds 20261101 to
20261108) and 24 run eight at a time, keeping only the operator counts
(seeds 20261201 to 20261224). Over all 36 chains per engine, the root scale
rates were 0.2296 and 0.2293 (z = 1.7), and no move differed by more than 1.7
standard errors. The 32 further chains alone gave z = 1.1 for the root scale
(1.7 for the eight, 0.6 for the 24), with Rove's rate higher in each batch.
In the first four chains, the ratio of the root height to its older child's
height, which decides how often the move fails, agreed: |z| ≤ 1.0 for its
mean logarithm and for the fraction of states where a scale of 0.75 fails.
The eight-chain replication met both criteria, with a largest |z| of 0.97.
The first differences are consistent with chance and with a spread
underestimated from four chains.

### Time

The machine was a MacBook Pro with an Apple M4 Max and 64 GB of memory, on
macOS 26.6.2 and AC power, with Temurin OpenJDK 17.0.15+6 and Rove `a4206c7`
built by rustc 1.99.1. Each chain ran alone, BEAST with `-threads 1`, and the
four BEAST chains of a configuration ran before Rove's. Wall time per chain,
from process start to exit:

| Configuration | Rove | BEAST |
| --- | ---: | ---: |
| Mkv | 11.97–12.02 s | 17.64–18.34 s |
| Mk | 11.63–12.62 s | 16.84–17.35 s |

BEAST's own calculation time was 0.16 to 0.18 s shorter than its wall time.
Median ESS per second of wall time:

| Statistic | Mkv, Rove | Mkv, BEAST | Mk, Rove | Mk, BEAST |
| --- | ---: | ---: | ---: | ---: |
| `posterior` | 64 | 32 | 738 | 518 |
| `likelihood` | 709 | 433 | 747 | 516 |
| λ − μ | 371 | 275 | 340 | 273 |
| `tree_height` | 200 | 116 | 672 | 485 |

## What the Mkv configuration tests

Each of the 50 characters needs exactly one change on the reference tree.
Given that a character varies, the probability of such a pattern tends to a
constant as every branch shrinks, so the Mkv likelihood does not penalize a
short tree. The birth–death density of a short tree grows with λ − μ, and in
both engines the posterior density of λ − μ rises all the way to the prior's
upper bound of 1000; `metadata.json` gives the histogram. The tree height
averages 0.0031 expected changes per character, and the topology stays at
the reference tree: narrow exchanges were accepted 4.6 (Rove) and 5.4
(BEAST) times in 100,000. This configuration therefore checks the Mkv
likelihood and the continuous moves on a posterior shaped by a prior bound,
while its clade comparison checks little. Without the correction, a short
tree makes the 50 changes unlikely: under Mk the tree height averages 0.19
and λ − μ about 11.7, and narrow exchanges are accepted 2.6% of the time, so
the clade comparison exercises the topology moves.

## Runs

The first Mk attempt stopped after BEAST's chains, when the snapshot export
in `ReferenceDiagnostics.java` called BEAST's ascertainment correction on an
alignment without one. After the fix, the configuration was rerun from the
start. BEAST's traces, trees and final states were identical to those of the
first attempt; `metadata.json` records the hashes of both.

## Reproduce

Install BEAST 2.7.8 and its packages as in the
[reference report](../../beast-2.7.8/2026-10-25/README.md#reproduce). From
the repository root, with output directories that do not exist yet:

```sh
cargo build --release --locked
python3 tools/beast/strict_clock.py \
  --packages /path/to/packages --launcher /path/to/launcher.jar \
  --xml benchmarks/strict-clock/2026-10-28/beast-mkv.xml \
  --matrix fixtures/reference/20x50/matrix.nex \
  --truth fixtures/reference/20x50/truth.newick \
  --rove target/release/rove --output /path/to/new-mkv-run \
  --seeds 20261028 20261029 20261030 20261031
```

Use `beast-mk.xml` for the Mk configuration. The script reads the chain
length, the constraint, the conditioning, the moves and the steps from the
XML, checks that each Rove run recorded the same moves, weights, steps and
starting values, and writes `comparison.json`. The replication used the same
command with seeds 20261101 to 20261108. The 24 count-only chains ran
`beastfx.app.beast.BeastMCMC -batch -seed <seed> -threads 1` on a copy of
`beast-mkv.xml`, and the Rove command that `strict_clock.py` builds from it,
with seeds 20261201 to 20261224. The pilot runs were:

```sh
target/release/rove sample fixtures/reference/20x50/matrix.nex \
  --output /path/to/pilot --seed 100 --steps 10000000 --sample-every 1000 \
  --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
```

with `--conditioning none` added for Mk. Their tuned values were 0.000759,
0.748, 0.237, 0.277 and 0.284 under Mkv, and 0.0497, 0.723, 0.457, 0.251 and
0.184 under Mk, in the order of the step table above.

## Files

- `beast-mkv.xml` and `beast-mk.xml`: the two configurations.
- `comparison-mkv.json`, `comparison-mk.json` and
  `comparison-mkv-replication.json`: the output of `strict_clock.py`, with
  pooled means, clades, R-hat, tree diagnostics, wall times and ESS per
  second.
- `summaries-mkv.json` and `summaries-mk.json`: the per-chain summaries of
  both engines.
- `snapshot-mkv-*.json` and `snapshot-mk-*.json`: the final state of each
  BEAST chain, with each posterior component and each pattern's likelihood.
- `metadata.json`: hardware, software versions, JAR and executable hashes,
  per-chain timings, operator counts and output hashes, the counts of the 24
  chains of the acceptance check, and the extra checks cited above.

## Limitations

These results cover one matrix, one machine and BEAST 2.7.8; they say nothing
about BEAST X. Four chains per engine set the standard errors of the main
comparison. Rove's adaptive tuning differs from BEAST's default and is not
compared, since both engines ran with fixed steps from Rove's pilots. Under
Mkv the clade comparison is nearly empty. The times describe this small
configuration on this machine, with BEAST's JVM startup included; they are
not the controlled benchmarks tracked in
[#8](https://github.com/getcortexlabs/rove/issues/8).
