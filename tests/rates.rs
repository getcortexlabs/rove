use rove::Backend;
use rove::data::{GapTreatment, Patterns};
use rove::mcmc::{Conditioning, Move, Sampler, Settings};
use rove::nexus;
use rove::rates::{GammaRates, RateVariation, RelaxedClock};
use rove::speciation::BirthDeath;
use rove::tree::Tree;

#[rustfmt::skip]
#[path = "support/rate_model.rs"]
mod rate_model;

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "support/reference_states.rs"]
mod reference_states;

fn data(text: &str) -> Patterns {
    let matrix = nexus::parse(text).unwrap();
    matrix
        .compress(
            std::slice::from_ref(&(0..matrix.character_count())),
            GapTreatment::Missing,
        )
        .unwrap()
        .remove(0)
}

fn small() -> Patterns {
    data(
        "#NEXUS\nBEGIN DATA; DIMENSIONS NTAX=6 NCHAR=6; FORMAT SYMBOLS=\"01\" MISSING=?; MATRIX a 010110 b 011010 c 101001 d 10?101 e 110011 f {01}01101; END;",
    )
}

fn close(actual: f64, expected: f64, tolerance: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: {actual} != {expected}; difference {}",
        actual - expected
    );
}

#[test]
fn midpoint_categories_match_the_beast_discretization() {
    let mut worst = 0.0f64;
    for &(shape, expected) in rate_model::CATEGORIES {
        let gamma = GammaRates::new(shape).unwrap();
        close(gamma.rates().iter().sum(), 4.0, 1e-14, "mean one");
        assert!(gamma.rates().windows(2).all(|w| w[0] <= w[1]));
        for (&actual, expected) in gamma.rates().iter().zip(expected) {
            worst = worst.max((actual - expected).abs());
            // BEAST's AS91 quantiles stop at much lower precision. This bound
            // covers the grid, including its small-quantile early return.
            close(actual, expected, 3e-8, &format!("shape {shape}"));
        }
    }
    eprintln!("largest category difference from BEAST: {worst:e}");
}

#[test]
fn shape_one_is_the_normalized_exponential_quantile() {
    let quantiles: [f64; 4] = std::array::from_fn(|i| -(1.0 - (i as f64 + 0.5) / 4.0).ln());
    let mean = quantiles.iter().sum::<f64>() / 4.0;
    for (&actual, expected) in GammaRates::new(1.0).unwrap().rates().iter().zip(quantiles) {
        close(actual, expected / mean, 3e-14, "exponential quantiles");
    }
}

#[test]
fn clock_priors_match_beast_with_the_jacobian_and_scale_parameter() {
    for &(sigma, sigma_prior, rate, rate_prior) in rate_model::PRIORS {
        let clock = RelaxedClock::new(sigma, vec![rate; 2]).unwrap();
        close(clock.log_sigma_prior(), sigma_prior, 2e-12, "sigma prior");
        close(
            clock.log_rate_prior(),
            2.0 * rate_prior,
            2e-14 * rate_prior.abs().max(1.0),
            "rate prior",
        );
    }
}

#[test]
fn every_root_uses_each_rate_once_without_normalizing_the_mean() {
    for &(root, parameters, branches) in rate_model::MAPPINGS {
        let clock = RelaxedClock::new(0.2, parameters.to_vec()).unwrap();
        for (node, expected) in branches.into_iter().enumerate() {
            assert_eq!(clock.branch_rate(node, root), expected);
        }
    }
    let clock = RelaxedClock::new(0.2, vec![0.25, 0.5, 1.5, 2.0, 3.0, 5.0]).unwrap();
    for root in 4..=6 {
        assert_eq!(clock.branch_rate(root, root), 1.0);
        let mut rates: Vec<_> = (0..=6)
            .filter(|&n| n != root)
            .map(|n| clock.branch_rate(n, root))
            .collect();
        rates.sort_by(f64::total_cmp);
        assert_eq!(rates, clock.rates());
        if root != 6 {
            assert_eq!(clock.branch_rate(6, root), clock.rates()[root]);
        }
    }
}

#[test]
fn invalid_rate_states_are_rejected() {
    for shape in [0.0, 0.0009, 1000.1, f64::INFINITY, f64::NAN] {
        assert!(GammaRates::new(shape).is_err());
    }
    for sigma in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        assert!(RelaxedClock::new(sigma, vec![1.0; 2]).is_err());
    }
    for rate in [0.0, -1e-101, f64::INFINITY, f64::NAN] {
        assert!(RelaxedClock::new(0.2, vec![rate; 2]).is_err());
    }
    // UCRelaxedClockModel overrides the XML's 1e-100 lower bound with zero.
    assert!(RelaxedClock::new(0.2, vec![1e-200; 2]).is_ok());
    let settings = Settings {
        weights: vec![(Move::BranchRateScale, 1.0)],
        ..Settings::default()
    };
    assert!(Sampler::new(&small(), &settings, 1).is_err());
}

#[test]
fn zero_categories_remain_constant_at_overflowing_branch_distances() {
    let patterns = data("#NEXUS\nBEGIN DATA; DIMENSIONS NTAX=2 NCHAR=1; MATRIX a 0 b 1; END;");
    let tree = Tree::new(vec![None, None, Some((0, 1))], vec![0.0, 0.0, 10.0]).unwrap();
    let rates = RateVariation {
        gamma: Some(GammaRates::new(0.001).unwrap()),
        clock: Some(RelaxedClock::new(1.0, vec![1e308; 2]).unwrap()),
    };
    let sampler = Sampler::with_rates(&patterns, &Settings::reference(), tree, rates, 1).unwrap();
    // Given that the two binary tips differ, either orientation has probability 1/2.
    close(
        sampler.posterior().likelihood,
        -2.0f64.ln(),
        1e-13,
        "conditional orientation",
    );
}

#[test]
fn complete_reference_states_match_beast_component_by_component() {
    let patterns = data(include_str!("../fixtures/reference/20x50/matrix.nex"));
    for state in reference_states::STATES {
        let settings = Settings {
            monophyly: vec![(0..19).collect()],
            initial: BirthDeath {
                birth_difference: state.birth_difference,
                relative_death: state.relative_death,
            },
            ..Settings::reference()
        };
        let rates = RateVariation {
            gamma: Some(GammaRates::new(state.gamma_shape).unwrap()),
            clock: Some(RelaxedClock::new(state.clock_sigma, state.branch_rates.to_vec()).unwrap()),
        };
        let tree = Tree::new(state.children.to_vec(), state.heights.to_vec()).unwrap();
        let sampler = Sampler::with_rates(&patterns, &settings, tree, rates, 1).unwrap();
        let p = sampler.posterior();
        for (actual, expected, name) in [
            (p.tree_prior, state.birth_death, "birth-death"),
            (p.gamma_prior, state.gamma_prior, "shape prior"),
            (p.clock_rate_prior, state.clock_rate_prior, "rates prior"),
            (p.clock_sigma_prior, state.clock_sigma_prior, "sigma prior"),
            (p.prior(), state.prior, "total prior"),
        ] {
            close(actual, expected, 2e-12, name);
        }
        // Unlike the kernel tests, this computes categories from shape. Their
        // small difference from BEAST's approximate quantiles enters the likelihood.
        close(p.likelihood, state.log_likelihood, 1e-8, "likelihood");
        close(p.total(), state.posterior, 1e-8, "posterior");
        eprintln!(
            "seed {}: likelihood difference {:e}, prior difference {:e}",
            state.seed,
            p.likelihood - state.log_likelihood,
            p.prior() - state.prior
        );
    }
}

#[test]
fn rate_moves_and_root_changes_restore_all_cached_values() {
    let patterns = small();
    for conditioning in [Conditioning::None, Conditioning::Variable] {
        let mut settings = Settings {
            conditioning,
            tuning_delay: 100,
            monophyly: vec![vec![0, 1]],
            ..Settings::reference()
        };
        for (kind, weight) in &mut settings.weights {
            if *kind == Move::GammaShapeScale {
                *weight = 10.0;
            }
        }
        settings.weights.extend([
            (Move::NodeTimeRates, 10.0),
            (Move::RootTimeRates, 5.0),
            (Move::ClockSigmaRates, 10.0),
        ]);
        let mut chain = Sampler::new(&patterns, &settings, 29).unwrap();
        let mut root_changes = 0;
        for _ in 0..5000 {
            let before = (
                chain.tree().clone(),
                chain.rates().clone(),
                chain.posterior(),
            );
            let (kind, accepted) = chain.step();
            if !accepted {
                assert_eq!(chain.tree(), &before.0);
                assert_eq!(chain.rates(), &before.1);
                assert_eq!(chain.posterior(), before.2);
            }
            root_changes += usize::from(chain.tree().root() != before.0.root());
            chain.tree().validate().unwrap();
            let fresh = chain.recompute().unwrap();
            close(
                chain.posterior().likelihood,
                fresh.likelihood,
                2e-10,
                &format!("{kind:?}"),
            );
            close(
                chain.posterior().prior(),
                fresh.prior(),
                2e-12,
                "cached priors",
            );
        }
        assert!(root_changes > 0, "root remapping must be exercised");
        for kind in [
            Move::GammaShapeScale,
            Move::ClockSigmaScale,
            Move::BranchRateScale,
            Move::NodeTimeRates,
            Move::RootTimeRates,
            Move::ClockSigmaRates,
        ] {
            let operator = chain.operators().iter().find(|o| o.kind == kind).unwrap();
            assert!(
                operator.accepted > 0 && operator.rejected + operator.failed > 0,
                "{operator:?}"
            );
        }
    }
}

#[test]
fn relaxed_clock_backends_give_identical_samples() {
    let data = small();
    let mut chains: Vec<_> = Backend::ALL
        .into_iter()
        .filter(|b| b.is_available())
        .map(|backend| {
            let mut settings = Settings {
                backend,
                ..Settings::reference()
            };
            settings.weights.extend([
                (Move::NodeTimeRates, 10.0),
                (Move::RootTimeRates, 5.0),
                (Move::ClockSigmaRates, 10.0),
            ]);
            Sampler::new(&data, &settings, 21).unwrap()
        })
        .collect();
    for _ in 0..3000 {
        let outcomes: Vec<_> = chains.iter_mut().map(Sampler::step).collect();
        for (chain, outcome) in chains.iter().zip(&outcomes).skip(1) {
            assert_eq!(outcome, &outcomes[0]);
            assert_eq!(chain.tree(), chains[0].tree());
            assert_eq!(chain.rates(), chains[0].rates());
            assert_eq!(chain.posterior(), chains[0].posterior());
        }
    }
}

#[test]
fn rate_proposals_preserve_the_reference_hyperpriors() {
    let patterns = data("#NEXUS\nBEGIN DATA; DIMENSIONS NTAX=2 NCHAR=1; MATRIX a 0 b 1; END;");
    let settings = Settings {
        sample_from_prior: true,
        tuning: false,
        // Holding the tree and its parameters fixed isolates this hierarchy.
        weights: vec![
            (Move::GammaShapeScale, 1.0),
            (Move::ClockSigmaScale, 2.0),
            (Move::BranchRateScale, 6.0),
        ],
        ..Settings::reference()
    };
    let mut chain = Sampler::new(&patterns, &settings, 191).unwrap();
    let (mut shape, mut sigma, mut sigma_squared, mut z, mut z_squared, mut samples) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for step in 0..600_000 {
        chain.step();
        if step >= 10_000 && step % 10 == 0 {
            let rates = chain.rates();
            let clock = rates.clock.as_ref().unwrap();
            shape += rates.gamma.unwrap().shape();
            sigma += clock.sigma();
            sigma_squared += clock.sigma().powi(2);
            for rate in clock.rates() {
                let standardized = (rate.ln() + 0.5 * clock.sigma().powi(2)) / clock.sigma();
                z += standardized / 2.0;
                z_squared += standardized.powi(2) / 2.0;
            }
            samples += 1.0;
        }
    }
    // Shape is Exponential(1) restricted to [0.001, 1000], so its mean is
    // 1.001 up to an unrepresentably small upper-tail correction. Sigma is
    // Gamma(5, 0.2); standardized log rates are marginally standard normal.
    close(shape / samples, 1.001, 0.05, "shape mean");
    close(sigma / samples, 1.0, 0.05, "sigma mean");
    close(
        sigma_squared / samples - (sigma / samples).powi(2),
        0.2,
        0.04,
        "sigma variance",
    );
    close(z / samples, 0.0, 0.05, "standardized log rate mean");
    close(
        z_squared / samples,
        1.0,
        0.06,
        "standardized log rate square",
    );
}
