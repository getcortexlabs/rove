//! Joint changes to times and explicit lognormal branch rates. These keep the
//! model's density in its original coordinates; proposal Jacobians belong here.

use crate::rates::RelaxedClock;
use crate::tree::Tree;

/// Change one internal height, preserving rate × duration on incident branches.
/// For a symmetric height draw, the Hastings factor is the product of old/new
/// branch durations. The root has two incident branches; other nodes have three.
pub(super) fn time_rates(
    tree: &mut Tree,
    clock: &mut RelaxedClock,
    node: usize,
    height: f64,
) -> Option<f64> {
    let (a, b) = tree.children(node)?;
    let lower = tree.height(a).max(tree.height(b));
    if !height.is_finite()
        || height <= lower
        || tree.parent(node).is_some_and(|p| height >= tree.height(p))
    {
        return None;
    }
    let branches = [a, b, node];
    let count = if node == tree.root() { 2 } else { 3 };
    let mut rates = [0.0; 3];
    let mut log_hastings = 0.0;
    for i in 0..count {
        let branch = branches[i];
        let old_duration = tree.branch_length(branch);
        let new_duration = if branch == node {
            tree.height(tree.parent(node)?) - height
        } else {
            height - tree.height(branch)
        };
        if old_duration <= 0.0 || new_duration <= 0.0 {
            return None;
        }
        let log_ratio = old_duration.ln() - new_duration.ln();
        rates[i] = (clock.branch_rate(branch, tree.root()).ln() + log_ratio).exp();
        if !rates[i].is_finite() || rates[i] <= 0.0 {
            return None;
        }
        log_hastings += log_ratio;
    }
    tree.set_height(node, height);
    for i in 0..count {
        // When the last node is not the root, it uses the root's parameter slot.
        let index = if branches[i] == tree.nodes() - 1 {
            tree.root()
        } else {
            branches[i]
        };
        clock.set_rate(index, rates[i]);
    }
    Some(log_hastings)
}

/// Sigma' = f sigma and z_i = (ln(rate_i) + sigma²/2) / sigma unchanged.
/// The rate Jacobian is f^d product(rate_i'/rate_i). A uniform multiplier on
/// [s, 1/s] contributes 1/f, giving (d-1) ln(f) + sum ln(rate_i'/rate_i).
pub(super) fn sigma_rates(clock: &mut RelaxedClock, factor: f64) -> Option<f64> {
    let old = clock.sigma();
    let new = old * factor;
    if !factor.is_finite() || factor <= 0.0 || !new.is_finite() || new <= 0.0 {
        return None;
    }
    let offset = 0.5 * new * (old - new);
    let mut log_hastings = (clock.rates().len() as f64 - 1.0) * factor.ln();
    // Validate before mutation. A second pass avoids allocating a rate vector.
    for &rate in clock.rates() {
        let log_rate = rate.ln();
        let proposed_log = factor * log_rate + offset;
        let proposed = proposed_log.exp();
        if !proposed.is_finite() || proposed <= 0.0 {
            return None;
        }
        log_hastings += proposed_log - log_rate;
    }
    for i in 0..clock.rates().len() {
        let proposed = (factor * clock.rates()[i].ln() + offset).exp();
        clock.set_rate(i, proposed);
    }
    clock.set_sigma(new);
    Some(log_hastings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Random;

    fn tree(root_last: bool) -> Tree {
        if root_last {
            Tree::new(
                vec![None, None, None, Some((0, 1)), Some((3, 2))],
                vec![0.0, 0.0, 0.0, 0.4, 1.0],
            )
            .unwrap()
        } else {
            Tree::new(
                vec![None, None, None, Some((4, 2)), Some((0, 1))],
                vec![0.0, 0.0, 0.0, 1.0, 0.4],
            )
            .unwrap()
        }
    }

    fn near(a: f64, b: f64, tolerance: f64) {
        assert!((a - b).abs() <= tolerance * b.abs().max(1.0), "{a} != {b}");
    }

    #[test]
    fn inverse_maps_preserve_distances_and_rate_indexing() {
        for root_last in [true, false] {
            let original = tree(root_last);
            let rates = RelaxedClock::new(0.7, vec![0.8, 1.3, 1.8, 0.4]).unwrap();
            for node in original.tips()..original.nodes() {
                let mut proposal = original.clone();
                let mut clock = rates.clone();
                let old_height = original.height(node);
                let h = time_rates(&mut proposal, &mut clock, node, old_height * 1.2).unwrap();
                for branch in 0..original.nodes() {
                    near(
                        proposal.branch_length(branch) * clock.branch_rate(branch, original.root()),
                        original.branch_length(branch) * rates.branch_rate(branch, original.root()),
                        2e-14,
                    );
                }
                let reverse = time_rates(&mut proposal, &mut clock, node, old_height).unwrap();
                near(h + reverse, 0.0, 2e-14);
                assert_eq!(proposal, original);
                for (a, b) in clock.rates().iter().zip(rates.rates()) {
                    near(*a, *b, 2e-14);
                }
                assert!(time_rates(&mut proposal, &mut clock, node, 0.0).is_none());
                assert!(time_rates(&mut proposal, &mut clock, node, f64::INFINITY).is_none());
                if node != original.root() {
                    assert!(
                        time_rates(
                            &mut proposal,
                            &mut clock,
                            node,
                            original.height(original.root())
                        )
                        .is_none()
                    );
                }
            }
            for f in [0.2, 0.8, 1.2, 3.0] {
                let mut clock = rates.clone();
                let h = sigma_rates(&mut clock, f).unwrap();
                for (&a, &b) in rates.rates().iter().zip(clock.rates()) {
                    near(
                        (a.ln() + 0.5 * rates.sigma().powi(2)) / rates.sigma(),
                        (b.ln() + 0.5 * clock.sigma().powi(2)) / clock.sigma(),
                        2e-13,
                    );
                }
                near(h + sigma_rates(&mut clock, 1.0 / f).unwrap(), 0.0, 2e-12);
                for (a, b) in clock.rates().iter().zip(rates.rates()) {
                    near(*a, *b, 2e-12);
                }
            }
            for f in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MAX] {
                let mut clock = rates.clone();
                assert!(sigma_rates(&mut clock, f).is_none());
                assert_eq!(clock, rates);
            }
        }
    }

    fn determinant(mut matrix: Vec<Vec<f64>>) -> f64 {
        let mut value = 1.0;
        for i in 0..matrix.len() {
            let pivot = (i..matrix.len())
                .max_by(|&a, &b| matrix[a][i].abs().total_cmp(&matrix[b][i].abs()))
                .unwrap();
            if pivot != i {
                matrix.swap(i, pivot);
                value = -value;
            }
            let diagonal = matrix[i][i];
            value *= diagonal;
            let (above, below) = matrix.split_at_mut(i + 1);
            for row in below {
                let scale = row[i] / diagonal;
                for (value, pivot) in row.iter_mut().zip(&above[i]).skip(i + 1) {
                    *value -= scale * pivot;
                }
            }
        }
        value.abs()
    }

    fn numerical_jacobian(x: &[f64], transform: impl Fn(&[f64]) -> Vec<f64>) -> f64 {
        let n = x.len();
        let mut matrix = vec![vec![0.0; n]; n];
        for col in 0..n {
            let mut above = x.to_vec();
            let mut below = x.to_vec();
            let delta = 1e-6 * x[col].abs().max(1.0);
            above[col] += delta;
            below[col] -= delta;
            for (row, (a, b)) in transform(&above).iter().zip(transform(&below)).enumerate() {
                matrix[row][col] = (a - b) / (2.0 * delta);
            }
        }
        determinant(matrix)
    }

    #[test]
    fn augmented_jacobians_match_finite_differences() {
        // These maps include the reverse auxiliary variable. Differentiating
        // only the state while holding its random draw fixed gives the wrong ratio.
        let x = [0.4, 0.8, 1.3, 0.6, 0.65];
        let actual = numerical_jacobian(&x, |x| {
            vec![
                x[4],
                x[1] * x[0] / x[4],
                x[2] * x[0] / x[4],
                x[3] * (1.0 - x[0]) / (1.0 - x[4]),
                x[0],
            ]
        });
        let mut t = tree(true);
        let mut c = RelaxedClock::new(0.7, vec![x[1], x[2], 1.0, x[3]]).unwrap();
        near(
            time_rates(&mut t, &mut c, 3, x[4]).unwrap().exp(),
            actual,
            1e-7,
        );
        let x = [1.0, 0.8, 1.3, 1.2];
        let actual = numerical_jacobian(&x, |x| {
            vec![
                x[0] * x[3],
                x[1] / x[3],
                x[2] * (x[0] - 0.4) / (x[0] * x[3] - 0.4),
                1.0 / x[3],
            ]
        });
        let mut t = tree(true);
        let mut c = RelaxedClock::new(0.7, vec![1.0, 1.0, x[1], x[2]]).unwrap();
        near(
            (time_rates(&mut t, &mut c, 4, x[0] * x[3]).unwrap() - x[3].ln()).exp(),
            actual,
            1e-7,
        );
        let x = [0.7, 0.8, 1.3, 1.2];
        let actual = numerical_jacobian(&x, |x| {
            let sigma = x[0] * x[3];
            vec![
                sigma,
                (x[3] * x[1].ln() + 0.5 * sigma * (x[0] - sigma)).exp(),
                (x[3] * x[2].ln() + 0.5 * sigma * (x[0] - sigma)).exp(),
                1.0 / x[3],
            ]
        });
        let mut c = RelaxedClock::new(x[0], vec![x[1], x[2]]).unwrap();
        near(sigma_rates(&mut c, x[3]).unwrap().exp(), actual, 1e-7);
    }

    #[test]
    fn time_rate_proposals_preserve_independent_uniform_heights_and_exponential_rates() {
        for root_move in [false, true] {
            let mut random = Random::new(if root_move { 27002 } else { 27001 });
            let mut t = tree(true);
            let mut c = RelaxedClock::new(0.7, vec![1.0; 4]).unwrap();
            let node = if root_move { 4 } else { 3 };
            let lower = if root_move { 0.4 } else { 0.0 };
            let mut sum = 0.0;
            let mut rate_sum = 0.0;
            let mut quarter = 0;
            let mut samples = 0;
            for step in 0..400_000 {
                if random.coin() {
                    for i in 0..4 {
                        c.set_rate(i, random.exponential());
                    }
                } else {
                    let mut proposed_tree = t.clone();
                    let mut proposed_clock = c.clone();
                    let factor = 0.6 + random.uniform() * (1.0 / 0.6 - 0.6);
                    let height = if root_move {
                        t.height(node) * factor
                    } else {
                        random.uniform()
                    };
                    if height < lower + 1.0 {
                        if let Some(mut h) =
                            time_rates(&mut proposed_tree, &mut proposed_clock, node, height)
                        {
                            if root_move {
                                h -= factor.ln();
                            }
                            let alpha = c.rates().iter().sum::<f64>()
                                - proposed_clock.rates().iter().sum::<f64>()
                                + h;
                            if alpha >= 0.0 || random.uniform() < alpha.exp() {
                                t = proposed_tree;
                                c = proposed_clock;
                            }
                        }
                    }
                }
                if step >= 10_000 {
                    sum += t.height(node) - lower;
                    quarter += usize::from(t.height(node) - lower < 0.25);
                    rate_sum += c.rates().iter().sum::<f64>() / 4.0;
                    samples += 1;
                }
            }
            near(sum / samples as f64, 0.5, 0.01);
            near(quarter as f64 / samples as f64, 0.25, 0.01);
            near(rate_sum / samples as f64, 1.0, 0.02);
        }
    }

    #[test]
    fn coupled_sigma_keeps_the_gamma_hyperprior_and_normal_standardized_rates() {
        let mut random = Random::new(27003);
        let mut clock = RelaxedClock::new(0.7, vec![1.0; 4]).unwrap();
        let (mut sum, mut square, mut z_sum, mut z_square, mut samples) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for step in 0..400_000 {
            if random.coin() {
                for i in 0..4 {
                    clock.set_rate(
                        i,
                        (clock.sigma() * random.normal() - 0.5 * clock.sigma().powi(2)).exp(),
                    );
                }
            } else {
                let mut proposal = clock.clone();
                let factor = 0.5 + 1.5 * random.uniform();
                if let Some(h) = sigma_rates(&mut proposal, factor) {
                    let alpha = proposal.log_sigma_prior() + proposal.log_rate_prior()
                        - clock.log_sigma_prior()
                        - clock.log_rate_prior()
                        + h;
                    if alpha >= 0.0 || random.uniform() < alpha.exp() {
                        clock = proposal;
                    }
                }
            }
            if step >= 10_000 {
                sum += clock.sigma();
                square += clock.sigma().powi(2);
                for rate in clock.rates() {
                    let z = (rate.ln() + 0.5 * clock.sigma().powi(2)) / clock.sigma();
                    z_sum += z / 4.0;
                    z_square += z * z / 4.0;
                }
                samples += 1.0;
            }
        }
        near(sum / samples, 1.0, 0.015);
        near(square / samples - (sum / samples).powi(2), 0.2, 0.015);
        near(z_sum / samples, 0.0, 0.015);
        near(z_square / samples, 1.0, 0.015);
    }
}
