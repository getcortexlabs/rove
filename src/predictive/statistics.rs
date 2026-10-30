use crate::data::Patterns;
use crate::model::CharacterOrdering;
use crate::tree::Tree;

#[derive(Clone, Copy, Debug, Default)]
pub struct CharacterStatistics {
    pub steps: usize,
    pub minimum: usize,
    pub maximum: usize,
    pub informative: bool,
    /// Negative log of the *unconditioned* pattern likelihood at this draw.
    /// A discrepancy, not a conditional likelihood or model probability.
    pub surprise: f64,
}
impl CharacterStatistics {
    pub fn consistency(&self) -> Option<f64> {
        (self.steps > 0).then(|| self.minimum as f64 / self.steps as f64)
    }
    pub fn retention(&self) -> Option<f64> {
        (self.maximum > self.minimum)
            .then(|| (self.maximum - self.steps) as f64 / (self.maximum - self.minimum) as f64)
    }
}

#[derive(Debug)]
pub struct Statistics {
    pub characters: Vec<CharacterStatistics>,
}
impl Statistics {
    pub fn total(&self) -> CharacterStatistics {
        self.characters
            .iter()
            .fold(CharacterStatistics::default(), |mut a, b| {
                a.steps += b.steps;
                a.minimum += b.minimum;
                a.maximum += b.maximum;
                a.surprise += b.surprise;
                a
            })
    }
    pub fn informative(&self) -> usize {
        self.characters.iter().filter(|s| s.informative).count()
    }
}

pub(super) fn character(patterns: &Patterns, pattern: usize, tree: &Tree) -> CharacterStatistics {
    let k = patterns.symbols().len();
    let ordered = patterns.ordering() == CharacterOrdering::Ordered;
    let cost = |a: usize, b: usize| {
        if ordered {
            a.abs_diff(b)
        } else {
            usize::from(a != b)
        }
    };
    let infinity = usize::MAX / 4;
    let mut scores = vec![0usize; tree.nodes() * k];
    let mut counts = vec![0usize; k];
    for (tip, partials) in patterns.tip_partials().iter().enumerate() {
        let allowed = &partials[pattern * k..(pattern + 1) * k];
        if allowed.iter().filter(|&&p| p > 0.0).count() == 1 {
            counts[allowed.iter().position(|&p| p > 0.0).unwrap()] += 1;
        }
        for (state, &p) in allowed.iter().enumerate() {
            scores[tip * k + state] = if p > 0.0 { 0 } else { infinity };
        }
    }
    for node in tree.postorder() {
        if let Some((a, b)) = tree.children(node) {
            for state in 0..k {
                let best = |child| {
                    (0..k)
                        .map(|s| scores[child * k + s] + cost(state, s))
                        .min()
                        .unwrap()
                };
                scores[node * k + state] = best(a) + best(b);
            }
        }
    }
    let states: Vec<_> = (0..k).filter(|&s| counts[s] > 0).collect();
    let minimum = if ordered {
        states.first().zip(states.last()).map_or(0, |(a, b)| b - a)
    } else {
        states.len().saturating_sub(1)
    };
    // Maximum parsimony length: minimum star-tree cost. For ordered states
    // this is the sum of absolute distances to a weighted median.
    let maximum = (0..k)
        .map(|a| (0..k).map(|b| counts[b] * cost(a, b)).sum())
        .min()
        .unwrap();
    CharacterStatistics {
        steps: *scores[tree.root() * k..(tree.root() + 1) * k]
            .iter()
            .min()
            .unwrap(),
        minimum,
        maximum,
        informative: maximum > minimum,
        surprise: 0.0,
    }
}

/// Inclusive tails and explicit ties for paired T(replica, theta), T(data, theta).
/// Undefined indices are excluded pairwise and remain visible in `pairs`.
#[derive(Clone, Debug, Default)]
pub struct TailCount {
    pub pairs: usize,
    pub lower: usize,
    pub upper: usize,
    pub ties: usize,
}
impl TailCount {
    pub fn add(&mut self, observed: Option<f64>, simulated: Option<f64>) {
        if let (Some(a), Some(b)) = (observed, simulated) {
            if !a.is_finite() || !b.is_finite() {
                return;
            }
            self.pairs += 1;
            self.lower += usize::from(b <= a);
            self.upper += usize::from(b >= a);
            self.ties += usize::from(a == b);
        }
    }
    pub fn probability(&self, count: usize) -> Option<f64> {
        (self.pairs > 0).then(|| count as f64 / self.pairs as f64)
    }
    /// Wilson interval for simulation error conditional on the finite source
    /// posterior; it does not quantify error from MCMC or model misspecification.
    pub fn interval(&self, count: usize) -> Option<[f64; 2]> {
        if self.pairs == 0 {
            return None;
        }
        let n = self.pairs as f64;
        let p = count as f64 / n;
        let z2 = 1.959963984540054f64.powi(2);
        let center = p + z2 / (2.0 * n);
        let half = (z2 * (p * (1.0 - p) / n + z2 / (4.0 * n * n))).sqrt();
        Some([
            ((center - half) / (1.0 + z2 / n)).max(0.0),
            ((center + half) / (1.0 + z2 / n)).min(1.0),
        ])
    }
}
