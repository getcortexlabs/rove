use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    TooFewStates(usize),
    /// A frequency is zero, negative or not finite. The state is zero-based.
    InvalidFrequency {
        state: usize,
        frequency: f64,
    },
    /// The frequencies differ from one by more than rounding error.
    UnnormalizedFrequencies {
        sum: f64,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewStates(states) => {
                write!(f, "Mk needs at least two states, not {states}")
            }
            Self::InvalidFrequency { state, frequency } => write!(
                f,
                "the frequency of state {state} is {frequency}; frequencies must be positive and finite"
            ),
            Self::UnnormalizedFrequencies { sum } => {
                write!(f, "the frequencies sum to {sum}, not one")
            }
        }
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug, PartialEq)]
pub struct Mk {
    frequencies: Vec<f64>,
    total_rate: f64,
}

impl Mk {
    pub fn uniform(states: usize) -> Self {
        assert!(states >= 2, "need at least two states");
        Self::with_frequencies(vec![1.0 / states as f64; states])
            .expect("uniform frequencies are valid")
    }

    /// Accepts at least two frequencies, each positive and finite, that sum to
    /// one up to rounding error. They are used as given, without normalizing.
    pub fn with_frequencies(frequencies: Vec<f64>) -> Result<Self, Error> {
        let states = frequencies.len();
        if states < 2 {
            return Err(Error::TooFewStates(states));
        }
        if let Some((state, &frequency)) = frequencies
            .iter()
            .enumerate()
            .find(|(_, f)| !(f.is_finite() && **f > 0.0))
        {
            return Err(Error::InvalidFrequency { state, frequency });
        }
        let sum: f64 = frequencies.iter().sum();
        if (sum - 1.0).abs() > 4.0 * states as f64 * f64::EPSILON {
            return Err(Error::UnnormalizedFrequencies { sum });
        }
        // One expected change per unit distance at equilibrium. Σπ(1 − π) equals
        // 1 − Σπ² here, without cancellation when one frequency is close to one.
        let total_rate = frequencies.iter().map(|f| f * (1.0 - f)).sum();
        Ok(Self {
            frequencies,
            total_rate,
        })
    }

    pub fn states(&self) -> usize {
        self.frequencies.len()
    }

    pub fn frequencies(&self) -> &[f64] {
        &self.frequencies
    }

    /// `e` and `1 − e` in `P(t) = e·I + (1 − e)·1πᵀ`. Both are computed
    /// directly, so `1 − e` keeps its precision on short branches.
    pub(crate) fn stay_and_change(&self, distance: f64) -> (f64, f64) {
        assert!(
            distance >= 0.0,
            "branch distances must be non-negative, not {distance}"
        );
        let x = -distance / self.total_rate;
        (x.exp(), -x.exp_m1())
    }

    pub fn transition_matrix(&self, distance: f64, out: &mut [f64]) {
        let k = self.states();
        assert_eq!(out.len(), k * k);
        let (stay, change) = self.stay_and_change(distance);
        for (i, row) in out.chunks_exact_mut(k).enumerate() {
            for (j, (p, f)) in row.iter_mut().zip(&self.frequencies).enumerate() {
                *p = f * change + if i == j { stay } else { 0.0 };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, Mk};

    fn matrix(model: &Mk, distance: f64) -> Vec<f64> {
        let k = model.states();
        let mut out = vec![0.0; k * k];
        model.transition_matrix(distance, &mut out);
        out
    }

    #[test]
    fn rows_sum_to_one() {
        for k in 2..=8 {
            let m = matrix(&Mk::uniform(k), 0.37);
            for row in m.chunks_exact(k) {
                assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn zero_distance_is_identity() {
        let m = matrix(&Mk::uniform(3), 0.0);
        assert_eq!(m, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn matches_lewis_closed_form() {
        let k = 5;
        let t = 0.8;
        let m = matrix(&Mk::uniform(k), t);
        let delta = (k as f64 / (k as f64 - 1.0)) * t;
        let stay = (1.0 + (k as f64 - 1.0) * (-delta).exp()) / k as f64;
        let change = (1.0 - (-delta).exp()) / k as f64;
        for i in 0..k {
            for j in 0..k {
                let expected = if i == j { stay } else { change };
                assert!((m[i * k + j] - expected).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn long_distance_converges_to_frequencies() {
        let model = Mk::with_frequencies(vec![0.1, 0.2, 0.7]).unwrap();
        let m = matrix(&model, 200.0);
        for row in m.chunks_exact(3) {
            for (p, f) in row.iter().zip(model.frequencies()) {
                assert!((p - f).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn short_distances_keep_one_expected_change_per_unit() {
        for model in [
            Mk::uniform(2),
            Mk::uniform(5),
            Mk::with_frequencies(vec![0.1, 0.2, 0.3, 0.4]).unwrap(),
        ] {
            let k = model.states();
            for distance in [1e-12, 1e-9, 1e-6] {
                let m = matrix(&model, distance);
                // Probability of ending in another state, from off-diagonal entries only.
                let changed: f64 = model
                    .frequencies()
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        f * (0..k)
                            .filter(|&j| j != i)
                            .map(|j| m[i * k + j])
                            .sum::<f64>()
                    })
                    .sum();
                assert!(
                    (changed / distance - 1.0).abs() <= 2.0 * distance + 1e-14,
                    "{k} states, distance {distance}: {changed}"
                );
            }
        }
    }

    #[test]
    fn a_frequency_close_to_one_keeps_a_positive_rate() {
        let model = Mk::with_frequencies(vec![1.0, 1e-17]).unwrap();
        assert_eq!(matrix(&model, 0.0), vec![1.0, 0.0, 0.0, 1.0]);
        for row in matrix(&model, 1e-12).chunks_exact(2) {
            assert_eq!(row, model.frequencies());
        }
    }

    #[test]
    fn frequencies_are_validated() {
        for (frequencies, error) in [
            (vec![], Error::TooFewStates(0)),
            (vec![1.0], Error::TooFewStates(1)),
            (
                vec![1.0, 0.0],
                Error::InvalidFrequency {
                    state: 1,
                    frequency: 0.0,
                },
            ),
            (
                vec![0.6, -0.1, 0.5],
                Error::InvalidFrequency {
                    state: 1,
                    frequency: -0.1,
                },
            ),
            (
                vec![0.5, f64::INFINITY],
                Error::InvalidFrequency {
                    state: 1,
                    frequency: f64::INFINITY,
                },
            ),
            (vec![0.5, 0.6], Error::UnnormalizedFrequencies { sum: 1.1 }),
            (
                vec![0.3, 0.3, 0.3],
                Error::UnnormalizedFrequencies {
                    sum: 0.3 + 0.3 + 0.3,
                },
            ),
        ] {
            assert_eq!(Mk::with_frequencies(frequencies), Err(error));
        }
        assert!(matches!(
            Mk::with_frequencies(vec![f64::NAN, 0.5]),
            Err(Error::InvalidFrequency { state: 0, .. })
        ));
        let off_by_more_than_rounding = 1.0 - 1e-12;
        assert!(Mk::with_frequencies(vec![0.5, off_by_more_than_rounding - 0.5]).is_err());
    }

    #[test]
    fn frequencies_that_sum_to_one_up_to_rounding_are_accepted() {
        for k in 2..=64 {
            assert_eq!(Mk::uniform(k).states(), k);
        }
        for frequencies in [
            vec![0.7, 0.2, 0.1],
            vec![0.1, 0.2, 0.7],
            vec![0.15, 0.25, 0.6],
            vec![0.05; 20],
        ] {
            assert!(
                Mk::with_frequencies(frequencies.clone()).is_ok(),
                "{frequencies:?}"
            );
        }
        let weights = [3.0, 7.0, 11.0, 13.0, 17.0, 19.0];
        let total: f64 = weights.iter().sum();
        let normalized = weights.iter().map(|w| w / total).collect();
        assert!(Mk::with_frequencies(normalized).is_ok());
    }
}
