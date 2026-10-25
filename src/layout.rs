pub const LANES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub states: usize,
    pub categories: usize,
    pub patterns: usize,
    pub stride: usize,
}

impl Layout {
    pub fn new(states: usize, categories: usize, patterns: usize) -> Self {
        assert!(states >= 2, "need at least two states");
        assert!(categories >= 1, "need at least one rate category");
        assert!(patterns >= 1, "need at least one pattern");
        Self {
            states,
            categories,
            patterns,
            stride: patterns.div_ceil(LANES) * LANES,
        }
    }

    pub fn partials_len(&self) -> usize {
        self.categories * self.states * self.stride
    }

    pub fn matrices_len(&self) -> usize {
        self.categories * self.states * self.states
    }

    #[inline]
    pub fn offset(&self, category: usize, state: usize) -> usize {
        (category * self.states + state) * self.stride
    }

    #[inline]
    pub fn matrix_offset(&self, category: usize) -> usize {
        category * self.states * self.states
    }
}
