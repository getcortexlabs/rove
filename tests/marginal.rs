use rove::data::{GapTreatment, Patterns};
use rove::marginal::{Analysis, PowerSample, analyse, schedule};
use rove::mcmc::{Conditioning, Move, Sampler, Settings};
use rove::nexus;
use rove::random::Random;
use rove::tree::Tree;

/// Prior N(0, 1) and log-likelihood −τθ²/2: the power posterior at β is
/// N(0, 1 / (1 + βτ)) and log Z = −½ ln(1 + τ).
fn gaussian_samples(tau: f64, powers: &[f64], draws: usize, seed: u64) -> Vec<PowerSample> {
    let mut random = Random::new(seed);
    powers
        .iter()
        .map(|&power| {
            let sd = 1.0 / (1.0 + power * tau).sqrt();
            PowerSample {
                power,
                log_likelihoods: (0..draws)
                    .map(|_| {
                        let theta = sd * random.normal();
                        -tau * theta * theta / 2.0
                    })
                    .collect(),
            }
        })
        .collect()
}

/// The trapezoidal rule applied to the exact mean log-likelihood,
/// `E_β[ℓ] = −τ / (2 (1 + βτ))`: what path sampling estimates, bias included.
fn trapezoid_expectation(tau: f64, powers: &[f64]) -> f64 {
    let n = powers.len();
    (0..n)
        .map(|k| {
            let before = if k == 0 {
                0.0
            } else {
                powers[k] - powers[k - 1]
            };
            let after = if k + 1 == n {
                0.0
            } else {
                powers[k + 1] - powers[k]
            };
            (before + after) / 2.0 * (-tau / (2.0 * (1.0 + powers[k] * tau)))
        })
        .sum()
}

#[test]
fn estimators_recover_a_gaussian_evidence_with_calibrated_errors() {
    let tau = 9.0;
    let expected = -0.5 * (1.0_f64 + tau).ln();
    let powers = schedule(24, 0.3).unwrap();
    let trapezoid = trapezoid_expectation(tau, &powers);
    assert!(
        (trapezoid - expected).abs() < 0.01,
        "{trapezoid} {expected}"
    );
    let mut within_two = [0, 0];
    for seed in 0..40 {
        let analysis = analyse(&gaussian_samples(tau, &powers, 4000, 100 + seed)).unwrap();
        for (slot, (estimate, target)) in [
            (&analysis.stepping_stone, expected),
            (&analysis.path_sampling, trapezoid),
        ]
        .into_iter()
        .enumerate()
        {
            let error = estimate.standard_error.unwrap();
            assert!(error > 0.0 && error < 0.05, "{estimate:?}");
            let z = (estimate.log_marginal_likelihood - target) / error;
            assert!(z.abs() < 5.0, "seed {seed}: {estimate:?} against {target}");
            if z.abs() < 2.0 {
                within_two[slot] += 1;
            }
        }
        if seed == 0 {
            assert_eq!(analysis.powers.len(), 24);
            assert_eq!(analysis.stones.len(), 23);
            assert!(analysis.powers.iter().all(|p| p.samples == 4000));
            assert!(analysis.stones.iter().all(|s| s.variance.unwrap() > 0.0));
        }
    }
    // Normal errors put about 95% of estimates within two standard errors.
    assert!(within_two.iter().all(|&n| n >= 32), "{within_two:?}");
}

fn three_taxa() -> Patterns {
    let text = "#NEXUS\nBEGIN DATA;\nDIMENSIONS NTAX=3 NCHAR=7;\nFORMAT SYMBOLS=\"01\";\nMATRIX\nA 0011010\nB 0011101\nC 1100110\n;\nEND;\n";
    let data = nexus::parse(text).unwrap();
    data.compress(std::slice::from_ref(&(0..7)), GapTreatment::Reject)
        .unwrap()
        .remove(0)
}

/// ((A, B), C) with the root fixed at height one and the inner node at `inner`.
fn fixed_tree(inner: f64) -> Tree {
    Tree::new(
        vec![None, None, None, Some((0, 1)), Some((3, 2))],
        vec![0.0, 0.0, 0.0, inner, 1.0],
    )
    .unwrap()
}

/// Only the inner height moves, so the posterior is one-dimensional and the
/// evidence is a quadrature over that height.
fn one_height_settings() -> Settings {
    Settings {
        conditioning: Conditioning::None,
        monophyly: vec![vec![0, 1]],
        weights: vec![(Move::NodeHeight, 1.0)],
        tuning: false,
        ..Settings::default()
    }
}

fn log_sum_exp(values: &[f64]) -> f64 {
    let largest = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    largest + values.iter().map(|v| (v - largest).exp()).sum::<f64>().ln()
}

/// log ∫ L(h) p(h) dh − log ∫ p(h) dh over the inner height, by the midpoint
/// rule on the engine's own fixed-state densities.
fn quadrature_log_evidence(patterns: &Patterns, settings: &Settings, cells: usize) -> f64 {
    let mut joint = Vec::with_capacity(cells);
    let mut prior = Vec::with_capacity(cells);
    for cell in 0..cells {
        let inner = (cell as f64 + 0.5) / cells as f64;
        let state = Sampler::with_tree(patterns, settings, fixed_tree(inner), 1).unwrap();
        let posterior = state.posterior();
        joint.push(posterior.likelihood + posterior.prior());
        prior.push(posterior.prior());
    }
    log_sum_exp(&joint) - log_sum_exp(&prior)
}

fn power_chain_analysis(patterns: &Patterns, settings: &Settings, seed: u64) -> Analysis {
    let powers = schedule(16, 0.3).unwrap();
    let mut chain = Sampler::with_tree(patterns, settings, fixed_tree(0.5), seed).unwrap();
    let mut samples: Vec<PowerSample> = powers
        .iter()
        .rev()
        .map(|&power| {
            chain.set_likelihood_power(power).unwrap();
            for _ in 0..2000 {
                chain.step();
            }
            let log_likelihoods = (0..20_000)
                .map(|_| {
                    for _ in 0..5 {
                        chain.step();
                    }
                    chain.posterior().likelihood
                })
                .collect();
            PowerSample {
                power,
                log_likelihoods,
            }
        })
        .collect();
    samples.reverse();
    analyse(&samples).unwrap()
}

#[test]
fn power_posteriors_recover_the_quadrature_evidence_of_a_three_taxon_tree() {
    let patterns = three_taxa();
    let settings = one_height_settings();
    let coarse = quadrature_log_evidence(&patterns, &settings, 2000);
    let fine = quadrature_log_evidence(&patterns, &settings, 8000);
    assert!((coarse - fine).abs() < 1e-6, "{coarse} {fine}");
    let analysis = power_chain_analysis(&patterns, &settings, 24);
    let stepping_stone = &analysis.stepping_stone;
    let error = stepping_stone.standard_error.unwrap();
    assert!(error > 0.0 && error < 0.02, "{stepping_stone:?}");
    assert!(
        (stepping_stone.log_marginal_likelihood - fine).abs() < 4.0 * error,
        "{stepping_stone:?} against {fine}"
    );
    let path_sampling = &analysis.path_sampling;
    assert!(
        (path_sampling.log_marginal_likelihood - fine).abs()
            < 4.0 * path_sampling.standard_error.unwrap() + 0.05,
        "{path_sampling:?} against {fine}"
    );
    assert!(analysis.powers.iter().all(|p| p.ess.unwrap() > 1000.0));
}
