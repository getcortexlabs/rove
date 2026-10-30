use rove::ascertainment::VariableCharacters;
use rove::data::{GapTreatment, Patterns};
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::nexus;
use rove::rates::{GammaRates, RateVariation, RelaxedClock};
use rove::speciation::BirthDeath;
use rove::tree::Tree;
use rove::{LikelihoodCore, Scaling};

#[rustfmt::skip]
#[path = "support/inference_states.rs"]
mod states;

fn with_frozen_categories(patterns: &Patterns, tree: &Tree, state: &states::State) -> f64 {
    let variable = state
        .variable
        .then(|| VariableCharacters::new(patterns).unwrap());
    let tips = variable
        .as_ref()
        .map_or(patterns.tip_partials(), VariableCharacters::tip_partials);
    let mut core = LikelihoodCore::mk(
        &rove::model::Mk::uniform(2),
        tips[0].len() / 2,
        4,
        tree.nodes(),
        Scaling::Always,
    );
    for (tip, partials) in tips.iter().enumerate() {
        core.set_tip_partials(tip, partials);
    }
    for node in tree.postorder() {
        if node != tree.root() {
            for (category, rate) in state.categories.iter().enumerate() {
                core.set_node_distance(
                    node,
                    category,
                    tree.branch_length(node) * state.rates[node] * rate,
                );
            }
        }
        if let Some((a, b)) = tree.children(node) {
            core.calculate_partials(a, b, node);
        }
    }
    core.integrate_partials(tree.root(), &[0.25; 4]);
    let logs = core.calculate_log_likelihoods(&[0.5; 2]);
    match variable {
        Some(variable) => variable.log_likelihood(logs).unwrap(),
        None => logs
            .iter()
            .zip(patterns.weights())
            .map(|(ll, w)| ll * *w as f64)
            .sum(),
    }
}

#[test]
fn inference_cases_match_beast_before_comparing_chains() {
    let mut discrepancies = Vec::new();
    for state in states::STATES {
        let matrix = nexus::parse(state.matrix).unwrap();
        let patterns = matrix
            .compress(
                std::slice::from_ref(&(0..matrix.character_count())),
                GapTreatment::Reject,
            )
            .unwrap()
            .remove(0);
        let settings = Settings {
            conditioning: if state.variable {
                Conditioning::Variable
            } else {
                Conditioning::None
            },
            monophyly: vec![(0..19).collect()],
            initial: BirthDeath {
                birth_difference: state.birth_difference,
                relative_death: state.relative_death,
            },
            ..Settings::reference()
        };
        let tree = Tree::new(state.children.to_vec(), state.heights.to_vec()).unwrap();
        let frozen = with_frozen_categories(&patterns, &tree, state);
        assert!(
            (frozen - state.components[0]).abs() < 1e-9,
            "{} frozen categories",
            state.name
        );
        let rates = RateVariation {
            mean: None,
            gamma: Some(GammaRates::new(state.shape).unwrap()),
            clock: Some(RelaxedClock::new(state.sigma, state.rates.to_vec()).unwrap()),
        };
        let chain = Sampler::with_rates(&patterns, &settings, tree, rates, 17).unwrap();
        let posterior = chain.posterior();
        let components = [
            posterior.likelihood,
            posterior.tree_prior,
            posterior.gamma_prior,
            posterior.clock_rate_prior,
            posterior.clock_sigma_prior,
            posterior.prior(),
            posterior.total(),
        ];
        for (index, (&actual, &expected)) in components.iter().zip(&state.components).enumerate() {
            // The independent quantile inversions differ by up to 1e-8 in
            // category rates here; the frozen-category check isolates pruning.
            let tolerance = if index == 0 || index == 6 {
                2e-7
            } else {
                1e-10
            };
            if (actual - expected).abs() > tolerance {
                discrepancies.push(format!(
                    "{} component {index}: {actual} != {expected}",
                    state.name
                ));
            }
        }
        let category_difference = GammaRates::new(state.shape)
            .unwrap()
            .rates()
            .iter()
            .zip(state.categories)
            .map(|(r, b)| (r - b).abs())
            .fold(0.0, f64::max);
        assert!(
            category_difference < 3e-8,
            "{} gamma categories",
            state.name
        );
        eprintln!(
            "{} likelihood difference: {:e}; frozen: {:e}; categories: {category_difference:e}",
            state.name,
            components[0] - state.components[0],
            frozen - state.components[0]
        );
    }
    assert!(discrepancies.is_empty(), "{}", discrepancies.join("\n"));
}
