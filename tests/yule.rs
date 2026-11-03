use rove::data::{GapTreatment, Patterns};
use rove::mcmc::{Move, Sampler, Settings};
use rove::nexus;
use rove::random::Random;
use rove::speciation::{BirthDeath, TreePrior, yule_log_density};
use rove::tree::Tree;
use std::collections::BTreeMap;

fn ladder(tips: usize, step: f64) -> Tree {
    let mut children = vec![None; tips];
    let mut heights = vec![0.0; tips];
    for i in 0..tips - 1 {
        children.push(Some((if i == 0 { 0 } else { tips + i - 1 }, i + 1)));
        heights.push((i + 1) as f64 * step);
    }
    Tree::new(children, heights).unwrap()
}

#[test]
fn pinned_beast_densities_zero_extinction_and_labeled_constants() {
    let table = include_str!("../fixtures/validation/yule/beast-2.7.8.tsv");
    assert_eq!(table.lines().count(), 25);
    for row in table.lines().skip(1) {
        let values: Vec<f64> = row.split('\t').map(|s| s.parse().unwrap()).collect();
        let n = values[0] as usize;
        let tree = ladder(n, values[2]);
        let actual = yule_log_density(&tree, values[1]);
        let bd = BirthDeath {
            birth_difference: values[1],
            relative_death: 0.0,
        };
        assert!((actual - values[3]).abs() < 1e-10, "{row}");
        assert!((actual - values[4]).abs() < 1e-10);
        assert!((actual - bd.log_density(&tree)).abs() < 1e-10);
        // A backwards simulation draws Exp(k*lambda) waits and one of k(k-1)/2
        // pairs at each event. Their density contributes 2/(k-1) per event.
        let log_coefficient: f64 = (2..=n).map(|k| (2.0 / (k - 1) as f64).ln()).sum();
        assert!((actual + log_coefficient - values[5]).abs() < 1e-10);
    }
    for rate in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(yule_log_density(&ladder(3, 1.0), rate), f64::NEG_INFINITY);
    }
    let dated = Tree::new(vec![None, None, Some((0, 1))], vec![0.0, 0.2, 1.0]).unwrap();
    assert_eq!(yule_log_density(&dated, 1.0), f64::NEG_INFINITY);
}

fn patterns() -> Patterns {
    let matrix =
        nexus::parse("#NEXUS BEGIN DATA; DIMENSIONS NTAX=4 NCHAR=1; MATRIX a 0 b 0 c 1 d 1; END;")
            .unwrap();
    matrix
        .compress(std::slice::from_ref(&(0..1)), GapTreatment::Reject)
        .unwrap()
        .remove(0)
}

fn clades(tree: &Tree) -> Vec<u32> {
    let mut masks = vec![0; tree.nodes()];
    for n in tree.postorder() {
        masks[n] = tree
            .children(n)
            .map_or(1 << n, |(a, b)| masks[a] | masks[b]);
    }
    let mut clades = masks[tree.tips()..].to_vec();
    clades.sort_unstable();
    clades
}

/// Independent generative prior; no engine density or proposal is used.
fn prior_tree(random: &mut Random, tips: usize, rate: f64) -> Tree {
    let mut active: Vec<_> = (0..tips).collect();
    let mut children = vec![None; tips];
    let mut heights = vec![0.0; tips];
    let mut height = 0.0;
    while active.len() > 1 {
        height += random.exponential() / (active.len() as f64 * rate);
        let a = active.swap_remove(random.below(active.len()));
        let b = active.swap_remove(random.below(active.len()));
        active.push(children.len());
        children.push(Some((a, b)));
        heights.push(height);
    }
    Tree::new(children, heights).unwrap()
}

fn scaled_statistics(tree: &Tree, rate: f64) -> [f64; 4] {
    let mut heights: Vec<_> = (tree.tips()..tree.nodes())
        .map(|n| tree.height(n))
        .collect();
    heights.sort_by(f64::total_cmp);
    [
        4.0 * rate * heights[0],
        3.0 * rate * (heights[1] - heights[0]),
        2.0 * rate * (heights[2] - heights[1]),
        rate * heights[2],
    ]
}

#[test]
fn prior_sampling_matches_independent_waits_heights_and_topologies_with_constraints() {
    for constrained in [false, true] {
        let mut settings = Settings {
            sample_from_prior: true,
            birth_difference_upper: 3.0,
            ..Settings::default()
        };
        settings.set_tree_prior(TreePrior::Yule);
        if constrained {
            settings.monophyly = vec![vec![0, 1]];
        }
        let mut random = Random::new(71);
        let mut expected = BTreeMap::<Vec<u32>, usize>::new();
        let mut independent = [0.0; 4];
        let mut draws = 0;
        let mut attempts = 0;
        while draws < 20_000 {
            attempts += 1;
            let tree = prior_tree(&mut random, 4, 1.3);
            let key = clades(&tree);
            if constrained && !key.contains(&3) {
                continue;
            }
            *expected.entry(key).or_default() += 1;
            for (total, value) in independent.iter_mut().zip(scaled_statistics(&tree, 1.3)) {
                *total += value;
            }
            draws += 1;
        }
        if constrained {
            assert!((20_000.0 / attempts as f64 - 2.0 / 9.0).abs() < 0.01);
        }
        let mut sampler = Sampler::new(patterns(), &settings, 83).unwrap();
        assert!(sampler.birth_death().is_none());
        let mut observed = BTreeMap::<Vec<u32>, usize>::new();
        let mut totals = [0.0; 4];
        let mut rate_sum = 0.0;
        for step in 0..330_000 {
            sampler.step();
            if step < 30_000 || step % 10 != 0 {
                continue;
            }
            let key = clades(sampler.tree());
            if constrained {
                assert!(key.contains(&3));
            }
            *observed.entry(key).or_default() += 1;
            let rate = sampler.speciation_rate();
            assert!((0.0..3.0).contains(&rate));
            rate_sum += rate;
            for (total, value) in totals
                .iter_mut()
                .zip(scaled_statistics(sampler.tree(), rate))
            {
                *total += value;
            }
        }
        assert!((rate_sum / 30_000.0 - 1.5).abs() < 0.10, "{rate_sum}");
        for (i, target) in [1.0, 1.0, 1.0, 13.0 / 12.0].into_iter().enumerate() {
            assert!((independent[i] / 20_000.0 - target).abs() < 0.025);
            assert!(
                (totals[i] / 30_000.0 - target).abs() < 0.08,
                "{constrained}: {totals:?}"
            );
        }
        assert_eq!(observed.len(), expected.len());
        for (topology, count) in expected {
            assert!(
                (observed[&topology] as f64 / 30_000.0 - count as f64 / 20_000.0).abs() < 0.035
            );
        }
    }
}

#[test]
fn extinction_proposals_and_inert_tuning_are_rejected_for_yule() {
    for movement in [Move::BirthDifferenceScale, Move::RelativeDeathScale] {
        let mut settings = Settings::default();
        settings.set_tree_prior(TreePrior::Yule);
        settings.tuning_values = vec![(movement, 0.5)];
        assert!(Sampler::new(patterns(), &settings, 1).is_err());
        settings.tuning_values.clear();
        settings.weights.push((movement, 1.0));
        assert!(Sampler::new(patterns(), &settings, 1).is_err());
    }
    let settings = Settings {
        tuning_values: vec![(Move::BirthRateScale, 0.5)],
        ..Settings::default()
    };
    assert!(Sampler::new(patterns(), &settings, 1).is_err());
}
