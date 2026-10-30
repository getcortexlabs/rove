---
description: "What the Rove likelihood core computes today, and how to call it from Rust: tips, Mk branch distances, transition matrices, partials, the Mkv correction, rescaling and the log likelihood."
---

# The likelihood core

`LikelihoodCore` computes the log likelihood of a rooted binary tree for a matrix of site patterns. Each branch is given per rate category, either as a distance under the Mk model or as a transition matrix. This page describes the API as it exists in the repository. There is no crate on crates.io yet; use the repository as a Git dependency to try it.

::: info Scope
The core takes branch distances or transition matrices and tip data, and returns likelihoods. The separate [NEXUS reader](./nexus) prepares observations and pattern weights, and the [Mkv correction](#conditioning-on-variable-characters) conditions the likelihood on variable characters. The [reference model](./reference-model) generates four gamma rate categories; tree proposals are part of the [sampler](./sampler). See the [roadmap](./roadmap).
:::

## Nodes and buffers

Nodes are numbered from `0` to `node_count − 1`. The core does not know which nodes are tips; it only needs `calculate_partials` to be called for each internal node after its two children are ready.

Every node owns two partials buffers and two buffers for its branch. Call `set_node_partials_for_update` or `set_node_matrix_for_update` before overwriting a node during a proposal, and use `store`, `restore` and `unstore` to manage which set is current. `set_node_matrix_for_update` switches the branch buffer in both kinds of core, whether it holds distances or matrices. The [design](./design#double-buffers-and-rescaling) page explains the mechanism.

## Tips

`set_tip_states(node, states)` takes one state index per pattern. A value equal to or greater than the number of states means missing or ambiguous data: every state gets a partial of 1.0.

`set_tip_partials(node, partials)` takes `patterns × states` values, pattern-major, for tips with uncertainty.

Both write to the node's two buffers, so tips survive `restore`.

## Mk branches

`LikelihoodCore::mk(&model, patterns, categories, nodes, scaling)` builds a core for an `Mk` model. `set_node_distance(node, category, distance)` sets the branch above `node`, in one rate category, to a distance in expected changes per character. Rate heterogeneity is up to the caller: pass the branch length multiplied by that category's rate.

The core never builds a transition matrix. Mk's matrix is `P = e·I + (1 − e)·1πᵀ`, with `e = exp(−d/β)` and `β = 1 − Σᵢ πᵢ²`, the normalization that makes one change expected per unit of distance. For each branch the core stores `e` and `1 − e`, and updates a child's partials `L` with

```text
(P·L)ᵢ = e·Lᵢ + (1 − e)·Σⱼ πⱼ·Lⱼ
```

The weighted sum is computed once per pattern and shared by every state, so a child update costs O(k) per pattern and rate category instead of the O(k²) of a matrix product. `1 − e` comes from `f64::exp_m1`, which keeps its precision on short branches.

`Mk::uniform(k)` uses equal frequencies. `Mk::with_frequencies` takes unequal ones: the same form holds for any equal-input model, in which a change draws the new state from `π`. It returns a `model::Error` unless there are at least two frequencies, each positive and finite, that sum to one up to rounding error. They are used as given, without normalizing, so values rounded to fewer digits, such as three frequencies of 0.3333333, are rejected: divide them by their sum first. `log_likelihood` still takes the root frequencies, as with a matrix core; pass `model.frequencies()`.

An Mk core panics on `set_node_matrix` and `node_matrix`; a matrix core panics on `set_node_distance` and `node_distance`. `set_node_distance` and `transition_matrix` also panic on a negative or NaN distance.

## Transition matrices

`LikelihoodCore::new(states, patterns, categories, nodes, scaling)` builds a core that takes matrices instead. `set_node_matrix(node, category, matrix)` stores a `states × states` row-major matrix for the branch above `node`, in one rate category, and the core multiplies it in full. This is the path for models other than Mk. `model::Mk` can also build its own matrices:

```rust
use rove::model::Mk;

let model = Mk::uniform(2);
let mut matrix = vec![0.0; 4];
model.transition_matrix(0.1, &mut matrix);
```

`transition_matrix(distance, out)` fills `out` for a branch of the given expected number of changes per character, with the same `e` and `1 − e` as an Mk core.

## Computing a likelihood

Four patterns of a binary character across three tips, on the tree `((A, B), C)`:

```rust
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling};

// Nodes: A = 0, B = 1, C = 2, (A, B) = 3, root = 4.
let patterns = 4;
let categories = 1;
let nodes = 5;

let model = Mk::uniform(2);
let mut core = LikelihoodCore::mk(&model, patterns, categories, nodes, Scaling::Always);
core.set_tip_states(0, &[0, 0, 1, 1]);
core.set_tip_states(1, &[0, 1, 1, 0]);
core.set_tip_states(2, &[0, 1, 0, 2]); // 2 is out of range: missing data

for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    core.set_node_distance(node, 0, length);
}

core.calculate_partials(0, 1, 3);
core.calculate_partials(3, 2, 4);

let pattern_weights = [1.0, 1.0, 1.0, 1.0];
let log_likelihood = core.log_likelihood(4, &[1.0], model.frequencies(), &pattern_weights);
assert!((log_likelihood - -8.957188856221643).abs() < 1e-12);
```

A matrix core, `LikelihoodCore::new(2, patterns, categories, nodes, Scaling::Always)`, given the matrices from `model.transition_matrix`, returns the same value up to rounding.

`log_likelihood` integrates the root partials over the category weights, multiplies by the state frequencies, takes the logarithm per pattern, adds any scalers and sums with the pattern weights. The steps are also available separately as `integrate_partials`, `calculate_log_likelihoods` and `pattern_log_likelihoods`.

The reference implementation gives the same value for this tree. That is what the equality tests check, on random inputs.

## Conditioning on variable characters

Morphological matrices often keep only characters that vary among the taxa. Their likelihood has to condition on that selection, as in Lewis' [Mkv model](https://doi.org/10.1080/106351501753462876). With `Pₛ` the probability that every taxon is in state `s`, mixed over the rate categories, each observed pattern `i` with likelihood `Lᵢ` and weight `wᵢ` contributes

```text
wᵢ · (ln Lᵢ − ln(1 − Σₛ Pₛ))
```

`ascertainment::VariableCharacters` appends one constant pattern per declared state to a partition's tip inputs. The core evaluates them together with the observations, on the same tree, branches, category weights and scalers. They carry no observation weight. Four variable characters on the tree from the previous example:

```rust
use rove::ascertainment::VariableCharacters;
use rove::data::GapTreatment;
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling, nexus};

let input = "#NEXUS
BEGIN DATA;
  DIMENSIONS NTAX=3 NCHAR=4;
  FORMAT SYMBOLS=\"01\" MISSING=?;
  MATRIX
    A 0011
    B 0110
    C 1?01
  ;
END;";
let data = nexus::parse(input)?;
let sites = 0..data.character_count();
let patterns = data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)?.remove(0);
let variable = VariableCharacters::new(&patterns)?;

// Nodes as before. Four observed patterns, then the two constant ones.
let model = Mk::uniform(2);
let mut core = LikelihoodCore::mk(&model, variable.pattern_count(), 1, 5, Scaling::Always);
for (tip, partials) in variable.tip_partials().iter().enumerate() {
    core.set_tip_partials(tip, partials);
}
for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    core.set_node_distance(node, 0, length);
}
core.calculate_partials(0, 1, 3);
core.calculate_partials(3, 2, 4);

core.integrate_partials(4, &[1.0]);
let pattern_log_likelihoods = core.calculate_log_likelihoods(model.frequencies());
let log_likelihood = variable.log_likelihood(pattern_log_likelihoods)?;
assert!((log_likelihood - -6.353478344891801).abs() < 1e-12);
```

`log_variable_probability` returns the correction `ln(1 − Σₛ Pₛ)` on its own.

`VariableCharacters::new` rejects a character that could be constant: one in which every cell, missing and uncertain cells included, allows the same state. `0 ? 0` and `{01} 1 1` produce an error naming the character. `0 {12} 0` is accepted, because it varies whichever state the uncertain cell holds. Every completion of an accepted character is variable, so the result is the likelihood of the observations given that the complete character varies. BEAST 2.7.8 treats observed columns identical to its constant columns differently: it warns and gives them zero weight.

Every declared state has its own constant pattern, including states no taxon shows. Partitions are conditioned separately. Conditioning on parsimony-informative characters and per-character corrections for missing data are not implemented.

The constant patterns go through the same pruning as the observations. When they hold almost all the probability, as on very short trees, their rounding errors are amplified by `Σₛ Pₛ / (1 − Σₛ Pₛ)`, which exceeds 100 at both archived reference states. With two taxa at distance `d`, the error of each conditioned log probability grows roughly as `10⁻¹⁶ / d`. If the total probability of the constant patterns is within a few rounding errors of one, the result is `ascertainment::Error::UndefinedCorrection` rather than a number.

At both archived BEAST 2.7.8 states of the [reference analysis](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md), Rove reproduces each pattern log likelihood within 1.4 × 10⁻¹² and the Mkv log likelihood within 2.2 × 10⁻¹². Almost all of the per-pattern difference is BEAST's own rounding: its LewisMK model computes `1 − e` as `1 − exp(−x)`, which loses digits on the reference's short branches. Against an evaluation of the same inputs with 60 significant digits, Rove's pattern log likelihoods are within 6 × 10⁻¹⁵ of the exact values, and BEAST's within 1.4 × 10⁻¹².

## Rescaling

Pass `Scaling::Always` to rescale after every internal-node update. For a finite positive per-pattern maximum `m` over all states and categories, choose `e = clamp(floor(log2(m)), -1023, 1022)` from its IEEE 754 exponent bits and multiply every partial by `2^-e`. The lower power is used, rather than rounding to the nearest power. The clamp keeps the multiplier finite and normal, including for subnormal maxima. A zero or nonfinite maximum uses `e = 0`; impossible patterns still return negative infinity.

Each buffer stores the cumulative subtree exponent as `i64`: `E = e + E_left + E_right`. After integrating the root over categories and states, the pattern log likelihood is `ln(L_scaled) + E_root * ln(2)`. This avoids per-node logarithms and repeated sums over all nodes. Integer totals cannot accumulate floating-point drift, and store/restore selects the matching partials and exponents together. `Scaling::None` skips rescaling.

The rounding differs from the previous maximum-based scaling; the statistical model is unchanged. Portable and NEON agree bit for bit except for NaN payloads. Checkpoints encode the new exponents and retain the existing requirement to resume with the exact executable that created them.

Tests cover all binary exponent boundaries and 36 independent 60-digit cases, including 2, 3, 4 and 8 states, one or four categories, missing data, distances from `1e-9` to `100`, and balanced and ladder trees with 2,048 tips whose unscaled likelihoods underflow. See the [measurements](https://github.com/getcortexlabs/rove/blob/main/benchmarks/scaling/2026-10-30/README.md).

## Backends

A core runs one of two backends. Both give the same results: every value that is not NaN agrees bit for bit, while a NaN may come out with another payload or sign. `Backend::Portable` is plain Rust for any target; in release builds the compiler vectorises its loops one at a time. `Backend::Neon`, on aarch64, runs the Mk node update as one pass over blocks of eight patterns: the weighted sums stay in registers and, when the core rescales, so does each pattern's running maximum within a rate category. Per pattern it performs the same floating-point operations in the same order as the portable code, without fused multiply–add. Matrix multiplication and the root computation use the same code under both backends. NEON also extracts the binary scaling exponents with vector integer operations.

A new core uses `Backend::NATIVE`: NEON where the target enables it, the portable code elsewhere. `backend` reports the current one and `set_backend` changes it; asking for a backend that the build does not include panics, and `Backend::is_available` tells which ones it does.

## Checking it

`cargo test` runs the property tests in `tests/equality.rs`. They generate random trees with 2 to 10 tips, 2 to 6 states, 1 to 4 rate categories and up to 40 patterns, and compare both kinds of core against `reference::Problem`, with tip states or tip partials and with or without rescaling. One of them spreads branch lengths from 10⁻⁹ to 100. Another applies random sequences of branch changes, accepts or rejects each one and checks the likelihood after every step; a rejected change must bring back the previous value exactly. A two-taxon test checks the probability of a change at distances between the taxa as short as 10⁻¹⁵, and unit tests in `src/model.rs` check frequency validation and the rate normalization. Another property test runs the same updates, kept changes and rejected changes on every backend the build includes, and requires the same bits in every partial, pattern log likelihood and total log likelihood. A further test feeds signed zeros, subnormals, infinities and NaNs through both backends and accepts a difference only between two NaNs.

`tests/ascertainment.rs` covers the Mkv correction: the archived BEAST states, against BEAST's values and the 60-digit evaluation, complete enumeration of the patterns on trees with two to four tips, the two-taxon closed form at distances from 10⁻⁸ to 1000, random trees against `reference::Problem::variable_log_likelihood`, characters that could be constant, restoration after a rejected proposal, and the undefined correction on a zero-length tree.
