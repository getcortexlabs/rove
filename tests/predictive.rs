use rove::data::{Cell, GapTreatment, Matrix};
use rove::mcmc::Conditioning;
use rove::nexus;
use rove::predictive::{Design, TailCount, to_nexus};
use rove::rates::{GammaRates, RateVariation, RelaxedClock};
use rove::tree::Tree;

fn quartet(pair: usize) -> Tree {
    let pairs = [[(0, 1), (2, 3)], [(0, 2), (1, 3)], [(0, 3), (1, 2)]];
    Tree::new(
        vec![
            None,
            None,
            None,
            None,
            Some(pairs[pair][0]),
            Some(pairs[pair][1]),
            Some((4, 5)),
        ],
        vec![0.0, 0.0, 0.0, 0.0, 0.13, 0.21, 0.45],
    )
    .unwrap()
}
fn data(rows: &[String], symbols: &str, ordered: bool) -> Matrix {
    let mut text = format!(
        "#NEXUS BEGIN DATA; DIMENSIONS NTAX={} NCHAR={}; FORMAT SYMBOLS=\"{symbols}\" MISSING=? GAP=-; MATRIX ",
        rows.len(),
        rows[0].len()
    );
    for (i, row) in rows.iter().enumerate() {
        text.push_str(&format!("T{i} {row}\n"));
    }
    text.push_str("; END;");
    if ordered {
        text.push_str(" BEGIN ASSUMPTIONS; OPTIONS DEFTYPE=ORD; END;");
    }
    nexus::parse(&text).unwrap()
}
fn design(matrix: &Matrix, conditioning: Conditioning) -> Design {
    let partitions = matrix
        .compress(
            std::slice::from_ref(&(0..matrix.character_count())),
            GapTreatment::Missing,
        )
        .unwrap();
    Design::new(matrix, &partitions, GapTreatment::Missing, conditioning).unwrap()
}

// Independent scalar closed forms; enumerate all ancestral states instead of
// using the pruning kernel, transition implementations or simulation code.
fn transition(k: usize, ordered: bool, a: usize, b: usize, t: f64) -> f64 {
    if !ordered {
        let e = (-t * k as f64 / (k - 1) as f64).exp();
        return (1.0 - e) / k as f64 + if a == b { e } else { 0.0 };
    }
    assert_eq!(k, 3);
    let x = (-0.75 * t).exp();
    let y = (-2.25 * t).exp();
    let p = [
        [
            1.0 / 3.0 + x / 2.0 + y / 6.0,
            (1.0 - y) / 3.0,
            1.0 / 3.0 - x / 2.0 + y / 6.0,
        ],
        [(1.0 - y) / 3.0, (1.0 + 2.0 * y) / 3.0, (1.0 - y) / 3.0],
        [
            1.0 / 3.0 - x / 2.0 + y / 6.0,
            (1.0 - y) / 3.0,
            1.0 / 3.0 + x / 2.0 + y / 6.0,
        ],
    ];
    p[a][b]
}
fn probability(tips: &[usize], k: usize, ordered: bool, tree: &Tree, rates: &RateVariation) -> f64 {
    let mut total = 0.0;
    for &category in rates.category_rates() {
        for root in 0..k {
            for left in 0..k {
                for right in 0..k {
                    let states = [tips[0], tips[1], tips[2], tips[3], left, right, root];
                    let mut value = 1.0 / k as f64;
                    for node in 0..6 {
                        value *= transition(
                            k,
                            ordered,
                            states[tree.parent(node).unwrap()],
                            states[node],
                            tree.branch_length(node)
                                * rates.branch_rate(node, tree.root())
                                * category,
                        );
                    }
                    total += value / rates.category_rates().len() as f64;
                }
            }
        }
    }
    total
}
fn digits(mut code: usize, base: usize) -> Vec<usize> {
    (0..4)
        .map(|_| {
            let n = code % base;
            code /= base;
            n
        })
        .collect()
}

#[test]
fn simulated_patterns_match_closed_form_probabilities_and_conditioned_mixtures() {
    let tree = quartet(0);
    for (k, ordered, conditioning, mixture) in [
        (2usize, false, Conditioning::None, false),
        (2, false, Conditioning::Variable, true),
        (2, false, Conditioning::Informative, true),
        (3, true, Conditioning::None, false),
        (3, true, Conditioning::Variable, true),
    ] {
        let rates = if mixture {
            RateVariation {
                gamma: Some(GammaRates::new(0.35).unwrap()),
                clock: Some(RelaxedClock::new(0.7, vec![0.3, 1.2, 2.0, 0.7, 1.5, 0.8]).unwrap()),
            }
        } else {
            RateVariation::default()
        };
        let n = 24_000;
        let matrix = data(
            &["0".repeat(n), "0".repeat(n), "1".repeat(n), "1".repeat(n)],
            if k == 2 { "01" } else { "012" },
            ordered,
        );
        let design = design(&matrix, conditioning);
        let simulated = design
            .simulate(&tree, &rates, 9102, 100_000, &|| false)
            .unwrap();
        let mut counts = vec![0usize; k.pow(4)];
        for site in 0..n {
            let mut code = 0;
            let mut power = 1;
            for row in simulated.rows() {
                let Cell::State(state) = row[site] else {
                    panic!()
                };
                code += state * power;
                power *= k;
            }
            counts[code] += 1;
        }
        let probabilities: Vec<_> = (0..k.pow(4))
            .map(|code| {
                let states = digits(code, k);
                let mut counts = vec![0; k];
                for &s in &states {
                    counts[s] += 1;
                }
                let keep = match conditioning {
                    Conditioning::None => true,
                    Conditioning::Variable => counts.iter().filter(|&&n| n > 0).count() > 1,
                    Conditioning::Informative => counts.iter().filter(|&&n| n >= 2).count() >= 2,
                };
                if keep {
                    probability(&states, k, ordered, &tree, &rates)
                } else {
                    0.0
                }
            })
            .collect();
        let normalization: f64 = probabilities.iter().sum();
        if conditioning == Conditioning::None {
            assert!((normalization - 1.0).abs() < 2e-14);
        }
        for (code, (count, p)) in counts.iter().zip(probabilities).enumerate() {
            let p = p / normalization;
            if p == 0.0 {
                assert_eq!(*count, 0);
                continue;
            }
            let tolerance = 6.0 * (p * (1.0 - p) / n as f64).sqrt() + 1.0 / n as f64;
            assert!(
                (*count as f64 / n as f64 - p).abs() < tolerance,
                "{k} {ordered} {conditioning:?} pattern {code}: {count} vs {p}"
            );
        }
    }
}

#[test]
fn missing_masks_follow_latent_ascertainment_and_do_not_change_the_denominator() {
    let n = 24_000;
    let matrix = data(
        &["0".repeat(n), "?".repeat(n), "1".repeat(n), "1".repeat(n)],
        "01",
        false,
    );
    let design = design(&matrix, Conditioning::Variable);
    let tree = quartet(0);
    let rates = RateVariation::default();
    let simulated = design
        .simulate(&tree, &rates, 22, 100_000, &|| false)
        .unwrap();
    let mut observed = [0usize; 8];
    for site in 0..n {
        assert_eq!(simulated.rows()[1][site], Cell::Missing);
        let mut code = 0;
        for (bit, &tip) in [0, 2, 3].iter().enumerate() {
            let Cell::State(s) = simulated.rows()[tip][site] else {
                panic!()
            };
            code += s << bit;
        }
        observed[code] += 1;
    }
    let mut expected = [0.0; 8];
    for code in 1..15 {
        let s = digits(code, 2);
        expected[s[0] + 2 * s[2] + 4 * s[3]] += probability(&s, 2, false, &tree, &rates);
    }
    let sum: f64 = expected.iter().sum();
    for (count, p) in observed.into_iter().zip(expected) {
        let p = p / sum;
        assert!(
            (count as f64 / n as f64 - p).abs()
                < 6.0 * (p * (1.0 - p) / n as f64).sqrt() + 1.0 / n as f64
        );
    }
    assert!(
        observed[0] > 0 && observed[7] > 0,
        "constant masked observations must remain possible"
    );
}

fn brute_steps(tips: &[usize], k: usize, ordered: bool, tree: &Tree) -> usize {
    let mut best = usize::MAX;
    for root in 0..k {
        for a in 0..k {
            for b in 0..k {
                let s = [tips[0], tips[1], tips[2], tips[3], a, b, root];
                let score = (0..6)
                    .map(|i| {
                        let x = s[i];
                        let y = s[tree.parent(i).unwrap()];
                        if x == k {
                            0
                        } else if ordered {
                            x.abs_diff(y)
                        } else {
                            usize::from(x != y)
                        }
                    })
                    .sum();
                best = best.min(score);
            }
        }
    }
    best
}

#[test]
fn sankoff_indices_and_pattern_fit_match_exhaustive_quartets() {
    for ordered in [false, true] {
        let k = 3usize;
        let mut rows = vec![String::new(); 4];
        for code in 0..(k + 1).pow(4) {
            for (row, s) in rows.iter_mut().zip(digits(code, k + 1)) {
                row.push(if s == k {
                    '?'
                } else {
                    char::from(b'0' + s as u8)
                });
            }
        }
        let matrix = data(&rows, "012", ordered);
        let design = design(&matrix, Conditioning::None);
        for pair in 0..3 {
            let tree = quartet(pair);
            let statistics = design
                .statistics(&matrix, &tree, &RateVariation::default(), &|| false)
                .unwrap();
            for (code, score) in statistics.characters.iter().enumerate() {
                let tips = digits(code, k + 1);
                let lengths: Vec<_> = (0..3)
                    .map(|p| brute_steps(&tips, k, ordered, &quartet(p)))
                    .collect();
                assert_eq!(score.steps, lengths[pair]);
                assert_eq!(score.minimum, *lengths.iter().min().unwrap());
                assert_eq!(score.maximum, *lengths.iter().max().unwrap());
                assert_eq!(
                    score.informative,
                    lengths.iter().min() != lengths.iter().max()
                );
                assert!(score.consistency().is_none_or(|v| (0.0..=1.0).contains(&v)));
                assert!(score.retention().is_none_or(|v| (0.0..=1.0).contains(&v)));
                // Sum all full patterns compatible with the missing mask.
                let p: f64 = (0..k.pow(4))
                    .map(|candidate| digits(candidate, k))
                    .filter(|s| s.iter().zip(&tips).all(|(a, b)| *b == k || a == b))
                    .map(|s| probability(&s, k, ordered, &tree, &RateVariation::default()))
                    .sum();
                assert!((score.surprise + p.ln()).abs() < 2e-13);
            }
        }
    }
}

#[test]
fn frozen_alphabets_labels_ordering_and_missing_cells_survive_roundtrip() {
    let matrix=nexus::parse("#NEXUS BEGIN DATA; DIMENSIONS NTAX=4 NCHAR=4; FORMAT SYMBOLS=\"0123\" MISSING=? GAP=-; CHARSTATELABELS 1 'body size' / small medium large, 3 'O''Brien'; STATELABELS 2 absent present; MATRIX 'A_A' 020? B 032- C 201? D 233-; END; BEGIN ASSUMPTIONS; TYPESET * test = ORD: 2; END;").unwrap();
    assert_eq!(nexus::parse(&to_nexus(&matrix)).unwrap(), matrix);
    let parts = matrix
        .compress(&[0..2, 2..4], GapTreatment::Missing)
        .unwrap();
    let fitted = Design::new(&matrix, &parts, GapTreatment::Missing, Conditioning::None).unwrap();
    let zero = Tree::new(
        vec![
            None,
            None,
            None,
            None,
            Some((0, 1)),
            Some((2, 3)),
            Some((4, 5)),
        ],
        vec![0.0; 7],
    )
    .unwrap();
    let a = fitted
        .simulate(&zero, &RateVariation::default(), 7, 100, &|| false)
        .unwrap();
    let b = fitted
        .simulate(&zero, &RateVariation::default(), 7, 100, &|| false)
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(a.ordering(), matrix.ordering());
    assert_eq!(a.character_labels(), matrix.character_labels());
    assert_eq!(a.state_labels(), matrix.state_labels());
    assert!(
        fitted
            .compress(&a)
            .unwrap()
            .iter()
            .all(|p| p.symbols().len() == 4)
    );
    for (a, b) in a.rows().iter().zip(matrix.rows()) {
        for (a, b) in a.iter().zip(b) {
            if matches!(b, Cell::Missing | Cell::Gap) {
                assert_eq!(a, b);
            }
        }
    }
    let complete = data(
        &["00".into(), "00".into(), "11".into(), "11".into()],
        "01",
        false,
    );
    let conditioned = design(&complete, Conditioning::Variable);
    assert!(
        conditioned
            .simulate(&zero, &RateVariation::default(), 1, 8, &|| false)
            .unwrap_err()
            .to_string()
            .contains("attempts")
    );
    assert!(
        conditioned
            .simulate(&quartet(0), &RateVariation::default(), 1, 100, &|| true)
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
    let uncertain=nexus::parse("#NEXUS BEGIN DATA; DIMENSIONS NTAX=2 NCHAR=1; FORMAT SYMBOLS=\"01\"; MATRIX A {01} B 1; END;").unwrap();
    assert!(
        Design::new(
            &uncertain,
            &uncertain
                .compress(std::slice::from_ref(&(0..1)), GapTreatment::Reject)
                .unwrap(),
            GapTreatment::Reject,
            Conditioning::None
        )
        .is_err()
    );
}

#[test]
fn observed_recoding_is_frozen_per_character_including_ordered_intermediate_states() {
    let matrix = nexus::parse("#NEXUS BEGIN DATA; DIMENSIONS NTAX=4 NCHAR=3; FORMAT SYMBOLS=\"0123\"; MATRIX A 000 B 130 C 032 D 132; END; BEGIN ASSUMPTIONS; TYPESET * test = ORD: 3; END;").unwrap();
    let parts = matrix.partition_by_states(GapTreatment::Reject).unwrap();
    let design = Design::new(&matrix, &parts, GapTreatment::Reject, Conditioning::None).unwrap();
    let zero = Tree::new(
        vec![
            None,
            None,
            None,
            None,
            Some((0, 1)),
            Some((2, 3)),
            Some((4, 5)),
        ],
        vec![0.0; 7],
    )
    .unwrap();
    let mut intermediate = false;
    for seed in 0..64 {
        let replica = design
            .simulate(&zero, &RateVariation::default(), seed, 10, &|| false)
            .unwrap();
        for row in replica.rows() {
            assert!(matches!(row[0], Cell::State(0 | 1)));
            assert!(matches!(row[1], Cell::State(0 | 3)));
            assert!(matches!(row[2], Cell::State(0..=2)));
            intermediate |= row[2] == Cell::State(1);
        }
        let p = design.compress(&replica).unwrap();
        assert_eq!(
            p.iter().map(|p| p.symbols().len()).collect::<Vec<_>>(),
            [2, 3, 2]
        );
        assert!(
            Design::new(
                &replica,
                &parts,
                GapTreatment::Reject,
                Conditioning::Variable
            )
            .is_err()
        );
    }
    assert!(intermediate);
}

#[test]
fn inclusive_tails_report_ties_undefined_indices_and_finite_simulation_intervals() {
    let mut count = TailCount::default();
    for value in [1.0, 2.0, 2.0, 3.0] {
        count.add(Some(2.0), Some(value));
    }
    count.add(None, Some(2.0));
    count.add(Some(2.0), None);
    assert_eq!(
        (count.pairs, count.lower, count.upper, count.ties),
        (4, 3, 3, 2)
    );
    assert_eq!(count.probability(count.upper), Some(0.75));
    let empty = TailCount::default();
    assert_eq!(empty.interval(0), None);
    let mut zero = TailCount::default();
    for _ in 0..10 {
        zero.add(Some(1.0), Some(0.0));
    }
    let interval = zero.interval(0).unwrap();
    assert!(interval[0] < 1e-15);
    assert!((interval[1] - 0.2775327998628892).abs() < 1e-12);
}
