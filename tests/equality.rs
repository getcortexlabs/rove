use proptest::collection::vec;
use proptest::prelude::*;
use rove::model::Mk;
use rove::random::Random;
use rove::reference::Problem;
use rove::{Backend, LikelihoodCore, Scaling};

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

impl Case {
    fn model(&self) -> Mk {
        Mk::with_frequencies(self.frequencies.clone()).unwrap()
    }
}

const MAX_TIPS: usize = 10;

fn normalized(v: Vec<f64>) -> Vec<f64> {
    let total: f64 = v.iter().sum();
    v.into_iter().map(|x| x / total).collect()
}

fn case() -> impl Strategy<Value = Case> {
    (2usize..=6, 1usize..=4, 1usize..=40, 2usize..=MAX_TIPS).prop_flat_map(
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

/// Branch lengths spread evenly in magnitude from 10⁻⁹ to 100, and state
/// frequencies that differ by up to six orders of magnitude.
fn extremes() -> impl Strategy<Value = Case> {
    (
        case(),
        vec(-9.0f64..2.0, 2 * MAX_TIPS - 1),
        vec(-6.0f64..0.0, 6),
    )
        .prop_map(|(mut case, lengths, weights)| {
            for (length, exponent) in case.lengths.iter_mut().zip(lengths) {
                *length = 10f64.powf(exponent);
            }
            let weights = weights.iter().take(case.states).map(|e| 10f64.powf(*e));
            case.frequencies = normalized(weights.collect());
            case
        })
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

#[derive(Clone, Copy, Debug)]
enum Path {
    Matrices,
    Mk,
}

const PATHS: [Path; 2] = [Path::Matrices, Path::Mk];

fn new_core(case: &Case, path: Path, nodes: usize, scaling: Scaling) -> LikelihoodCore {
    match path {
        Path::Matrices => {
            LikelihoodCore::new(case.states, case.patterns, case.categories, nodes, scaling)
        }
        Path::Mk => LikelihoodCore::mk(
            &case.model(),
            case.patterns,
            case.categories,
            nodes,
            scaling,
        ),
    }
}

/// Sets the branch above `node` in every rate category.
fn set_branch(core: &mut LikelihoodCore, path: Path, case: &Case, node: usize, length: f64) {
    let model = case.model();
    let k = case.states;
    let mut matrix = vec![0.0; k * k];
    for (category, &rate) in case.rates.iter().enumerate() {
        match path {
            Path::Matrices => {
                model.transition_matrix(length * rate, &mut matrix);
                core.set_node_matrix(node, category, &matrix);
            }
            Path::Mk => core.set_node_distance(node, category, length * rate),
        }
    }
}

enum Tips<'a> {
    States(&'a [Vec<u32>]),
    Partials(&'a [Vec<f64>]),
}

fn run_core(
    case: &Case,
    children: &[Option<(usize, usize)>],
    tips: Tips,
    path: Path,
    scaling: Scaling,
) -> f64 {
    let nodes = children.len();
    let root = nodes - 1;
    let mut core = new_core(case, path, nodes, scaling);
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
    for node in 0..root {
        set_branch(&mut core, path, case, node, case.lengths[node]);
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

fn reference(
    case: &Case,
    children: &[Option<(usize, usize)>],
    tips: &[Vec<f64>],
    lengths: &[f64],
) -> f64 {
    let model = case.model();
    let k = case.states;
    let matrices: Vec<Vec<f64>> = lengths
        .iter()
        .map(|&length| {
            let mut m = vec![0.0; case.categories * k * k];
            for (matrix, rate) in m.chunks_exact_mut(k * k).zip(&case.rates) {
                model.transition_matrix(length * rate, matrix);
            }
            m
        })
        .collect();
    Problem {
        children,
        root: children.len() - 1,
        tip_partials: tips,
        matrices: &matrices,
        category_weights: &case.weights,
        frequencies: &case.frequencies,
        pattern_weights: &case.pattern_weights,
    }
    .log_likelihood()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(1.0)
}

/// Every partial, log likelihood and score of a run of updates, kept and
/// rejected changes, as bits.
fn trace(
    case: &Case,
    children: &[Option<(usize, usize)>],
    parents: &[Option<usize>],
    moves: &[(usize, f64, u8)],
    path: Path,
    scaling: Scaling,
    backend: Backend,
) -> Vec<u64> {
    let nodes = children.len();
    let root = nodes - 1;
    let mut core = new_core(case, path, nodes, scaling);
    core.set_backend(backend);
    for tip in 0..case.tips {
        if tip % 2 == 0 {
            core.set_tip_states(tip, &case.tip_states[tip]);
        } else {
            core.set_tip_partials(tip, &case.tip_values[tip]);
        }
    }
    for node in 0..root {
        set_branch(&mut core, path, case, node, case.lengths[node]);
    }
    let mut bits = Vec::new();
    let mut record = |core: &mut LikelihoodCore| {
        let score = core.log_likelihood(
            root,
            &case.weights,
            &case.frequencies,
            &case.pattern_weights,
        );
        bits.push(score.to_bits());
        bits.extend(core.pattern_log_likelihoods().iter().map(|x| x.to_bits()));
        for node in case.tips..nodes {
            bits.extend(core.node_partials(node).iter().map(|x| x.to_bits()));
        }
    };
    for (parent, pair) in children.iter().enumerate().skip(case.tips) {
        let (a, b) = pair.unwrap();
        core.calculate_partials(a, b, parent);
    }
    record(&mut core);
    for &(node, exponent, outcome) in moves {
        let node = node % root;
        core.store();
        core.set_node_matrix_for_update(node);
        set_branch(&mut core, path, case, node, 10f64.powf(exponent));
        let mut child = node;
        while let Some(parent) = parents[child] {
            let (a, b) = children[parent].unwrap();
            core.set_node_partials_for_update(parent);
            core.calculate_partials(a, b, parent);
            child = parent;
        }
        record(&mut core);
        match outcome {
            0 => {}
            1 => core.restore(),
            _ => core.unstore(),
        }
        record(&mut core);
    }
    bits
}

proptest! {
    #[test]
    fn backends_agree_bit_for_bit(
        case in case(),
        moves in vec((any::<usize>(), -3.0f64..1.0, 0u8..3), 1..8),
    ) {
        let children = tree(case.tips, &case.joins);
        let mut parents = vec![None; children.len()];
        for (parent, pair) in children.iter().enumerate() {
            if let Some((a, b)) = *pair {
                parents[a] = Some(parent);
                parents[b] = Some(parent);
            }
        }
        for path in PATHS {
            for scaling in [Scaling::None, Scaling::Always] {
                let portable = trace(&case, &children, &parents, &moves, path, scaling, Backend::Portable);
                for backend in [Backend::Neon].into_iter().filter(|b| b.is_available()) {
                    let other = trace(&case, &children, &parents, &moves, path, scaling, backend);
                    prop_assert!(portable == other, "{path:?}, {scaling:?}, {backend:?}");
                }
            }
        }
    }

    #[test]
    fn tip_states_match_reference(case in case()) {
        let children = tree(case.tips, &case.joins);
        let tips: Vec<Vec<f64>> = case.tip_states.iter().map(|s| expand(s, case.states)).collect();
        let expected = reference(&case, &children, &tips, &case.lengths);
        prop_assert!(expected.is_finite());
        for path in PATHS {
            for scaling in [Scaling::None, Scaling::Always] {
                let actual = run_core(&case, &children, Tips::States(&case.tip_states), path, scaling);
                prop_assert!(close(actual, expected), "{path:?}, {scaling:?}: {actual} vs {expected}");
            }
        }
    }

    #[test]
    fn tip_partials_match_reference(case in case()) {
        let children = tree(case.tips, &case.joins);
        let expected = reference(&case, &children, &case.tip_values, &case.lengths);
        for path in PATHS {
            for scaling in [Scaling::None, Scaling::Always] {
                let actual = run_core(&case, &children, Tips::Partials(&case.tip_values), path, scaling);
                prop_assert!(close(actual, expected), "{path:?}, {scaling:?}: {actual} vs {expected}");
            }
        }
    }

    #[test]
    fn extreme_branches_and_frequencies_match_reference(case in extremes()) {
        let children = tree(case.tips, &case.joins);
        let tips: Vec<Vec<f64>> = case.tip_states.iter().map(|s| expand(s, case.states)).collect();
        let expected = reference(&case, &children, &tips, &case.lengths);
        prop_assert!(expected.is_finite());
        for path in PATHS {
            for scaling in [Scaling::None, Scaling::Always] {
                let actual = run_core(&case, &children, Tips::States(&case.tip_states), path, scaling);
                prop_assert!(close(actual, expected), "{path:?}, {scaling:?}: {actual} vs {expected}");
            }
        }
    }

    #[test]
    fn rejected_proposals_restore_the_stored_state(
        case in case(),
        moves in vec((any::<usize>(), -3.0f64..1.0, 0u8..3), 1..12),
    ) {
        let children = tree(case.tips, &case.joins);
        let nodes = children.len();
        let root = nodes - 1;
        let mut parents = vec![None; nodes];
        for (parent, pair) in children.iter().enumerate() {
            if let Some((a, b)) = *pair {
                parents[a] = Some(parent);
                parents[b] = Some(parent);
            }
        }
        let score = |core: &mut LikelihoodCore| {
            core.log_likelihood(root, &case.weights, &case.frequencies, &case.pattern_weights)
        };
        for path in PATHS {
            for scaling in [Scaling::None, Scaling::Always] {
                let mut core = new_core(&case, path, nodes, scaling);
                for (tip, values) in case.tip_values.iter().enumerate() {
                    core.set_tip_partials(tip, values);
                }
                for node in 0..root {
                    set_branch(&mut core, path, &case, node, case.lengths[node]);
                }
                for (parent, pair) in children.iter().enumerate().skip(case.tips) {
                    let (a, b) = pair.unwrap();
                    core.calculate_partials(a, b, parent);
                }
                let mut lengths = case.lengths.clone();
                // Keep the change, reject it with `restore`, or with `unstore`.
                for &(node, exponent, outcome) in &moves {
                    let before = score(&mut core);
                    let node = node % root;
                    let mut proposed = lengths.clone();
                    proposed[node] = 10f64.powf(exponent);

                    core.store();
                    core.set_node_matrix_for_update(node);
                    set_branch(&mut core, path, &case, node, proposed[node]);
                    let mut child = node;
                    while let Some(parent) = parents[child] {
                        let (a, b) = children[parent].unwrap();
                        core.set_node_partials_for_update(parent);
                        core.calculate_partials(a, b, parent);
                        child = parent;
                    }
                    let expected = reference(&case, &children, &case.tip_values, &proposed);
                    let actual = score(&mut core);
                    prop_assert!(close(actual, expected), "{path:?}, {scaling:?}: {actual} vs {expected}");

                    match outcome {
                        0 => lengths = proposed,
                        1 => core.restore(),
                        _ => core.unstore(),
                    }
                    if outcome != 0 {
                        prop_assert_eq!(score(&mut core), before);
                    }
                }
                let expected = reference(&case, &children, &case.tip_values, &lengths);
                prop_assert!(close(score(&mut core), expected));
            }
        }
    }
}

#[test]
fn a_change_on_a_short_branch_keeps_its_precision() {
    // For two taxa, P(0, 1) = π₀π₁(1 − e^(−d/β)) at total distance d.
    let model = Mk::with_frequencies(vec![0.2, 0.8]).unwrap();
    let beta = 1.0 - 0.2 * 0.2 - 0.8 * 0.8;
    for path in PATHS {
        for distance in [1e-15, 1e-12, 1e-9, 1e-6] {
            let mut core = match path {
                Path::Matrices => LikelihoodCore::new(2, 1, 1, 3, Scaling::None),
                Path::Mk => LikelihoodCore::mk(&model, 1, 1, 3, Scaling::None),
            };
            core.set_tip_states(0, &[0]);
            core.set_tip_states(1, &[1]);
            for (node, length) in [(0, 0.25 * distance), (1, 0.75 * distance)] {
                match path {
                    Path::Matrices => {
                        let mut matrix = [0.0; 4];
                        model.transition_matrix(length, &mut matrix);
                        core.set_node_matrix(node, 0, &matrix);
                    }
                    Path::Mk => core.set_node_distance(node, 0, length),
                }
            }
            core.calculate_partials(0, 1, 2);
            let actual = core.log_likelihood(2, &[1.0], model.frequencies(), &[1.0]);
            let expected = (0.2 * 0.8 * -(-distance / beta).exp_m1()).ln();
            assert!(
                (actual - expected).abs() < 1e-14,
                "{path:?}, distance {distance}: {actual} != {expected}"
            );
        }
    }
}

#[test]
#[should_panic(expected = "use set_node_distance")]
fn an_mk_core_refuses_matrices() {
    let mut core = LikelihoodCore::mk(&Mk::uniform(2), 1, 1, 3, Scaling::None);
    core.set_node_matrix(0, 0, &[1.0, 0.0, 0.0, 1.0]);
}

#[test]
#[should_panic(expected = "use set_node_matrix")]
fn a_matrix_core_refuses_distances() {
    let mut core = LikelihoodCore::new(2, 1, 1, 3, Scaling::None);
    core.set_node_distance(0, 0, 0.1);
}

#[test]
fn a_new_core_uses_the_native_backend() {
    let core = LikelihoodCore::mk(&Mk::uniform(2), 1, 1, 3, Scaling::None);
    assert_eq!(core.backend(), Backend::NATIVE);
    assert!(Backend::NATIVE.is_available() && Backend::Portable.is_available());
}

#[test]
#[cfg(target_arch = "aarch64")]
fn aarch64_runs_the_neon_kernel_by_default() {
    assert_eq!(Backend::NATIVE, Backend::Neon);
}

/// Signed zeros, subnormals, infinities and NaNs at the tips, and branches
/// from zero to infinite length. Every result that is not NaN must agree bit
/// for bit; a NaN may come out with another payload or sign.
#[test]
fn backends_agree_on_special_values() {
    let tips = [
        0.0,
        -0.0,
        1.0,
        0.5,
        5e-324,
        2.2e-308,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0123),
        f64::from_bits(0x7ff0_0000_0000_0001),
        1e300,
        1e-3,
    ];
    let distances = [0.0, 1e-300, 1e-9, 0.1, 5.0, 1e300, f64::INFINITY];
    for seed in 0..40 {
        for states in [2, 3, 7] {
            for categories in [1, 3] {
                for patterns in [1, 7, 9, 17] {
                    for scaling in [Scaling::None, Scaling::Always] {
                        let run = |backend| {
                            let mut random = Random::new(seed);
                            let nodes = 2 * (2 + seed as usize % 4) - 1;
                            let model = Mk::uniform(states);
                            let mut core =
                                LikelihoodCore::mk(&model, patterns, categories, nodes, scaling);
                            core.set_backend(backend);
                            let leaves = nodes.div_ceil(2);
                            for tip in 0..leaves {
                                let values: Vec<f64> = (0..patterns * states)
                                    .map(|_| tips[random.below(tips.len())])
                                    .collect();
                                core.set_tip_partials(tip, &values);
                            }
                            for node in 0..nodes - 1 {
                                for c in 0..categories {
                                    let d = distances[random.below(distances.len())];
                                    core.set_node_distance(node, c, d);
                                }
                            }
                            let mut values = Vec::new();
                            let mut previous = 0;
                            for (n, parent) in (leaves..nodes).enumerate() {
                                core.calculate_partials(previous, n + 1, parent);
                                previous = parent;
                                values.extend_from_slice(core.node_partials(parent));
                            }
                            core.integrate_partials(
                                nodes - 1,
                                &vec![1.0 / categories as f64; categories],
                            );
                            values.extend_from_slice(
                                core.calculate_log_likelihoods(model.frequencies()),
                            );
                            values
                        };
                        let portable = run(Backend::Portable);
                        for backend in [Backend::Neon].into_iter().filter(|b| b.is_available()) {
                            for (a, b) in portable.iter().zip(run(backend)) {
                                assert!(
                                    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()),
                                    "seed {seed}, {states} states, {categories} categories, {patterns} patterns, {scaling:?}: {a:e} != {b:e}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
#[cfg_attr(
    all(target_arch = "aarch64", target_feature = "neon"),
    ignore = "this target has NEON"
)]
#[should_panic(expected = "not available")]
fn an_unavailable_backend_is_refused() {
    let mut core = LikelihoodCore::mk(&Mk::uniform(2), 1, 1, 3, Scaling::None);
    core.set_backend(Backend::Neon);
}
