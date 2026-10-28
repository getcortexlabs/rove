use std::collections::HashMap;

use rove::data::{GapTreatment, Patterns};
use rove::mcmc::{Conditioning, Move, Sampler, Settings};
use rove::nexus;
use rove::speciation::BirthDeath;
use rove::tree::Tree;

#[rustfmt::skip]
#[allow(dead_code)] // Each test crate reads a different part of the fixture.
#[path = "support/reference_states.rs"]
mod reference_states;

fn patterns(rows: &[&str]) -> Patterns {
    let matrix: String = rows
        .iter()
        .enumerate()
        .map(|(taxon, row)| format!("t{taxon} {row}\n"))
        .collect();
    let text = format!(
        "#NEXUS\nBEGIN DATA;\nDIMENSIONS NTAX={} NCHAR={};\nFORMAT SYMBOLS=\"01\" MISSING=?;\nMATRIX\n{matrix};\nEND;\n",
        rows.len(),
        rows[0].len()
    );
    let data = nexus::parse(&text).unwrap();
    let sites = 0..data.character_count();
    data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)
        .unwrap()
        .remove(0)
}

/// Five taxa and three variable characters; the data are ignored when
/// sampling the prior.
fn five_taxa() -> Patterns {
    patterns(&["010", "011", "101", "100", "110"])
}

fn prior_settings(birth_death: BirthDeath, weights: &[(Move, f64)]) -> Settings {
    Settings {
        sample_from_prior: true,
        initial: birth_death,
        weights: weights.to_vec(),
        tuning: false,
        ..Settings::default()
    }
}

const HEIGHT_MOVES: [(Move, f64); 3] = [
    (Move::NodeHeight, 2.0),
    (Move::RootScale, 1.0),
    (Move::TreeScale, 1.0),
];

/// Clades as tip bit sets, sorted, root included.
fn clades(tree: &Tree) -> Vec<u32> {
    let mut masks = vec![0u32; tree.nodes()];
    for node in tree.postorder() {
        masks[node] = match tree.children(node) {
            None => 1 << node,
            Some((a, b)) => masks[a] | masks[b],
        };
    }
    let mut internal: Vec<u32> = masks[tree.tips()..].to_vec();
    internal.sort_unstable();
    internal
}

/// Every rooted topology on the tips in `mask`, as sorted clade lists.
fn topologies(mask: u32) -> Vec<Vec<u32>> {
    if mask.count_ones() == 1 {
        return vec![Vec::new()];
    }
    let lowest = mask & mask.wrapping_neg();
    let mut result = Vec::new();
    let rest = mask ^ lowest;
    // Subsets of `rest` give every split once, with `lowest` on the left.
    let mut subset = rest;
    loop {
        let left = lowest | subset;
        if left != mask {
            for a in topologies(left) {
                for b in topologies(mask ^ left) {
                    let mut clades: Vec<u32> = a.iter().chain(&b).copied().collect();
                    clades.push(mask);
                    clades.sort_unstable();
                    result.push(clades);
                }
            }
        }
        if subset == 0 {
            break;
        }
        subset = (subset - 1) & rest;
    }
    result
}

/// Rankings of the internal nodes consistent with the topology.
fn rankings(clades: &[u32]) -> f64 {
    let internal = clades.len() as f64;
    let factorial: f64 = (1..=clades.len()).map(|k| k as f64).product();
    debug_assert!(internal >= 1.0);
    factorial
        / clades
            .iter()
            .map(|c| f64::from(c.count_ones() - 1))
            .product::<f64>()
}

/// `F̂(h) = (1 − e^(−rh)) / (1 − a·e^(−rh))`, the distribution function of the
/// speciation times under the birth–death prior.
fn time_cdf(birth_death: BirthDeath, height: f64) -> f64 {
    let decay = (-birth_death.birth_difference * height).exp();
    (1.0 - decay) / (1.0 - birth_death.relative_death * decay)
}

struct PriorSample {
    topologies: HashMap<Vec<u32>, usize>,
    root: Vec<f64>,
    nodes: Vec<f64>,
}

fn sample_prior(settings: &Settings, steps: u64, thin: u64, seed: u64) -> PriorSample {
    let data = five_taxa();
    let mut sampler = Sampler::new(&data, settings, seed).unwrap();
    let mut sample = PriorSample {
        topologies: HashMap::new(),
        root: Vec::new(),
        nodes: Vec::new(),
    };
    for step in 1..=steps {
        sampler.step();
        if step % thin == 0 {
            let tree = sampler.tree();
            let birth_death = sampler.birth_death();
            *sample.topologies.entry(clades(tree)).or_default() += 1;
            sample
                .root
                .push(time_cdf(birth_death, tree.height(tree.root())));
            let internal = tree.tips()..tree.nodes();
            let mean = internal
                .clone()
                .map(|n| time_cdf(birth_death, tree.height(n)))
                .sum::<f64>()
                / internal.len() as f64;
            sample.nodes.push(mean);
        }
    }
    sample
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Half the L1 distance between the sampled and expected topology frequencies.
fn total_variation(sample: &PriorSample, expected: &HashMap<Vec<u32>, f64>) -> f64 {
    let total: usize = sample.topologies.values().sum();
    let unexpected = sample
        .topologies
        .keys()
        .filter(|t| !expected.contains_key(*t))
        .count();
    assert_eq!(unexpected, 0, "sampled a topology outside the support");
    expected
        .iter()
        .map(|(topology, p)| {
            let observed = sample.topologies.get(topology).copied().unwrap_or(0);
            (observed as f64 / total as f64 - p).abs()
        })
        .sum::<f64>()
        / 2.0
}

fn expected_topologies(keep: impl Fn(&[u32]) -> bool) -> HashMap<Vec<u32>, f64> {
    let all: Vec<Vec<u32>> = topologies(0b11111)
        .into_iter()
        .filter(|t| keep(t))
        .collect();
    let total: f64 = all.iter().map(|t| rankings(t)).sum();
    all.into_iter()
        .map(|t| {
            let p = rankings(&t) / total;
            (t, p)
        })
        .collect()
}

#[test]
fn enumeration_counts_ranked_histories() {
    let all = topologies(0b11111);
    assert_eq!(all.len(), 105);
    // n!(n − 1)!/2^(n − 1) ranked labeled histories for five tips.
    assert_eq!(all.iter().map(|t| rankings(t)).sum::<f64>(), 180.0);
}

/// `F̂(root height) ~ Beta(n − 1, 2)` and `E[mean F̂] = n / (2(n + 1))` for
/// the `n = 4` internal nodes of a five-tip tree.
fn assert_node_times(sample: &PriorSample, context: &str) {
    let root = mean(&sample.root);
    let nodes = mean(&sample.nodes);
    assert!((root - 4.0 / 6.0).abs() < 0.01, "{context}: root {root}");
    assert!(
        (nodes - 5.0 / 12.0).abs() < 0.01,
        "{context}: nodes {nodes}"
    );
}

#[test]
fn height_moves_sample_the_prior_node_times() {
    let birth_death = BirthDeath {
        birth_difference: 1.5,
        relative_death: 0.4,
    };
    let sample = sample_prior(&prior_settings(birth_death, &HEIGHT_MOVES), 400_000, 4, 3);
    assert_eq!(sample.topologies.len(), 1);
    assert_node_times(&sample, "height moves");
}

/// The Hastings ratio of each topology move is tested separately in
/// `mcmc::moves`; here they run together under the birth–death prior.
#[test]
fn the_prior_has_uniform_ranked_histories_and_known_node_times() {
    let birth_death = BirthDeath {
        birth_difference: 1.5,
        relative_death: 0.4,
    };
    let mut settings = prior_settings(birth_death, &Settings::default().weights);
    settings.weights.retain(|(kind, _)| kind.changes_tree());
    let sample = sample_prior(&settings, 800_000, 4, 20261028);
    let distance = total_variation(&sample, &expected_topologies(|_| true));
    assert!(distance < 0.02, "total variation {distance}");
    assert_node_times(&sample, "all tree moves");
}

#[test]
fn constraints_restrict_the_prior_to_monophyletic_topologies() {
    let birth_death = BirthDeath {
        birth_difference: 0.8,
        relative_death: 0.0,
    };
    let mut settings = prior_settings(birth_death, &Settings::default().weights);
    settings.weights.retain(|(kind, _)| kind.changes_tree());
    settings.monophyly = vec![vec![0, 1, 2]];
    let sample = sample_prior(&settings, 400_000, 4, 7);
    let expected = expected_topologies(|t| t.contains(&0b00111));
    assert_eq!(expected.len(), 9);
    let distance = total_variation(&sample, &expected);
    assert!(distance < 0.03, "total variation {distance}");
    assert_node_times(&sample, "constrained");
}

/// With every move, λ − μ and μ/λ keep their uniform priors, because the
/// birth–death density integrates to the same constant for any rates.
#[test]
fn the_rate_parameters_keep_their_uniform_priors() {
    let settings = Settings {
        sample_from_prior: true,
        tuning: false,
        ..Settings::default()
    };
    let mut sampler = Sampler::new(&five_taxa(), &settings, 2).unwrap();
    let (mut r, mut a, mut root, mut samples) = (0.0, 0.0, 0.0, 0.0);
    for step in 1..=2_000_000u64 {
        sampler.step();
        if step % 10 == 0 {
            let birth_death = sampler.birth_death();
            let tree = sampler.tree();
            r += birth_death.birth_difference / settings.birth_difference_upper;
            a += birth_death.relative_death;
            root += time_cdf(birth_death, tree.height(tree.root()));
            samples += 1.0;
        }
    }
    let (r, a, root) = (r / samples, a / samples, root / samples);
    assert!((r - 0.5).abs() < 0.05, "mean (λ − μ)/1000 {r}");
    assert!((a - 0.5).abs() < 0.05, "mean μ/λ {a}");
    assert!((root - 4.0 / 6.0).abs() < 0.015, "root {root}");
}

fn data_settings(conditioning: Conditioning) -> Settings {
    Settings {
        conditioning,
        monophyly: vec![vec![0, 1, 2, 3]],
        tuning_delay: 200,
        ..Settings::default()
    }
}

#[test]
fn cached_calculations_match_a_fresh_evaluation_and_rejections_restore_the_state() {
    let data = patterns(&[
        "0101101?1100",
        "0101100011?0",
        "1101101011{01}1",
        "001101011010",
        "101001010101",
        "101001110001",
    ]);
    for conditioning in [Conditioning::Variable, Conditioning::None] {
        let mut sampler = Sampler::new(&data, &data_settings(conditioning), 5).unwrap();
        let mut accepted = HashMap::<Move, (u64, u64)>::new();
        for _ in 0..4000 {
            let (tree, birth_death, posterior) = (
                sampler.tree().clone(),
                sampler.birth_death(),
                sampler.posterior(),
            );
            let (kind, accept) = sampler.step();
            let counts = accepted.entry(kind).or_default();
            if accept {
                counts.0 += 1;
            } else {
                counts.1 += 1;
                assert_eq!(sampler.tree(), &tree, "{kind:?}");
                assert_eq!(sampler.birth_death(), birth_death);
                assert_eq!(sampler.posterior(), posterior);
            }
            sampler.tree().validate().unwrap();
            assert!(
                sampler
                    .tree()
                    .is_monophyletic(&[true, true, true, true, false, false])
            );
            let fresh = sampler.recompute().unwrap();
            let current = sampler.posterior();
            assert!(
                (fresh.total() - current.total()).abs() <= 1e-12 * current.total().abs().max(1.0),
                "{kind:?}: {fresh:?} != {current:?}"
            );
        }
        for kind in Move::ALL {
            let (yes, no) = accepted.get(&kind).copied().unwrap_or_default();
            assert!(yes > 0 && no > 0, "{kind:?} accepted {yes}, rejected {no}");
        }
    }
}

#[test]
fn mkv_rejects_constant_characters_and_settings_are_validated() {
    let constant = patterns(&["01", "00", "01", "01"]);
    assert!(Sampler::new(&constant, &Settings::default(), 1).is_err());
    assert!(Sampler::new(&constant, &data_settings(Conditioning::None), 1).is_ok());
    let data = patterns(&["01", "10", "01", "10"]);
    let crossing = Settings {
        monophyly: vec![vec![0, 1], vec![1, 2]],
        ..Settings::default()
    };
    assert!(Sampler::new(&data, &crossing, 1).is_err());
    let single = Settings {
        monophyly: vec![vec![3]],
        ..Settings::default()
    };
    assert!(Sampler::new(&data, &single, 1).is_err());
    let unweighted = Settings {
        weights: vec![(Move::NodeHeight, 0.0)],
        ..Settings::default()
    };
    assert!(Sampler::new(&data, &unweighted, 1).is_err());
}

#[test]
fn the_tree_prior_matches_beast_at_the_archived_states() {
    for state in reference_states::STATES {
        let tree = Tree::new(state.children.to_vec(), state.heights.to_vec()).unwrap();
        let birth_death = BirthDeath {
            birth_difference: state.birth_difference,
            relative_death: state.relative_death,
        };
        let density = birth_death.log_density(&tree);
        assert!(
            (density - state.birth_death).abs() < 1e-12,
            "seed {}: {density} != {}",
            state.seed,
            state.birth_death
        );
    }
}
