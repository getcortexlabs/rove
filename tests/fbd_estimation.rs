use rove::data::GapTreatment;
use rove::dating::{Dating, FossilizedBirthDeath, ProcessPriors, TipAge};
use rove::mcmc::{Conditioning, Move, Sampler, Settings};
use rove::tree::Tree;

fn model() -> Dating {
    Dating {
        process: FossilizedBirthDeath {
            birth: 0.8,
            death: 0.3,
            fossil_sampling: 0.2,
            present_sampling: 0.7,
            origin: 4.0,
        },
        process_priors: Some(ProcessPriors {
            birth_rate_mean: Some(1.1),
            death_rate_mean: Some(0.6),
            fossil_sampling_rate_mean: Some(0.3),
            present_sampling_beta: Some((2.0, 3.0)),
            origin_excess_mean: Some(2.0),
        }),
        ages: vec![
            TipAge {
                minimum: 0.0,
                maximum: 0.0,
            },
            TipAge {
                minimum: 0.7,
                maximum: 0.7,
            },
        ],
        clock_rate: 0.1,
        clock_prior_mean: None,
    }
}
fn chain(d: Dating, ops: &[Move], prior: bool, seed: u64) -> (Sampler, Settings) {
    let matrix = rove::nexus::parse(
        r#"#NEXUS BEGIN DATA; DIMENSIONS NTAX=2 NCHAR=4;
        FORMAT SYMBOLS="01"; MATRIX A 0011 B 1100; END;"#,
    )
    .unwrap();
    let patterns = matrix
        .compress(std::slice::from_ref(&(0..4)), GapTreatment::Reject)
        .unwrap();
    let heights = vec![d.ages[0].minimum, d.ages[1].minimum, 2.0];
    let tree = Tree::new(vec![None, None, Some((0, 1))], heights).unwrap();
    let mut s = Settings {
        sample_from_prior: prior,
        tuning: false,
        conditioning: Conditioning::None,
        ..Settings::default()
    };
    s.set_dating(d);
    s.weights = ops.iter().map(|m| (*m, 1.0)).collect();
    if ops.contains(&Move::RootScale) {
        s.tuning_values.push((Move::RootScale, 0.3));
    }
    (Sampler::with_tree(patterns, &s, tree, seed).unwrap(), s)
}
fn average(mut s: Sampler, value: impl Fn(&Sampler) -> f64) -> f64 {
    let mut total = 0.0;
    for i in 0..300_000 {
        s.step();
        if i >= 20_000 {
            total += value(&s);
        }
    }
    total / 280_000.0
}

#[test]
fn fixed_tree_conditional_parameter_targets_have_known_distributions() {
    for (kind, mean) in [
        (Move::FbdBirthScale, 1.1),
        (Move::FbdSamplingScale, 0.3),
        (Move::FbdOriginScale, 2.0),
    ] {
        let mut d = model();
        d.process.death = 0.0;
        d.process.present_sampling = 1.0;
        d.ages[1] = d.ages[0];
        let mut p = ProcessPriors::default();
        match kind {
            Move::FbdBirthScale => p.birth_rate_mean = Some(mean),
            Move::FbdSamplingScale => p.fossil_sampling_rate_mean = Some(mean),
            _ => p.origin_excess_mean = Some(mean),
        };
        d.process_priors = Some(p);
        // For two extant tips, the unconditioned sampling factor is
        // 2*lambda*exp[-(lambda+psi)*(origin+root)]. Specimen count remains data.
        let expected = match kind {
            Move::FbdBirthScale => 2.0 / (4.0 + 2.0 + 1.0 / mean),
            Move::FbdSamplingScale => 1.0 / (4.0 + 2.0 + 1.0 / mean),
            _ => 2.0 + 1.0 / (0.8 + 0.2 + 1.0 / mean),
        };
        let (s, _) = chain(d, &[kind], true, 413);
        let sampled = average(s, |s| {
            let p = s.dating_process().unwrap();
            match kind {
                Move::FbdBirthScale => p.birth,
                Move::FbdSamplingScale => p.fossil_sampling,
                _ => p.origin,
            }
        });
        assert!(
            (sampled - expected).abs() < 0.012,
            "{kind:?}: {sampled} vs {expected}"
        );
    }
}

#[test]
fn joint_tree_and_birth_sampling_matches_integrated_mass() {
    let mut d = model();
    d.process.death = 0.0;
    d.process.present_sampling = 1.0;
    d.ages[1] = d.ages[0];
    d.process_priors = Some(ProcessPriors {
        birth_rate_mean: Some(1.0),
        ..ProcessPriors::default()
    });
    let mut mass = 0.0;
    let mut height = 0.0;
    let mut birth = 0.0;
    for i in 0..10_000 {
        let h = (i as f64 + 0.5) * 4.0 / 10_000.0;
        let r = 4.0 + h + 1.0;
        let w = (-0.2 * (4.0 + h)).exp() / (r * r);
        mass += w;
        height += h * w;
        birth += 2.0 / r * w;
    }
    let (mut s, _) = chain(d, &[Move::FbdBirthScale, Move::RootScale], true, 91);
    let (mut sum_h, mut sum_b, mut n) = (0.0, 0.0, 0);
    for i in 0..600_000 {
        s.step();
        if i >= 20_000 && i % 10 == 0 {
            sum_h += s.tree().height(2);
            sum_b += s.dating_process().unwrap().birth;
            n += 1;
        }
    }
    assert!(
        (sum_h / n as f64 - height / mass).abs() < 0.025,
        "height {} vs {}; birth {} vs {}",
        sum_h / n as f64,
        height / mass,
        sum_b / n as f64,
        birth / mass
    );
    assert!((sum_b / n as f64 - birth / mass).abs() < 0.008);
}

// Independent backward ODE rather than the production closed form.
fn fossil_density(p: FossilizedBirthDeath) -> f64 {
    let f = |(u, q): (f64, f64)| {
        (
            p.death - (p.birth + p.death + p.fossil_sampling) * u + p.birth * u * u,
            (2.0 * p.birth * u - p.birth - p.death - p.fossil_sampling) * q,
        )
    };
    let add = |(u, q): (f64, f64), (a, b): (f64, f64), h: f64| (u + h * a, q + h * b);
    let dt = 0.002;
    let mut y = (1.0 - p.present_sampling, 1.0);
    let mut fossil = (0.0, 0.0);
    let mut root = 0.0;
    for i in 1..=2000 {
        let a = f(y);
        let b = f(add(y, a, dt / 2.0));
        let c = f(add(y, b, dt / 2.0));
        let d = f(add(y, c, dt));
        y = add(
            y,
            (
                a.0 + 2.0 * b.0 + 2.0 * c.0 + d.0,
                a.1 + 2.0 * b.1 + 2.0 * c.1 + d.1,
            ),
            dt / 6.0,
        );
        if i == 350 {
            fossil = y;
        }
        if i == 1000 {
            root = y.1;
        }
    }
    2.0 * p.birth * p.fossil_sampling * p.present_sampling * y.1 * root * fossil.0 / fossil.1
}

#[test]
fn death_and_sampling_probability_match_ode_quadrature() {
    for kind in [Move::FbdDeathScale, Move::FbdRhoLogit] {
        let mut d = model();
        d.process_priors = Some(if kind == Move::FbdDeathScale {
            ProcessPriors {
                death_rate_mean: Some(0.6),
                ..ProcessPriors::default()
            }
        } else {
            ProcessPriors {
                present_sampling_beta: Some((2.0, 3.0)),
                ..ProcessPriors::default()
            }
        });
        let mut mass = 0.0;
        let mut moment = 0.0;
        for i in 0..1000 {
            let x = (i as f64 + 0.5) / 1000.0
                * (if kind == Move::FbdDeathScale {
                    12.0
                } else {
                    1.0
                });
            let mut p = d.process;
            let prior = if kind == Move::FbdDeathScale {
                p.death = x;
                (-x / 0.6).exp() / 0.6
            } else {
                p.present_sampling = x;
                12.0 * x * (1.0 - x).powi(2)
            };
            let w = fossil_density(p) * prior;
            mass += w;
            moment += x * w;
        }
        let (s, _) = chain(d, &[kind], true, 632);
        let actual = average(s, |s| {
            let p = s.dating_process().unwrap();
            if kind == Move::FbdDeathScale {
                p.death
            } else {
                p.present_sampling
            }
        });
        assert!(
            (actual - moment / mass).abs() < 0.012,
            "{kind:?}: {actual} vs {}",
            moment / mass
        );
    }
}

#[test]
fn process_updates_restore_state_and_resume_without_changing_samples() {
    let d = model();
    let (mut s, settings) = chain(
        d,
        &[
            Move::FbdBirthScale,
            Move::FbdDeathScale,
            Move::FbdSamplingScale,
            Move::FbdRhoLogit,
            Move::FbdOriginScale,
            Move::RootScale,
        ],
        false,
        9,
    );
    let matrix = rove::nexus::parse(
        r#"#NEXUS BEGIN DATA; DIMENSIONS NTAX=2 NCHAR=4;
        FORMAT SYMBOLS="01"; MATRIX A 0011 B 1100; END;"#,
    )
    .unwrap();
    let patterns = matrix
        .compress(std::slice::from_ref(&(0..4)), GapTreatment::Reject)
        .unwrap();
    for i in 0..8000 {
        let old = s.dating_process();
        let (_, accepted) = s.step();
        if !accepted {
            assert_eq!(old, s.dating_process());
        }
        if i % 101 == 0 {
            assert!((s.recompute().unwrap().total() - s.posterior().total()).abs() < 1e-9);
        }
    }
    let bytes = s.checkpoint(&settings).unwrap();
    let mut resumed = Sampler::resume(&patterns, &settings, &bytes).unwrap();
    for _ in 0..2000 {
        assert_eq!(s.step(), resumed.step());
        assert_eq!(s.dating_process(), resumed.dating_process());
        assert_eq!(s.posterior(), resumed.posterior());
        assert_eq!(s.tree(), resumed.tree());
    }
    for op in s.operators() {
        assert!(
            op.accepted > 0 && op.rejected + op.failed > 0,
            "{}",
            op.kind.name()
        );
    }
    let mut wrong = settings.clone();
    wrong
        .dating
        .as_mut()
        .unwrap()
        .process_priors
        .as_mut()
        .unwrap()
        .birth_rate_mean = Some(1.2);
    assert!(Sampler::resume(&patterns, &wrong, &bytes).is_err());
}

#[test]
fn complete_joint_density_matches_beast_with_proper_hyperpriors() {
    for row in include_str!("../fixtures/validation/fbd-estimation/beast-sa-2.1.1.tsv")
        .lines()
        .skip(1)
    {
        let c: Vec<f64> = row.split('\t').map(|v| v.parse().unwrap()).collect();
        let mut d = model();
        d.process = FossilizedBirthDeath {
            birth: c[0],
            death: c[1],
            fossil_sampling: c[2],
            present_sampling: c[3],
            origin: c[4],
        };
        d.ages[1] = TipAge {
            minimum: c[5],
            maximum: c[5],
        };
        let tree = Tree::new(vec![None, None, Some((0, 1))], vec![0.0, c[5], c[6]]).unwrap();
        let f = d.process.log_density(&tree);
        let p = d.log_parameter_prior(&tree, d.clock_rate);
        assert!((f - c[7]).abs() < 1e-12, "tree {row}: {f}");
        assert!((p - c[8]).abs() < 1e-12, "prior {row}: {p}");
        assert!((f + p - c[9]).abs() < 2e-12, "joint {row}");
    }
}
