use rove::model::Mk;
use rove::reference::Problem;
use rove::{LikelihoodCore, Scaling};
use proptest::collection::vec;
use proptest::prelude::*;

#[derive(Clone, Debug)]
struct Case {
    states: usize,
    categories: usize,
    patterns: usize,
    tips: usize,
    joins: Vec<u32>,
    lengths: Vec<f64>,
    rates: Vec<f64>,
    weights: Vec<f64>,
    tip_states: Vec<Vec<u32>>,
    tip_values: Vec<Vec<f64>>,
    pattern_weights: Vec<f64>,
    frequencies: Vec<f64>,
}

fn normalized(v: Vec<f64>) -> Vec<f64> {
    let total: f64 = v.iter().sum();
    v.into_iter().map(|x| x / total).collect()
}

fn case() -> impl Strategy<Value = Case> {
    (2usize..=6, 1usize..=4, 1usize..=40, 2usize..=10).prop_flat_map(
        |(states, categories, patterns, tips)| {
            let nodes = 2 * tips - 1;
            (
                vec(any::<u32>(), tips - 1),
                vec(0.001f64..2.0, nodes),
                vec(0.05f64..3.0, categories),
                vec(0.1f64..1.0, categories),
                vec(vec(0u32..=states as u32, patterns), tips),
                vec(vec(0.01f64..1.0, patterns * states), tips),
                vec(1u32..5, patterns),
                vec(0.1f64..1.0, states),
            )
                .prop_map(
                    move |(
                        joins,
                        lengths,
                        rates,
                        weights,
                        tip_states,
                        tip_values,
                        pattern_weights,
                        frequencies,
                    )| Case {
                        states,
                        categories,
                        patterns,
                        tips,
                        joins,
                        lengths,
                        rates,
                        weights: normalized(weights),
                        tip_states,
                        tip_values,
                        pattern_weights: pattern_weights.into_iter().map(f64::from).collect(),
                        frequencies: normalized(frequencies),
                    },
                )
        },
    )
}

fn tree(tips: usize, joins: &[u32]) -> Vec<Option<(usize, usize)>> {
    let mut children = vec![None; 2 * tips - 1];
    let mut available: Vec<usize> = (0..tips).collect();
    for (n, &r) in joins.iter().enumerate() {
        let a = available.swap_remove(r as usize % available.len());
        let b = available.swap_remove((r >> 16) as usize % available.len());
        children[tips + n] = Some((a, b));
        available.push(tips + n);
    }
    children
}

fn matrices(case: &Case, model: &Mk) -> Vec<Vec<f64>> {
    let k = case.states;
    case.lengths
        .iter()
        .map(|&len| {
            let mut m = vec![0.0; case.categories * k * k];
            for (c, &rate) in case.rates.iter().enumerate() {
                model.transition_matrix(len * rate, &mut m[c * k * k..(c + 1) * k * k]);
            }
            m
        })
        .collect()
}

fn expand(states: &[u32], k: usize) -> Vec<f64> {
    let mut out = vec![0.0; states.len() * k];
    for (p, &s) in states.iter().enumerate() {
        if (s as usize) < k {
            out[p * k + s as usize] = 1.0;
        } else {
            out[p * k..(p + 1) * k].fill(1.0);
        }
    }
    out
}

enum Tips<'a> {
    States(&'a [Vec<u32>]),
    Partials(&'a [Vec<f64>]),
}

fn run_core(
    case: &Case,
    children: &[Option<(usize, usize)>],
    matrices: &[Vec<f64>],
    tips: Tips,
    scaling: Scaling,
) -> f64 {
    let nodes = children.len();
    let root = nodes - 1;
    let k = case.states;
    let mut core = LikelihoodCore::new(k, case.patterns, case.categories, nodes, scaling);
    match tips {
        Tips::States(all) => {
            for (tip, states) in all.iter().enumerate() {
                core.set_tip_states(tip, states);
            }
        }
        Tips::Partials(all) => {
            for (tip, partials) in all.iter().enumerate() {
                core.set_tip_partials(tip, partials);
            }
        }
    }
    for (node, m) in matrices.iter().enumerate().take(root) {
        for c in 0..case.categories {
            core.set_node_matrix(node, c, &m[c * k * k..(c + 1) * k * k]);
        }
    }
    for (parent, node) in children.iter().enumerate().skip(case.tips) {
        let (a, b) = node.unwrap();
        core.set_node_partials_for_update(parent);
        core.calculate_partials(a, b, parent);
    }
    core.log_likelihood(
        root,
        &case.weights,
        &case.frequencies,
        &case.pattern_weights,
    )
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(1.0)
}

proptest! {
    #[test]
    fn tip_states_match_reference(case in case()) {
        let children = tree(case.tips, &case.joins);
        let model = Mk::with_frequencies(case.frequencies.clone());
        let matrices = matrices(&case, &model);
        let tips: Vec<Vec<f64>> = case.tip_states.iter().map(|s| expand(s, case.states)).collect();
        let expected = Problem {
            children: &children,
            root: children.len() - 1,
            tip_partials: &tips,
            matrices: &matrices,
            category_weights: &case.weights,
            frequencies: &case.frequencies,
            pattern_weights: &case.pattern_weights,
        }
        .log_likelihood();
        let plain = run_core(&case, &children, &matrices, Tips::States(&case.tip_states), Scaling::None);
        let scaled = run_core(&case, &children, &matrices, Tips::States(&case.tip_states), Scaling::Always);
        prop_assert!(expected.is_finite());
        prop_assert!(close(plain, expected), "plain {plain} vs {expected}");
        prop_assert!(close(scaled, expected), "scaled {scaled} vs {expected}");
    }

    #[test]
    fn tip_partials_match_reference(case in case()) {
        let children = tree(case.tips, &case.joins);
        let model = Mk::with_frequencies(case.frequencies.clone());
        let matrices = matrices(&case, &model);
        let expected = Problem {
            children: &children,
            root: children.len() - 1,
            tip_partials: &case.tip_values,
            matrices: &matrices,
            category_weights: &case.weights,
            frequencies: &case.frequencies,
            pattern_weights: &case.pattern_weights,
        }
        .log_likelihood();
        let plain = run_core(&case, &children, &matrices, Tips::Partials(&case.tip_values), Scaling::None);
        let scaled = run_core(&case, &children, &matrices, Tips::Partials(&case.tip_values), Scaling::Always);
        prop_assert!(close(plain, expected), "plain {plain} vs {expected}");
        prop_assert!(close(scaled, expected), "scaled {scaled} vs {expected}");
    }
}

#[test]
fn restore_brings_back_the_stored_state() {
    let children = [None, None, None, Some((0, 1)), Some((3, 2))];
    let model = Mk::uniform(2);
    let mut core = LikelihoodCore::new(2, 4, 1, 5, Scaling::Always);
    core.set_tip_states(0, &[0, 1, 0, 1]);
    core.set_tip_states(1, &[0, 0, 1, 1]);
    core.set_tip_states(2, &[1, 1, 0, 2]);
    let mut m = [0.0; 4];
    for (node, len) in [(0, 0.1), (1, 0.2), (2, 0.3), (3, 0.4)] {
        model.transition_matrix(len, &mut m);
        core.set_node_matrix(node, 0, &m);
    }
    for (parent, node) in children.iter().enumerate().skip(3) {
        let (a, b) = node.unwrap();
        core.set_node_partials_for_update(parent);
        core.calculate_partials(a, b, parent);
    }
    let weights = [1.0; 4];
    let before = core.log_likelihood(4, &[1.0], &[0.5, 0.5], &weights);
    core.store();

    core.set_node_matrix_for_update(3);
    model.transition_matrix(1.5, &mut m);
    core.set_node_matrix(3, 0, &m);
    core.set_node_partials_for_update(4);
    core.calculate_partials(3, 2, 4);
    let proposed = core.log_likelihood(4, &[1.0], &[0.5, 0.5], &weights);
    assert_ne!(proposed, before);

    core.restore();
    let after = core.log_likelihood(4, &[1.0], &[0.5, 0.5], &weights);
    assert_eq!(after, before);
}
