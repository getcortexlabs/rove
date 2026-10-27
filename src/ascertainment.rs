//! Likelihoods conditioned on observing only variable characters, as in the
//! Mkv model of Lewis (2001).
//!
//! Each observed pattern's probability is divided by the probability that a
//! character is variable, `1 − Σₛ Pₛ`, where `Pₛ` is the probability that every
//! taxon is in state `s`. One constant pattern per declared state is evaluated
//! in the same core as the observations, so the correction uses the same tree,
//! model, branch rates, rate-category mixture and scaling.
//!
//! Characters that could be constant are rejected. For the others, every
//! completion of missing and uncertain cells is variable, so the result is the
//! likelihood of the observations given that the complete character varies.
//! Other schemes, such as conditioning on parsimony-informative characters or
//! per-character corrections for missing data, are not implemented.

use std::fmt;

use crate::data::Patterns;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// Every cell of the character, including missing and uncertain ones,
    /// allows `state`. The site index is zero-based within the matrix.
    ConstantCharacter { site: usize, state: char },
    /// The constant patterns hold all the probability, up to rounding error,
    /// or one of their log likelihoods is NaN or positive infinity.
    UndefinedCorrection,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConstantCharacter { site, state } => write!(
                f,
                "character {} could be constant in state {state:?}; the variable-character correction excludes constant characters",
                site + 1
            ),
            Self::UndefinedCorrection => write!(
                f,
                "the probability of a variable character is zero or undefined in double precision"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// One partition's observed patterns followed by one constant pattern per
/// declared state. The constant patterns have no observation weight.
#[derive(Clone, Debug, PartialEq)]
pub struct VariableCharacters {
    states: usize,
    weights: Vec<f64>,
    characters: f64,
    tip_partials: Vec<Vec<f64>>,
}

impl VariableCharacters {
    /// Fails on the first character in which every cell allows the same state,
    /// such as `0 ? 0` or `{01} 1 1`. Unobserved declared states still get a
    /// constant pattern.
    pub fn new(patterns: &Patterns) -> Result<Self, Error> {
        let states = patterns.symbols().len();
        let observed = patterns.weights().len();
        let tips = patterns.tip_partials();
        for pattern in 0..observed {
            let shared = (0..states).find(|&state| {
                tips.iter()
                    .all(|partials| partials[pattern * states + state] > 0.0)
            });
            if let Some(state) = shared {
                let site = patterns
                    .site_to_pattern()
                    .iter()
                    .position(|&p| p == pattern)
                    .expect("every pattern represents a site");
                return Err(Error::ConstantCharacter {
                    site: patterns.sites().start + site,
                    state: patterns.symbols()[state],
                });
            }
        }

        let mut constant = vec![0.0; states * states];
        for state in 0..states {
            constant[state * states + state] = 1.0;
        }
        Ok(Self {
            states,
            weights: patterns.weights().iter().map(|&w| w as f64).collect(),
            characters: patterns.weights().iter().sum::<usize>() as f64,
            tip_partials: tips
                .iter()
                .map(|partials| [partials.as_slice(), &constant].concat())
                .collect(),
        })
    }

    /// Observed patterns plus one constant pattern per state: the pattern
    /// count for `LikelihoodCore::new`.
    pub fn pattern_count(&self) -> usize {
        self.weights.len() + self.states
    }

    /// Pattern-major partials for each taxon, ready for `set_tip_partials`.
    pub fn tip_partials(&self) -> &[Vec<f64>] {
        &self.tip_partials
    }

    /// `ln(1 − Σₛ Pₛ)` from the log likelihoods of all `pattern_count` patterns,
    /// as returned by `LikelihoodCore::calculate_log_likelihoods`.
    pub fn log_variable_probability(&self, pattern_log_likelihoods: &[f64]) -> Result<f64, Error> {
        assert_eq!(pattern_log_likelihoods.len(), self.pattern_count());
        let constant = &pattern_log_likelihoods[self.weights.len()..];
        if constant.iter().any(|ll| ll.is_nan()) {
            return Err(Error::UndefinedCorrection);
        }
        let max = constant.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if max == f64::NEG_INFINITY {
            return Ok(0.0);
        }
        let log_constant = max + constant.iter().map(|ll| (ll - max).exp()).sum::<f64>().ln();
        // Within a few rounding errors of one, 1 − Σₛ Pₛ has no correct digits.
        let rounding = 4.0 * constant.len() as f64 * f64::EPSILON;
        if log_constant.is_nan() || log_constant > -rounding {
            return Err(Error::UndefinedCorrection);
        }
        // ln(1 − eˣ) without cancellation on either side of x = −ln 2 (Mächler, 2012).
        Ok(if log_constant > -std::f64::consts::LN_2 {
            (-log_constant.exp_m1()).ln()
        } else {
            (-log_constant.exp()).ln_1p()
        })
    }

    /// Weighted log likelihood of the observed characters, conditioned on each
    /// character being variable.
    pub fn log_likelihood(&self, pattern_log_likelihoods: &[f64]) -> Result<f64, Error> {
        let correction = self.log_variable_probability(pattern_log_likelihoods)?;
        let observed: f64 = pattern_log_likelihoods[..self.weights.len()]
            .iter()
            .zip(&self.weights)
            .map(|(ll, w)| ll * w)
            .sum();
        Ok(observed - self.characters * correction)
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, VariableCharacters};

    fn binary(constant_log_likelihoods: [f64; 2]) -> Result<f64, Error> {
        let variable = VariableCharacters {
            states: 2,
            weights: vec![1.0],
            characters: 1.0,
            tip_partials: Vec::new(),
        };
        let mut lls = vec![-1.0];
        lls.extend(constant_log_likelihoods);
        variable.log_variable_probability(&lls)
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance * expected.abs(),
            "{actual} != {expected}"
        );
    }

    #[test]
    fn small_constant_probability_keeps_full_precision() {
        let actual = binary([-20.0 * std::f64::consts::LN_10, f64::NEG_INFINITY]).unwrap();
        close(actual, -1e-20, 1e-14);
        assert_eq!(binary([f64::NEG_INFINITY; 2]), Ok(0.0));
    }

    #[test]
    fn constant_probability_near_one_keeps_input_precision() {
        // P = exp(-1e-12), so ln(1 - P) = ln(1e-12) - 5e-13 to double precision.
        let actual = binary([-1e-12, f64::NEG_INFINITY]).unwrap();
        close(actual, 1e-12f64.ln() - 5e-13, 1e-15);
    }

    #[test]
    fn total_constant_probability_of_one_is_undefined() {
        let half = 0.5f64.ln();
        assert_eq!(binary([half, half]), Err(Error::UndefinedCorrection));
        let sum_rounds_below_one = [0.3f64.ln(), 0.7f64.ln()];
        assert_eq!(
            binary(sum_rounds_below_one),
            Err(Error::UndefinedCorrection)
        );
        assert_eq!(binary([0.0, -3.0]), Err(Error::UndefinedCorrection));
        assert_eq!(
            binary([f64::INFINITY, -3.0]),
            Err(Error::UndefinedCorrection)
        );
        assert_eq!(binary([f64::NAN, -3.0]), Err(Error::UndefinedCorrection));
        assert_eq!(binary([f64::NAN; 2]), Err(Error::UndefinedCorrection));
    }
}
