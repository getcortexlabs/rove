/// A symmetric nearest-neighbour CTMC on a finite path. Adjacent rates are
/// k / (2(k - 1)), giving one expected change per unit distance under uniform
/// stationary frequencies. Nonadjacent states can communicate through multiple
/// changes along a branch; their instantaneous rates, not probabilities, are zero.
#[derive(Clone, Debug)]
pub struct Ordered {
    states: usize,
    term: Vec<f64>,
    scratch: Vec<f64>,
}

impl Ordered {
    pub fn new(states: usize) -> Self {
        assert!(states >= 2, "need at least two states");
        Self {
            states,
            term: vec![0.0; states * states],
            scratch: vec![0.0; states * states],
        }
    }

    pub fn states(&self) -> usize {
        self.states
    }

    /// Uniformization uses only nonnegative terms, retaining tiny probabilities
    /// on short branches without subtracting nearly equal eigenmode sums.
    /// Scaling and squaring bounds the Poisson mean by one. Workspaces are reused.
    pub fn transition_matrix(&mut self, distance: f64, out: &mut [f64]) {
        assert!(distance >= 0.0, "branch distances must be non-negative");
        let k = self.states;
        assert_eq!(out.len(), k * k);
        let rate = k as f64 / (k - 1) as f64;
        let gap = 2.0 * rate * (std::f64::consts::PI / (2 * k) as f64).sin().powi(2);
        if distance >= 40.0 / gap {
            // Every nonstationary eigenmode is below exp(-40).
            out.fill(1.0 / k as f64);
            return;
        }
        out.fill(0.0);
        self.term.fill(0.0);
        let mean = rate * distance;
        let squares = if mean > 1.0 {
            mean.log2().ceil() as i32
        } else {
            0
        };
        let mean = mean * 2.0_f64.powi(-squares);
        for i in 0..k {
            self.term[i * k + i] = (-mean).exp();
            out[i * k + i] = self.term[i * k + i];
        }
        if distance == 0.0 {
            return;
        }
        for n in 1.. {
            // R = I + Q/rate: off-diagonal neighbours are 1/2, with a
            // self-loop of 1/2 at each endpoint and zero elsewhere.
            let factor = mean / (2 * n) as f64;
            let mut negligible = n >= k - 1;
            for i in 0..k {
                for j in 0..k {
                    let at = i * k + j;
                    let left = if j > 0 {
                        self.term[at - 1]
                    } else {
                        self.term[at]
                    };
                    let right = if j + 1 < k {
                        self.term[at + 1]
                    } else {
                        self.term[at]
                    };
                    let value = (left + right) * factor;
                    self.scratch[at] = value;
                    out[at] += value;
                    negligible &= value <= out[at] * (f64::EPSILON / 8.0);
                }
            }
            std::mem::swap(&mut self.term, &mut self.scratch);
            if negligible {
                break;
            }
        }
        stochastic_diagonal(out, k);
        for _ in 0..squares {
            for i in 0..k {
                for j in i..k {
                    let value = (0..k).map(|s| out[i * k + s] * out[s * k + j]).sum();
                    self.scratch[i * k + j] = value;
                    self.scratch[j * k + i] = value;
                }
            }
            out.copy_from_slice(&self.scratch);
            stochastic_diagonal(out, k);
        }
    }
}

fn stochastic_diagonal(matrix: &mut [f64], states: usize) {
    // Correct only roundoff in row sums; off-diagonal probabilities remain
    // positive and symmetric, including values much smaller than epsilon.
    for (i, row) in matrix.chunks_exact_mut(states).enumerate() {
        row[i] = 1.0
            - row
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, p)| p)
                .sum::<f64>();
    }
}
