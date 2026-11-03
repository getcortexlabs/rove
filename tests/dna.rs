use rove::alignment;
use rove::data::{Cell, CharacterSet, DataType, GapTreatment, Matrix};
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::predictive::Design;
use rove::rates::RateVariation;
use rove::tree::Tree;
use rove::{Backend, nexus};

const FASTA: &str = include_str!("../fixtures/validation/dna/matrix.fasta");
const NEXUS: &str = include_str!("../fixtures/validation/dna/matrix.nex");

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
fn aligned_formats_preserve_iupac_identity_and_the_complete_alphabet() {
    let fasta = alignment::parse(FASTA).unwrap();
    let nexus = alignment::parse(NEXUS).unwrap();
    assert_eq!(fasta, nexus);
    assert_eq!(fasta.data_type(), DataType::Dna);
    assert_eq!(fasta.taxa()[0], "Alpha one");
    assert_eq!(fasta.rows()[0][5], Cell::Uncertain(vec![0, 2]));
    assert_eq!(fasta.rows()[0][6], Cell::Uncertain(vec![1, 3]));
    assert_eq!(fasta.rows()[0][7], Cell::Uncertain(vec![0, 1, 2, 3]));
    assert_eq!(fasta.rows()[0][8], Cell::Missing);
    assert_eq!(fasta.rows()[0][9], Cell::Gap);
    assert!(fasta.partition_by_states(GapTreatment::Reject).is_err());
    let parts = fasta.partition_by_states(GapTreatment::Missing).unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].symbols(), ['A', 'C', 'G', 'T']);
    assert_eq!(parts[0].weights().iter().sum::<usize>(), 12);
    // The first, constant-A column still has four states in the model.
    assert!(
        parts[0]
            .state_symbols()
            .iter()
            .all(|s| s == &['A', 'C', 'G', 'T'])
    );
    assert!(
        fasta
            .compress_sets(
                &[CharacterSet {
                    sites: (0..12).collect(),
                    symbols: vec!['A', 'C']
                }],
                GapTreatment::Missing
            )
            .is_err()
    );
    assert!(fasta.partition_by_labels(GapTreatment::Missing).is_err());
    assert_eq!(
        nexus::parse(&rove::predictive::to_nexus(&fasta)).unwrap(),
        fasta
    );
    assert!(Sampler::new(&parts, &Settings::default(), 1).is_err());
    let every = alignment::parse(">A\nacgtryswkmbdhvn\n>B\nACGTRYSWKMBDHVN\n").unwrap();
    assert_eq!(every.rows()[0], every.rows()[1]);
    for (i, states) in [
        vec![0, 2],
        vec![1, 3],
        vec![1, 2],
        vec![0, 3],
        vec![2, 3],
        vec![0, 1],
        vec![1, 2, 3],
        vec![0, 2, 3],
        vec![0, 1, 3],
        vec![0, 1, 2],
        vec![0, 1, 2, 3],
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(every.rows()[0][i + 4], Cell::Uncertain(states));
    }
    let missing_n = NEXUS.replace("MISSING=?", "MISSING=N").replace("?", "N");
    assert_eq!(
        nexus::parse(&missing_n).unwrap().rows()[0][7],
        Cell::Missing
    );
}

#[test]
fn malformed_alignments_and_unsupported_models_are_rejected() {
    for text in [
        ">a\nAC\n>b\nA\n",
        ">a\n>b\nAC\n",
        ">\nAC\n",
        ">a\nAC\n>A other\nAG\n",
        ">a\nACU\n",
        ">___\nAC\n",
        ">a\nAC.\n",
        ">a\nAC0\n",
    ] {
        assert!(alignment::parse(text).is_err(), "{text}");
    }
    for option in [
        "DATATYPE=RNA",
        "DATATYPE=PROTEIN",
        "DATATYPE=DNA SYMBOLS=\"AG\"",
        "DATATYPE=DNA RESPECTCASE",
        "DATATYPE=DNA GAP=R",
        "DATATYPE=DNA MISSING=A",
    ] {
        let text = NEXUS.replace("DATATYPE=DNA", option);
        assert!(nexus::parse(&text).is_err(), "{option}");
    }
    assert!(
        nexus::parse(&(NEXUS.to_owned() + "BEGIN ASSUMPTIONS; OPTIONS DEFTYPE=ORD; END;")).is_err()
    );
    let data = alignment::parse(FASTA).unwrap();
    let parts = data.partition_by_states(GapTreatment::Missing).unwrap();
    let error = Design::new(&data, &parts, GapTreatment::Missing, Conditioning::None)
        .err()
        .unwrap();
    assert!(error.to_string().contains("observation model"));
}

/// Sum all internal states explicitly, using the analytical JC69 transition
/// probabilities. This does not use pruning, the kernel or Mk's transition code.
fn brute_force(matrix: &Matrix, tree: &Tree) -> f64 {
    let transition = |a, b, distance: f64| {
        let decay = (-4.0 * distance / 3.0).exp();
        if a == b {
            0.25 + 0.75 * decay
        } else {
            0.25 - 0.25 * decay
        }
    };
    let mut log_likelihood = 0.0;
    for site in 0..matrix.character_count() {
        let mut probability = 0.0;
        for code in 0..4usize.pow((tree.tips() - 1) as u32) {
            let mut remaining = code;
            let mut states = vec![0; tree.nodes()];
            for state in &mut states[tree.tips()..] {
                *state = remaining % 4;
                remaining /= 4;
            }
            let mut term = 0.25;
            for node in 0..tree.nodes() {
                let Some(parent) = tree.parent(node) else {
                    continue;
                };
                let distance = tree.branch_length(node);
                term *= if node >= tree.tips() {
                    transition(states[parent], states[node], distance)
                } else {
                    (0..4)
                        .filter(|s| match &matrix.rows()[node][site] {
                            Cell::State(x) => x == s,
                            Cell::Uncertain(xs) => xs.contains(s),
                            Cell::Missing | Cell::Gap => true,
                            Cell::Polymorphic(_) => unreachable!(),
                        })
                        .map(|s| transition(states[parent], s, distance))
                        .sum::<f64>()
                };
            }
            probability += term;
        }
        log_likelihood += probability.ln();
    }
    log_likelihood
}

#[test]
fn fixed_states_match_analytical_enumeration_and_pinned_beast() {
    let data = alignment::parse(FASTA).unwrap();
    let patterns = data.partition_by_states(GapTreatment::Missing).unwrap();
    for row in include_str!("../fixtures/validation/dna/beast-2.7.8.tsv")
        .lines()
        .skip(1)
    {
        let values: Vec<f64> = row
            .split('\t')
            .take(4)
            .map(|x| x.parse().unwrap())
            .collect();
        let tree = ladder(5, values[0]);
        for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
            let settings = Settings {
                conditioning: Conditioning::None,
                gamma_shape: (values[1] == 4.0).then_some(values[2]),
                backend,
                ..Settings::default()
            };
            let chain = Sampler::with_tree(&patterns, &settings, tree.clone(), 1).unwrap();
            let actual = chain.posterior().likelihood;
            // Use BEAST's actual category rates to isolate pruning from the
            // known AS91 inverse-gamma approximation difference (rates.rs).
            let reference_rates: Vec<f64> = row
                .split('\t')
                .nth(4)
                .unwrap()
                .split(',')
                .map(|x| x.parse().unwrap())
                .collect();
            let partition = &patterns[0];
            let mut core = rove::LikelihoodCore::mk(
                &rove::model::Mk::uniform(4),
                partition.weights().len(),
                reference_rates.len(),
                tree.nodes(),
                rove::Scaling::Always,
            );
            core.set_backend(backend);
            for (tip, partials) in partition.tip_partials().iter().enumerate() {
                core.set_tip_partials(tip, partials);
            }
            for node in tree.postorder() {
                for (category, rate) in reference_rates.iter().enumerate() {
                    core.set_node_distance(node, category, tree.branch_length(node) * rate);
                }
                if let Some((a, b)) = tree.children(node) {
                    core.calculate_partials(a, b, node);
                }
            }
            core.integrate_partials(
                tree.root(),
                &vec![1.0 / reference_rates.len() as f64; reference_rates.len()],
            );
            let exact: f64 = core
                .calculate_log_likelihoods(&[0.25; 4])
                .iter()
                .zip(partition.weights())
                .map(|(ll, weight)| ll * *weight as f64)
                .sum();
            assert!(
                (exact - values[3]).abs() < 1e-10,
                "fixed category rates: {row}: {exact}"
            );
            if values[1] == 4.0 {
                for (actual, expected) in chain
                    .rates()
                    .gamma
                    .as_ref()
                    .unwrap()
                    .rates()
                    .iter()
                    .zip(reference_rates)
                {
                    assert!((actual - expected).abs() < 3e-8);
                }
            }
            assert!(
                (actual - values[3]).abs() < if values[1] == 1.0 { 1e-10 } else { 1e-7 },
                "{row}: {actual}"
            );
            if values[1] == 1.0 {
                assert!((actual - brute_force(&data, &tree)).abs() < 1e-10);
            }
        }
    }
}

#[test]
fn relaxed_dna_cache_and_checkpoint_keep_exact_model_identity() {
    let data = alignment::parse(FASTA).unwrap();
    let patterns = data.partition_by_states(GapTreatment::Missing).unwrap();
    let settings = Settings {
        conditioning: Conditioning::None,
        tuning_delay: 10,
        monophyly: vec![vec![0, 1]],
        ..Settings::reference()
    };
    let mut chain = Sampler::new(&patterns, &settings, 932).unwrap();
    for i in 0..1000 {
        chain.step();
        if i % 20 == 0 {
            assert!((chain.posterior().total() - chain.recompute().unwrap().total()).abs() < 1e-10);
        }
    }
    let checkpoint = chain.checkpoint(&settings).unwrap();
    let mut resumed = Sampler::resume(&patterns, &settings, &checkpoint).unwrap();
    for _ in 0..500 {
        assert_eq!(chain.step(), resumed.step());
        assert_eq!(chain.posterior(), resumed.posterior());
        assert_eq!(chain.tree(), resumed.tree());
        assert_eq!(chain.rates(), resumed.rates());
    }
    let standard =
        nexus::parse(&NEXUS.replace("DATATYPE=DNA", "DATATYPE=STANDARD SYMBOLS=\"ACGTRYN\""))
            .unwrap();
    let other = standard
        .compress(std::slice::from_ref(&(0..12)), GapTreatment::Missing)
        .unwrap();
    assert!(Sampler::resume(&other, &settings, &checkpoint).is_err());
}

#[test]
fn predictive_jc69_simulation_preserves_identity_and_expected_frequencies() {
    let sequence = "A".repeat(5000);
    let data = alignment::parse(&format!(">A\n{sequence}\n>B\n{sequence}\n")).unwrap();
    let patterns = data.partition_by_states(GapTreatment::Reject).unwrap();
    let design = Design::new(&data, &patterns, GapTreatment::Reject, Conditioning::None).unwrap();
    let tree = ladder(2, 0.25);
    let simulated = design
        .simulate(&tree, &RateVariation::default(), 82, 1, &|| false)
        .unwrap();
    assert_eq!(simulated.data_type(), DataType::Dna);
    let mut counts = [0; 4];
    let mut same = 0;
    for site in 0..5000 {
        let Cell::State(state) = simulated.rows()[0][site] else {
            panic!("non-base")
        };
        counts[state] += 1;
        same += usize::from(simulated.rows()[0][site] == simulated.rows()[1][site]);
    }
    for count in counts {
        assert!((count as f64 / 5000.0 - 0.25).abs() < 0.025);
    }
    let expected = 0.25 + 0.75 * (-4.0f64 * 0.5 / 3.0).exp();
    assert!((same as f64 / 5000.0 - expected).abs() < 0.025);
    let text = rove::predictive::to_nexus(&simulated);
    assert!(text.contains("DATATYPE=DNA"));
    assert_eq!(nexus::parse(&text).unwrap(), simulated);
}
