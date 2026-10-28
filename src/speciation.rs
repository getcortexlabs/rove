//! The birth–death tree prior of Gernhard (2008), as BEAST 2.7 evaluates
//! `BirthDeathGernhard08Model` with `type="unscaled"`: every extant lineage is
//! sampled, and the density is conditioned on the number of tips but not on
//! the root height or an origin.

use crate::tree::Tree;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BirthDeath {
    /// λ − μ, the net diversification rate.
    pub birth_difference: f64,
    /// μ / λ, the relative death rate.
    pub relative_death: f64,
}

impl BirthDeath {
    /// Log density of the node heights and the ranked topology. All tips must
    /// be at height zero. Outside `λ − μ > 0` and `0 ≤ μ/λ < 1` it is −∞.
    pub fn log_density(&self, tree: &Tree) -> f64 {
        let (r, a) = (self.birth_difference, self.relative_death);
        if !(r > 0.0 && r.is_finite() && (0.0..1.0).contains(&a)) {
            return f64::NEG_INFINITY;
        }
        debug_assert!((0..tree.tips()).all(|t| tree.height(t) == 0.0));
        let tips = tree.tips() as f64;
        let mut log = (tips - 1.0) * r.ln() + tips * (-a).ln_1p();
        for node in tree.tips()..tree.nodes() {
            let decay = -r * tree.height(node);
            // ln(1 − a·e^(−r·h)), one factor per speciation and one more at the root.
            let survival = (-a * decay.exp()).ln_1p();
            log += decay - 2.0 * survival;
            if node == tree.root() {
                log += decay - survival;
            }
        }
        log
    }
}

#[cfg(test)]
mod tests {
    use super::BirthDeath;
    use crate::tree::Tree;

    #[test]
    fn three_tips_match_the_closed_form() {
        let (r, a) = (1.3, 0.4);
        let tree = Tree::new(
            vec![None, None, None, Some((0, 1)), Some((3, 2))],
            vec![0.0, 0.0, 0.0, 0.7, 1.9],
        )
        .unwrap();
        let f = |h: f64| (-r * h).exp() / (1.0 - a * (-r * h).exp()).powi(2);
        let survival = |h: f64| (-r * h).exp() / (1.0 - a * (-r * h).exp());
        let expected = (r * r * (1.0 - a).powi(3) * f(0.7) * f(1.9) * survival(1.9)).ln();
        let actual = BirthDeath {
            birth_difference: r,
            relative_death: a,
        }
        .log_density(&tree);
        assert!((actual - expected).abs() < 1e-13, "{actual} != {expected}");
    }

    #[test]
    fn invalid_rates_have_no_density() {
        let tree = Tree::new(vec![None, None, Some((0, 1))], vec![0.0, 0.0, 1.0]).unwrap();
        for (r, a) in [
            (0.0, 0.5),
            (-1.0, 0.5),
            (1.0, 1.0),
            (1.0, -0.1),
            (f64::NAN, 0.5),
        ] {
            let density = BirthDeath {
                birth_difference: r,
                relative_death: a,
            }
            .log_density(&tree);
            assert_eq!(density, f64::NEG_INFINITY, "r = {r}, a = {a}");
        }
    }
}
