use rove::ascertainment::{Error, InformativeProbability, validate_informative};
use rove::data::{GapTreatment, Patterns};
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::model::Mk;
use rove::rates::{GammaRates, RateVariation, RelaxedClock};
use rove::reference::Problem;
use rove::{Backend, LikelihoodCore, Scaling, nexus};

fn patterns(states: usize, columns: &[Vec<String>]) -> Patterns {
    let tips = columns[0].len();
    let alphabet: String = (0..states)
        .map(|s| char::from_digit(s as u32, 36).unwrap())
        .collect();
    let mut text = format!(
        "#NEXUS BEGIN DATA; DIMENSIONS NTAX={tips} NCHAR={}; FORMAT SYMBOLS=\"{alphabet}\" MISSING=?; MATRIX\n",
        columns.len()
    );
    for tip in 0..tips {
        text += &format!(
            "T{tip} {}\n",
            columns.iter().map(|c| c[tip].as_str()).collect::<String>()
        );
    }
    text += "; END;";
    nexus::parse(&text)
        .unwrap()
        .compress(
            std::slice::from_ref(&(0..columns.len())),
            GapTreatment::Reject,
        )
        .unwrap()
        .remove(0)
}

fn digits(mut value: usize, tips: usize, states: usize) -> Vec<usize> {
    (0..tips)
        .map(|_| {
            let digit = value % states;
            value /= states;
            digit
        })
        .collect()
}

fn informative(pattern: &[usize], states: usize) -> bool {
    (0..states)
        .filter(|s| pattern.iter().filter(|x| x == &s).count() >= 2)
        .count()
        >= 2
}

fn children(tips: usize, balanced: bool) -> Vec<Option<(usize, usize)>> {
    let mut active: Vec<_> = (0..tips).collect();
    let mut children = vec![None; tips];
    while active.len() > 1 {
        let a = active.remove(0);
        let b = active.remove(0);
        if balanced {
            active.push(children.len());
        } else {
            active.insert(0, children.len());
        }
        children.push(Some((a, b)));
    }
    children
}

#[test]
fn probabilities_match_complete_enumeration_on_small_trees() {
    let rates = [0.0, 0.2, 1.1, 3.7];
    let weights = [0.125, 0.2, 0.3, 0.375];
    for states in [2usize, 3, 4, 6] {
        let model = Mk::uniform(states);
        for tips in 2..=5 {
            for balanced in [false, true] {
                let children = children(tips, balanced);
                let root = children.len() - 1;
                for scale in [0.0001, 0.3, 20.0] {
                    let mut core =
                        LikelihoodCore::mk(&model, 1, rates.len(), children.len(), Scaling::Always);
                    let mut correction = InformativeProbability::new(states, tips, rates.len());
                    let mut matrices =
                        vec![vec![0.0; states * states * rates.len()]; children.len()];
                    for node in 0..children.len() {
                        for (category, rate) in rates.iter().enumerate() {
                            let distance = scale * (0.1 + (node * 7 % 11) as f64 / 10.0) * rate;
                            core.set_node_distance(node, category, distance);
                            model.transition_matrix(
                                distance,
                                &mut matrices[node]
                                    [category * states * states..(category + 1) * states * states],
                            );
                        }
                        if let Some((a, b)) = children[node] {
                            correction.calculate_partials(a, b, node, &core);
                        }
                    }
                    let mut expected = 0.0;
                    for code in 0..states.pow(tips as u32) {
                        let pattern = digits(code, tips, states);
                        if !informative(&pattern, states) {
                            continue;
                        }
                        let tip_partials: Vec<_> = pattern
                            .iter()
                            .map(|&state| {
                                (0..states)
                                    .map(|s| if s == state { 1.0 } else { 0.0 })
                                    .collect()
                            })
                            .collect();
                        let oracle = Problem {
                            children: &children,
                            root,
                            tip_partials: &tip_partials,
                            matrices: &matrices,
                            category_weights: &weights,
                            frequencies: model.frequencies(),
                            pattern_weights: &[1.0],
                        };
                        expected += oracle.log_likelihood().exp();
                    }
                    let actual = correction.log_probability(root, &weights);
                    if tips < 4 {
                        assert_eq!(actual, Err(Error::UndefinedInformativeCorrection));
                    } else {
                        let probability = actual.unwrap().exp();
                        assert!(
                            (probability - expected).abs() < 3e-13,
                            "k={states}, n={tips}, balanced={balanced}, scale={scale}: {probability} != {expected}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn zero_length_and_large_trees_do_not_produce_spurious_corrections() {
    for tips in [4usize, 90, 500] {
        for states in [2usize, 4, 32] {
            let children = children(tips, true);
            let root = children.len() - 1;
            let model = Mk::uniform(states);
            for distance in [0.0, 1e-12, 1000.0] {
                let mut core = LikelihoodCore::mk(&model, 1, 1, children.len(), Scaling::Always);
                let mut correction = InformativeProbability::new(states, tips, 1);
                for (node, pair) in children.iter().enumerate() {
                    core.set_node_distance(node, 0, distance);
                    if let Some((a, b)) = pair {
                        correction.calculate_partials(*a, *b, node, &core);
                    }
                }
                let actual = correction.log_probability(root, &[1.0]);
                if distance == 0.0 {
                    assert_eq!(actual, Err(Error::UndefinedInformativeCorrection));
                } else if distance < 1e-10 {
                    // An internal-branch change can already make a pattern informative.
                    // Any resolved retained mass is bounded by the chance of any change.
                    if let Ok(log_probability) = actual {
                        assert!(log_probability.exp() < 2.0 * children.len() as f64 * distance);
                    }
                } else if tips == 4 {
                    // Independent uniform tips: only the three pairings of two states survive.
                    let expected = (3.0 * (states - 1) as f64 / (states as f64).powi(3)).ln();
                    assert!((actual.unwrap() - expected).abs() < 2e-11);
                } else {
                    // With at least 90 independent tips, excluded mass is below 1e-20.
                    assert!(actual.unwrap().abs() < 1e-20);
                }
            }
        }
    }
}

#[test]
fn complete_patterns_are_classified_by_two_repeated_states() {
    for states in 2usize..=4 {
        for tips in 2..=5 {
            for code in 0..states.pow(tips as u32) {
                let pattern = digits(code, tips, states);
                let data = patterns(states, &[pattern.iter().map(usize::to_string).collect()]);
                assert_eq!(
                    validate_informative(&data).is_ok(),
                    informative(&pattern, states),
                    "{pattern:?}"
                );
            }
        }
    }
    // More than a single exceptional tip can still be uninformative.
    assert!(
        validate_informative(&patterns(
            4,
            &[vec!["0", "0", "1", "2", "3"]
                .into_iter()
                .map(str::to_owned)
                .collect()]
        ))
        .is_err()
    );
}

#[test]
fn uncertainty_is_accepted_only_if_every_completion_is_informative() {
    let mut random = rove::random::Random::new(120);
    for _ in 0..240 {
        let states = 3usize;
        let masks: Vec<_> = (0..5)
            .map(|_| 1 + (random.uniform() * 7.0) as usize)
            .collect();
        let column = masks
            .iter()
            .map(|&mask| {
                if mask == 7 {
                    "?".to_owned()
                } else {
                    let set: String = (0..states)
                        .filter(|&s| mask & (1 << s) != 0)
                        .map(|s| char::from_digit(s as u32, 10).unwrap())
                        .collect();
                    if set.len() == 1 {
                        set
                    } else {
                        format!("{{{set}}}")
                    }
                }
            })
            .collect();
        let all_informative = (0..states.pow(5))
            .map(|code| digits(code, 5, states))
            .filter(|p| p.iter().zip(&masks).all(|(&s, &m)| m & (1 << s) != 0))
            .all(|p| informative(&p, states));
        assert_eq!(
            validate_informative(&patterns(states, &[column])).is_ok(),
            all_informative,
            "{masks:?}"
        );
    }
    let good = vec!["0", "0", "1", "1", "?"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert!(validate_informative(&patterns(3, &[good])).is_ok());
}

#[test]
fn rejected_moves_and_checkpoints_preserve_the_correction() {
    let columns = [
        ["0", "0", "1", "1", "2", "2"],
        ["0", "1", "0", "1", "2", "3"],
        ["0", "1", "2", "0", "1", "2"],
        ["0", "0", "1", "1", "?", "{23}"],
        ["0", "0", "1", "1", "2", "2"],
    ]
    .map(|column| column.into_iter().map(str::to_owned).collect());
    let patterns = patterns(4, &columns);
    for backend in [Backend::Portable, Backend::NATIVE] {
        for relaxed in [false, true] {
            let settings = Settings {
                conditioning: Conditioning::Informative,
                backend,
                tuning_delay: 30,
                ..if relaxed {
                    Settings::reference()
                } else {
                    Settings::default()
                }
            };
            let mut sampler =
                Sampler::new(std::slice::from_ref(&patterns), &settings, 1401).unwrap();
            let mut accepted = 0;
            let mut rejected = 0;
            for _ in 0..300 {
                if sampler.step().1 {
                    accepted += 1;
                } else {
                    rejected += 1;
                }
                let expected = sampler.recompute().unwrap().likelihood;
                assert!((sampler.posterior().likelihood - expected).abs() < 1e-10);
            }
            assert!(accepted > 0 && rejected > 0);
            let bytes = sampler.checkpoint(&settings).unwrap();
            let mut resumed =
                Sampler::resume(std::slice::from_ref(&patterns), &settings, &bytes).unwrap();
            for _ in 0..300 {
                assert_eq!(sampler.step(), resumed.step());
                assert_eq!(sampler.posterior(), resumed.posterior());
                assert_eq!(sampler.tree(), resumed.tree());
            }
        }
    }
}

#[test]
fn fixed_states_match_beast_with_all_excluded_columns() {
    let archived: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/validation/informative/beast-likelihoods.json"
    ))
    .unwrap();
    let parts = nexus::parse(include_str!(
        "../fixtures/validation/informative/matrix.nex"
    ))
    .unwrap()
    .partition_by_states(GapTreatment::Reject)
    .unwrap();
    let mut worst = 0.0_f64;
    for backend in [Backend::Portable, Backend::NATIVE] {
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
                conditioning: Conditioning::Informative,
                backend,
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
    }
    eprintln!("largest informative log-likelihood difference from BEAST: {worst:e}");
}

#[test]
fn native_and_portable_chains_agree_step_by_step() {
    let parts = nexus::parse(include_str!(
        "../fixtures/validation/informative/matrix.nex"
    ))
    .unwrap()
    .partition_by_states(GapTreatment::Reject)
    .unwrap();
    let settings = Settings {
        conditioning: Conditioning::Informative,
        backend: Backend::Portable,
        ..Settings::reference()
    };
    let mut portable = Sampler::new(&parts, &settings, 111).unwrap();
    let mut native = Sampler::new(
        &parts,
        &Settings {
            backend: Backend::NATIVE,
            ..settings
        },
        111,
    )
    .unwrap();
    for _ in 0..1000 {
        assert_eq!(portable.step(), native.step());
        assert_eq!(portable.posterior(), native.posterior());
        assert_eq!(portable.tree(), native.tree());
    }
}
