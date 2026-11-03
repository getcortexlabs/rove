//! A morphology/DNA rate ratio. DNA fixes the reference rate at one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelativeRate {
    /// Fixed value or the estimated rate's starting value.
    pub value: f64,
    /// None fixes the rate; Some(s) uses log(rate) ~ Normal(0, s²).
    pub log_sd: Option<f64>,
}
impl RelativeRate {
    pub fn validate(self) -> Result<(), super::Error> {
        if !self.value.is_finite()
            || self.value <= 0.0
            || self.log_sd.is_some_and(|s| !s.is_finite() || s <= 0.0)
        {
            return Err(super::Error::Model(
                "morphology rate and lognormal log-SD must be positive and finite".into(),
            ));
        }
        Ok(())
    }
    /// Normalized density with respect to d(rate), including its Jacobian.
    pub fn log_density(self, rate: f64) -> f64 {
        if !rate.is_finite() || rate <= 0.0 {
            return f64::NEG_INFINITY;
        }
        match self.log_sd {
            None => {
                if rate == self.value {
                    0.0
                } else {
                    f64::NEG_INFINITY
                }
            }
            Some(s) => {
                let z = rate.ln();
                -z - s.ln() - 0.5 * (2.0 * std::f64::consts::PI).ln() - 0.5 * (z / s).powi(2)
            }
        }
    }
}
