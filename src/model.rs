#[derive(Clone, Debug, PartialEq)]
pub struct Mk {
    frequencies: Vec<f64>,
    total_rate: f64,
}

impl Mk {
    pub fn uniform(states: usize) -> Self {
        assert!(states >= 2, "need at least two states");
        Self::with_frequencies(vec![1.0 / states as f64; states])
    }

    pub fn with_frequencies(frequencies: Vec<f64>) -> Self {
        assert!(frequencies.len() >= 2, "need at least two states");
        let total_rate = 1.0 - frequencies.iter().map(|f| f * f).sum::<f64>();
        Self {
            frequencies,
            total_rate,
        }
    }

    pub fn states(&self) -> usize {
        self.frequencies.len()
    }

    pub fn frequencies(&self) -> &[f64] {
        &self.frequencies
    }

    pub fn transition_matrix(&self, distance: f64, out: &mut [f64]) {
        let k = self.states();
        assert_eq!(out.len(), k * k);
        let stay = (-distance / self.total_rate).exp();
        let change = 1.0 - stay;
        for (i, row) in out.chunks_exact_mut(k).enumerate() {
            for (j, (p, f)) in row.iter_mut().zip(&self.frequencies).enumerate() {
                *p = f * change + if i == j { stay } else { 0.0 };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Mk;

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
        let model = Mk::with_frequencies(vec![0.1, 0.2, 0.7]);
        let m = matrix(&model, 200.0);
        for row in m.chunks_exact(3) {
            for (p, f) in row.iter().zip(model.frequencies()) {
                assert!((p - f).abs() < 1e-12);
            }
        }
    }
}
