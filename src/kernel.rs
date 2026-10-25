use crate::layout::Layout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scaling {
    None,
    Always,
}

pub struct LikelihoodCore {
    layout: Layout,
    node_count: usize,
    scaling: Scaling,
    partials: Vec<Vec<f64>>,
    matrices: Vec<Vec<f64>>,
    log_scalers: Vec<Vec<f64>>,
    current_partials: Vec<u8>,
    stored_partials: Vec<u8>,
    current_matrices: Vec<u8>,
    stored_matrices: Vec<u8>,
    root_partials: Vec<f64>,
    pattern_log_likelihoods: Vec<f64>,
    acc1: Vec<f64>,
    acc2: Vec<f64>,
    scale: Vec<f64>,
}

impl LikelihoodCore {
    pub fn new(
        states: usize,
        patterns: usize,
        categories: usize,
        node_count: usize,
        scaling: Scaling,
    ) -> Self {
        let layout = Layout::new(states, categories, patterns);
        let buffers = node_count * 2;
        Self {
            layout,
            node_count,
            scaling,
            partials: (0..buffers)
                .map(|_| vec![0.0; layout.partials_len()])
                .collect(),
            matrices: (0..buffers)
                .map(|_| vec![0.0; layout.matrices_len()])
                .collect(),
            log_scalers: (0..buffers).map(|_| vec![0.0; layout.stride]).collect(),
            current_partials: vec![0; node_count],
            stored_partials: vec![0; node_count],
            current_matrices: vec![0; node_count],
            stored_matrices: vec![0; node_count],
            root_partials: vec![0.0; states * layout.stride],
            pattern_log_likelihoods: vec![0.0; patterns],
            acc1: vec![0.0; layout.stride],
            acc2: vec![0.0; layout.stride],
            scale: vec![0.0; layout.stride],
        }
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn scaling(&self) -> Scaling {
        self.scaling
    }

    #[inline]
    fn partials_buffer(&self, node: usize) -> usize {
        node * 2 + self.current_partials[node] as usize
    }

    #[inline]
    fn matrices_buffer(&self, node: usize) -> usize {
        node * 2 + self.current_matrices[node] as usize
    }

    pub fn set_tip_states(&mut self, node: usize, states: &[u32]) {
        let l = self.layout;
        assert_eq!(states.len(), l.patterns);
        for buffer in [node * 2, node * 2 + 1] {
            let partials = &mut self.partials[buffer];
            partials.fill(0.0);
            for category in 0..l.categories {
                for (pattern, &state) in states.iter().enumerate() {
                    let state = state as usize;
                    if state < l.states {
                        partials[l.offset(category, state) + pattern] = 1.0;
                    } else {
                        for s in 0..l.states {
                            partials[l.offset(category, s) + pattern] = 1.0;
                        }
                    }
                }
            }
        }
    }

    pub fn set_tip_partials(&mut self, node: usize, tip_partials: &[f64]) {
        let l = self.layout;
        assert_eq!(tip_partials.len(), l.patterns * l.states);
        for buffer in [node * 2, node * 2 + 1] {
            let partials = &mut self.partials[buffer];
            partials.fill(0.0);
            for category in 0..l.categories {
                for state in 0..l.states {
                    let base = l.offset(category, state);
                    for pattern in 0..l.patterns {
                        partials[base + pattern] = tip_partials[pattern * l.states + state];
                    }
                }
            }
        }
    }

    pub fn set_node_partials_for_update(&mut self, node: usize) {
        self.current_partials[node] ^= 1;
    }

    pub fn set_node_matrix_for_update(&mut self, node: usize) {
        self.current_matrices[node] ^= 1;
    }

    pub fn set_node_matrix(&mut self, node: usize, category: usize, matrix: &[f64]) {
        let l = self.layout;
        assert_eq!(matrix.len(), l.states * l.states);
        let buffer = self.matrices_buffer(node);
        let start = l.matrix_offset(category);
        self.matrices[buffer][start..start + matrix.len()].copy_from_slice(matrix);
    }

    pub fn node_matrix(&self, node: usize, category: usize) -> &[f64] {
        let l = self.layout;
        let start = l.matrix_offset(category);
        &self.matrices[self.matrices_buffer(node)][start..start + l.states * l.states]
    }

    pub fn node_partials(&self, node: usize) -> &[f64] {
        &self.partials[self.partials_buffer(node)]
    }

    pub fn calculate_partials(&mut self, child1: usize, child2: usize, parent: usize) {
        debug_assert!(parent != child1 && parent != child2 && child1 != child2);
        let l = self.layout;
        let (k, stride) = (l.states, l.stride);
        let in1 = self.partials_buffer(child1);
        let in2 = self.partials_buffer(child2);
        let out = self.partials_buffer(parent);
        let m1 = self.matrices_buffer(child1);
        let m2 = self.matrices_buffer(child2);
        let mut dst = std::mem::take(&mut self.partials[out]);
        {
            let p1 = &self.partials[in1];
            let p2 = &self.partials[in2];
            let mat1 = &self.matrices[m1];
            let mat2 = &self.matrices[m2];
            for c in 0..l.categories {
                let mo = l.matrix_offset(c);
                for i in 0..k {
                    let row1 = &mat1[mo + i * k..mo + (i + 1) * k];
                    let row2 = &mat2[mo + i * k..mo + (i + 1) * k];
                    self.acc1.fill(0.0);
                    self.acc2.fill(0.0);
                    for (j, (&a, &b)) in row1.iter().zip(row2).enumerate() {
                        let o = l.offset(c, j);
                        for (acc, x) in self.acc1.iter_mut().zip(&p1[o..o + stride]) {
                            *acc += a * x;
                        }
                        for (acc, x) in self.acc2.iter_mut().zip(&p2[o..o + stride]) {
                            *acc += b * x;
                        }
                    }
                    let o = l.offset(c, i);
                    for ((d, a), b) in dst[o..o + stride]
                        .iter_mut()
                        .zip(&self.acc1)
                        .zip(&self.acc2)
                    {
                        *d = a * b;
                    }
                }
            }
        }
        if self.scaling == Scaling::Always {
            self.rescale(&mut dst, out);
        }
        self.partials[out] = dst;
    }

    fn rescale(&mut self, partials: &mut [f64], buffer: usize) {
        let l = self.layout;
        let stride = l.stride;
        self.scale.fill(0.0);
        for c in 0..l.categories {
            for s in 0..l.states {
                let o = l.offset(c, s);
                for (m, x) in self.scale.iter_mut().zip(&partials[o..o + stride]) {
                    *m = m.max(*x);
                }
            }
        }
        for (m, log) in self.scale.iter_mut().zip(&mut self.log_scalers[buffer]) {
            if *m > 0.0 {
                *log = m.ln();
                *m = 1.0 / *m;
            } else {
                *log = 0.0;
                *m = 1.0;
            }
        }
        for c in 0..l.categories {
            for s in 0..l.states {
                let o = l.offset(c, s);
                for (x, inv) in partials[o..o + stride].iter_mut().zip(&self.scale) {
                    *x *= inv;
                }
            }
        }
    }

    pub fn integrate_partials(&mut self, root: usize, category_weights: &[f64]) {
        let l = self.layout;
        assert_eq!(category_weights.len(), l.categories);
        let src = &self.partials[self.partials_buffer(root)];
        self.root_partials.fill(0.0);
        for (c, &w) in category_weights.iter().enumerate() {
            for s in 0..l.states {
                let o = l.offset(c, s);
                let dst = &mut self.root_partials[s * l.stride..(s + 1) * l.stride];
                for (d, x) in dst.iter_mut().zip(&src[o..o + l.stride]) {
                    *d += w * x;
                }
            }
        }
    }

    pub fn calculate_log_likelihoods(&mut self, frequencies: &[f64]) -> &[f64] {
        let l = self.layout;
        assert_eq!(frequencies.len(), l.states);
        self.acc1.fill(0.0);
        for (s, &f) in frequencies.iter().enumerate() {
            let src = &self.root_partials[s * l.stride..(s + 1) * l.stride];
            for (a, x) in self.acc1.iter_mut().zip(src) {
                *a += f * x;
            }
        }
        for (ll, x) in self.pattern_log_likelihoods.iter_mut().zip(&self.acc1) {
            *ll = x.ln();
        }
        if self.scaling == Scaling::Always {
            for node in 0..self.node_count {
                let buffer = self.partials_buffer(node);
                let scalers = &self.log_scalers[buffer];
                for (ll, s) in self.pattern_log_likelihoods.iter_mut().zip(scalers) {
                    *ll += s;
                }
            }
        }
        &self.pattern_log_likelihoods
    }

    pub fn pattern_log_likelihoods(&self) -> &[f64] {
        &self.pattern_log_likelihoods
    }

    pub fn log_likelihood(
        &mut self,
        root: usize,
        category_weights: &[f64],
        frequencies: &[f64],
        pattern_weights: &[f64],
    ) -> f64 {
        assert_eq!(pattern_weights.len(), self.layout.patterns);
        self.integrate_partials(root, category_weights);
        self.calculate_log_likelihoods(frequencies)
            .iter()
            .zip(pattern_weights)
            .map(|(ll, w)| ll * w)
            .sum()
    }

    pub fn store(&mut self) {
        self.stored_partials.copy_from_slice(&self.current_partials);
        self.stored_matrices.copy_from_slice(&self.current_matrices);
    }

    pub fn restore(&mut self) {
        std::mem::swap(&mut self.current_partials, &mut self.stored_partials);
        std::mem::swap(&mut self.current_matrices, &mut self.stored_matrices);
    }

    pub fn unstore(&mut self) {
        self.current_partials.copy_from_slice(&self.stored_partials);
        self.current_matrices.copy_from_slice(&self.stored_matrices);
    }
}
