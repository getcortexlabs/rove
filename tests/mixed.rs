use rove::{
    alignment,
    data::{DataType, GapTreatment},
    mcmc::{Conditioning, Move, RelativeRate, Sampler, Settings},
    mixed,
    predictive::Design,
    tree::Tree,
};
const MORPH: &str = include_str!("../fixtures/validation/mixed/morphology.nex");
const DNA: &str = include_str!("../fixtures/validation/dna/complete.fasta");
fn input(morph: &str, dna: &str, missing: &str) -> String {
    mixed::manifest(("morphology.nex", morph), ("dna.fasta", dna), missing).unwrap()
}
fn ladder(step: f64) -> Tree {
    Tree::new(
        vec![
            None,
            None,
            None,
            None,
            None,
            Some((0, 1)),
            Some((5, 2)),
            Some((6, 3)),
            Some((7, 4)),
        ],
        vec![
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            step,
            2.0 * step,
            3.0 * step,
            4.0 * step,
        ],
    )
    .unwrap()
}
#[test]
fn sources_match_names_preserve_blocks_and_validate_fingerprints() {
    let text = input(MORPH, DNA, "reject");
    let data = alignment::parse(&text).unwrap();
    assert_eq!(data.data_type(), DataType::Mixed);
    assert_eq!(data.character_count(), 18);
    let parts = data.partition_by_states(GapTreatment::Reject).unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].data_type(), DataType::Standard);
    assert_eq!(parts[1].data_type(), DataType::Dna);
    assert_eq!(parts[1].sites(), &(6..18).collect::<Vec<_>>());
    assert_eq!(parts[1].symbols(), ['A', 'C', 'G', 'T']);
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["sources"][0]["content"] = serde_json::json!(MORPH.replace("010101", "110101"));
    assert!(alignment::parse(&json.to_string()).is_err());
    let reversed = DNA
        .trim()
        .split('>')
        .filter(|x| !x.is_empty())
        .rev()
        .map(|s| format!(">{s}\n"))
        .collect::<String>();
    let reordered = alignment::parse(&input(MORPH, &reversed, "reject")).unwrap();
    assert_eq!(data.rows(), reordered.rows());
    let missing = DNA.split(">Epsilon_five").next().unwrap().to_owned();
    assert!(mixed::manifest(("m.nex", MORPH), ("d.fa", &missing), "reject").is_err());
    let union = alignment::parse(&input(MORPH, &missing, "missing")).unwrap();
    assert_eq!(
        union.mixed_metadata().unwrap().sources[1].missing_taxa,
        ["Epsilon five"]
    );
    assert!(
        union.rows()[4][6..]
            .iter()
            .all(|c| matches!(c, rove::data::Cell::Missing))
    );
    let wrong = DNA.replace("Alpha_one", "alpha_one");
    assert!(mixed::manifest(("m.nex", MORPH), ("d.fa", &wrong), "missing").is_err());
    assert!(Sampler::new(&parts, &Settings::default(), 1).is_err());
}
#[test]
fn joint_density_equals_independent_partitions_with_separate_conditioning() {
    let data = alignment::parse(&input(MORPH, DNA, "reject")).unwrap();
    let parts = data.partition_by_states(GapTreatment::Reject).unwrap();
    let components = data.component_matrices();
    let morph = components[0]
        .partition_by_states(GapTreatment::Reject)
        .unwrap();
    let dna = components[1]
        .partition_by_states(GapTreatment::Reject)
        .unwrap();
    for conditioning in [
        Conditioning::None,
        Conditioning::Variable,
        Conditioning::Informative,
    ] {
        for ratio in [0.1, 1.0, 3.5] {
            for reference in [false, true] {
                let mut settings = if reference {
                    Settings::reference()
                } else {
                    Settings::default()
                };
                settings.conditioning = conditioning;
                let a = Sampler::with_tree(&morph, &settings, ladder(0.1 * ratio), 7).unwrap();
                settings.conditioning = Conditioning::None;
                let b = Sampler::with_tree(&dna, &settings, ladder(0.1), 7).unwrap();
                settings.conditioning = conditioning;
                settings.set_morphology_rate(RelativeRate {
                    value: ratio,
                    log_sd: None,
                });
                let joint = Sampler::with_tree(&parts, &settings, ladder(0.1), 7).unwrap();
                assert!(
                    (joint.posterior().likelihood
                        - a.posterior().likelihood
                        - b.posterior().likelihood)
                        .abs()
                        < 1e-10
                );
            }
        }
    }
}
#[test]
fn relative_rate_cache_and_exact_checkpoint_replay() {
    let data = alignment::parse(&input(MORPH, DNA, "reject")).unwrap();
    let parts = data.partition_by_states(GapTreatment::Reject).unwrap();
    let mut settings = Settings::reference();
    settings.set_morphology_rate(RelativeRate {
        value: 1.0,
        log_sd: Some(0.8),
    });
    settings.monophyly = vec![vec![0, 1]];
    let mut sampler = Sampler::new(&parts, &settings, 42).unwrap();
    for _ in 0..2000 {
        sampler.step();
        assert!(
            (sampler.posterior().likelihood - sampler.recompute().unwrap().likelihood).abs() < 1e-9
        );
    }
    let saved = sampler.checkpoint(&settings).unwrap();
    let mut resumed = Sampler::resume(&parts, &settings, &saved).unwrap();
    for _ in 0..1000 {
        assert_eq!(sampler.step(), resumed.step());
        assert_eq!(sampler.posterior(), resumed.posterior());
        assert_eq!(sampler.morphology_rate(), resumed.morphology_rate());
    }
    settings.morphology_rate.as_mut().unwrap().log_sd = Some(0.9);
    assert!(Sampler::resume(&parts, &settings, &saved).is_err());
}
#[test]
fn normalized_relative_rate_prior_and_scale_stationarity() {
    let data = alignment::parse(&input(MORPH, DNA, "reject")).unwrap();
    let parts = data.partition_by_states(GapTreatment::Reject).unwrap();
    let rate = RelativeRate {
        value: 1.0,
        log_sd: Some(0.7),
    };
    let h = 12.0 / 10000.0;
    let integral: f64 = (0..10000)
        .map(|i| {
            let z = -6.0 + (i as f64 + 0.5) * h;
            (rate.log_density(z.exp()) + z).exp() * h
        })
        .sum();
    assert!((integral - 1.0).abs() < 1e-12);
    let mut settings = Settings {
        sample_from_prior: true,
        tuning: false,
        ..Settings::default()
    };
    settings.set_morphology_rate(rate);
    settings.weights = vec![(Move::MorphologyRateScale, 1.0)];
    settings.tuning_values = vec![(Move::MorphologyRateScale, 0.35)];
    let mut sampler = Sampler::new(&parts, &settings, 78).unwrap();
    let (mut sum, mut squares) = (0.0, 0.0);
    for i in 0..110000 {
        sampler.step();
        if i >= 10000 {
            let z = sampler.morphology_rate().unwrap().ln();
            sum += z;
            squares += z * z;
        }
    }
    assert!(
        (sum / 100000.0).abs() < 0.035,
        "mean={} second={}",
        sum / 100000.0,
        squares / 100000.0
    );
    assert!((squares / 100000.0 - 0.49).abs() < 0.035);
}
#[test]
fn prediction_preserves_models_masks_and_roundtrip_identity() {
    let missing = DNA.split(">Epsilon_five").next().unwrap().to_owned();
    let data = alignment::parse(&input(MORPH, &missing, "missing")).unwrap();
    let parts = data.partition_by_states(GapTreatment::Reject).unwrap();
    let mut design =
        Design::new(&data, &parts, GapTreatment::Reject, Conditioning::Variable).unwrap();
    design.set_morphology_rate(2.5).unwrap();
    let tree = ladder(0.05);
    let rates = rove::rates::RateVariation::default();
    let simulated = design
        .simulate(&tree, &rates, 23, 100000, &|| false)
        .unwrap();
    let text = rove::predictive::to_input(&simulated);
    let roundtrip = alignment::parse(&text).unwrap();
    assert_eq!(roundtrip.rows(), simulated.rows());
    assert_eq!(
        roundtrip.mixed_metadata().unwrap().sources[1].missing_taxa,
        ["Epsilon five"]
    );
    assert_eq!(
        design.compress(&roundtrip).unwrap()[1].symbols(),
        ['A', 'C', 'G', 'T']
    );
    design
        .statistics(&roundtrip, &tree, &rates, &|| false)
        .unwrap();
}

#[test]
fn joint_states_match_pinned_beast_with_each_ascertainment_model() {
    let data = alignment::parse(&input(MORPH, DNA, "reject")).unwrap();
    let parts = data.partition_by_states(GapTreatment::Reject).unwrap();
    for line in include_str!("../fixtures/validation/mixed/beast-2.7.8.tsv")
        .lines()
        .skip(1)
    {
        let values: Vec<f64> = line.split('\t').map(|v| v.parse().unwrap()).collect();
        let conditioning = [
            Conditioning::None,
            Conditioning::Variable,
            Conditioning::Informative,
        ][values[2] as usize];
        for backend in rove::Backend::ALL.into_iter().filter(|b| b.is_available()) {
            let mut settings = Settings {
                conditioning,
                backend,
                gamma_shape: (values[1] == 4.0).then_some(0.7),
                ..Settings::default()
            };
            if settings.gamma_shape.is_some() {
                settings.weights.push((Move::GammaShapeScale, 3.0));
            }
            settings.set_morphology_rate(RelativeRate {
                value: values[3],
                log_sd: None,
            });
            let sampler = Sampler::with_tree(&parts, &settings, ladder(values[0]), 42).unwrap();
            let error = (sampler.posterior().likelihood - values[6]).abs();
            assert!(error < 1e-7, "{line}: error={error}");
        }
    }
}

#[test]
fn mixed_multistate_ordering_labels_and_iupac_keep_source_semantics() {
    let morph = "#NEXUS BEGIN DATA; DIMENSIONS NTAX=4 NCHAR=3; FORMAT SYMBOLS=\"0123\"; STATELABELS 1 absent present, 2 zero one two three, 3 zero one two; MATRIX A 000 B 130 C 032 D 132; END; BEGIN ASSUMPTIONS; TYPESET * test = ORD: 3; END;";
    let dna = ">D\nAAAA\n>C\nCCCC\n>B\nGGGG\n>A\nTTTT\n";
    let matrix = alignment::parse(&input(morph, dna, "reject")).unwrap();
    for mode in [0, 1, 2] {
        let parts = match mode {
            0 => matrix.partition_by_states(GapTreatment::Reject),
            1 => matrix.partition_by_labels(GapTreatment::Reject),
            _ => matrix.compress(std::slice::from_ref(&(0..7)), GapTreatment::Reject),
        }
        .unwrap();
        let originals = match mode {
            0 => matrix.component_matrices()[0].partition_by_states(GapTreatment::Reject),
            1 => matrix.component_matrices()[0].partition_by_labels(GapTreatment::Reject),
            _ => matrix.component_matrices()[0]
                .compress(std::slice::from_ref(&(0..3)), GapTreatment::Reject),
        }
        .unwrap();
        assert_eq!(&parts[..parts.len() - 1], &originals);
        let design =
            Design::new(&matrix, &parts, GapTreatment::Reject, Conditioning::None).unwrap();
        let tree = Tree::new(
            vec![
                None,
                None,
                None,
                None,
                Some((0, 1)),
                Some((2, 3)),
                Some((4, 5)),
            ],
            vec![0.0, 0.0, 0.0, 0.0, 0.1, 0.2, 0.3],
        )
        .unwrap();
        for seed in 0..16 {
            let simulated = design
                .simulate(
                    &tree,
                    &rove::rates::RateVariation::default(),
                    seed,
                    1000,
                    &|| false,
                )
                .unwrap();
            let frozen = design.compress(&simulated).unwrap();
            let layout = |patterns: &[rove::data::Patterns]| {
                patterns
                    .iter()
                    .flat_map(|p| {
                        p.sites()
                            .iter()
                            .zip(p.state_symbols())
                            .map(|(s, a)| (*s, (p.ordering(), a.clone(), p.data_type())))
                    })
                    .collect::<std::collections::BTreeMap<_, _>>()
            };
            assert_eq!(layout(&frozen), layout(&parts));
        }
    }
    let ambiguous =
        alignment::parse(&input(morph, &dna.replace("TTTT", "RNY?"), "reject")).unwrap();
    let parts = ambiguous.partition_by_states(GapTreatment::Reject).unwrap();
    let mut settings = Settings::default();
    settings.set_morphology_rate(RelativeRate {
        value: 1.0,
        log_sd: None,
    });
    assert!(Sampler::new(&parts, &settings, 1).is_ok());
    assert!(
        Design::new(
            &ambiguous,
            &parts,
            GapTreatment::Reject,
            Conditioning::Variable
        )
        .is_err()
    );
}
