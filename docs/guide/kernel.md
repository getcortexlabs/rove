---
description: "What the Rove likelihood core computes today, and how to call it from Rust: tips, transition matrices, partials, rescaling and the log likelihood."
---

# The likelihood core

`LikelihoodCore` computes the log likelihood of a rooted binary tree for a matrix of site patterns, given a transition matrix per branch and per rate category. This page describes the API as it exists in the repository. There is no crate on crates.io yet; use the repository as a Git dependency to try it.

::: info Scope
The core takes transition matrices and tip data and returns likelihoods. The separate [NEXUS reader](./nexus) prepares observations and pattern weights. Tree proposals and gamma-rate computation remain future work. See the [roadmap](./roadmap).
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

## Rescaling

Pass `Scaling::Always` to divide each internal node's partials by their per-pattern maximum as they are computed. The logarithm of each maximum is kept and added back at the end, so the result matches the unscaled computation without the underflow on deep trees. `Scaling::None` skips the work.

## Checking it

`cargo test` runs the property tests in `tests/equality.rs`. They generate random trees with 2 to 10 tips, 2 to 6 states, 1 to 4 rate categories and up to 40 patterns, and compare the core against `reference::Problem` in four combinations: tip states or tip partials, with or without rescaling. A separate test checks that `restore` brings back the likelihood computed before a proposal.
