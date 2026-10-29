//! Within-chain trace diagnostics, shared by the CLI and native application.
mod spectrum;
mod trace;

pub use trace::{Column, PlotPoint, Trace};

use std::fmt;
pub const MAX_LAG: usize = 2000;
pub const ESS_METHOD: &str =
    "paired-positive autocovariance (lags 1+2, 3+4; lag < 2000), BEAST 2.7.8 convention";
pub const INTERVAL_METHOD: &str = "95% equal-tailed empirical interval; linear quantiles, R type 7";

#[derive(Debug)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub struct Statistics {
    pub mean: f64,
    pub sd: f64,
    pub interval: Option<[f64; 2]>,
    pub ess: Option<f64>,
    pub ess_reason: Option<&'static str>,
    pub last_lag: usize,
    pub lag_limit_reached: bool,
}

fn sum(values: impl Iterator<Item = f64>) -> f64 {
    let (mut total, mut correction) = (0.0, 0.0);
    for value in values {
        let next = value - correction;
        let updated = total + next;
        correction = (updated - total) - next;
        total = updated;
    }
    total
}

/// Linear interpolation on sorted observations, with the first and last values
/// at probabilities zero and one (R type 7).
pub fn quantile(sorted: &[f64], probability: f64) -> f64 {
    assert!(!sorted.is_empty() && (0.0..=1.0).contains(&probability));
    let position = (sorted.len() - 1) as f64 * probability;
    let index = position.floor() as usize;
    let fraction = position - index as f64;
    let a = sorted[index];
    let b = sorted[(index + 1).min(sorted.len() - 1)];
    ((1.0 - fraction) * a + fraction * b).clamp(a, b)
}

pub fn statistics(values: &[f64], cancel: &dyn Fn() -> bool) -> Result<Statistics> {
    if cancel() {
        return Err(Error("trace diagnostics cancelled".into()));
    }
    if values.is_empty() || values.iter().any(|x| !x.is_finite()) {
        return Err(Error("statistics require finite retained samples".into()));
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let center = min / 2.0 + max / 2.0;
    let scale = (max - center).abs().max((min - center).abs());
    let mut normalized: Vec<_> = values
        .iter()
        .map(|x| {
            if scale > 0.0 {
                (x - center) / scale
            } else {
                0.0
            }
        })
        .collect();
    let average = sum(normalized.iter().copied()) / values.len() as f64;
    let mean = (center + scale * average).clamp(min, max);
    for x in &mut normalized {
        *x -= average;
    }
    let variance = sum(normalized.iter().map(|x| x * x)) / values.len() as f64;
    let sd = scale * variance.sqrt();
    if !mean.is_finite() || !sd.is_finite() {
        return Err(Error(
            "trace magnitude exceeds supported numerical range".into(),
        ));
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(f64::total_cmp);
    let interval =
        (values.len() >= 2).then(|| [quantile(&sorted, 0.025), quantile(&sorted, 0.975)]);
    let mut result = Statistics {
        mean,
        sd,
        interval,
        ess: None,
        ess_reason: None,
        last_lag: 0,
        lag_limit_reached: false,
    };
    if min == max || variance == 0.0 {
        result.ess_reason = Some("constant");
        return Ok(result);
    }
    if values.len() < 4 {
        result.ess_reason = Some("too_short");
        return Ok(result);
    }
    let covariance = spectrum::autocovariance(&normalized, MAX_LAG.min(values.len()), cancel)?;
    let mut integral = variance;
    let mut stopped = false;
    for lag in (2..covariance.len()).step_by(2) {
        result.last_lag = lag;
        let pair = covariance[lag - 1] + covariance[lag];
        if pair <= 0.0 {
            stopped = true;
            break;
        }
        integral += 2.0 * pair;
    }
    result.lag_limit_reached = !stopped;
    let ess = values.len() as f64 * variance / integral;
    if ess.is_finite() && ess > 0.0 {
        result.ess = Some(ess.min(values.len() as f64));
    } else {
        result.ess_reason = Some("numerical_range");
    }
    Ok(result)
}
