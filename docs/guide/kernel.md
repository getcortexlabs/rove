---
description: "What the Rove likelihood core computes today, and how to call it from Rust: tips, transition matrices, partials, the Mkv correction, rescaling and the log likelihood."
---

# The likelihood core

`LikelihoodCore` computes the log likelihood of a rooted binary tree for a matrix of site patterns, given a transition matrix per branch and per rate category. This page describes the API as it exists in the repository. There is no crate on crates.io yet; use the repository as a Git dependency to try it.

::: info Scope
The core takes transition matrices and tip data and returns likelihoods. The separate [NEXUS reader](./nexus) prepares observations and pattern weights, and the [Mkv correction](#conditioning-on-variable-characters) conditions the likelihood on variable characters. Tree proposals and gamma-rate computation remain future work. See the [roadmap](./roadmap).
:::

## Nodes and buffers

Nodes are numbered from `0` to `node_count − 1`. The core does not know which nodes are tips; it only needs `calculate_partials` to be called for each internal node after its two children are ready.

Every node owns two partials buffers and two transition-matrix buffers. Call `set_node_partials_for_update` or `set_node_matrix_for_update` before overwriting a node during a proposal, and use `store`, `restore` and `unstore` to manage which set is current. The [design](./design#double-buffers-and-rescaling) page explains the mechanism.

## Tips

`set_tip_states(node, states)` takes one state index per pattern. A value equal to or greater than the number of states means missing or ambiguous data: every state gets a partial of 1.0.

`set_tip_partials(node, partials)` takes `patterns × states` values, pattern-major, for tips with uncertainty.

Both write to the node's two buffers, so tips survive `restore`.

## Transition matrices

`set_node_matrix(node, category, matrix)` stores a `states × states` row-major matrix for the branch above `node`, in one rate category. `model::Mk` builds those matrices:

```rust
use rove::model::Mk;

let model = Mk::uniform(2);
let mut matrix = vec![0.0; 4];
model.transition_matrix(0.1, &mut matrix);
```

`Mk::uniform(k)` uses equal frequencies; `Mk::with_frequencies` takes your own. `transition_matrix(distance, out)` fills `out` for a branch of the given expected number of changes per character. Rate heterogeneity is up to the caller: compute one matrix per category with the branch length multiplied by that category's rate.

## Computing a likelihood

Four patterns of a binary character across three tips, on the tree `((A, B), C)`:

```rust
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling};

// Nodes: A = 0, B = 1, C = 2, (A, B) = 3, root = 4.
let states = 2;
let patterns = 4;
let categories = 1;
let nodes = 5;

let mut core = LikelihoodCore::new(states, patterns, categories, nodes, Scaling::Always);
core.set_tip_states(0, &[0, 0, 1, 1]);
core.set_tip_states(1, &[0, 1, 1, 0]);
core.set_tip_states(2, &[0, 1, 0, 2]); // 2 is out of range: missing data

let model = Mk::uniform(states);
let mut matrix = vec![0.0; states * states];
for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    model.transition_matrix(length, &mut matrix);
    core.set_node_matrix(node, 0, &matrix);
}

core.calculate_partials(0, 1, 3);
core.calculate_partials(3, 2, 4);

let pattern_weights = [1.0, 1.0, 1.0, 1.0];
let log_likelihood = core.log_likelihood(4, &[1.0], model.frequencies(), &pattern_weights);
assert!((log_likelihood - -8.957188856221643).abs() < 1e-12);
```

`log_likelihood` integrates the root partials over the category weights, multiplies by the state frequencies, takes the logarithm per pattern, adds any scalers and sums with the pattern weights. The steps are also available separately as `integrate_partials`, `calculate_log_likelihoods` and `pattern_log_likelihoods`.

The reference implementation gives the same value for this tree. That is what the equality tests check, on random inputs.

## Conditioning on variable characters

Morphological matrices often keep only characters that vary among the taxa. Their likelihood has to condition on that selection, as in Lewis' [Mkv model](https://doi.org/10.1080/106351501753462876). With `Pₛ` the probability that every taxon is in state `s`, mixed over the rate categories, each observed pattern `i` with likelihood `Lᵢ` and weight `wᵢ` contributes

```text
wᵢ · (ln Lᵢ − ln(1 − Σₛ Pₛ))
```

`ascertainment::VariableCharacters` appends one constant pattern per declared state to a partition's tip inputs. The core evaluates them together with the observations, on the same tree, branch matrices, category weights and scalers. They carry no observation weight. Four variable characters on the tree from the previous example:

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
let mut core = LikelihoodCore::new(2, variable.pattern_count(), 1, 5, Scaling::Always);
for (tip, partials) in variable.tip_partials().iter().enumerate() {
    core.set_tip_partials(tip, partials);
}
let model = Mk::uniform(2);
let mut matrix = vec![0.0; 4];
for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    model.transition_matrix(length, &mut matrix);
    core.set_node_matrix(node, 0, &matrix);
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

At both archived BEAST 2.7.8 states of the [reference analysis](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md), Rove reproduces each pattern log likelihood, the correction and the Mkv log likelihood. The largest difference, 4 × 10⁻¹², is in the Mkv value.

## Rescaling

Pass `Scaling::Always` to divide each internal node's partials by their per-pattern maximum as they are computed. The logarithm of each maximum is kept and added back at the end, so the result matches the unscaled computation without the underflow on deep trees. `Scaling::None` skips the work.

## Checking it

`cargo test` runs the property tests in `tests/equality.rs`. They generate random trees with 2 to 10 tips, 2 to 6 states, 1 to 4 rate categories and up to 40 patterns, and compare the core against `reference::Problem` in four combinations: tip states or tip partials, with or without rescaling. A separate test checks that `restore` brings back the likelihood computed before a proposal.

`tests/ascertainment.rs` covers the Mkv correction: the archived BEAST states, complete enumeration of the patterns on trees with two to four tips, the two-taxon closed form at distances from 10⁻⁸ to 1000, random trees against `reference::Problem::variable_log_likelihood`, characters that could be constant, restoration after a rejected proposal, and the undefined correction on a zero-length tree.
