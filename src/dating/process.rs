use super::FossilizedBirthDeath;
use crate::{mcmc::Move, random::Random};

/// Proper hyperpriors for the joint sampling model. Missing entries fix that
/// parameter. Specimen counts, ages and imposed tree constraints remain data;
/// the restricted tree density is not renormalized separately for each process.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProcessPriors {
    pub birth_rate_mean: Option<f64>,
    pub death_rate_mean: Option<f64>,
    pub fossil_sampling_rate_mean: Option<f64>,
    pub present_sampling_beta: Option<(f64, f64)>,
    /// Exponential prior on origin minus the oldest allowed tip age.
    pub origin_excess_mean: Option<f64>,
}

impl ProcessPriors {
    pub fn estimates(self, kind: Move) -> bool {
        match kind {
            Move::FbdBirthScale => self.birth_rate_mean.is_some(),
            Move::FbdDeathScale => self.death_rate_mean.is_some(),
            Move::FbdSamplingScale => self.fossil_sampling_rate_mean.is_some(),
            Move::FbdRhoLogit => self.present_sampling_beta.is_some(),
            Move::FbdOriginScale => self.origin_excess_mean.is_some(),
            _ => false,
        }
    }

    pub fn validate(self) -> Result<(), String> {
        let means = [
            self.birth_rate_mean,
            self.death_rate_mean,
            self.fossil_sampling_rate_mean,
            self.origin_excess_mean,
        ];
        if means.iter().flatten().any(|x| !x.is_finite() || *x <= 0.0)
            || self.present_sampling_beta.is_some_and(|(a, b)| {
                !a.is_finite()
                    || !b.is_finite()
                    || !(0.001..=1000.0).contains(&a)
                    || !(0.001..=1000.0).contains(&b)
            })
            || (means.iter().all(Option::is_none) && self.present_sampling_beta.is_none())
        {
            return Err("process priors need at least one estimated parameter, positive finite exponential means, and beta shapes in [0.001,1000]".into());
        }
        Ok(())
    }

    pub fn log_density(
        self,
        current: FossilizedBirthDeath,
        initial: FossilizedBirthDeath,
        offset: f64,
    ) -> f64 {
        let mut log = 0.0;
        for (value, fixed, mean) in [
            (current.birth, initial.birth, self.birth_rate_mean),
            (current.death, initial.death, self.death_rate_mean),
            (
                current.fossil_sampling,
                initial.fossil_sampling,
                self.fossil_sampling_rate_mean,
            ),
            (
                current.origin - offset,
                initial.origin - offset,
                self.origin_excess_mean,
            ),
        ] {
            if let Some(mean) = mean {
                if !value.is_finite() || value <= 0.0 {
                    return f64::NEG_INFINITY;
                }
                log += -mean.ln() - value / mean;
            } else if value != fixed {
                return f64::NEG_INFINITY;
            }
        }
        let rho = current.present_sampling;
        if let Some((a, b)) = self.present_sampling_beta {
            if rho <= 0.0 || rho >= 1.0 || !rho.is_finite() {
                return f64::NEG_INFINITY;
            }
            log +=
                (a - 1.0) * rho.ln() + (b - 1.0) * (-rho).ln_1p() + crate::rates::log_gamma(a + b)
                    - crate::rates::log_gamma(a)
                    - crate::rates::log_gamma(b);
        } else if rho != initial.present_sampling {
            return f64::NEG_INFINITY;
        }
        log
    }

    pub(crate) fn propose(
        self,
        kind: Move,
        process: &mut FossilizedBirthDeath,
        offset: f64,
        tuning: f64,
        random: &mut Random,
    ) -> Option<f64> {
        if !self.estimates(kind) {
            return None;
        }
        if kind == Move::FbdRhoLogit {
            let old = process.present_sampling;
            let z = old.ln() - (-old).ln_1p() + (2.0 * random.uniform() - 1.0) * (-tuning.ln());
            let next = if z >= 0.0 {
                1.0 / (1.0 + (-z).exp())
            } else {
                z.exp() / (1.0 + z.exp())
            };
            if next <= 0.0 || next >= 1.0 {
                return None;
            }
            process.present_sampling = next;
            return Some(next.ln() + (-next).ln_1p() - old.ln() - (-old).ln_1p());
        }
        let factor = tuning + random.uniform() * (1.0 / tuning - tuning);
        let value = match kind {
            Move::FbdBirthScale => &mut process.birth,
            Move::FbdDeathScale => &mut process.death,
            Move::FbdSamplingScale => &mut process.fossil_sampling,
            Move::FbdOriginScale => &mut process.origin,
            _ => return None,
        };
        let shift = if kind == Move::FbdOriginScale {
            offset
        } else {
            0.0
        };
        let next = shift + (*value - shift) * factor;
        if !next.is_finite() || next <= shift {
            return None;
        }
        *value = next;
        Some(-factor.ln())
    }
}
