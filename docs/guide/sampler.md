---
description: "How to run Rove's first standalone sampler on a binary morphological matrix: the model, the command, the files it writes, the proposals and the checks behind them."
---

# Sampling trees

`rove sample` runs a Markov chain Monte Carlo analysis of a binary morphological matrix without BEAST. The model is deliberately small. Gamma rate variation, the relaxed clock and the other components of the reference analysis are on the [roadmap](./roadmap).

## The model

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
| `--conditioning variable\|none` | Mkv (the default) or Mk. |
| `--sample-from-prior` | Ignore the characters and sample the prior. |
| `--no-tuning` | Keep every slide size and scale factor fixed. |
| `--tuning <move>=<value>` | Starting slide size or scale factor for a move, such as `subtree_slide=0.001`. Can be repeated. |
| `--print-every <n>` | Steps between progress lines on standard error; by default, a twentieth of the chain. |

The command stops with a message on input it does not support: characters with more than two states, partitions or model blocks in the NEXUS file, characters that could be constant under Mkv, taxon labels with characters other than letters, digits, spaces, `_`, `.` and `-` (or with both spaces and `_`), unknown taxa in a constraint, overlapping constraints that are not nested, out-of-range tuning values, and options it does not know or that are given twice.

Given the same executable, input, settings and seed on the same platform, a run produces the same samples. The logarithms and exponentials come from the platform's math library, so another platform can differ in the last digits and then diverge.

## Output files

- **`trace.log`** is tab-separated, after lines starting with `#` that record the version, the command and the SHA-256 of the input and of the executable. Its columns are `Sample`, `posterior`, `likelihood`, `prior`, `birth_death` (the log density of the tree prior), `birth_difference_rate`, `relative_death_rate`, `tree_height` and `tree_length`. The layout is that of a BEAST trace, and BEAST's LogAnalyser reads it.
- **`trees.nexus`** is a NEXUS trees block with a translate table and one tree per sample, which BEAST's NexusParser reads. Spaces in taxon labels are written as `_`. Branch lengths are in expected changes per character.
- **`run.json`** records the command's arguments, the platform, the input's path and SHA-256, the executable's SHA-256 taken when the run starts, the model, every setting, each move's weight, tuning and acceptance counts, the final posterior and the wall time.

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

`tests/allocations.rs` checks that 20,000 steps on the reference matrix make no heap allocation. `tests/cli.rs` runs the command, checks the three files and the rejected settings, and checks that closing standard error does not stop a run.

## Limitations

- One partition of binary characters. Gamma rate variation, relaxed clocks, tip dates, calibrations and other tree priors are not implemented; the components of the reference analysis are tracked in [#16](https://github.com/getcortexlabs/rove/issues/16).
- The starting tree is random, with coalescent-like node times, and satisfies the constraints.
- One chain on one thread. A run cannot be resumed from a checkpoint.
- The command does not compute convergence diagnostics. The repository's scripts in `tools/beast` compute them from its output.
