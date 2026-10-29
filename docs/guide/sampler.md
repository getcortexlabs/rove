---
description: "How to run Rove's first standalone sampler on a binary morphological matrix: the model, the command, the files it writes, the proposals and the checks behind them."
---

# Sampling trees

`rove sample` runs a Markov chain Monte Carlo analysis of a binary morphological matrix without BEAST. The default preset uses a strict clock and one rate category. `--model reference` enables the [reference model](./reference-model), with four gamma categories and explicit lognormal branch rates.

## The default model

- **Characters.** One partition of binary characters under Mk with equal frequencies and one rate category. By default the likelihood is conditioned on variable characters with the [Mkv correction](./kernel#conditioning-on-variable-characters); `--conditioning none` uses plain Mk.
- **Clock.** Strict, with the rate fixed at 1. Node heights are therefore in expected changes per character, the convention of the reference analysis, which has no calibrations. All tips are at height zero.
- **Tree prior.** The birth–death prior of Gernhard (2008), computed as BEAST 2.7 computes `BirthDeathGernhard08Model` with `type="unscaled"`: every lineage is sampled, and the density is conditioned on the number of tips but not on the root height. Its parameters are r = λ − μ, with a Uniform(0, 1000) prior, and a = μ/λ, with a Uniform(0, 1) prior. These are the definitions of the reference XML, where the parameters are named `BDBirthRate` and `BDDeathRate`.
- **Constraints.** Each `--monophyly` option lists taxa that must form a clade. Proposals that would break a constraint are rejected. Constraints must be nested or disjoint.

## Running it

```sh
cargo build --release
target/release/rove sample fixtures/reference/20x50/matrix.nex \
  --output runs/seed-1 --seed 1 --steps 10000000 --sample-every 1000 \
  --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
```

| Option | Meaning |
| --- | --- |
| `--output <directory>` | Directory for the output files. It must not exist yet. |
| `--seed <n>` | Seed of the random number generator. Required. |
| `--steps <n>` | Number of steps in the chain. Required. |
| `--sample-every <n>` | Steps between samples; it must divide `--steps`. The initial state is sample 0. Required. |
| `--monophyly <taxa>` | Comma-separated names of taxa that must form a clade; as in the matrix, `_` stands for a space. Can be repeated. |
| `--gaps reject\|missing` | Reject matrices with gaps (the default), or read gaps as missing data. |
| `--model strict\|reference` | Strict clock with one category (default), or the gamma and relaxed-clock reference model. |
| `--conditioning variable\|none` | Mkv (the default) or Mk. |
| `--sample-from-prior` | Ignore the characters and sample the prior. |
| `--no-tuning` | Keep every slide size and scale factor fixed. |
| `--tuning <move>=<value>` | Starting slide size or scale factor for a move, such as `subtree_slide=0.001`. Can be repeated. |
| `--weight <move>=<value>` | Relative selection weight; finite and nonnegative. Zero disables a move. Repeat to override several moves; the last value for a move wins. |
| `--backend portable\|neon` | The likelihood [backend](./kernel#backends): the portable code, or the NEON kernel (the default on aarch64). Both give the same samples. |
| `--print-every <n>` | Steps between progress lines on standard error; by default, a twentieth of the chain. |

The command stops with a message on input it does not support: characters with more than two states, partitions or model blocks in the NEXUS file, characters that could be constant under Mkv, taxon labels with characters other than letters, digits, spaces, `_`, `.` and `-` (or with both spaces and `_`), unknown taxa in a constraint, overlapping constraints that are not nested, out-of-range tuning values, a backend that the build does not include, and options it does not know or that are given twice.

Given the same executable, input, settings and seed on the same platform, a run produces the same samples, whichever the backend. The logarithms and exponentials come from the platform's math library, so another platform can differ in the last digits and then diverge.

## Output files

The files below describe the default preset. The [reference preset](./reference-model#proposals-and-output) adds rate parameters and priors to the trace and annotates branch rates in the trees.

- **`trace.log`** is tab-separated, after lines starting with `#` that record the version, the command and the SHA-256 of the input and of the executable. Its columns are `Sample`, `posterior`, `likelihood`, `prior`, `birth_death` (the log density of the tree prior), `birth_difference_rate`, `relative_death_rate`, `tree_height` and `tree_length`. The layout is that of a BEAST trace, and BEAST's LogAnalyser reads it.
- **`trees.nexus`** is a NEXUS trees block with a translate table and one tree per sample, which BEAST's NexusParser reads. Spaces in taxon labels are written as `_`. Branch lengths are in expected changes per character.
- **`run.json`** records the command's arguments, the platform, the input's path and SHA-256, the executable's SHA-256 taken when the run starts, the model, every setting (including the backend), each move's weight, tuning and acceptance counts, the final posterior and the wall time.

## Proposals

Each move follows the definition and Hastings ratio of the BEAST 2.7 operator in the second column. The weights are those of the strict-clock comparison with BEAST.

| Move | BEAST 2 operator | What it changes | Weight |
| --- | --- | --- | ---: |
| `node_height` | `Uniform` | The height of a non-root internal node, uniform between its older child and its parent | 30 |
| `subtree_slide` | `SubtreeSlide` | Moves a node up or down its lineage by a Gaussian step, regrafting its subtree when it passes another node | 15 |
| `narrow_exchange` | `Exchange`, narrow | Swaps a node with its uncle | 15 |
| `wide_exchange` | `Exchange`, wide | Swaps two subtrees whose heights allow it | 3 |
| `wilson_balding` | `WilsonBalding` | Prunes a subtree and regrafts it on another branch, never at the root | 3 |
| `root_scale` | `ScaleOperator`, root only | Scales the root height | 3 |
| `tree_scale` | `ScaleOperator` on the tree | Scales every internal height | 3 |
| `birth_difference_scale` | `ScaleOperator` | Scales λ − μ | 3 |
| `relative_death_scale` | `ScaleOperator` | Scales μ/λ | 3 |

The slide size and the scale factors are tuned during the run toward an acceptance rate of 0.234, with BEAST's update rule. Tuning starts after 10,000 tunable proposals. After a move's `n`-th proposal since then, the step is `(α − 0.234)/√(n + 1)`: this is BEAST's optional square-root transform, while BEAST's default divides by `n + 1`. A proposal that is invalid or breaks a constraint counts as failed and enters the tuning with `α = 0`. For an invalid proposal, BEAST reuses the previous step's α instead. With `--no-tuning` the values stay fixed, which makes the proposals of two engines comparable.

## State and restoration

Before each step the sampler saves the tree and the rate parameters, and stores the [likelihood core's](./kernel#nodes-and-buffers) buffer indices. After a proposal it compares the new tree with the saved one. Every branch whose length or parent changed gets its new distance in a fresh buffer, and only the partials of its ancestors are recomputed. A rejected step brings back the saved tree, the parameters and the core's buffers. Moves of λ − μ and μ/λ leave the likelihood untouched.

## Checking it

`tests/sampler.rs` and the unit tests in `src/mcmc/moves.rs` check the sampler against results known in advance:

- **The prior, on five tips.** Under this prior every ranked topology is equally likely, so each topology's frequency is proportional to the number of ways to order its internal nodes. With F the distribution function of the speciation times, F(root height) follows Beta(4, 2) and the mean of F over the internal nodes is 5/12. These are checked for the height moves alone, for all tree moves together and with a monophyly constraint.
- **Each topology move on its own.** Under the birth–death prior, lowering a node always raises the density, so every downward slide is accepted whatever its Hastings term. A synthetic target whose density grows with node height, with each topology still weighted by its rankings, checks each move's Hastings ratio. Removing the counts from the subtree slide's ratio roughly triples its distance from the exact frequencies.
- **The scale move** keeps a uniform target on [0, 1], and λ − μ and μ/λ keep their uniform priors when the chain samples the prior with every move.
- **Tuning** moves each value in the right direction by the right amount, checked against a hand-computed step.
- **Bookkeeping.** On a six-taxon matrix with missing and uncertain cells, after every step of 4,000-step chains under Mkv and Mk, the cached posterior matches an evaluation from scratch, a rejected step restores the exact previous state, the constraint holds, and every move is both accepted and rejected.
- **The tree prior** matches BEAST 2.7.8's value at both archived states of the reference analysis within 10⁻¹².
- **Fixed states.** At the final state of each BEAST chain of the comparison below, Rove's likelihood and posterior match BEAST 2.7.8's within 10⁻¹⁰, and its tree prior and prior within 10⁻¹².
- **Backends.** The same chain on every backend the build includes makes the same moves and reaches the same posterior, bit for bit, at every step.

`tests/allocations.rs` checks that 20,000 steps on the reference matrix make no heap allocation. `tests/cli.rs` runs the command, checks the three files and the rejected settings, and checks that closing standard error does not stop a run.

## Comparison with BEAST

Rove and BEAST 2.7.8 sampled the reference matrix under the strict-clock model with one category, with and without the Mkv correction, using the same moves, weights and fixed step sizes and four chains each. Every posterior mean was within four combined Monte Carlo standard errors of BEAST's and every clade frequency within 0.05, the criteria fixed before the runs. Acceptance rates were not part of the criteria; with 36 Mkv chains per engine, no move's rate differed by more than 1.7 standard errors.

The matrix has no homoplasy, so under Mkv the likelihood does not penalize short trees: the posterior tree height is about 0.003 expected changes per character, the posterior of λ − μ is cut off by its prior's upper bound of 1000, and the topology barely moves. Under Mk, the reference topology has a posterior frequency of 0.82 to 0.83 in every chain of both engines. The [report](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md) gives the numbers, the run times and the commands.

## Limitations

- One partition of binary characters. Tip dates, calibrations, other clock distributions and other tree priors are not implemented.
- The starting tree is random, with coalescent-like node times, and satisfies the constraints.
- Each independent chain uses one thread. Checkpoints preserve all chains at the same sampled step.

## Independent chains

Add `--chains 2` or `--chains 4` to run independent samplers on separate threads. The CLI accepts 1–16 chains; one remains the default. Each chain receives the base seed plus its zero-based index, wrapping modulo 2⁶⁴. Its scientific samples match a separate run with that seed, on the same executable and platform. Classical proposals remain the default.

For multiple chains, `--output` contains `chain-01/`, `chain-02/`, and so on, each with `trace.log`, `trees.nexus` and `run.json`. The top-level `run.json` records all seeds, member statuses and the termination reason. `comparison.json` reports per-chain ESS, rank-normalized split R-hat including folded ranks, and clade frequencies. There is no concatenated trace or combined MCC tree.

`--burnin-percent` defaults to 10. Diagnostics use the common sample prefix, discarding that percentage of rows from each chain. `rove compare <run-directory> <run-directory> ... --burnin-percent 25` recalculates a comparison as JSON on stdout; the runs must have the same input and model, distinct seeds and matching sample states. ESS uses the existing BEAST convention separately within each chain, not pooled bulk or tail ESS. Constants, short traces and infinite R-hat are marked unavailable, with a reason. Constrained clades are identified separately from support recovered from data; constant clade indicators cannot establish exploration of tree space.

Optional `--stop-when-converged` checks **every traced scalar statistic** against `--min-ess` (200 per chain) and `--max-rhat` (1.01). It requires at least 1,000 retained samples per chain, two consecutive successful checks, and no unavailable or lag-limited ESS estimate. `--check-every` defaults to 1,000 sampling intervals and must be a positive multiple of `--sample-every`. Workers pause at the same recorded state for each check. The step limit still applies; `run.json` records the checks, actual completed steps and `diagnostic_thresholds` or `step_limit` as the reason.

Stopping is disabled by default. These scalar thresholds **do not certify topology convergence**. Review clade agreement and the traces before interpreting results. Canceling a group closes all its workers and preserves partial files. Resume uses the last committed checkpoint; pause saves the next sampling boundary before stopping.


## Checkpoints and resume

Checkpoints are written at state zero, approximately every 30 seconds by default, and at completion. `--checkpoint-every <steps>` uses a fixed step interval instead; it must be a positive multiple of `--sample-every`.

```sh
rove sample --resume /path/to/run-directory
```

Use the single run directory or the parent of an independent-chain group. Keep the original directory, input and executable. Resume restores the saved settings; changing options such as seed, model, step limit or backend is refused. A completed run cannot be resumed, and old runs without checkpoints cannot acquire them retroactively.

The checkpoint includes the tree, branch and category rates, RNG state, move tuning and counters, live likelihood buffers, and trace/tree offsets with SHA-256 fingerprints. A group commits all members together, including its diagnostic check history. Files are flushed and synchronized before an atomic manifest replacement; an interrupted save leaves the previous committed checkpoint usable. A process lock prevents simultaneous writers.

Resume verifies every member before appending. Output after the saved offsets is discarded and those steps are repeated. Float bits and sampling order are preserved, so samples match an uninterrupted run with the same executable and settings. This is restart reproducibility, not a convergence test. `run.json` remains a report; `checkpoint.json` and its referenced binary files are needed to restart.

For owners using `--cancel-on-stdin-close`, sending `pause\n` on standard input saves at the next common sampling boundary and exits with status `paused` (exit code 130). Closing standard input cancels; abrupt termination can recover only the most recent committed checkpoint.
