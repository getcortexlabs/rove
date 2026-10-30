use rove::data::{GapTreatment, Patterns};
use rove::dating::{Dating, FossilizedBirthDeath, TipAge};
use rove::mcmc::{Conditioning, Move, Sampler, Settings};
use rove::rates::RateVariation;
use rove::tree::Tree;

fn patterns(tips: usize) -> Vec<Patterns> {
    let rows: String = (0..tips)
        .map(|i| format!("T{i} {}\n", if i % 2 == 0 { "010" } else { "101" }))
        .collect();
    let matrix = rove::nexus::parse(&format!("#NEXUS\nBEGIN DATA; DIMENSIONS NTAX={tips} NCHAR=3; FORMAT SYMBOLS=\"01\"; MATRIX\n{rows}; END;")).unwrap();
    matrix
        .compress(std::slice::from_ref(&(0..3)), GapTreatment::Reject)
        .unwrap()
}
fn model(ages: &[(f64, f64)]) -> Dating {
    Dating {
        process: FossilizedBirthDeath {
            birth: 0.8,
            death: 0.3,
            fossil_sampling: 0.2,
            present_sampling: 0.7,
            origin: 4.0,
        },
        ages: ages
            .iter()
            .map(|&(minimum, maximum)| TipAge { minimum, maximum })
            .collect(),
        clock_rate: 0.1,
        clock_prior_mean: Some(0.1),
    }
}
fn settings(dating: Dating, prior: bool) -> Settings {
    let mut s = Settings {
        sample_from_prior: prior,
        tuning: false,
        conditioning: Conditioning::None,
        ..Settings::default()
    };
    s.set_dating(dating);
    s
}

#[test]
fn fixed_densities_match_beast_sa() {
    for row in include_str!("../fixtures/validation/dating/beast-sa-2.1.1.tsv")
        .lines()
        .skip(1)
    {
        let c: Vec<f64> = row.split('\t').map(|v| v.parse().unwrap()).collect();
        let process = FossilizedBirthDeath {
            birth: c[0],
            death: c[1],
            fossil_sampling: c[2],
            present_sampling: c[3],
            origin: c[4],
        };
        let tree = Tree::new(
            vec![None, None, None, Some((0, 1)), Some((3, 2))],
            vec![c[5], c[6], c[5], c[7], c[8]],
        )
        .unwrap();
        let actual = process.log_density(&tree);
        // SA's p0 subtraction loses precision for a fossil only 1e-9 time units old.
        let tolerance = if c[6] == 1e-9 { 1e-6 } else { 1e-12 };
        assert!(
            (actual - c[9]).abs() < tolerance,
            "{row}: {actual}, difference {}",
            actual - c[9]
        );
    }
}

#[test]
fn fbd_limits_are_finite_and_invalid_states_have_zero_density() {
    let mut d = model(&[(0.0, 0.0), (1.0, 1.0)]);
    let tree = Tree::new(vec![None, None, Some((0, 1))], vec![0.0, 1.0, 2.0]).unwrap();
    d.process.origin = 2000.0;
    assert!(d.process.log_density(&tree).is_finite());
    d.process.origin = 1.5;
    assert_eq!(d.process.log_density(&tree), f64::NEG_INFINITY);
    d.process.origin = 4.0;
    let zero = Tree::new(vec![None, None, Some((0, 1))], vec![0.0, 1.0, 1.0]).unwrap();
    assert_eq!(d.process.log_density(&zero), f64::NEG_INFINITY);
    d.ages[1].minimum = 0.0;
    assert!(d.validate(2).is_err());
    assert!(Sampler::with_tree(patterns(2), &Settings::default(), tree, 1).is_err());
}

#[test]
fn dated_likelihood_scales_time_into_expected_changes() {
    let mut d = model(&[(0.0, 0.0), (0.7, 0.7), (0.0, 0.0)]);
    d.clock_prior_mean = None;
    let tree = Tree::new(
        vec![None, None, None, Some((0, 1)), Some((3, 2))],
        vec![0.0, 0.7, 0.0, 1.5, 2.5],
    )
    .unwrap();
    let a = Sampler::with_tree(patterns(3), &settings(d.clone(), false), tree.clone(), 1).unwrap();
    // Changing the time unit by 10 and dividing every process/clock rate by 10
    // preserves all transition probabilities, including on fossil branches.
    d.clock_rate /= 10.0;
    d.process.birth /= 10.0;
    d.process.death /= 10.0;
    d.process.fossil_sampling /= 10.0;
    d.process.origin *= 10.0;
    for age in &mut d.ages {
        age.minimum *= 10.0;
        age.maximum *= 10.0;
    }
    let scaled = Tree::new(
        vec![None, None, None, Some((0, 1)), Some((3, 2))],
        vec![0.0, 7.0, 0.0, 15.0, 25.0],
    )
    .unwrap();
    let b = Sampler::with_tree(patterns(3), &settings(d, false), scaled, 1).unwrap();
    assert!((a.posterior().likelihood - b.posterior().likelihood).abs() < 1e-12);
    let rates = RateVariation {
        mean: Some(0.2),
        ..RateVariation::default()
    };
    assert_eq!(rates.branch_rate(0, 4), 0.2);
}

#[test]
fn dated_proposals_restore_likelihood_and_resume_exactly() {
    let d = model(&[(0.0, 0.0), (0.4, 1.2), (0.0, 0.0), (0.1, 0.3), (0.7, 0.7)]);
    let mut s = Settings::reference();
    s.tuning = false;
    s.conditioning = Conditioning::None;
    s.set_dating(d);
    s.weights.extend([
        (Move::NodeTimeRates, 3.0),
        (Move::RootTimeRates, 3.0),
        (Move::ClockSigmaRates, 3.0),
    ]);
    let p = patterns(5);
    let mut a = Sampler::new(&p, &s, 35).unwrap();
    for i in 0..6000 {
        a.step();
        if i % 113 == 0 {
            a.tree().validate().unwrap();
            let fresh = a.recompute().unwrap();
            assert!((fresh.total() - a.posterior().total()).abs() < 1e-9);
        }
    }
    let bytes = a.checkpoint(&s).unwrap();
    let mut b = Sampler::resume(&p, &s, &bytes).unwrap();
    for _ in 0..1000 {
        assert_eq!(a.step(), b.step());
        assert_eq!(a.posterior(), b.posterior());
        assert_eq!(a.rates(), b.rates());
        assert_eq!(a.tree(), b.tree());
    }
    let mut wrong = s.clone();
    wrong.dating.as_mut().unwrap().ages[1].maximum = 1.3;
    assert!(Sampler::resume(&p, &wrong, &bytes).is_err());
    for op in a.operators().iter().filter(|op| op.weight > 0.0) {
        assert!(op.accepted > 0, "{} never accepted", op.kind.name());
    }
}

#[test]
fn two_extant_tips_follow_the_truncated_exponential_root_distribution() {
    // With mu=0, rho=1, two extant samples and no fossils, the FBD root
    // density is proportional to exp(-(lambda+psi)*h), 0<h<origin.
    let mut d = model(&[(0.0, 0.0), (0.0, 0.0)]);
    d.process.death = 0.0;
    d.process.present_sampling = 1.0;
    let s = settings(d.clone(), true);
    let mut chain = Sampler::new(patterns(2), &s, 27).unwrap();
    let mut counts = [0usize; 3];
    let mut clock = 0.0;
    let mut n = 0;
    for i in 0..400_000 {
        chain.step();
        if i >= 20_000 && i % 10 == 0 {
            for (k, h) in [0.3, 1.0, 2.0].iter().enumerate() {
                counts[k] += usize::from(chain.tree().height(chain.tree().root()) < *h);
            }
            clock += chain.rates().mean.unwrap();
            n += 1;
        }
    }
    let rate = d.process.birth + d.process.fossil_sampling;
    for (k, h) in [0.3, 1.0, 2.0].iter().enumerate() {
        let expected = -(-rate * h).exp_m1() / -(-rate * d.process.origin).exp_m1();
        assert!(
            (counts[k] as f64 / n as f64 - expected).abs() < 0.015,
            "root CDF at {h}"
        );
    }
    assert!(
        (clock / n as f64 - 0.1).abs() < 0.006,
        "clock mean {}",
        clock / n as f64
    );
}

// Independently integrate the birth–death backward equations with RK4:
// p0'=mu-(lambda+mu+psi)*p0+lambda*p0^2; q'=(2*lambda*p0-lambda-mu-psi)*q.
fn backward(d: FossilizedBirthDeath, dt: f64, n: usize) -> Vec<(f64, f64)> {
    let f = |(p, q): (f64, f64)| {
        (
            d.death - (d.birth + d.death + d.fossil_sampling) * p + d.birth * p * p,
            (2.0 * d.birth * p - d.birth - d.death - d.fossil_sampling) * q,
        )
    };
    let add = |(p, q): (f64, f64), (a, b): (f64, f64), w: f64| (p + w * a, q + w * b);
    let mut y = (1.0 - d.present_sampling, 1.0);
    let mut v = vec![y];
    for _ in 0..n {
        let a = f(y);
        let b = f(add(y, a, dt / 2.0));
        let c = f(add(y, b, dt / 2.0));
        let e = f(add(y, c, dt));
        y = add(
            y,
            (
                a.0 + 2.0 * b.0 + 2.0 * c.0 + e.0,
                a.1 + 2.0 * b.1 + 2.0 * c.1 + e.1,
            ),
            dt / 6.0,
        );
        v.push(y);
    }
    v
}

#[test]
fn fossil_age_and_root_moves_match_independent_quadrature() {
    let mut d = model(&[(0.0, 0.0), (0.3, 1.3)]);
    d.clock_prior_mean = None;
    let grid = backward(d.process, 0.001, 4000);
    let (mut mass, mut age, mut root) = (0.0, 0.0, 0.0);
    // Joint density is proportional to p0(age)/q(age)*q(root), root>age.
    for a in 300..1300 {
        for h in a + 1..4000 {
            let w = grid[a].0 / grid[a].1 * grid[h].1;
            mass += w;
            age += w * (a as f64 * 0.001);
            root += w * (h as f64 * 0.001);
        }
    }
    let s = settings(d, true);
    let mut chain = Sampler::new(patterns(2), &s, 51).unwrap();
    let (mut sampled_age, mut sampled_root, mut n) = (0.0, 0.0, 0);
    for i in 0..350_000 {
        chain.step();
        if i >= 20_000 && i % 10 == 0 {
            sampled_age += chain.tree().height(1);
            sampled_root += chain.tree().height(chain.tree().root());
            n += 1;
        }
    }
    assert!(
        (sampled_age / n as f64 - age / mass).abs() < 0.012,
        "fossil mean {} vs {}",
        sampled_age / n as f64,
        age / mass
    );
    assert!(
        (sampled_root / n as f64 - root / mass).abs() < 0.025,
        "root mean {} vs {}",
        sampled_root / n as f64,
        root / mass
    );
}

#[test]
fn fossil_topology_moves_match_integrated_prior_masses() {
    let mut d = model(&[(0.0, 0.0), (0.8, 0.8), (0.0, 0.0)]);
    d.clock_prior_mean = None;
    let grid = backward(d.process, 0.001, 4000);
    let (mut outgroup, mut cherry) = (0.0, 0.0);
    let (mut q0, mut qf) = (0.0, 0.0);
    for (i, &(_, q)) in grid.iter().enumerate() {
        if i >= 800 {
            outgroup += q * q0;
            cherry += q * qf;
            qf += q;
        }
        q0 += q;
    }
    let expected = outgroup / (outgroup + 2.0 * cherry);
    for topology_move in [Move::NarrowExchange, Move::SubtreeSlide, Move::WideExchange] {
        let mut s = settings(d.clone(), true);
        s.weights = vec![
            (Move::NodeHeight, 2.0),
            (Move::RootScale, 1.0),
            (Move::TreeScale, 1.0),
            (topology_move, 3.0),
        ];
        let mut chain = Sampler::new(patterns(3), &s, 19).unwrap();
        let (mut count, mut n) = (0, 0);
        for i in 0..350_000 {
            chain.step();
            if i >= 20_000 && i % 10 == 0 {
                count += usize::from(chain.tree().parent(1) == Some(chain.tree().root()));
                n += 1;
            }
        }
        assert!(
            (count as f64 / n as f64 - expected).abs() < 0.025,
            "{}: {} vs {expected}",
            topology_move.name(),
            count as f64 / n as f64
        );
    }
}
