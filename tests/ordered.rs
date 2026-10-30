use rove::data::{CharacterSet, CompressionError, GapTreatment, Patterns};
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::model::{CharacterOrdering, Mk, Ordered};
use rove::reference::Problem;
use rove::{Backend, nexus};

const MATRIX: &str = include_str!("../fixtures/validation/ordered/matrix.nex");

fn parts() -> Vec<Patterns> {
    nexus::parse(MATRIX)
        .unwrap()
        .partition_by_states(GapTreatment::Reject)
        .unwrap()
}

#[test]
fn transition_matrices_match_independent_high_precision_exponentials() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/validation/ordered/matrices.json")).unwrap();
    for case in reference["cases"].as_array().unwrap() {
        let k = case["states"].as_u64().unwrap() as usize;
        let distance = case["distance"].as_f64().unwrap();
        let mut matrix = vec![0.0; k * k];
        Ordered::new(k).transition_matrix(distance, &mut matrix);
        for (actual, expected) in matrix.iter().zip(case["matrix"].as_array().unwrap()) {
            let expected: f64 = expected.as_str().unwrap().parse().unwrap();
            let tolerance = expected.abs() * 3e-13 + 1e-300;
            assert!(
                (actual - expected).abs() <= tolerance,
                "{k} states, t={distance}: {actual:e} vs {expected:e}"
            );
        }
    }
}

#[test]
fn three_state_closed_form_and_binary_mk_agree() {
    let mut ordered = Ordered::new(3);
    for t in [0.0001, 0.1, 1.0, 10.0, 1000.0] {
        let mut p = [0.0; 9];
        ordered.transition_matrix(t, &mut p);
        let a = (-0.75 * t).exp();
        let b = (-2.25 * t).exp();
        let edge = 1.0 / 3.0 + a / 2.0 + b / 6.0;
        let adjacent = (1.0 - b) / 3.0;
        let opposite = 1.0 / 3.0 - a / 2.0 + b / 6.0;
        let expected = [
            edge,
            adjacent,
            opposite,
            adjacent,
            (1.0 + 2.0 * b) / 3.0,
            adjacent,
            opposite,
            adjacent,
            edge,
        ];
        for (actual, expected) in p.iter().zip(expected) {
            assert!((actual - expected).abs() < 2e-14);
        }
    }
    let mut ordered = Ordered::new(2);
    for t in [0.0, 1e-14, 0.1, 1.0, 100.0, f64::INFINITY] {
        let mut a = [0.0; 4];
        let mut b = a;
        ordered.transition_matrix(t, &mut a);
        Mk::uniform(2).transition_matrix(t, &mut b);
        for (a, b) in a.iter().zip(b) {
            assert!((a - b).abs() < 2e-14);
        }
    }
}

#[test]
fn rows_are_stochastic_and_short_branches_have_unit_equilibrium_rate() {
    for k in [2, 3, 4, 8, 32, 64] {
        let mut model = Ordered::new(k);
        let mut p = vec![0.0; k * k];
        for t in [0.0, 1e-300, 1e-14, 0.1, 1.0, 1000.0, f64::INFINITY] {
            model.transition_matrix(t, &mut p);
            for i in 0..k {
                assert!((p[i * k..(i + 1) * k].iter().sum::<f64>() - 1.0).abs() < 3e-14);
                for j in 0..k {
                    assert!(p[i * k + j] >= 0.0 && p[i * k + j] <= 1.0);
                    assert!((p[i * k + j] - p[j * k + i]).abs() < 3e-14);
                }
            }
        }
        model.transition_matrix(1e-9, &mut p);
        let changes: f64 = (0..k)
            .map(|i| {
                (0..k)
                    .filter(|&j| j != i)
                    .map(|j| p[i * k + j])
                    .sum::<f64>()
                    / k as f64
            })
            .sum();
        assert!((changes / 1e-9 - 1.0).abs() < 2e-9);
    }
}

fn assumptions(text: &str) -> Result<rove::data::Matrix, nexus::Error> {
    nexus::parse(&format!(
        "#NEXUS BEGIN DATA; DIMENSIONS NTAX=3 NCHAR=4; FORMAT SYMBOLS=\"0123\"; MATRIX a 0000 b 2222 c 3333; END; BEGIN ASSUMPTIONS; {text} END;"
    ))
}

#[test]
fn typesets_defaults_ranges_and_vectors_preserve_ordering() {
    use CharacterOrdering::{Ordered as O, Unordered as U};
    for definition in [
        "TYPESET * test = ORD: 1-3\\2;",
        "TYPESET test (STANDARD) = ORD: 1 - 3 \\ 2;",
        "OPTIONS DEFTYPE=ORD; TYPESET test = UNORD: 2 4;",
        "TYPESET test (VECTOR) = ORD UNORD ORD UNORD;",
    ] {
        assert_eq!(assumptions(definition).unwrap().ordering(), [O, U, O, U]);
    }
    assert_eq!(
        assumptions("OPTIONS DEFTYPE=ORD;").unwrap().ordering(),
        [O; 4]
    );
    assert_eq!(
        assumptions("TYPESET test=ORD: 1-.;").unwrap().ordering(),
        [O; 4]
    );
    assert_eq!(
        assumptions("TYPESET test=ORD: ALL;").unwrap().ordering(),
        [O; 4]
    );
    for invalid in [
        "TYPESET test = ORD: 0;",
        "TYPESET test = ORD: 5;",
        "TYPESET test = ORD: 3-1;",
        "TYPESET test = ORD: 1-;",
        "TYPESET test = ORD: 1-3\\0;",
        "TYPESET test = ORD: 1-3\\;",
        "TYPESET test = ORD: 1 1;",
        "TYPESET test = ORD: 1, UNORD: 1;",
        "TYPESET test = ORD:;",
        "TYPESET test = ORD: 1,;",
        "TYPESET test = IRREV: ALL;",
        "TYPESET test = ORD: my_set;",
        "TYPESET test (VECTOR) = ORD UNORD;",
        "TYPESET test = ORD: 1; TYPESET other = ORD: 2;",
        "OPTIONS DEFTYPE=DOLLO;",
        "OPTIONS GAPMODE=NEWSTATE;",
        "WTSET weights=2:1;",
        "TYPESET test = ORD: '1';",
        "TYPESET test = ORD: 1--3;",
    ] {
        let error = assumptions(invalid).unwrap_err();
        assert!(error.line > 0 && error.column > 0, "{invalid}: {error}");
    }
}

#[test]
fn partitions_preserve_intermediate_states_and_do_not_compress_across_models() {
    let data = assumptions("TYPESET test = ORD: 1 3;").unwrap();
    let parts = data.partition_by_states(GapTreatment::Reject).unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].ordering(), CharacterOrdering::Unordered);
    assert_eq!(parts[0].symbols(), ['0', '2', '3']);
    assert_eq!(parts[0].sites(), [1, 3]);
    assert_eq!(parts[1].ordering(), CharacterOrdering::Ordered);
    assert_eq!(parts[1].symbols(), ['0', '1', '2', '3']);
    assert_eq!(parts[1].sites(), [0, 2]);
    assert_eq!(parts[1].weights(), [2]);
    let declared = data
        .compress(std::slice::from_ref(&(0..4)), GapTreatment::Reject)
        .unwrap();
    assert_eq!(declared.len(), 2);
    assert!(
        declared
            .iter()
            .all(|p| p.symbols().len() == 4 && p.weights() == [2])
    );
    assert!(matches!(
        data.compress_sets(
            &[CharacterSet {
                sites: (0..4).collect(),
                symbols: vec!['0', '2', '3']
            }],
            GapTreatment::Reject
        ),
        Err(CompressionError::OrderedAlphabet { site: 0 })
    ));
    assert!(
        rove::ascertainment::validate_informative(&parts[1])
            .unwrap_err()
            .to_string()
            .contains("ordered")
    );
}

fn reference(partition: &Patterns, sampler: &Sampler, conditioning: Conditioning) -> f64 {
    let tree = sampler.tree();
    let rates = sampler.rates();
    let k = partition.symbols().len();
    let mut ordered = Ordered::new(k);
    let mk = Mk::uniform(k);
    let matrices: Vec<_> = (0..tree.nodes())
        .map(|node| {
            let mut matrix = vec![0.0; rates.category_rates().len() * k * k];
            for (category, &rate) in rates.category_rates().iter().enumerate() {
                let distance =
                    tree.branch_length(node) * rates.branch_rate(node, tree.root()) * rate;
                let out = &mut matrix[category * k * k..(category + 1) * k * k];
                if partition.ordering() == CharacterOrdering::Ordered {
                    ordered.transition_matrix(distance, out);
                } else {
                    mk.transition_matrix(distance, out);
                }
            }
            matrix
        })
        .collect();
    let children: Vec<_> = (0..tree.nodes()).map(|n| tree.children(n)).collect();
    let weights: Vec<_> = partition.weights().iter().map(|&w| w as f64).collect();
    let problem = Problem {
        children: &children,
        root: tree.root(),
        tip_partials: partition.tip_partials(),
        matrices: &matrices,
        category_weights: rates.category_weights(),
        frequencies: mk.frequencies(),
        pattern_weights: &weights,
    };
    if conditioning == Conditioning::Variable {
        problem.variable_log_likelihood()
    } else {
        problem.log_likelihood()
    }
}

#[test]
fn mixed_sampling_rollback_and_checkpoints_match_the_reference() {
    let parts = parts();
    for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
        for base in [Settings::default(), Settings::reference()] {
            for conditioning in [Conditioning::None, Conditioning::Variable] {
                let settings = Settings {
                    backend,
                    conditioning,
                    tuning_delay: 10,
                    ..base.clone()
                };
                let mut sampler = Sampler::new(&parts, &settings, 425).unwrap();
                let mut accepted = 0;
                let mut rejected = 0;
                for step in 0..300 {
                    if sampler.step().1 {
                        accepted += 1;
                    } else {
                        rejected += 1;
                    }
                    if step % 11 == 0 {
                        let expected: f64 = parts
                            .iter()
                            .map(|p| reference(p, &sampler, conditioning))
                            .sum();
                        assert!((expected - sampler.posterior().likelihood).abs() < 2e-10);
                    }
                }
                assert!(accepted > 0 && rejected > 0);
                let saved = sampler.checkpoint(&settings).unwrap();
                let mut resumed = Sampler::resume(&parts, &settings, &saved).unwrap();
                for _ in 0..100 {
                    assert_eq!(sampler.step(), resumed.step());
                    assert_eq!(sampler.tree(), resumed.tree());
                    assert_eq!(sampler.posterior(), resumed.posterior());
                }
                let different = nexus::parse(&MATRIX.replace("ORD: 2-3 5 8", "ORD: 2-3 5"))
                    .unwrap()
                    .partition_by_states(GapTreatment::Reject)
                    .unwrap();
                assert!(Sampler::resume(&different, &settings, &saved).is_err());
            }
        }
    }
}

#[test]
fn ordered_fixed_states_match_beast_2_mm_ordinal() {
    let archived: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/validation/ordered/beast-likelihoods.json"
    ))
    .unwrap();
    let parts = parts();
    let mut worst = 0.0_f64;
    for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
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
                backend,
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
            let rates = rove::rates::RateVariation {
                gamma: relaxed.then(|| rove::rates::GammaRates::new(1.0).unwrap()),
                clock: relaxed.then(|| {
                    rove::rates::RelaxedClock::new(
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
            assert_eq!(parts.len(), case["partitions"].as_array().unwrap().len());
            for (part, expected) in parts.iter().zip(case["partitions"].as_array().unwrap()) {
                assert_eq!(
                    part.ordering().name(),
                    expected["ordering"].as_str().unwrap()
                );
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
    }
    eprintln!("largest ordered/partitioned log-likelihood difference from BEAST: {worst:e}");
}
