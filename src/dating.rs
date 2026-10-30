//! Tip dating with a constant-rate fossilized birth–death process.
//!
//! With fixed process parameters, the restricted tree density has a constant
//! omitted normalizer. Optional process priors instead define a joint sampling
//! model: specimen counts, age evidence and imposed terminal/clade restrictions
//! inform the process. This is not a per-process normalized conditional tree
//! prior. Both modes exclude sampled ancestors from the tree state space.

mod process;
pub use process::ProcessPriors;

use crate::{random::Random, tree::Tree};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FossilizedBirthDeath {
    pub birth: f64,
    pub death: f64,
    pub fossil_sampling: f64,
    pub present_sampling: f64,
    pub origin: f64,
}

impl FossilizedBirthDeath {
    pub fn validate(self) -> Result<(), String> {
        let (c, b) = self.constants();
        if ![self.birth, self.fossil_sampling, self.origin]
            .iter()
            .all(|x| x.is_finite() && *x > 0.0)
            || !self.death.is_finite()
            || self.death < 0.0
            || !self.present_sampling.is_finite()
            || !(0.0..=1.0).contains(&self.present_sampling)
            || !c.is_finite()
            || c <= 0.0
            || !b.is_finite()
            || b <= 0.0
            || !(self.birth + self.death + self.fossil_sampling + c).is_finite()
        {
            return Err("invalid FBD parameters: positive birth, fossil sampling and origin; nonnegative death; present sampling in [0,1] required".into());
        }
        Ok(())
    }

    fn constants(self) -> (f64, f64) {
        let c = (self.birth - self.death - self.fossil_sampling)
            .hypot(2.0 * self.birth.sqrt() * self.fossil_sampling.sqrt());
        let b = 1.0
            + (2.0 * self.birth * self.present_sampling - self.birth
                + self.death
                + self.fossil_sampling)
                / c;
        (c, b)
    }

    fn log_q(c: f64, b: f64, t: f64) -> f64 {
        let z = c * t;
        z + 2.0 * (2.0 * (-z).exp() + b * -(-z).exp_m1()).ln()
    }

    fn p0(self, c: f64, t: f64) -> f64 {
        // Positive terms avoid cancellation near t=0 and rho=1.
        let sum = self.birth + self.death + self.fossil_sampling + c;
        let low = 2.0 * self.death / sum;
        let high = sum / (2.0 * self.birth);
        let initial = 1.0 - self.present_sampling;
        let e = (-c * t).exp();
        let a = (high - initial) * -(-c * t).exp_m1();
        let b = c / self.birth * e;
        (low * a + initial * b) / (a + b)
    }

    /// BEAST 2 SA's origin-conditioned density, removal probability zero,
    /// without survival/rho-sampling conditioning, on a tree with no SAs.
    /// Ages are measured backwards from the present (zero). Call `validate`
    /// before evaluating a process constructed from external parameters.
    pub fn log_density(self, tree: &Tree) -> f64 {
        if tree.height(tree.root()) >= self.origin
            || (0..tree.nodes()).any(|i| {
                tree.parent(i)
                    .is_some_and(|p| tree.height(p) <= tree.height(i))
            })
        {
            return f64::NEG_INFINITY;
        }
        let (c, b) = self.constants();
        let log_q = |t| Self::log_q(c, b, t);
        let log_birth = self.birth.ln();
        let log_sampling = self.fossil_sampling.ln();
        let mut log = -log_q(self.origin) + (tree.tips() - 1) as f64 * 2.0f64.ln();
        for tip in 0..tree.tips() {
            let age = tree.height(tip);
            log += if age == 0.0 && self.present_sampling > 0.0 {
                (4.0 * self.present_sampling).ln()
            } else {
                log_sampling + log_q(age) + self.p0(c, age).ln()
            };
        }
        for node in tree.tips()..tree.nodes() {
            log += log_birth - log_q(tree.height(node));
        }
        if log.is_nan() { f64::NEG_INFINITY } else { log }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TipAge {
    pub minimum: f64,
    pub maximum: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dating {
    pub process: FossilizedBirthDeath,
    /// None preserves the legacy fixed-process conditional analysis.
    /// Some enables the explicit joint sampling model described in ProcessPriors.
    pub process_priors: Option<ProcessPriors>,
    /// One age constraint per tip, in the matrix's taxon order.
    pub ages: Vec<TipAge>,
    /// Expected changes per character per unit time.
    pub clock_rate: f64,
    /// None fixes the clock; Some(mean) estimates it with an exponential prior.
    pub clock_prior_mean: Option<f64>,
}

impl Dating {
    pub fn validate(&self, tips: usize) -> Result<(), String> {
        self.process.validate()?;
        if let Some(priors) = self.process_priors {
            priors.validate()?;
            if !priors
                .log_density(self.process, self.process, self.origin_offset())
                .is_finite()
            {
                return Err("initial FBD values must lie inside their prior support".into());
            }
        }
        if self.ages.len() != tips
            || self.ages.iter().any(|a| {
                !a.minimum.is_finite()
                    || !a.maximum.is_finite()
                    || a.minimum < 0.0
                    || a.maximum < a.minimum
                    || a.maximum >= self.process.origin
                    || (a.minimum == 0.0 && a.maximum > 0.0)
            })
        {
            return Err("provide one fixed age or positive fossil age interval per taxon, strictly younger than the origin; an interval cannot include the present".into());
        }
        if self.process.present_sampling == 0.0 && self.ages.iter().any(|a| a.minimum == 0.0) {
            return Err(
                "present sampling must be positive when the matrix contains extant tips".into(),
            );
        }
        if !self.clock_rate.is_finite()
            || self.clock_rate <= 0.0
            || self
                .clock_prior_mean
                .is_some_and(|x| !x.is_finite() || x <= 0.0)
        {
            return Err("clock rate and exponential prior mean must be positive and finite".into());
        }
        Ok(())
    }

    pub(crate) fn initialize(&self, tree: &mut Tree) {
        let mut oldest: f64 = 0.0;
        for (tip, age) in self.ages.iter().enumerate() {
            let height = age.minimum + (age.maximum - age.minimum) / 2.0;
            tree.set_height(tip, height);
            oldest = oldest.max(height);
        }
        let scale = 0.8 * (self.process.origin - oldest) / tree.height(tree.root());
        for node in tree.tips()..tree.nodes() {
            tree.set_height(node, oldest + tree.height(node) * scale);
        }
    }

    pub fn origin_offset(&self) -> f64 {
        self.ages.iter().map(|a| a.maximum).fold(0.0, f64::max)
    }

    pub fn log_parameter_prior(&self, tree: &Tree, clock: f64) -> f64 {
        self.log_prior_at(tree, clock, self.process)
    }

    pub fn log_prior_at(&self, tree: &Tree, clock: f64, process: FossilizedBirthDeath) -> f64 {
        if process.validate().is_err() || process.origin <= self.origin_offset() {
            return f64::NEG_INFINITY;
        }
        let process_prior = match self.process_priors {
            Some(priors) => priors.log_density(process, self.process, self.origin_offset()),
            None if process == self.process => 0.0,
            None => return f64::NEG_INFINITY,
        };
        if !clock.is_finite()
            || clock <= 0.0
            || (self.clock_prior_mean.is_none() && clock != self.clock_rate)
        {
            return f64::NEG_INFINITY;
        }
        let mut log = self.clock_prior_mean.map_or(0.0, |m| -m.ln() - clock / m);
        for (tip, age) in self.ages.iter().enumerate() {
            if !(age.minimum..=age.maximum).contains(&tree.height(tip)) {
                return f64::NEG_INFINITY;
            }
            if age.minimum != age.maximum {
                log -= (age.maximum - age.minimum).ln();
            }
        }
        log + process_prior
    }

    pub(crate) fn propose_age(&self, tree: &mut Tree, random: &mut Random) -> Option<f64> {
        let tip = random.below(tree.tips());
        let age = self.ages[tip];
        let upper = age.maximum.min(tree.height(tree.parent(tip)?));
        if upper <= age.minimum {
            return None;
        }
        // The parent is unchanged, so the truncated proposal interval is
        // identical in both directions. Fixed ages have no continuous move.
        tree.set_height(tip, age.minimum + random.uniform() * (upper - age.minimum));
        Some(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tip_age_move_samples_a_polynomial_target_with_parent_truncation() {
        let dating = Dating {
            process_priors: None,
            process: FossilizedBirthDeath {
                birth: 0.8,
                death: 0.3,
                fossil_sampling: 0.2,
                present_sampling: 1.0,
                origin: 4.0,
            },
            ages: vec![
                TipAge {
                    minimum: 0.0,
                    maximum: 0.0,
                },
                TipAge {
                    minimum: 0.3,
                    maximum: 1.3,
                },
            ],
            clock_rate: 0.1,
            clock_prior_mean: None,
        };
        for parent in [1.0_f64, 2.0] {
            let mut tree =
                Tree::new(vec![None, None, Some((0, 1))], vec![0.0, 0.7, parent]).unwrap();
            let mut random = Random::new(91);
            let (mut below, mut n) = (0, 0);
            for i in 0..100_000 {
                let previous = tree.height(1);
                if let Some(hastings) = dating.propose_age(&mut tree, &mut random) {
                    let alpha = 2.0 * (tree.height(1) / previous).ln() + hastings;
                    if random.uniform() > alpha.exp() {
                        tree.set_height(1, previous);
                    }
                }
                assert_eq!(tree.height(0), 0.0);
                assert!(tree.height(1) < parent);
                if i > 5000 {
                    below += usize::from(tree.height(1) < 0.8);
                    n += 1;
                }
            }
            let expected =
                (0.8_f64.powi(3) - 0.3_f64.powi(3)) / (parent.min(1.3).powi(3) - 0.3_f64.powi(3));
            assert!((below as f64 / n as f64 - expected).abs() < 0.01);
        }
    }
}
