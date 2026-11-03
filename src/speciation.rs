//! The birth–death tree prior of Gernhard (2008), as BEAST 2.7 evaluates
//! `BirthDeathGernhard08Model` with `type="unscaled"`: every extant lineage is
//! sampled, and the density is conditioned on the number of tips but not on
//! the root height or an origin. Optional Yule uses the same convention with
//! a birth rate and no extinction coordinate.

use crate::tree::Tree;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreePrior {
    BirthDeath,
    Yule,
}

/// The live undated speciation parameters. Yule has no extinction coordinate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpeciationState {
    BirthDeath(BirthDeath),
    Yule { birth_rate: f64 },
}

impl SpeciationState {
    pub fn rate(&self) -> f64 {
        match *self {
            Self::BirthDeath(p) => p.birth_difference,
            Self::Yule { birth_rate } => birth_rate,
        }
    }
    pub fn birth_death(&self) -> Option<BirthDeath> {
        match *self {
            Self::BirthDeath(p) => Some(p),
            Self::Yule { .. } => None,
        }
    }
    pub fn log_density(&self, tree: &Tree) -> f64 {
        match *self {
            Self::BirthDeath(p) => p.log_density(tree),
            Self::Yule { birth_rate } => yule_log_density(tree, birth_rate),
        }
    }
    pub fn kind(&self) -> TreePrior {
        match self {
            Self::BirthDeath(_) => TreePrior::BirthDeath,
            Self::Yule { .. } => TreePrior::Yule,
        }
    }
}

impl From<BirthDeath> for SpeciationState {
    fn from(value: BirthDeath) -> Self {
        Self::BirthDeath(value)
    }
}

/// BEAST 2.7.8's unscaled, tip-count-conditioned Yule convention. The root
/// contributes twice; no root/origin conditioning or tip calibrations.
pub fn yule_log_density(tree: &Tree, birth_rate: f64) -> f64 {
    if !birth_rate.is_finite()
        || birth_rate <= 0.0
        || (0..tree.tips()).any(|t| tree.height(t) != 0.0)
    {
        return f64::NEG_INFINITY;
    }
    let total = (tree.tips()..tree.nodes())
        .map(|n| tree.height(n))
        .sum::<f64>()
        + tree.height(tree.root());
    (tree.tips() - 1) as f64 * birth_rate.ln() - birth_rate * total
}

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
