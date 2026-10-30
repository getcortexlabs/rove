//! Rate variation of the reference model: four midpoint gamma quantiles,
//! and independent lognormal branch rates with real-space mean one.

use std::fmt;

/// Bounds imposed by BEAST 2.7.8's SiteModel, including when sampling the prior.
pub const SHAPE_BOUNDS: std::ops::RangeInclusive<f64> = 1e-3..=1e3;
const LOG_SQRT_TAU: f64 = 0.918_938_533_204_672_7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Shape,
    Sigma,
    BranchRates,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Shape => "gamma shape must be finite and between 0.001 and 1000",
            Self::Sigma => "clock standard deviation must be positive and finite",
            Self::BranchRates => {
                "branch rates must be positive and finite, with one entry per non-root node"
            }
        })
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GammaRates {
    shape: f64,
    rates: [f64; 4],
}

impl GammaRates {
    pub const WEIGHTS: [f64; 4] = [0.25; 4];

    /// Quantiles at 1/8, 3/8, 5/8 and 7/8, normalized to arithmetic mean one.
    /// These are midpoint quantiles, not the means of four probability bins.
    pub fn new(shape: f64) -> Result<Self, Error> {
        if !SHAPE_BOUNDS.contains(&shape) {
            return Err(Error::Shape);
        }
        let log_gamma = log_gamma(shape + 1.0);
        let logs = std::array::from_fn::<_, 4, _>(|i| {
            log_quantile(shape, (i as f64 + 0.5) / 4.0, log_gamma)
        });
        // The scale 1/shape cancels in normalization. Subtracting the largest
        // log quantile also preserves relative rates when tiny quantiles underflow.
        let mut rates = logs.map(|x| (x - logs[3]).exp());
        let mean = rates.iter().sum::<f64>() / 4.0;
        for rate in &mut rates {
            *rate /= mean;
        }
        Ok(Self { shape, rates })
    }

    pub fn shape(&self) -> f64 {
        self.shape
    }

    pub fn rates(&self) -> &[f64; 4] {
        &self.rates
    }

    /// Exponential(mean=1), as evaluated in the reference XML. The parameter
    /// bounds restrict its support without adding a truncation normalizer.
    pub fn log_prior(&self) -> f64 {
        -self.shape
    }
}

/// Log Gamma for positive arguments, by recurrence and Stirling's series
/// (DLMF 5.5.1 and 5.11.1). Shifting to at least 16 makes the first omitted
/// term smaller than 2e-18; rounding dominates that truncation error.
fn log_gamma(mut x: f64) -> f64 {
    let mut shift = 0.0;
    while x < 16.0 {
        shift += x.ln();
        x += 1.0;
    }
    let inv = 1.0 / x;
    let square = inv * inv;
    let correction = inv
        * (1.0 / 12.0
            + square
                * (-1.0 / 360.0
                    + square
                        * (1.0 / 1260.0
                            + square
                                * (-1.0 / 1680.0
                                    + square * (1.0 / 1188.0 - square * 691.0 / 360360.0)))));
    (x - 0.5) * x.ln() - x + LOG_SQRT_TAU + correction - shift
}

/// Invert the regularized lower gamma in log(x), using bracketed Newton
/// steps and the positive series in DLMF 8.7.1. The derivative of log(P)
/// with respect to log(x) is shape/sum. Working in logs keeps the first
/// quantile usable even at shape=0.001, where x itself rounds to zero.
fn log_quantile(shape: f64, probability: f64, log_gamma: f64) -> f64 {
    let target = probability.ln();
    let mut lower = (target + log_gamma) / shape - 1.0;
    let mut upper = (shape + 10.0 * shape.sqrt() + 10.0).ln();
    let mut value = shape.ln().clamp(lower, upper);
    for _ in 0..100 {
        let x = value.exp();
        let (mut term, mut sum) = (1.0, 1.0);
        for n in 1..10_000 {
            term *= x / (shape + n as f64);
            sum += term;
            if term <= sum * f64::EPSILON {
                break;
            }
        }
        let residual = shape * value - x - log_gamma + sum.ln() - target;
        if residual > 0.0 {
            upper = value;
        } else {
            lower = value;
        }
        let step = residual * sum / shape;
        if step.abs() <= 8.0 * f64::EPSILON * value.abs().max(1.0)
            || upper - lower <= 8.0 * f64::EPSILON * value.abs().max(1.0)
        {
            return value;
        }
        let next = value - step;
        value = if lower < next && next < upper {
            next
        } else {
            (lower + upper) / 2.0
        };
    }
    value
}

/// The explicit rate parameterization of UCRelaxedClockModel, with
/// normalize=false and clock.rate=1. Rates remain attached to parameter
/// indices through topology moves. If the last node is not the root, it uses
/// the root's index in the rate vector; the root itself has no rate.
#[derive(Debug, PartialEq)]
pub struct RelaxedClock {
    sigma: f64,
    rates: Vec<f64>,
}

impl Clone for RelaxedClock {
    fn clone(&self) -> Self {
        Self {
            sigma: self.sigma,
            rates: self.rates.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.sigma = source.sigma;
        self.rates.clone_from(&source.rates);
    }
}

impl RelaxedClock {
    pub fn new(sigma: f64, rates: Vec<f64>) -> Result<Self, Error> {
        if !(sigma > 0.0 && sigma.is_finite()) {
            return Err(Error::Sigma);
        }
        if rates.len() < 2 || rates.iter().any(|&r| !valid_rate(r)) {
            return Err(Error::BranchRates);
        }
        Ok(Self { sigma, rates })
    }

    pub fn sigma(&self) -> f64 {
        self.sigma
    }

    pub fn rates(&self) -> &[f64] {
        &self.rates
    }

    pub fn branch_rate(&self, node: usize, root: usize) -> f64 {
        assert!(node <= self.rates.len() && root <= self.rates.len());
        if node == root {
            1.0
        } else {
            self.rates[if node == self.rates.len() { root } else { node }]
        }
    }

    /// Independent LogNormal(mu=-sigma²/2, sigma), so E[rate]=1. The realized
    /// arithmetic or branch-length-weighted mean is not normalized.
    pub fn log_rate_prior(&self) -> f64 {
        let mean = -0.5 * self.sigma * self.sigma;
        let normalizer = self.sigma.ln() + LOG_SQRT_TAU;
        self.rates
            .iter()
            .map(|r| {
                let log = r.ln();
                let z = (log - mean) / self.sigma;
                -0.5 * z * z - normalizer - log
            })
            .sum()
    }

    /// Gamma(shape=5, scale=0.2), not Gamma(shape=5, rate=0.2).
    pub fn log_sigma_prior(&self) -> f64 {
        4.0 * self.sigma.ln() - self.sigma / 0.2 - 24.0f64.ln() - 5.0 * 0.2f64.ln()
    }

    pub(crate) fn set_sigma(&mut self, sigma: f64) -> bool {
        if !(sigma > 0.0 && sigma.is_finite()) {
            return false;
        }
        self.sigma = sigma;
        true
    }

    pub(crate) fn set_rate(&mut self, index: usize, rate: f64) -> bool {
        if !valid_rate(rate) {
            return false;
        }
        self.rates[index] = rate;
        true
    }
}

fn valid_rate(rate: f64) -> bool {
    rate > 0.0 && rate.is_finite()
}

/// Optional variation across characters and branches, with an explicit
/// global clock for dated analyses. The mutation-rate multiplier remains one.
#[derive(Debug, PartialEq, Default)]
pub struct RateVariation {
    /// Global clock rate for dated trees; None preserves the unit clock.
    pub mean: Option<f64>,
    pub gamma: Option<GammaRates>,
    pub clock: Option<RelaxedClock>,
}

impl Clone for RateVariation {
    fn clone(&self) -> Self {
        Self {
            mean: self.mean,
            gamma: self.gamma,
            clock: self.clock.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.mean = source.mean;
        self.gamma = source.gamma;
        self.clock.clone_from(&source.clock);
    }
}

impl RateVariation {
    pub fn category_rates(&self) -> &[f64] {
        self.gamma.as_ref().map_or(&[1.0], |gamma| &gamma.rates)
    }

    pub fn category_weights(&self) -> &[f64] {
        if self.gamma.is_some() {
            &GammaRates::WEIGHTS
        } else {
            &[1.0]
        }
    }

    pub fn branch_rate(&self, node: usize, root: usize) -> f64 {
        self.mean.unwrap_or(1.0)
            * self
                .clock
                .as_ref()
                .map_or(1.0, |clock| clock.branch_rate(node, root))
    }
}

#[cfg(test)]
mod tests {
    use super::{log_gamma, log_quantile};

    #[test]
    fn integer_gamma_quantiles_satisfy_the_erlang_cdf() {
        // An independent finite expression: P(n,x) = 1-e^-x Σ(k<n) x^k/k!.
        for shape in [1, 2, 5, 10] {
            for probability in [0.125, 0.375, 0.625, 0.875] {
                let x =
                    log_quantile(shape as f64, probability, log_gamma(shape as f64 + 1.0)).exp();
                let (mut term, mut sum) = (1.0, 1.0);
                for k in 1..shape {
                    term *= x / k as f64;
                    sum += term;
                }
                let cdf = 1.0 - (-x).exp() * sum;
                assert!(
                    (cdf - probability).abs() < 2e-14,
                    "shape {shape}, p={probability}: {cdf}"
                );
            }
        }
    }
}
