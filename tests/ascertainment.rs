use proptest::collection::vec;
use proptest::prelude::*;
use rove::ascertainment::{Error, VariableCharacters};
use rove::data::{GapTreatment, Patterns};
use rove::model::Mk;
use rove::reference::Problem;
use rove::{LikelihoodCore, Scaling, nexus};

#[rustfmt::skip]
#[path = "support/reference_states.rs"]
mod reference_states;

const REFERENCE: &str = include_str!("../fixtures/reference/20x50/matrix.nex");

#[derive(Clone, Copy, Debug)]
enum Path {
    Matrices,
    Mk,
}

const PATHS: [Path; 2] = [Path::Matrices, Path::Mk];

/// A rooted tree whose parents follow their children, with the model on it.
#[derive(Debug)]
struct Setup {
    children: Vec<Option<(usize, usize)>>,
    lengths: Vec<f64>,
    rates: Vec<f64>,
    category_weights: Vec<f64>,
    model: Mk,
}

impl Setup {
    fn reference(state: &reference_states::State) -> Self {
        Self {
            children: state.children.to_vec(),
            lengths: state.distances.to_vec(),
            rates: state.rates.to_vec(),
            category_weights: state.category_weights.to_vec(),
            model: Mk::with_frequencies(state.frequencies.to_vec()).unwrap(),
        }
    }

    fn root(&self) -> usize {
        self.children.len() - 1
    }

    /// One `states × states` matrix per category, concatenated, for each branch.
    fn matrices(&self) -> Vec<Vec<f64>> {
        let k = self.model.states();
        self.lengths
            .iter()
            .map(|&length| {
                let mut matrices = vec![0.0; self.rates.len() * k * k];
                for (matrix, rate) in matrices.chunks_exact_mut(k * k).zip(&self.rates) {
                    self.model.transition_matrix(length * rate, matrix);
                }
                matrices
            })
            .collect()
    }

    fn core(&self, variable: &VariableCharacters, scaling: Scaling, path: Path) -> LikelihoodCore {
        let (patterns, categories, nodes) = (
            variable.pattern_count(),
            self.rates.len(),
            self.children.len(),
        );
        let mut core = match path {
            Path::Matrices => {
                LikelihoodCore::new(self.model.states(), patterns, categories, nodes, scaling)
            }
            Path::Mk => LikelihoodCore::mk(&self.model, patterns, categories, nodes, scaling),
        };
        for (tip, partials) in variable.tip_partials().iter().enumerate() {
            core.set_tip_partials(tip, partials);
        }
        for node in 0..self.root() {
            self.set_branch(&mut core, path, node, self.lengths[node]);
        }
        self.prune(&mut core);
        core
    }

    /// Sets the branch above `node` in every rate category.
    fn set_branch(&self, core: &mut LikelihoodCore, path: Path, node: usize, length: f64) {
        let k = self.model.states();
        let mut matrix = vec![0.0; k * k];
        for (category, rate) in self.rates.iter().enumerate() {
            match path {
                Path::Matrices => {
                    self.model.transition_matrix(length * rate, &mut matrix);
                    core.set_node_matrix(node, category, &matrix);
                }
                Path::Mk => core.set_node_distance(node, category, length * rate),
            }
        }
    }

    fn prune(&self, core: &mut LikelihoodCore) {
        for (parent, pair) in self.children.iter().enumerate() {
            if let Some((a, b)) = *pair {
                core.calculate_partials(a, b, parent);
            }
        }
    }

    /// Every pattern's log likelihood after the category mixture and scaling.
    fn pattern_log_likelihoods(&self, core: &mut LikelihoodCore) -> Vec<f64> {
        core.integrate_partials(self.root(), &self.category_weights);
        core.calculate_log_likelihoods(self.model.frequencies())
            .to_vec()
    }
}

fn reference_patterns() -> Patterns {
    let data = nexus::parse(REFERENCE).unwrap();
    assert_eq!(data.taxa(), reference_states::TAXA);
    let sites = 0..data.character_count();
    data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)
        .unwrap()
        .remove(0)
}

fn symbols(states: impl IntoIterator<Item = usize>) -> String {
    states.into_iter().map(|state| state.to_string()).collect()
}

/// A single-state cell, `?` for every state, or an uncertain set otherwise.
fn cell(allowed: &[usize], states: usize) -> String {
    match allowed {
        [state] => state.to_string(),
        _ if allowed.len() == states => "?".into(),
        _ => format!("{{{}}}", symbols(allowed.iter().copied())),
    }
}

/// Compress character columns, given as one cell per taxon, as one partition.
fn compress(states: usize, columns: &[Vec<String>]) -> Patterns {
    let taxa = columns[0].len();
    let rows: String = (0..taxa)
        .map(|taxon| {
            let cells: String = columns.iter().map(|c| c[taxon].as_str()).collect();
            format!("t{taxon} {cells}\n")
        })
        .collect();
    let text = format!(
        "#NEXUS\nBEGIN DATA;\nDIMENSIONS NTAX={taxa} NCHAR={};\nFORMAT SYMBOLS=\"{}\" MISSING=?;\nMATRIX\n{rows};\nEND;\n",
        columns.len(),
        symbols(0..states)
    );
    let data = nexus::parse(&text).unwrap();
    let sites = 0..data.character_count();
    data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)
        .unwrap()
        .remove(0)
}

/// Every fully observed pattern in which at least two taxa differ.
fn variable_columns(taxa: usize, states: usize) -> Vec<Vec<usize>> {
    (0..states.pow(taxa as u32))
        .map(|mut code| {
            (0..taxa)
                .map(|_| {
                    let state = code % states;
                    code /= states;
                    state
                })
                .collect::<Vec<_>>()
        })
        .filter(|column| column.iter().any(|&state| state != column[0]))
        .collect()
}

fn observed(columns: &[Vec<usize>]) -> Vec<Vec<String>> {
    columns
        .iter()
        .map(|column| column.iter().map(|state| state.to_string()).collect())
        .collect()
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

fn normalized(values: Vec<f64>) -> Vec<f64> {
    let total: f64 = values.iter().sum();
    values.into_iter().map(|value| value / total).collect()
}

fn setup(
    tips: usize,
    states: usize,
    categories: usize,
    longest: f64,
) -> impl Strategy<Value = Setup> {
    (
        vec(any::<u32>(), tips - 1),
        vec(0.001f64..longest, 2 * tips - 1),
        vec(0.1f64..3.0, categories),
        vec(0.1f64..1.0, categories),
        vec(0.1f64..1.0, states),
    )
        .prop_map(move |(joins, lengths, rates, weights, frequencies)| Setup {
            children: tree(tips, &joins),
            lengths,
            rates,
            category_weights: normalized(weights),
            model: Mk::with_frequencies(normalized(frequencies)).unwrap(),
        })
}

/// Allowed states for each taxon, with missing and uncertain cells, in a
/// character that still varies: two taxa allow disjoint sets of states.
fn variable_cells(taxa: usize, states: usize) -> impl Strategy<Value = Vec<Vec<usize>>> {
    let all = (1u32 << states) - 1;
    (vec(1..=all, taxa), 1..all, any::<u32>(), any::<u32>()).prop_map(
        move |(mut masks, disjoint, other, r)| {
            let first = r as usize % taxa;
            let second = (first + 1 + (r >> 8) as usize % (taxa - 1)) % taxa;
            let rest = all & !disjoint;
            masks[first] = disjoint;
            masks[second] = if other & rest == 0 {
                rest
            } else {
                other & rest
            };
            masks
                .iter()
                .map(|mask| (0..states).filter(|s| mask & (1 << s) != 0).collect())
                .collect()
        },
    )
}

fn assert_within(actual: f64, expected: f64, tolerance: f64, context: &str) {
    assert!(
        (actual - expected).abs() < tolerance,
        "{context}: {actual} != {expected}"
    );
}

#[test]
fn archived_beast_states_are_reproduced() {
    let patterns = reference_patterns();
    let variable = VariableCharacters::new(&patterns).unwrap();
    assert_eq!(patterns.weights().len(), 23);
    assert_eq!(patterns.weights().iter().sum::<usize>(), 50);
    assert_eq!(variable.pattern_count(), 25);

    for (state, path) in reference_states::STATES
        .iter()
        .flat_map(|s| PATHS.map(|p| (s, p)))
    {
        let setup = Setup::reference(state);
        assert_eq!(state.patterns.len(), variable.pattern_count());
        for scaling in [Scaling::None, Scaling::Always] {
            let context = format!("seed {}, {path:?}, {scaling:?}", state.seed);
            let mut core = setup.core(&variable, scaling, path);
            let lls = setup.pattern_log_likelihoods(&mut core);
            for (pattern, &ll) in lls.iter().enumerate() {
                let column: Vec<u8> = variable
                    .tip_partials()
                    .iter()
                    .map(|partials| partials[pattern * 2 + 1] as u8)
                    .collect();
                let beast = state
                    .patterns
                    .iter()
                    .find(|p| p.states == column)
                    .expect("BEAST evaluated the same pattern");
                let weight = patterns.weights().get(pattern).copied().unwrap_or(0);
                assert_eq!(beast.weight, weight);
                let context = format!("{context}, pattern {pattern}");
                assert_within(ll, beast.exact_log_likelihood, 2e-14, &context);
                // BEAST 2.7.8 computes 1 − e as 1 − exp(−x), which loses digits on
                // these short branches: its own values are up to 1.3e-12 from exact.
                assert_within(ll, beast.log_likelihood, 2e-12, &context);
            }
            let log_variable = variable.log_variable_probability(&lls).unwrap();
            assert_within(
                log_variable,
                state.exact_log_variable_probability,
                1e-13,
                &context,
            );
            assert_within(
                log_variable,
                state.log_variable_probability,
                1e-12,
                &context,
            );
            // Fifty characters each subtract the amplified correction.
            let log_likelihood = variable.log_likelihood(&lls).unwrap();
            assert_within(log_likelihood, state.exact_log_likelihood, 1e-11, &context);
            assert_within(log_likelihood, state.log_likelihood, 1e-10, &context);
        }
    }
}

#[test]
fn rejected_proposals_restore_the_constant_patterns() {
    let setup = Setup::reference(&reference_states::STATES[0]);
    let variable = VariableCharacters::new(&reference_patterns()).unwrap();
    for path in PATHS {
        let mut core = setup.core(&variable, Scaling::Always, path);
        let before = variable
            .log_likelihood(&setup.pattern_log_likelihoods(&mut core))
            .unwrap();

        core.store();
        core.set_node_matrix_for_update(0);
        setup.set_branch(&mut core, path, 0, 10.0 * setup.lengths[0]);
        for (parent, pair) in setup.children.iter().enumerate() {
            if pair.is_some() {
                core.set_node_partials_for_update(parent);
            }
        }
        setup.prune(&mut core);
        let proposed = variable
            .log_likelihood(&setup.pattern_log_likelihoods(&mut core))
            .unwrap();
        assert!((proposed - before).abs() > 1e-3, "{path:?}");

        core.restore();
        let after = variable
            .log_likelihood(&setup.pattern_log_likelihoods(&mut core))
            .unwrap();
        assert_eq!(after, before, "{path:?}");
    }
}

#[test]
fn characters_that_could_be_constant_are_rejected() {
    for (cells, constant) in [
        (["1", "1", "1"], Some('1')),
        (["0", "?", "0"], Some('0')),
        (["?", "?", "?"], Some('0')),
        (["{01}", "1", "1"], Some('1')),
        (["{01}", "{12}", "?"], Some('1')),
        (["0", "{12}", "0"], None),
        (["0", "?", "1"], None),
    ] {
        let columns = [["0", "1", "0"], cells].map(|column| column.map(String::from).to_vec());
        let patterns = compress(3, &columns);
        match VariableCharacters::new(&patterns) {
            Ok(variable) => {
                assert_eq!(constant, None, "{cells:?} was accepted");
                // State 2 has a constant pattern even where it is not observed.
                assert_eq!(variable.pattern_count(), 2 + 3);
            }
            Err(error) => assert_eq!(
                Some(error),
                constant.map(|state| Error::ConstantCharacter { site: 1, state }),
                "{cells:?}"
            ),
        }
    }
}

#[test]
fn constant_characters_are_reported_by_matrix_position() {
    let data = nexus::parse(
        "#NEXUS BEGIN DATA; DIMENSIONS NTAX=3 NCHAR=3; MATRIX A 011 B 101 C 1?1; END;",
    )
    .unwrap();
    let partitions = data.compress(&[0..1, 1..3], GapTreatment::Reject).unwrap();
    assert!(VariableCharacters::new(&partitions[0]).is_ok());
    assert_eq!(
        VariableCharacters::new(&partitions[1]),
        Err(Error::ConstantCharacter {
            site: 2,
            state: '1'
        })
    );
}

#[test]
fn two_taxa_reach_the_closed_form_at_short_and_long_branches() {
    for frequencies in [
        vec![0.5, 0.5],
        vec![0.2, 0.3, 0.5],
        vec![0.1, 0.2, 0.3, 0.4],
    ] {
        let k = frequencies.len();
        let columns = variable_columns(2, k);
        let patterns = compress(k, &observed(&columns));
        let variable = VariableCharacters::new(&patterns).unwrap();
        let homozygosity: f64 = frequencies.iter().map(|f| f * f).sum();
        for (distance, tolerance) in [
            (1e-8, 1e-6),
            (1e-4, 1e-10),
            (1.0, 1e-13),
            (40.0, 1e-13),
            (1e3, 1e-13),
        ] {
            // P(i, j | variable) = πᵢπⱼ / (1 − Σ π²) for any distance and rate mixture.
            let setup = Setup {
                children: vec![None, None, Some((0, 1))],
                lengths: vec![0.25 * distance, 0.75 * distance, 0.0],
                rates: vec![0.5, 1.5],
                category_weights: vec![0.3, 0.7],
                model: Mk::with_frequencies(frequencies.clone()).unwrap(),
            };
            for path in PATHS {
                let mut core = setup.core(&variable, Scaling::Always, path);
                let lls = setup.pattern_log_likelihoods(&mut core);
                let log_variable = variable.log_variable_probability(&lls).unwrap();
                for (site, column) in columns.iter().enumerate() {
                    let expected = (frequencies[column[0]] * frequencies[column[1]]
                        / (1.0 - homozygosity))
                        .ln();
                    let actual = lls[patterns.site_to_pattern()[site]] - log_variable;
                    assert!(
                        (actual - expected).abs() < tolerance,
                        "{k} states, {path:?}, distance {distance}: {actual} != {expected}"
                    );
                }
            }
        }
    }
}

#[test]
fn zero_length_trees_leave_the_correction_undefined() {
    for frequencies in [
        vec![0.5, 0.5],
        vec![0.3, 0.7],
        vec![1.0 / 3.0; 3],
        vec![0.2, 0.3, 0.5],
        vec![0.1, 0.2, 0.3, 0.4],
    ] {
        let k = frequencies.len();
        let patterns = compress(k, &observed(&variable_columns(2, k)));
        let variable = VariableCharacters::new(&patterns).unwrap();
        let setup = Setup {
            children: vec![None, None, Some((0, 1))],
            lengths: vec![0.0; 3],
            rates: vec![1.0],
            category_weights: vec![1.0],
            model: Mk::with_frequencies(frequencies.clone()).unwrap(),
        };
        for path in PATHS {
            let mut core = setup.core(&variable, Scaling::Always, path);
            let lls = setup.pattern_log_likelihoods(&mut core);
            assert_eq!(
                variable.log_likelihood(&lls),
                Err(Error::UndefinedCorrection),
                "{frequencies:?}, {path:?}"
            );
        }
    }
}

proptest! {
    #[test]
    fn conditioned_probabilities_match_enumeration(
        (setup, partial) in (2usize..=4, 2usize..=3, 1usize..=3).prop_flat_map(
            |(taxa, states, categories)| {
                (setup(taxa, states, categories, 3.0), vec(variable_cells(taxa, states), 0..=4))
            }
        )
    ) {
        let k = setup.model.states();
        let taxa = setup.children.len().div_ceil(2);
        let complete = variable_columns(taxa, k);
        let mut columns = observed(&complete);
        columns.extend(
            partial
                .iter()
                .map(|cells| cells.iter().map(|allowed| cell(allowed, k)).collect()),
        );
        let patterns = compress(k, &columns);
        let variable = VariableCharacters::new(&patterns).unwrap();
        for path in PATHS {
            let mut core = setup.core(&variable, Scaling::Always, path);
            let lls = setup.pattern_log_likelihoods(&mut core);
            let log_variable = variable.log_variable_probability(&lls).unwrap();
            let conditioned =
                |site: usize| (lls[patterns.site_to_pattern()[site]] - log_variable).exp();

            let total: f64 = (0..complete.len()).map(conditioned).sum();
            prop_assert!((total - 1.0).abs() < 1e-10, "{path:?}: variable patterns sum to {total}");
            for (offset, cells) in partial.iter().enumerate() {
                let expected: f64 = complete
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.iter().zip(cells).all(|(state, allowed)| allowed.contains(state)))
                    .map(|(site, _)| conditioned(site))
                    .sum();
                let actual = conditioned(complete.len() + offset);
                prop_assert!(
                    (actual - expected).abs() <= 1e-10 * expected,
                    "{path:?}, {cells:?}: {actual} != {expected}"
                );
            }
        }
    }

    #[test]
    fn random_trees_match_the_reference(
        (setup, cells) in (2usize..=10, 2usize..=5, 0usize..=1, 1usize..=4).prop_flat_map(
            |(taxa, states, unused, categories)| {
                (
                    setup(taxa, states + unused, categories, 2.0),
                    vec(variable_cells(taxa, states), 1..=30),
                )
            }
        )
    ) {
        let k = setup.model.states();
        let columns: Vec<Vec<String>> = cells
            .iter()
            .map(|column| column.iter().map(|allowed| cell(allowed, k)).collect())
            .collect();
        let patterns = compress(k, &columns);
        let variable = VariableCharacters::new(&patterns).unwrap();
        let weights: Vec<f64> = patterns.weights().iter().map(|&w| w as f64).collect();
        let matrices = setup.matrices();
        let expected = Problem {
            children: &setup.children,
            root: setup.root(),
            tip_partials: patterns.tip_partials(),
            matrices: &matrices,
            category_weights: &setup.category_weights,
            frequencies: setup.model.frequencies(),
            pattern_weights: &weights,
        }
        .variable_log_likelihood();
        prop_assert!(expected.is_finite());
        for (path, scaling) in PATHS.into_iter().flat_map(|p| [(p, Scaling::None), (p, Scaling::Always)]) {
            let mut core = setup.core(&variable, scaling, path);
            let lls = setup.pattern_log_likelihoods(&mut core);
            let actual = variable.log_likelihood(&lls).unwrap();
            prop_assert!(
                (actual - expected).abs() <= 1e-9 * expected.abs().max(1.0),
                "{path:?}, {scaling:?}: {actual} != {expected}"
            );
        }
    }
}
