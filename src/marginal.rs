//! Marginal likelihood from power posteriors: a schedule of likelihood
//! exponents, and the stepping-stone and path-sampling estimators with
//! their Monte Carlo standard errors.

use std::fmt;

use crate::diagnostics::{self, statistics};

#[derive(Clone, Debug, PartialEq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<diagnostics::Error> for Error {
    fn from(error: diagnostics::Error) -> Self {
        Self(error.0)
    }
}

/// Likelihood exponents `0 = β_0 < … < β_{n−1} = 1`: the quantiles of a
/// Beta(α, 1) distribution at `i / (n − 1)`, so `β_i = (i / (n − 1))^(1/α)`.
/// Small `α` places more exponents near zero, where the integrand changes
/// fastest (Xie et al. 2011).
pub fn schedule(powers: usize, alpha: f64) -> Result<Vec<f64>, Error> {
    if powers < 2 {
        return Err(Error(format!(
            "a schedule needs at least two powers, not {powers}"
        )));
    }
    if !alpha.is_finite() || alpha <= 0.0 {
        return Err(Error(format!(
            "the schedule shape must be finite and positive, not {alpha}"
        )));
    }
    let last = (powers - 1) as f64;
    Ok((0..powers)
        .map(|i| (i as f64 / last).powf(1.0 / alpha))
        .collect())
}

/// Retained log-likelihood samples from one power posterior.
#[derive(Clone, Debug, PartialEq)]
pub struct PowerSample {
    pub power: f64,
    pub log_likelihoods: Vec<f64>,
}

/// Summary of one power posterior's log-likelihood samples.
#[derive(Clone, Debug, PartialEq)]
pub struct PowerSummary {
    pub power: f64,
    pub samples: usize,
    pub mean: f64,
    pub sd: f64,
    pub ess: Option<f64>,
    pub lag_limit_reached: bool,
}

/// One ratio `Z(upper) / Z(lower)` estimated from samples at `lower`.
#[derive(Clone, Debug, PartialEq)]
pub struct Stone {
    pub lower: f64,
    pub upper: f64,
    pub log_ratio: f64,
    /// Variance of `log_ratio`; None when the weights' ESS is unavailable.
    pub variance: Option<f64>,
    pub ess: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Estimate {
    pub log_marginal_likelihood: f64,
    /// None when any contributing ESS is unavailable.
    pub standard_error: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Analysis {
    pub powers: Vec<PowerSummary>,
    pub stones: Vec<Stone>,
    pub stepping_stone: Estimate,
    pub path_sampling: Estimate,
}

/// Mean of `values` and the variance of that mean, `sd² / ESS`. A constant
/// series has variance zero; a series without a usable ESS has None.
fn mean_and_variance(values: &[f64]) -> Result<(diagnostics::Statistics, Option<f64>), Error> {
    let stats = statistics(values, &|| false)?;
    let variance = match (stats.ess, stats.ess_reason) {
        _ if values.len() < 4 => None,
        (Some(ess), _) => Some(stats.sd * stats.sd / ess),
        (None, Some("constant")) => Some(0.0),
        _ => None,
    };
    Ok((stats, variance))
}

fn sum_option(terms: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    terms.sum()
}

/// Stepping-stone and path-sampling estimates from samples ordered by
/// increasing power, starting at zero and ending at one.
///
/// Stepping stone (Xie et al. 2011): `log Z = Σ_k log r_k` with
/// `r_k = E_{β_{k−1}}[L^{β_k − β_{k−1}}]`, each estimated from the samples at
/// `β_{k−1}` after factoring out the largest log-likelihood. Its error adds
/// the delta-method variances of the `log r_k`, treating stones as
/// independent. Path sampling (Lartillot and Philippe 2006) applies the
/// trapezoidal rule to the mean log-likelihood along the schedule; its error
/// adds the variances of those means with the trapezoid coefficients. The
/// path-sampling discretization error is not estimated.
pub fn analyse(samples: &[PowerSample]) -> Result<Analysis, Error> {
    if samples.len() < 2 {
        return Err(Error("at least two power posteriors are required".into()));
    }
    let powers: Vec<f64> = samples.iter().map(|s| s.power).collect();
    let increasing = powers
        .windows(2)
        .all(|w| w[1].partial_cmp(&w[0]) == Some(std::cmp::Ordering::Greater));
    if powers[0] != 0.0 || *powers.last().unwrap() != 1.0 || !increasing {
        return Err(Error(
            "powers must increase strictly from zero to one".into(),
        ));
    }
    let mut summaries = Vec::with_capacity(samples.len());
    let mut mean_variances = Vec::with_capacity(samples.len());
    for sample in samples {
        let (stats, variance) = mean_and_variance(&sample.log_likelihoods)?;
        summaries.push(PowerSummary {
            power: sample.power,
            samples: sample.log_likelihoods.len(),
            mean: stats.mean,
            sd: stats.sd,
            ess: stats.ess,
            lag_limit_reached: stats.lag_limit_reached,
        });
        mean_variances.push(variance);
    }

    let mut stones = Vec::with_capacity(samples.len() - 1);
    for k in 1..samples.len() {
        let (lower, upper) = (powers[k - 1], powers[k]);
        let difference = upper - lower;
        let values = &samples[k - 1].log_likelihoods;
        let largest = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let weights: Vec<f64> = values
            .iter()
            .map(|&l| (difference * (l - largest)).exp())
            .collect();
        let (stats, variance) = mean_and_variance(&weights)?;
        stones.push(Stone {
            lower,
            upper,
            log_ratio: difference * largest + stats.mean.ln(),
            variance: variance.map(|v| v / (stats.mean * stats.mean)),
            ess: stats.ess,
        });
    }
    let stepping_stone = Estimate {
        log_marginal_likelihood: stones.iter().map(|s| s.log_ratio).sum(),
        standard_error: sum_option(stones.iter().map(|s| s.variance)).map(f64::sqrt),
    };

    let n = powers.len();
    let coefficient = |k: usize| {
        let before = if k == 0 {
            0.0
        } else {
            powers[k] - powers[k - 1]
        };
        let after = if k + 1 == n {
            0.0
        } else {
            powers[k + 1] - powers[k]
        };
        (before + after) / 2.0
    };
    let path_sampling = Estimate {
        log_marginal_likelihood: (0..n).map(|k| coefficient(k) * summaries[k].mean).sum(),
        standard_error: sum_option(
            (0..n).map(|k| mean_variances[k].map(|v| coefficient(k) * coefficient(k) * v)),
        )
        .map(f64::sqrt),
    };
    Ok(Analysis {
        powers: summaries,
        stones,
        stepping_stone,
        path_sampling,
    })
}

#[cfg(test)]
mod tests {
    use super::{PowerSample, analyse, schedule};

    #[test]
    fn the_schedule_follows_beta_quantiles() {
        let powers = schedule(5, 0.5).unwrap();
        assert_eq!(powers, vec![0.0, 0.0625, 0.25, 0.5625, 1.0]);
        assert_eq!(schedule(2, 0.3).unwrap(), vec![0.0, 1.0]);
        assert!(schedule(1, 0.3).is_err());
        assert!(schedule(4, 0.0).is_err());
        assert!(schedule(4, f64::NAN).is_err());
    }

    #[test]
    fn constant_likelihoods_give_the_exact_answer() {
        // With L ≡ e^c the marginal likelihood is e^c under any prior.
        let powers = schedule(6, 0.3).unwrap();
        let samples: Vec<PowerSample> = powers
            .iter()
            .map(|&power| PowerSample {
                power,
                log_likelihoods: vec![-12.5; 50],
            })
            .collect();
        let analysis = analyse(&samples).unwrap();
        assert!((analysis.stepping_stone.log_marginal_likelihood + 12.5).abs() < 1e-12);
        assert!((analysis.path_sampling.log_marginal_likelihood + 12.5).abs() < 1e-12);
        assert_eq!(analysis.stepping_stone.standard_error, Some(0.0));
        assert_eq!(analysis.path_sampling.standard_error, Some(0.0));
        assert_eq!(analysis.stones.len(), 5);
    }

    #[test]
    fn short_series_have_no_standard_error() {
        let samples: Vec<PowerSample> = schedule(4, 0.3)
            .unwrap()
            .into_iter()
            .map(|power| PowerSample {
                power,
                log_likelihoods: vec![-2.0 - power, -3.0],
            })
            .collect();
        let analysis = analyse(&samples).unwrap();
        assert!(analysis.stepping_stone.log_marginal_likelihood.is_finite());
        assert_eq!(analysis.stepping_stone.standard_error, None);
        assert_eq!(analysis.path_sampling.standard_error, None);
        assert!(analysis.stones.iter().all(|s| s.variance.is_none()));
        let single: Vec<PowerSample> = samples
            .iter()
            .map(|s| PowerSample {
                power: s.power,
                log_likelihoods: vec![-5.0],
            })
            .collect();
        assert_eq!(
            analyse(&single).unwrap().stepping_stone.standard_error,
            None
        );
    }

    #[test]
    fn samples_must_cover_the_unit_interval_in_order() {
        let sample = |power| PowerSample {
            power,
            log_likelihoods: vec![1.0, 2.0, 3.0, 4.0],
        };
        assert!(analyse(&[sample(0.0)]).is_err());
        assert!(analyse(&[sample(0.0), sample(0.5)]).is_err());
        assert!(analyse(&[sample(0.1), sample(1.0)]).is_err());
        assert!(analyse(&[sample(0.0), sample(0.5), sample(0.5), sample(1.0)]).is_err());
        assert!(analyse(&[sample(0.0), sample(1.0)]).is_ok());
    }
}
