//! Parsimony-informative conditioning for the equal-frequency, unordered Mk model.
//!
//! A complete pattern is informative when at least two states occur twice.
//! Fix one state A and count patterns in which every other state occurs at most
//! once. Symmetry lets us track the number of those singleton states, rather
//! than their labels or their positions among the tips. At the root, patterns
//! with A repeated have k disjoint labelings; all-distinct patterns count once.
//!
//! The three partials sum over singleton subsets conditional on the ancestral
//! state being A, a particular singleton state present in the subset, or that
//! state absent from it. Combining children requires disjoint singleton subsets.
//! Their overlap probabilities are precomputed without factorials. Each partial
//! is a probability bounded by one, so combinatorial counts cannot overflow.
//! Underflow in an excluded event contributes less than machine precision to
//! the retained probability. Observed likelihoods keep the core's own scaling.

use crate::LikelihoodCore;
use crate::data::Patterns;
use crate::model::Mk;

use super::Error;

/// Requires every completion of missing/uncertain cells to be informative.
/// A non-informative completion has at most one repeated state. For each
/// candidate repeated state, match all tips that exclude it to distinct other
/// states. A complete matching is a witness that this character must be rejected.
pub fn validate_informative(patterns: &Patterns) -> Result<(), Error> {
    let states = patterns.symbols().len();
    let tips = patterns.tip_partials();
    let mut owners = vec![None; states];
    let mut visited = vec![false; states];
    for pattern in 0..patterns.weights().len() {
        for repeated in 0..states {
            owners.fill(None);
            let mut possible = true;
            for tip in 0..tips.len() {
                if tips[tip][pattern * states + repeated] > 0.0 {
                    continue;
                }
                visited.fill(false);
                if !match_tip(
                    tip,
                    pattern * states,
                    repeated,
                    tips,
                    &mut owners,
                    &mut visited,
                ) {
                    possible = false;
                    break;
                }
            }
            if possible {
                let site = patterns
                    .site_to_pattern()
                    .iter()
                    .position(|&p| p == pattern)
                    .expect("every pattern represents a site");
                return Err(Error::UninformativeCharacter {
                    site: patterns.sites()[site],
                });
            }
        }
    }
    Ok(())
}

fn match_tip(
    tip: usize,
    offset: usize,
    repeated: usize,
    tips: &[Vec<f64>],
    owners: &mut [Option<usize>],
    visited: &mut [bool],
) -> bool {
    for state in 0..owners.len() {
        if state == repeated || visited[state] || tips[tip][offset + state] == 0.0 {
            continue;
        }
        visited[state] = true;
        if owners[state]
            .is_none_or(|other| match_tip(other, offset, repeated, tips, owners, visited))
        {
            owners[state] = Some(tip);
            return true;
        }
    }
    false
}

/// Cached probability of retaining a character under uniform unordered Mk.
/// Category distances come from the same core that evaluates the observations.
/// This recurrence does not apply to ordered or unequal-frequency models.
pub struct InformativeProbability {
    model: Mk,
    tips: usize,
    nodes: usize,
    categories: usize,
    width: usize,
    partials: Vec<[f64; 3]>,
    counts: Vec<usize>,
    active: Vec<usize>,
    stored: Vec<usize>,
    disjoint: Vec<f64>,
    disjoint_other: Vec<f64>,
    left: Vec<[f64; 3]>,
    right: Vec<[f64; 3]>,
}

impl InformativeProbability {
    pub fn new(states: usize, tips: usize, categories: usize) -> Self {
        assert!(states >= 2 && tips >= 2 && categories > 0);
        let nodes = 2 * tips - 1;
        let width = (states - 1).min(tips) + 1;
        let mut value = Self {
            model: Mk::uniform(states),
            tips,
            nodes,
            categories,
            width,
            partials: vec![[0.0; 3]; 2 * nodes * categories * width],
            counts: vec![0; 2 * nodes],
            active: vec![0; nodes],
            stored: vec![0; nodes],
            disjoint: disjoint_table(states - 1, width),
            disjoint_other: disjoint_table(states - 2, width),
            left: vec![[0.0; 3]; width],
            right: vec![[0.0; 3]; width],
        };
        for tip in 0..tips {
            value.counts[tip] = 1;
            for category in 0..categories {
                let start = value.offset(tip, category);
                value.partials[start] = [1.0, 0.0, 0.0];
                value.partials[start + 1] = [0.0, 1.0, 0.0];
            }
        }
        value
    }

    fn offset(&self, node: usize, category: usize) -> usize {
        ((self.active[node] * self.nodes + node) * self.categories + category) * self.width
    }

    fn count(&self, node: usize) -> usize {
        self.counts[self.active[node] * self.nodes + node]
    }

    pub fn store(&mut self) {
        self.stored.clone_from(&self.active);
    }
    pub fn restore(&mut self) {
        std::mem::swap(&mut self.active, &mut self.stored);
    }

    pub fn set_node_partials_for_update(&mut self, node: usize) {
        if self.active[node] == self.stored[node] {
            self.active[node] ^= 1;
        }
    }

    /// Reuses scratch space; no allocations occur during a likelihood update.
    pub fn calculate_partials(&mut self, a: usize, b: usize, node: usize, core: &LikelihoodCore) {
        let left_count = self.count(a);
        let right_count = self.count(b);
        self.counts[self.active[node] * self.nodes + node] = left_count + right_count;
        let last_left = left_count.min(self.width - 1);
        let last_right = right_count.min(self.width - 1);
        let others = (self.model.states() - 1) as f64;
        for category in 0..self.categories {
            let left = self.offset(a, category);
            let right = self.offset(b, category);
            let parent = self.offset(node, category);
            let (stay_a, change_a) = self.model.stay_and_change(core.node_distance(a, category));
            let (stay_b, change_b) = self.model.stay_and_change(core.node_distance(b, category));
            for r in 0..=last_left {
                self.left[r] = transition(self.partials[left + r], r, others, stay_a, change_a);
            }
            for r in 0..=last_right {
                self.right[r] = transition(self.partials[right + r], r, others, stay_b, change_b);
            }
            self.partials[parent..parent + self.width].fill([0.0; 3]);
            for a in 0..=last_left {
                let [u, v, w] = self.left[a];
                for b in 0..=last_right.min(self.width - 1 - a) {
                    let [x, y, z] = self.right[b];
                    let row = &mut self.partials[parent + a + b];
                    row[0] += u * x * self.disjoint[a * self.width + b];
                    if a > 0 {
                        row[1] += v * z * self.disjoint_other[(a - 1) * self.width + b];
                    }
                    if b > 0 {
                        row[1] += w * y * self.disjoint_other[a * self.width + b - 1];
                    }
                    row[2] += w * z * self.disjoint_other[a * self.width + b];
                }
            }
        }
    }

    /// The mixture is conditioned after integrating rate categories, not by
    /// averaging separately conditioned categories.
    pub fn log_probability(&self, root: usize, category_weights: &[f64]) -> Result<f64, Error> {
        assert_eq!(category_weights.len(), self.categories);
        assert_eq!(self.count(root), self.tips);
        let states = self.model.states() as f64;
        let mut excluded = 0.0;
        let mut compensation = 0.0;
        for (category, &weight) in category_weights.iter().enumerate() {
            let start = self.offset(root, category);
            for r in 0..self.width {
                let [u, v, w] = self.partials[start + r];
                let probability = (u + (states - 1.0) * (v + w))
                    * if r + 2 <= self.tips {
                        1.0
                    } else {
                        1.0 / states
                    };
                let term = weight * probability - compensation;
                let sum = excluded + term;
                compensation = (sum - excluded) - term;
                excluded = sum;
            }
        }
        let rounding = 32.0 * self.tips as f64 * states * f64::EPSILON;
        if !excluded.is_finite() || excluded < 0.0 || excluded >= 1.0 - rounding {
            return Err(Error::UndefinedInformativeCorrection);
        }
        Ok((-excluded).ln_1p())
    }
}

fn transition([u, v, w]: [f64; 3], count: usize, others: f64, stay: f64, change: f64) -> [f64; 3] {
    let r = count as f64;
    let different = change / (others + 1.0);
    let singleton = v + w;
    [
        stay * u + different * (u + others * singleton),
        stay * v + different * (r / others * u + r * singleton),
        stay * w + different * ((others - r) / others * u + (others - r) * singleton),
    ]
}

// Probability that two uniformly chosen subsets of sizes a and b do not overlap.
fn disjoint_table(states: usize, width: usize) -> Vec<f64> {
    let mut table = vec![0.0; width * width];
    for a in 0..width.min(states + 1) {
        table[a * width] = 1.0;
        for b in 1..width.min(states - a + 1) {
            table[a * width + b] =
                table[a * width + b - 1] * (states - a - b + 1) as f64 / (states - b + 1) as f64;
        }
    }
    table
}
