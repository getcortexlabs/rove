pub struct Problem<'a> {
    pub children: &'a [Option<(usize, usize)>],
    pub root: usize,
    pub tip_partials: &'a [Vec<f64>],
    pub matrices: &'a [Vec<f64>],
    pub category_weights: &'a [f64],
    pub frequencies: &'a [f64],
    pub pattern_weights: &'a [f64],
}

impl Problem<'_> {
    pub fn log_likelihood(&self) -> f64 {
        self.site_likelihoods()
            .iter()
            .zip(self.pattern_weights)
            .map(|(s, w)| s.ln() * w)
            .sum()
    }

    /// Mkv: each site likelihood divided by one minus the probabilities of the
    /// patterns in which every tip is observed in the same state.
    pub fn variable_log_likelihood(&self) -> f64 {
        let states = self.frequencies.len();
        let mut constant = vec![0.0; states * states];
        for state in 0..states {
            constant[state * states + state] = 1.0;
        }
        let tips = vec![constant; self.tip_partials.len()];
        let weights = vec![1.0; states];
        let constant = Problem {
            tip_partials: &tips,
            pattern_weights: &weights,
            ..*self
        };
        let variable = 1.0 - constant.site_likelihoods().iter().sum::<f64>();
        self.site_likelihoods()
            .iter()
            .zip(self.pattern_weights)
            .map(|(s, w)| (s / variable).ln() * w)
            .sum()
    }

    fn site_likelihoods(&self) -> Vec<f64> {
        let states = self.frequencies.len();
        let patterns = self.pattern_weights.len();
        let mut site = vec![0.0; patterns];
        for (category, &weight) in self.category_weights.iter().enumerate() {
            let root = self.partials(self.root, category);
            for (p, s) in site.iter_mut().enumerate() {
                let row = &root[p * states..(p + 1) * states];
                *s += weight
                    * row
                        .iter()
                        .zip(self.frequencies)
                        .map(|(x, f)| x * f)
                        .sum::<f64>();
            }
        }
        site
    }

    fn partials(&self, node: usize, category: usize) -> Vec<f64> {
        let Some((a, b)) = self.children[node] else {
            return self.tip_partials[node].clone();
        };
        let k = self.frequencies.len();
        let patterns = self.pattern_weights.len();
        let pa = self.partials(a, category);
        let pb = self.partials(b, category);
        let ma = &self.matrices[a][category * k * k..(category + 1) * k * k];
        let mb = &self.matrices[b][category * k * k..(category + 1) * k * k];
        let mut out = vec![0.0; patterns * k];
        for p in 0..patterns {
            for i in 0..k {
                let mut s1 = 0.0;
                let mut s2 = 0.0;
                for j in 0..k {
                    s1 += ma[i * k + j] * pa[p * k + j];
                    s2 += mb[i * k + j] * pb[p * k + j];
                }
                out[p * k + i] = s1 * s2;
            }
        }
        out
    }
}
