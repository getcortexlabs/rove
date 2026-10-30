use rove::ascertainment::VariableCharacters;
use rove::data::{CharacterSet, CompressionError, GapTreatment, Patterns};
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::model::Mk;
use rove::rates::{GammaRates, RateVariation, RelaxedClock};
use rove::reference::Problem;
use rove::{Backend, nexus};

const MATRIX: &str = include_str!("../fixtures/validation/multistate/matrix.nex");

fn partitions() -> Vec<Patterns> {
    nexus::parse(MATRIX)
        .unwrap()
        .partition_by_states(GapTreatment::Reject)
        .unwrap()
}

#[test]
fn nonconsecutive_symbols_are_recoded_and_compressed_within_state_counts() {
    let data = nexus::parse(MATRIX).unwrap();
    assert_eq!(&data.taxa()[..2], &["Canella-A", "Canella-B"]);
    let parts = partitions();
    assert_eq!(
        parts.iter().map(|p| p.symbols().len()).collect::<Vec<_>>(),
        [2, 3, 4]
    );
    assert_eq!(parts[0].sites(), [0, 3, 5]);
    assert_eq!(parts[1].sites(), [1, 4, 6, 7]);
    assert_eq!(parts[2].sites(), [2]);
    assert_eq!(parts[0].weights(), [2, 1]);
    assert_eq!(parts[1].weights(), [2, 1, 1]);
    assert_eq!(parts[2].weights(), [1]);
    assert_eq!(
        parts[0].state_symbols(),
        [vec!['0', '1'], vec!['0', '3'], vec!['0', '1']]
    );
    assert_eq!(parts[1].state_symbols()[1], ['0', '2', '3']);
    for part in parts {
        let k = part.symbols().len();
        for (local, &site) in part.sites().iter().enumerate() {
            let symbols = &part.state_symbols()[local];
            for (taxon, row) in data.rows().iter().enumerate() {
                let partial = &part.tip_partials()[taxon][part.site_to_pattern()[local] * k..][..k];
                for (state, &symbol) in symbols.iter().enumerate() {
                    let original = data.symbols().iter().position(|&s| s == symbol).unwrap();
                    let allowed = match &row[site] {
                        rove::data::Cell::State(s) => *s == original,
                        rove::data::Cell::Uncertain(states) => states.contains(&original),
                        rove::data::Cell::Missing => true,
                        _ => panic!("unexpected fixture observation"),
                    };
                    assert_eq!(partial[state], f64::from(allowed));
                }
            }
        }
    }
}

fn matrix(rows: &str) -> rove::data::Matrix {
    nexus::parse(&format!("#NEXUS BEGIN DATA; DIMENSIONS NTAX=3 NCHAR=2; FORMAT SYMBOLS=\"0123\" GAP=-; MATRIX {rows}; END;")).unwrap()
}

#[test]
fn missing_gaps_and_uncertainty_do_not_silently_choose_a_model() {
    for rows in ["a ?0 b ?1 c ?2", "a -0 b ?1 c ?2", "a 00 b ?1 c ?2"] {
        assert_eq!(
            matrix(rows).partition_by_states(GapTreatment::Missing),
            Err(CompressionError::StateCount { site: 0 })
        );
        let declared = matrix(rows)
            .compress(&[0..1, 1..2], GapTreatment::Missing)
            .unwrap();
        assert_eq!(declared[0].symbols().len(), 4);
        assert!(VariableCharacters::new(&declared[0]).is_err());
        assert!(
            Sampler::new(
                &declared,
                &Settings {
                    conditioning: Conditioning::None,
                    ..Settings::default()
                },
                3
            )
            .is_ok()
        );
    }
    let data = matrix("a 00 b -1 c 12");
    assert!(matches!(
        data.partition_by_states(GapTreatment::Reject),
        Err(CompressionError::Gap { site: 0, .. })
    ));
    let parts = data.partition_by_states(GapTreatment::Missing).unwrap();
    assert_eq!(parts[0].symbols().len(), 2);
    assert_eq!(parts[1].symbols().len(), 3);
    // Explicit uncertainty contributes its possible states; '?' does not.
    let data = matrix("a {03}0 b ?1 c 32");
    let parts = data.partition_by_states(GapTreatment::Reject).unwrap();
    assert_eq!(parts[0].state_symbols()[0], ['0', '3']);
    assert_eq!(parts[0].tip_partials()[0], [1.0, 1.0]);
    assert!(matches!(
        VariableCharacters::new(&parts[0]),
        Err(rove::ascertainment::Error::ConstantCharacter {
            site: 0,
            state: '3'
        })
    ));
    assert!(matches!(
        matrix("a (03)0 b 01 c 32").partition_by_states(GapTreatment::Reject),
        Err(CompressionError::Polymorphism { .. })
    ));
}

#[test]
fn explicit_sets_keep_unobserved_states_and_validate_coverage() {
    let data = matrix("a 00 b 12 c ?3");
    let sets = [
        CharacterSet {
            sites: vec![0],
            symbols: vec!['0', '1'],
        },
        CharacterSet {
            sites: vec![1],
            symbols: vec!['0', '1', '2', '3'],
        },
    ];
    let parts = data.compress_sets(&sets, GapTreatment::Reject).unwrap();
    assert_eq!(parts[1].symbols().len(), 4);
    for bad in [
        vec![],
        vec![sets[0].clone()],
        vec![sets[0].clone(), sets[0].clone()],
        vec![CharacterSet {
            sites: vec![1, 0],
            symbols: data.symbols().to_vec(),
        }],
    ] {
        assert_eq!(
            data.compress_sets(&bad, GapTreatment::Reject),
            Err(CompressionError::InvalidPartitions)
        );
    }
    let mut bad = sets.clone();
    bad[1].symbols = vec!['0', '2'];
    assert_eq!(
        data.compress_sets(&bad, GapTreatment::Reject),
        Err(CompressionError::ExcludedState {
            site: 1,
            symbol: '3'
        })
    );
}

fn reference(partition: &Patterns, sampler: &Sampler, conditioning: Conditioning) -> f64 {
    let tree = sampler.tree();
    let rates = sampler.rates();
    let model = Mk::uniform(partition.symbols().len());
    let k = model.states();
    let matrices = (0..tree.nodes())
        .map(|node| {
            let mut matrix = vec![0.0; rates.category_rates().len() * k * k];
            for (category, &rate) in rates.category_rates().iter().enumerate() {
                model.transition_matrix(
                    tree.branch_length(node) * rates.branch_rate(node, tree.root()) * rate,
                    &mut matrix[category * k * k..(category + 1) * k * k],
                );
            }
            matrix
        })
        .collect::<Vec<_>>();
    let children: Vec<_> = (0..tree.nodes()).map(|n| tree.children(n)).collect();
    let weights: Vec<_> = partition.weights().iter().map(|&w| w as f64).collect();
    let problem = Problem {
        children: &children,
        root: tree.root(),
        tip_partials: partition.tip_partials(),
        matrices: &matrices,
        category_weights: rates.category_weights(),
        frequencies: model.frequencies(),
        pattern_weights: &weights,
    };
    match conditioning {
        Conditioning::None => problem.log_likelihood(),
        Conditioning::Variable => problem.variable_log_likelihood(),
        Conditioning::Informative => {
            unreachable!("informative conditioning has its own enumeration oracle")
        }
    }
}

#[test]
fn shared_tree_and_rates_match_reference_and_single_partition_sums() {
    let parts = partitions();
    for settings in [Settings::default(), Settings::reference()] {
        for conditioning in [Conditioning::None, Conditioning::Variable] {
            let settings = Settings {
                conditioning,
                tuning_delay: 10,
                ..settings.clone()
            };
            let mut chain = Sampler::new(&parts, &settings, 42).unwrap();
            for step in 0..500 {
                chain.step();
                if step % 13 != 0 {
                    continue;
                }
                let mut total = 0.0;
                let mut oracle = 0.0;
                for part in &parts {
                    total += Sampler::with_rates(
                        part,
                        &settings,
                        chain.tree().clone(),
                        chain.rates().clone(),
                        1,
                    )
                    .unwrap()
                    .posterior()
                    .likelihood;
                    oracle += reference(part, &chain, conditioning);
                }
                assert!((total - chain.posterior().likelihood).abs() < 1e-10);
                assert!((oracle - chain.posterior().likelihood).abs() < 1e-10);
                assert!(
                    (chain.recompute().unwrap().total() - chain.posterior().total()).abs() < 1e-10
                );
            }
        }
    }
}

#[test]
fn all_partition_caches_resume_exactly_and_backends_agree() {
    let parts = partitions();
    for base in [Settings::default(), Settings::reference()] {
        for conditioning in [Conditioning::None, Conditioning::Variable] {
            for prior in [false, true] {
                let settings = Settings {
                    conditioning,
                    sample_from_prior: prior,
                    tuning_delay: 10,
                    backend: Backend::Portable,
                    ..base.clone()
                };
                let mut chain = Sampler::new(&parts, &settings, 17).unwrap();
                for _ in 0..87 {
                    chain.step();
                }
                let checkpoint = chain.checkpoint(&settings).unwrap();
                let mut resumed = Sampler::resume(&parts, &settings, &checkpoint).unwrap();
                let mut reordered = parts.clone();
                reordered.swap(0, 1);
                assert!(Sampler::resume(&reordered, &settings, &checkpoint).is_err());
                for length in [0, 8, checkpoint.len() / 2, checkpoint.len() - 1] {
                    assert!(Sampler::resume(&parts, &settings, &checkpoint[..length]).is_err());
                }
                for _ in 0..500 {
                    assert_eq!(chain.step(), resumed.step());
                    assert_eq!(chain.posterior(), resumed.posterior());
                    assert_eq!(chain.tree(), resumed.tree());
                    assert_eq!(chain.operators(), resumed.operators());
                }
                if Backend::Neon.is_available() {
                    let mut portable = Sampler::new(&parts, &settings, 1).unwrap();
                    let mut neon = Sampler::new(
                        &parts,
                        &Settings {
                            backend: Backend::Neon,
                            ..settings.clone()
                        },
                        1,
                    )
                    .unwrap();
                    for _ in 0..500 {
                        assert_eq!(portable.step(), neon.step());
                        assert_eq!(portable.posterior(), neon.posterior());
                    }
                }
            }
        }
    }
}

#[test]
fn binary_partitioning_preserves_declared_alphabet_and_trajectory() {
    let data = nexus::parse(include_str!("../fixtures/reference/20x50/matrix.nex")).unwrap();
    let old = data
        .compress(
            std::slice::from_ref(&(0..data.character_count())),
            GapTreatment::Reject,
        )
        .unwrap();
    let new = data.partition_by_states(GapTreatment::Reject).unwrap();
    assert_eq!(old, new);
}

#[test]
fn hyphenated_labels_survive_interleaving_taxlabels_and_gap_cells() {
    let data = nexus::parse("#NEXUS BEGIN TAXA; DIMENSIONS NTAX=2; TAXLABELS Canella-A Canella-B; END; BEGIN CHARACTERS; DIMENSIONS NCHAR=4; FORMAT INTERLEAVE GAP=-; MATRIX\nCanella-B 0-\nCanella-A 10\nCanella-B 1?\nCanella-A -1\n; END;").unwrap();
    assert_eq!(data.taxa(), &["Canella-A", "Canella-B"]);
    assert_eq!(data.rows()[0][2], rove::data::Cell::Gap);
    assert_eq!(data.rows()[1][1], rove::data::Cell::Gap);
}

#[test]
fn fixed_states_match_beast_2_mm_partition_by_partition() {
    let archived: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/validation/multistate/beast-likelihoods.json"
    ))
    .unwrap();
    let parts = partitions();
    let mut worst = 0.0_f64;
    for case in archived["cases"].as_array().unwrap() {
        let scale = case["scale"].as_f64().unwrap();
        let mut children = vec![None; 6];
        children.extend(archived["children"].as_array().unwrap().iter().map(|pair| {
            Some((
                pair[0].as_u64().unwrap() as usize,
                pair[1].as_u64().unwrap() as usize,
            ))
        }));
        let heights = archived["heights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h.as_f64().unwrap() * scale)
            .collect();
        let tree = rove::tree::Tree::new(children, heights).unwrap();
        let relaxed = case["relaxed"].as_bool().unwrap();
        let settings = Settings {
            conditioning: if case["variable"].as_bool().unwrap() {
                Conditioning::Variable
            } else {
                Conditioning::None
            },
            ..if relaxed {
                Settings::reference()
            } else {
                Settings::default()
            }
        };
        let rates = RateVariation {
            gamma: relaxed.then(|| GammaRates::new(1.0).unwrap()),
            clock: relaxed.then(|| {
                RelaxedClock::new(
                    0.2,
                    archived["branch_rates"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|r| r.as_f64().unwrap())
                        .collect(),
                )
                .unwrap()
            }),
        };
        for (part, expected) in parts.iter().zip(case["partitions"].as_array().unwrap()) {
            assert_eq!(
                part.symbols().len() as u64,
                expected["states"].as_u64().unwrap()
            );
            assert_eq!(
                part.sites().iter().map(|s| s + 1).collect::<Vec<_>>(),
                expected["sites"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_u64().unwrap() as usize)
                    .collect::<Vec<_>>()
            );
            let actual = Sampler::with_rates(part, &settings, tree.clone(), rates.clone(), 1)
                .unwrap()
                .posterior()
                .likelihood;
            let difference = (actual - expected["log_likelihood"].as_f64().unwrap()).abs();
            worst = worst.max(difference);
            assert!(
                difference < 2e-8,
                "{case}: {actual}, difference {difference}"
            );
        }
        let actual = Sampler::with_rates(&parts, &settings, tree, rates, 1)
            .unwrap()
            .posterior()
            .likelihood;
        let difference = (actual - case["log_likelihood"].as_f64().unwrap()).abs();
        worst = worst.max(difference);
        assert!(
            difference < 3e-8,
            "{case}: {actual}, difference {difference}"
        );
    }
    eprintln!("largest partitioned log-likelihood difference from BEAST: {worst:e}");
}
