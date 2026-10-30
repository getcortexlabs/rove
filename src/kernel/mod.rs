use crate::layout::Layout;
use crate::model::Mk;

mod checkpoint;
#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
mod neon;
mod scaling;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scaling {
    None,
    /// Power-of-two scaling after every node update, shared by all states
    /// and rate categories of a pattern.
    Always,
}

const NEON: bool = cfg!(all(target_arch = "aarch64", target_feature = "neon"));

/// The code a core runs. Per pattern, every backend performs the same
/// floating-point operations in the same order, so every result that is not
/// NaN agrees bit for bit. The compiler may order the operands of an addition
/// or multiplication differently, which can change only the payload and sign
/// of a NaN.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// Plain Rust for any target, which the compiler may vectorize loop by
    /// loop. The reference for the others.
    Portable,
    /// aarch64 NEON for the Mk node update and scaling exponent extraction;
    /// everything else as in `Portable`.
    Neon,
}

impl Backend {
    pub const ALL: [Self; 2] = [Self::Portable, Self::Neon];

    /// NEON where the target enables it, the portable code elsewhere.
    pub const NATIVE: Self = if NEON { Self::Neon } else { Self::Portable };

    /// Whether this build includes the backend.
    pub const fn is_available(self) -> bool {
        match self {
            Self::Portable => true,
            Self::Neon => NEON,
        }
    }

    /// The name the command line and the benchmarks use.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::Neon => "neon",
        }
    }
}

/// What each node's transition buffers hold for its branch.
enum Transition {
    /// One `states × states` row-major matrix per rate category.
    Matrix,
    /// The distance, `e` and `1 − e` of the Mk closed form per rate category.
    Mk(Mk),
}

impl Transition {
    fn len(&self, layout: &Layout) -> usize {
        match self {
            Self::Matrix => layout.matrices_len(),
            Self::Mk(_) => 3 * layout.categories,
        }
    }
}

pub struct LikelihoodCore {
    layout: Layout,
    node_count: usize,
    scaling: Scaling,
    backend: Backend,
    transition: Transition,
    partials: Vec<Vec<f64>>,
    transitions: Vec<Vec<f64>>,
    // Subtree totals, paired with partials so rollback needs no arithmetic.
    exponents: Vec<Vec<i64>>,
    root_exponents: Vec<i64>,
    current_partials: Vec<u8>,
    stored_partials: Vec<u8>,
    current_transitions: Vec<u8>,
    stored_transitions: Vec<u8>,
    root_partials: Vec<f64>,
    pattern_log_likelihoods: Vec<f64>,
    acc1: Vec<f64>,
    acc2: Vec<f64>,
    scale: Vec<f64>,
}

impl LikelihoodCore {
    /// A core that multiplies a transition matrix per branch and rate category.
    pub fn new(
        states: usize,
        patterns: usize,
        categories: usize,
        node_count: usize,
        scaling: Scaling,
    ) -> Self {
        let layout = Layout::new(states, categories, patterns);
        Self::with_transition(layout, node_count, scaling, Transition::Matrix)
    }

    /// A core for the Mk model that takes a distance per branch and rate
    /// category, and prunes with the closed form instead of matrices.
    pub fn mk(
        model: &Mk,
        patterns: usize,
        categories: usize,
        node_count: usize,
        scaling: Scaling,
    ) -> Self {
        let layout = Layout::new(model.states(), categories, patterns);
        Self::with_transition(layout, node_count, scaling, Transition::Mk(model.clone()))
    }

    fn with_transition(
        layout: Layout,
        node_count: usize,
        scaling: Scaling,
        transition: Transition,
    ) -> Self {
        let buffers = node_count * 2;
        Self {
            layout,
            node_count,
            scaling,
            backend: Backend::NATIVE,
            partials: (0..buffers)
                .map(|_| vec![0.0; layout.partials_len()])
                .collect(),
            transitions: (0..buffers)
                .map(|_| vec![0.0; transition.len(&layout)])
                .collect(),
            transition,
            exponents: (0..buffers).map(|_| vec![0; layout.stride]).collect(),
            root_exponents: vec![0; layout.stride],
            current_partials: vec![0; node_count],
            stored_partials: vec![0; node_count],
            current_transitions: vec![0; node_count],
            stored_transitions: vec![0; node_count],
            root_partials: vec![0.0; layout.states * layout.stride],
            pattern_log_likelihoods: vec![0.0; layout.patterns],
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

    pub fn backend(&self) -> Backend {
        self.backend
    }

    /// Runs later calculations with `backend`, which this build must include.
    pub fn set_backend(&mut self, backend: Backend) {
        assert!(
            backend.is_available(),
            "the {} backend is not available on this target",
            backend.name()
        );
        self.backend = backend;
    }

    #[inline]
    fn partials_buffer(&self, node: usize) -> usize {
        node * 2 + self.current_partials[node] as usize
    }

    #[inline]
    fn transitions_buffer(&self, node: usize) -> usize {
        node * 2 + self.current_transitions[node] as usize
    }

    pub fn set_tip_states(&mut self, node: usize, states: &[u32]) {
        let l = self.layout;
        assert_eq!(states.len(), l.patterns);
        for buffer in [node * 2, node * 2 + 1] {
            self.exponents[buffer].fill(0);
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
            self.exponents[buffer].fill(0);
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

    /// Switches the buffer that holds the branch above `node`: its matrices,
    /// or its distances in an Mk core.
    pub fn set_node_matrix_for_update(&mut self, node: usize) {
        self.current_transitions[node] ^= 1;
    }

    pub fn set_node_matrix(&mut self, node: usize, category: usize, matrix: &[f64]) {
        assert!(
            matches!(self.transition, Transition::Matrix),
            "an Mk core takes distances; use set_node_distance"
        );
        let l = self.layout;
        assert_eq!(matrix.len(), l.states * l.states);
        let buffer = self.transitions_buffer(node);
        let start = l.matrix_offset(category);
        self.transitions[buffer][start..start + matrix.len()].copy_from_slice(matrix);
    }

    pub fn node_matrix(&self, node: usize, category: usize) -> &[f64] {
        assert!(
            matches!(self.transition, Transition::Matrix),
            "an Mk core stores distances; use node_distance"
        );
        let l = self.layout;
        let start = l.matrix_offset(category);
        &self.transitions[self.transitions_buffer(node)][start..start + l.states * l.states]
    }

    /// Sets the branch above `node`, in one rate category, to `distance`
    /// expected changes per character. Only for cores made by `mk`.
    pub fn set_node_distance(&mut self, node: usize, category: usize, distance: f64) {
        let Transition::Mk(model) = &self.transition else {
            panic!("a matrix core takes transition matrices; use set_node_matrix");
        };
        let (stay, change) = model.stay_and_change(distance);
        let buffer = self.transitions_buffer(node);
        self.transitions[buffer][3 * category..3 * category + 3]
            .copy_from_slice(&[distance, stay, change]);
    }

    pub fn node_distance(&self, node: usize, category: usize) -> f64 {
        assert!(
            matches!(self.transition, Transition::Mk(_)),
            "a matrix core stores matrices; use node_matrix"
        );
        self.transitions[self.transitions_buffer(node)][3 * category]
    }

    pub fn node_partials(&self, node: usize) -> &[f64] {
        &self.partials[self.partials_buffer(node)]
    }

    pub fn calculate_partials(&mut self, child1: usize, child2: usize, parent: usize) {
        debug_assert!(parent != child1 && parent != child2 && child1 != child2);
        let out = self.partials_buffer(parent);
        let mut dst = std::mem::take(&mut self.partials[out]);
        let maxima = match self.transition {
            Transition::Matrix => {
                self.multiply_matrices(child1, child2, &mut dst);
                false
            }
            #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
            Transition::Mk(_) if self.backend == Backend::Neon => {
                self.apply_mk_neon(child1, child2, &mut dst)
            }
            Transition::Mk(_) => {
                self.apply_mk(child1, child2, &mut dst);
                false
            }
        };
        if self.scaling == Scaling::Always {
            self.rescale(&mut dst, out, maxima);
            let mut exponents = std::mem::take(&mut self.exponents[out]);
            let first = &self.exponents[self.partials_buffer(child1)];
            let second = &self.exponents[self.partials_buffer(child2)];
            for ((total, a), b) in exponents.iter_mut().zip(first).zip(second) {
                *total += a + b;
            }
            self.exponents[out] = exponents;
        }
        self.partials[out] = dst;
    }

    /// `apply_mk` through the NEON kernel, which also finds the maxima that
    /// `rescale` needs when the core rescales. Returns whether it did.
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    fn apply_mk_neon(&mut self, child1: usize, child2: usize, dst: &mut [f64]) -> bool {
        let Transition::Mk(model) = &self.transition else {
            unreachable!("only Mk cores store distances");
        };
        let buffers = [child1, child2]
            .map(|node| (self.partials_buffer(node), self.transitions_buffer(node)));
        let children = buffers.map(|(partials, transitions)| neon::Child {
            partials: &self.partials[partials],
            transition: &self.transitions[transitions],
        });
        let maxima = self.scaling == Scaling::Always;
        neon::mk(
            &self.layout,
            model.frequencies(),
            children,
            dst,
            maxima.then_some(&mut self.scale[..]),
        );
        maxima
    }

    fn multiply_matrices(&mut self, child1: usize, child2: usize, dst: &mut [f64]) {
        let l = self.layout;
        let (k, stride) = (l.states, l.stride);
        let p1 = &self.partials[self.partials_buffer(child1)];
        let p2 = &self.partials[self.partials_buffer(child2)];
        let mat1 = &self.transitions[self.transitions_buffer(child1)];
        let mat2 = &self.transitions[self.transitions_buffer(child2)];
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

    /// `(P·L)ᵢ = e·Lᵢ + (1 − e)·Σⱼ πⱼ·Lⱼ` for each child: one weighted sum per
    /// pattern, shared by every state, instead of a row of `P` per state.
    fn apply_mk(&mut self, child1: usize, child2: usize, dst: &mut [f64]) {
        let Transition::Mk(model) = &self.transition else {
            unreachable!("only Mk cores store distances");
        };
        let l = self.layout;
        let stride = l.stride;
        let p1 = &self.partials[self.partials_buffer(child1)];
        let p2 = &self.partials[self.partials_buffer(child2)];
        let t1 = &self.transitions[self.transitions_buffer(child1)];
        let t2 = &self.transitions[self.transitions_buffer(child2)];
        for c in 0..l.categories {
            let (stay1, change1) = (t1[3 * c + 1], t1[3 * c + 2]);
            let (stay2, change2) = (t2[3 * c + 1], t2[3 * c + 2]);
            self.acc1.fill(0.0);
            self.acc2.fill(0.0);
            for (j, &f) in model.frequencies().iter().enumerate() {
                let o = l.offset(c, j);
                let (a, b) = (change1 * f, change2 * f);
                for (acc, x) in self.acc1.iter_mut().zip(&p1[o..o + stride]) {
                    *acc += a * x;
                }
                for (acc, x) in self.acc2.iter_mut().zip(&p2[o..o + stride]) {
                    *acc += b * x;
                }
            }
            for i in 0..l.states {
                let o = l.offset(c, i);
                let children = p1[o..o + stride].iter().zip(&p2[o..o + stride]);
                let sums = self.acc1.iter().zip(&self.acc2);
                for ((d, (x1, x2)), (s1, s2)) in
                    dst[o..o + stride].iter_mut().zip(children).zip(sums)
                {
                    *d = (stay1 * x1 + s1) * (stay2 * x2 + s2);
                }
            }
        }
    }

    /// Rescales by a power of two chosen from each pattern's maximum over
    /// states and categories. With `maxima`, `scale` already holds those values.
    fn rescale(&mut self, partials: &mut [f64], buffer: usize, maxima: bool) {
        let l = self.layout;
        let stride = l.stride;
        if !maxima {
            self.scale.fill(0.0);
            for c in 0..l.categories {
                for s in 0..l.states {
                    let o = l.offset(c, s);
                    for (m, x) in self.scale.iter_mut().zip(&partials[o..o + stride]) {
                        *m = m.max(*x);
                    }
                }
            }
        }
        scaling::factors(&mut self.scale, &mut self.exponents[buffer], self.backend);
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
        if self.scaling == Scaling::Always {
            let buffer = self.partials_buffer(root);
            self.root_exponents.copy_from_slice(&self.exponents[buffer]);
        }
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
            for (ll, &exponent) in self
                .pattern_log_likelihoods
                .iter_mut()
                .zip(&self.root_exponents)
            {
                *ll += exponent as f64 * std::f64::consts::LN_2;
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
        self.stored_transitions
            .copy_from_slice(&self.current_transitions);
    }

    pub fn restore(&mut self) {
        std::mem::swap(&mut self.current_partials, &mut self.stored_partials);
        std::mem::swap(&mut self.current_transitions, &mut self.stored_transitions);
    }

    pub fn unstore(&mut self) {
        self.current_partials.copy_from_slice(&self.stored_partials);
        self.current_transitions
            .copy_from_slice(&self.stored_transitions);
    }
}
