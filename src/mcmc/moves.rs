//! Proposals on the tree and the birth–death parameters, following the
//! definitions and Hastings ratios of the corresponding BEAST 2.7 operators.
//! Joint time/rate and sigma/rate proposals are derived in `coupled`.
//! Each returns the log Hastings ratio, or `None` when the move is invalid.

use super::coupled;
use crate::random::Random;
use crate::rates::{GammaRates, RateVariation};
use crate::speciation::BirthDeath;
use crate::tree::Tree;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Move {
    FbdBirthScale,
    FbdDeathScale,
    FbdSamplingScale,
    FbdRhoLogit,
    FbdOriginScale,
    TipAge,
    ClockRateScale,
    /// `Uniform`: a new height for a non-root internal node, uniform between
    /// its older child and its parent.
    NodeHeight,
    /// `SubtreeSlide` with a Gaussian step; it can change the topology.
    SubtreeSlide,
    /// `Exchange` with `isNarrow="true"`.
    NarrowExchange,
    /// `Exchange` with `isNarrow="false"`.
    WideExchange,
    /// `WilsonBalding`, which never moves the root.
    WilsonBalding,
    /// `ScaleOperator` with `rootOnly="true"`.
    RootScale,
    /// `ScaleOperator` on the whole tree.
    TreeScale,
    /// `ScaleOperator` on λ − μ.
    BirthDifferenceScale,
    /// `ScaleOperator` on μ / λ.
    RelativeDeathScale,
    GammaShapeScale,
    ClockSigmaScale,
    BranchRateScale,
    /// Move an internal height and compensate its incident branch rates.
    NodeTimeRates,
    /// Scale the root height and compensate its two incident branch rates.
    RootTimeRates,
    /// Scale clock sigma while preserving standardized log branch rates.
    ClockSigmaRates,
}

impl Move {
    pub const ALL: [Move; 22] = [
        Move::FbdBirthScale,
        Move::FbdDeathScale,
        Move::FbdSamplingScale,
        Move::FbdRhoLogit,
        Move::FbdOriginScale,
        Move::TipAge,
        Move::ClockRateScale,
        Move::NodeHeight,
        Move::SubtreeSlide,
        Move::NarrowExchange,
        Move::WideExchange,
        Move::WilsonBalding,
        Move::RootScale,
        Move::TreeScale,
        Move::BirthDifferenceScale,
        Move::RelativeDeathScale,
        Move::GammaShapeScale,
        Move::ClockSigmaScale,
        Move::BranchRateScale,
        Move::NodeTimeRates,
        Move::RootTimeRates,
        Move::ClockSigmaRates,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::FbdBirthScale => "fbd_birth_scale",
            Self::FbdDeathScale => "fbd_death_scale",
            Self::FbdSamplingScale => "fbd_sampling_scale",
            Self::FbdRhoLogit => "fbd_rho_logit",
            Self::FbdOriginScale => "fbd_origin_scale",
            Self::TipAge => "tip_age",
            Self::ClockRateScale => "clock_rate_scale",
            Self::NodeHeight => "node_height",
            Self::SubtreeSlide => "subtree_slide",
            Self::NarrowExchange => "narrow_exchange",
            Self::WideExchange => "wide_exchange",
            Self::WilsonBalding => "wilson_balding",
            Self::RootScale => "root_scale",
            Self::TreeScale => "tree_scale",
            Self::BirthDifferenceScale => "birth_difference_scale",
            Self::RelativeDeathScale => "relative_death_scale",
            Self::GammaShapeScale => "gamma_shape_scale",
            Self::ClockSigmaScale => "clock_sigma_scale",
            Self::BranchRateScale => "branch_rate_scale",
            Self::NodeTimeRates => "node_time_rates",
            Self::RootTimeRates => "root_time_rates",
            Self::ClockSigmaRates => "clock_sigma_rates",
        }
    }

    /// The slide size or the scale factor, for moves that have one.
    pub fn initial_tuning(self) -> Option<f64> {
        match self {
            Self::SubtreeSlide => Some(1.0),
            Self::FbdBirthScale
            | Self::FbdDeathScale
            | Self::FbdSamplingScale
            | Self::FbdRhoLogit
            | Self::FbdOriginScale
            | Self::ClockRateScale
            | Self::RootScale
            | Self::TreeScale
            | Self::BirthDifferenceScale
            | Self::RelativeDeathScale
            | Self::RootTimeRates => Some(0.75),
            Self::GammaShapeScale
            | Self::ClockSigmaScale
            | Self::BranchRateScale
            | Self::ClockSigmaRates => Some(0.5),
            _ => None,
        }
    }

    pub fn changes_fbd(self) -> bool {
        matches!(
            self,
            Self::FbdBirthScale
                | Self::FbdDeathScale
                | Self::FbdSamplingScale
                | Self::FbdRhoLogit
                | Self::FbdOriginScale
        )
    }

    pub fn changes_tree(self) -> bool {
        matches!(
            self,
            Self::TipAge
                | Self::NodeHeight
                | Self::SubtreeSlide
                | Self::NarrowExchange
                | Self::WideExchange
                | Self::WilsonBalding
                | Self::RootScale
                | Self::TreeScale
                | Self::NodeTimeRates
                | Self::RootTimeRates
        )
    }

    pub fn changes_likelihood(self) -> bool {
        self.changes_tree()
            || matches!(
                self,
                Self::ClockRateScale
                    | Self::GammaShapeScale
                    | Self::BranchRateScale
                    | Self::ClockSigmaRates
            )
    }

    pub fn changes_topology(self) -> bool {
        matches!(
            self,
            Self::SubtreeSlide | Self::NarrowExchange | Self::WideExchange | Self::WilsonBalding
        )
    }

    /// The tuning value after an adaptation step `delta` (BEAST's `optimize`).
    pub(crate) fn adapt(self, tuning: f64, delta: f64) -> f64 {
        match self {
            Self::SubtreeSlide => (delta + tuning.ln()).exp(),
            _ => (1.0 / ((delta + (1.0 / tuning - 1.0).ln()).exp() + 1.0)).clamp(1e-8, 1.0 - 1e-8),
        }
    }

    pub(crate) fn propose(
        self,
        tree: &mut Tree,
        birth_death: &mut BirthDeath,
        rates: &mut RateVariation,
        upper: f64,
        tuning: Option<f64>,
        random: &mut Random,
    ) -> Option<f64> {
        let tuning = tuning.unwrap_or(f64::NAN);
        match self {
            Self::FbdBirthScale
            | Self::FbdDeathScale
            | Self::FbdSamplingScale
            | Self::FbdRhoLogit
            | Self::FbdOriginScale
            | Self::TipAge => None, // Dating proposals are handled by the sampler.
            Self::ClockRateScale => scale(rates.mean.as_mut()?, f64::MAX, tuning, random),
            Self::NodeHeight => node_height(tree, random),
            Self::SubtreeSlide => subtree_slide(tree, tuning, random),
            Self::NarrowExchange => narrow_exchange(tree, random),
            Self::WideExchange => wide_exchange(tree, random),
            Self::WilsonBalding => wilson_balding(tree, random),
            Self::RootScale => root_scale(tree, tuning, random),
            Self::TreeScale => tree_scale(tree, tuning, random),
            Self::BirthDifferenceScale => {
                scale(&mut birth_death.birth_difference, upper, tuning, random)
            }
            Self::RelativeDeathScale => scale(&mut birth_death.relative_death, 1.0, tuning, random),
            Self::GammaShapeScale => {
                let shape = rates.gamma.as_ref()?.shape();
                let factor = scaler(tuning, random);
                rates.gamma = Some(GammaRates::new(shape * factor).ok()?);
                Some(-factor.ln())
            }
            Self::ClockSigmaScale => {
                let clock = rates.clock.as_mut()?;
                let factor = scaler(tuning, random);
                clock
                    .set_sigma(clock.sigma() * factor)
                    .then(|| -factor.ln())
            }
            Self::BranchRateScale => {
                let clock = rates.clock.as_mut()?;
                let index = random.below(clock.rates().len());
                let factor = scaler(tuning, random);
                clock
                    .set_rate(index, clock.rates()[index] * factor)
                    .then(|| -factor.ln())
            }
            Self::NodeTimeRates => {
                if tree.nodes() - tree.tips() < 2 {
                    return None;
                }
                let clock = rates.clock.as_mut()?;
                let node = loop {
                    let node = tree.tips() + random.below(tree.nodes() - tree.tips());
                    if node != tree.root() {
                        break node;
                    }
                };
                let (a, b) = tree.children(node)?;
                let lower = tree.height(a).max(tree.height(b));
                let upper = tree.height(tree.parent(node)?);
                coupled::time_rates(
                    tree,
                    clock,
                    node,
                    lower + random.uniform() * (upper - lower),
                )
            }
            Self::RootTimeRates => {
                let factor = scaler(tuning, random);
                let root = tree.root();
                let height = tree.height(root) * factor;
                coupled::time_rates(tree, rates.clock.as_mut()?, root, height)
                    .map(|jacobian| jacobian - factor.ln())
            }
            Self::ClockSigmaRates => {
                coupled::sigma_rates(rates.clock.as_mut()?, scaler(tuning, random))
            }
        }
    }
}

/// A factor uniform on `[s, 1/s]`.
fn scaler(s: f64, random: &mut Random) -> f64 {
    s + random.uniform() * (1.0 / s - s)
}

fn non_root(tree: &Tree, random: &mut Random) -> usize {
    loop {
        let node = random.below(tree.nodes());
        if node != tree.root() {
            return node;
        }
    }
}

fn node_height(tree: &mut Tree, random: &mut Random) -> Option<f64> {
    let internal = tree.nodes() - tree.tips();
    if internal < 2 {
        return None;
    }
    let node = loop {
        let node = tree.tips() + random.below(internal);
        if node != tree.root() {
            break node;
        }
    };
    let (a, b) = tree.children(node)?;
    let lower = tree.height(a).max(tree.height(b));
    let upper = tree.height(tree.parent(node)?);
    tree.set_height(node, lower + random.uniform() * (upper - lower));
    Some(0.0)
}

fn root_scale(tree: &mut Tree, factor: f64, random: &mut Random) -> Option<f64> {
    let f = scaler(factor, random);
    let root = tree.root();
    let (a, b) = tree.children(root)?;
    let height = tree.height(root) * f;
    if height < tree.height(a).max(tree.height(b)) {
        return None;
    }
    tree.set_height(root, height);
    Some(-f.ln())
}

/// Scales every internal height; with `k` internal nodes the Hastings ratio
/// is `f^(k − 2)`, counting the factor and its inverse.
fn tree_scale(tree: &mut Tree, factor: f64, random: &mut Random) -> Option<f64> {
    let f = scaler(factor, random);
    for node in tree.tips()..tree.nodes() {
        tree.set_height(node, tree.height(node) * f);
    }
    let internal = (tree.nodes() - tree.tips()) as f64;
    Some((internal - 2.0) * f.ln())
}

fn scale(value: &mut f64, upper: f64, factor: f64, random: &mut Random) -> Option<f64> {
    if *value == 0.0 {
        return None;
    }
    let f = scaler(factor, random);
    let scaled = *value * f;
    if !(0.0..=upper).contains(&scaled) {
        return None;
    }
    *value = scaled;
    Some(-f.ln())
}

/// The number of branches in the subtree of `node`, its own included, that
/// cross `height`.
fn crossing(tree: &Tree, node: usize, height: f64) -> usize {
    let Some(parent) = tree.parent(node) else {
        return 0;
    };
    if tree.height(parent) < height {
        0
    } else if tree.height(node) < height {
        1
    } else if let Some((a, b)) = tree.children(node) {
        crossing(tree, a, height) + crossing(tree, b, height)
    } else {
        0
    }
}

/// The branch numbered `index` among those `crossing` counts, in the same order.
fn nth_crossing(tree: &Tree, node: usize, height: f64, index: &mut usize) -> Option<usize> {
    if tree.height(tree.parent(node)?) < height {
        return None;
    }
    if tree.height(node) < height {
        if *index == 0 {
            return Some(node);
        }
        *index -= 1;
        return None;
    }
    let (a, b) = tree.children(node)?;
    nth_crossing(tree, a, height, index).or_else(|| nth_crossing(tree, b, height, index))
}

fn subtree_slide(tree: &mut Tree, size: f64, random: &mut Random) -> Option<f64> {
    let node = non_root(tree, random);
    let parent = tree.parent(node)?;
    let sibling = tree.sibling(node);
    let grandparent = tree.parent(parent);
    let old = tree.height(parent);
    let new = old + random.normal() * size;
    if new > old {
        let Some(above) = grandparent.filter(|&g| tree.height(g) < new) else {
            tree.set_height(parent, new);
            return Some(0.0);
        };
        let mut child = above;
        let mut destination = tree.parent(above);
        while let Some(d) = destination.filter(|&d| tree.height(d) < new) {
            child = d;
            destination = tree.parent(d);
        }
        tree.replace_child(parent, sibling, child);
        tree.replace_child(above, parent, sibling);
        match destination {
            None => tree.make_root(parent),
            Some(d) => tree.replace_child(d, child, parent),
        }
        tree.set_height(parent, new);
        Some(-(crossing(tree, child, old) as f64).ln())
    } else {
        if tree.height(node) > new {
            return None;
        }
        if tree.height(sibling) <= new {
            tree.set_height(parent, new);
            return Some(0.0);
        }
        let (a, b) = tree.children(sibling)?;
        let destinations = crossing(tree, a, new) + crossing(tree, b, new);
        if destinations == 0 {
            return None;
        }
        let mut index = random.below(destinations);
        let child = nth_crossing(tree, a, new, &mut index)
            .or_else(|| nth_crossing(tree, b, new, &mut index))?;
        let destination = tree.parent(child)?;
        tree.replace_child(parent, sibling, child);
        match grandparent {
            None => tree.make_root(sibling),
            Some(g) => tree.replace_child(g, parent, sibling),
        }
        tree.replace_child(destination, child, parent);
        tree.set_height(parent, new);
        Some((destinations as f64).ln())
    }
}

fn narrow_exchange(tree: &mut Tree, random: &mut Random) -> Option<f64> {
    let internal = tree.nodes() - tree.tips();
    if internal <= 1 {
        return None;
    }
    let is_tip = |tree: &Tree, n: usize| tree.children(n).is_none();
    // An internal node with at least one internal child.
    let grandparent_like = |tree: &Tree, n: usize| {
        tree.children(n)
            .is_some_and(|(a, b)| !is_tip(tree, a) || !is_tip(tree, b))
    };
    let count = |tree: &Tree| {
        (tree.tips()..tree.nodes())
            .filter(|&n| grandparent_like(tree, n))
            .count()
    };
    let grandparent = loop {
        let node = tree.tips() + random.below(internal);
        if grandparent_like(tree, node) {
            break node;
        }
    };
    let (mut parent, mut uncle) = tree.children(grandparent)?;
    if tree.height(parent) < tree.height(uncle) {
        std::mem::swap(&mut parent, &mut uncle);
    }
    let (a, b) = tree.children(parent)?;
    let before = count(tree);
    let child = if random.coin() { a } else { b };
    tree.replace_child(parent, child, uncle);
    tree.replace_child(grandparent, uncle, child);
    Some((before as f64 / count(tree) as f64).ln())
}

fn wide_exchange(tree: &mut Tree, random: &mut Random) -> Option<f64> {
    let i = non_root(tree, random);
    let j = loop {
        let j = non_root(tree, random);
        if j != i {
            break j;
        }
    };
    let (p, q) = (tree.parent(i)?, tree.parent(j)?);
    if p == q
        || i == q
        || j == p
        || tree.height(j) >= tree.height(p)
        || tree.height(i) >= tree.height(q)
    {
        return None;
    }
    tree.replace_child(p, i, j);
    tree.replace_child(q, j, i);
    Some(0.0)
}

fn wilson_balding(tree: &mut Tree, random: &mut Random) -> Option<f64> {
    let i = non_root(tree, random);
    let p = tree.parent(i)?;
    let j = loop {
        let j = random.below(tree.nodes());
        let above_i = tree
            .parent(j)
            .is_none_or(|q| tree.height(q) > tree.height(i));
        if j != i && above_i {
            break j;
        }
    };
    if j == tree.root() || p == tree.root() {
        return None;
    }
    let q = tree.parent(j)?;
    if q == p || j == p || q == i {
        return None;
    }
    let sibling = tree.sibling(i);
    let grandparent = tree.parent(p)?;
    let new_min = tree.height(i).max(tree.height(j));
    let new_range = tree.height(q) - new_min;
    let old_range = tree.height(grandparent) - tree.height(i).max(tree.height(sibling));
    if old_range == 0.0 || new_range == 0.0 {
        return None;
    }
    let height = new_min + random.uniform() * new_range;
    tree.replace_child(grandparent, p, sibling);
    tree.replace_child(p, sibling, j);
    tree.replace_child(q, j, p);
    tree.set_height(p, height);
    Some((new_range / old_range.abs()).ln())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Move, scale};
    use crate::random::Random;
    use crate::rates::RateVariation;
    use crate::speciation::BirthDeath;
    use crate::tree::Tree;

    /// Metropolis–Hastings with only the scale move must keep a uniform
    /// target on `[0, 1]`.
    #[test]
    fn scaling_preserves_a_uniform_target() {
        let mut random = Random::new(11);
        let mut value = 0.5;
        let (mut sum, mut below, steps) = (0.0, 0usize, 400_000);
        for _ in 0..steps {
            let mut proposal = value;
            if let Some(log_hastings) = scale(&mut proposal, 1.0, 0.6, &mut random) {
                if log_hastings >= 0.0 || random.uniform() < log_hastings.exp() {
                    value = proposal;
                }
            }
            sum += value;
            below += usize::from(value < 0.25);
        }
        let mean = sum / steps as f64;
        let quarter = below as f64 / steps as f64;
        assert!((mean - 0.5).abs() < 0.01, "mean {mean}");
        assert!((quarter - 0.25).abs() < 0.01, "below 0.25: {quarter}");
    }

    fn clades(tree: &Tree) -> Vec<u32> {
        let mut masks = vec![0u32; tree.nodes()];
        for node in tree.postorder() {
            masks[node] = match tree.children(node) {
                None => 1 << node,
                Some((a, b)) => masks[a] | masks[b],
            };
        }
        let mut internal = masks[tree.tips()..].to_vec();
        internal.sort_unstable();
        internal
    }

    /// Orderings of the internal nodes that respect the topology.
    fn rankings(clades: &[u32]) -> f64 {
        let factorial: f64 = (1..=clades.len()).map(|k| k as f64).product();
        factorial
            / clades
                .iter()
                .map(|c| f64::from(c.count_ones() - 1))
                .product::<f64>()
    }

    /// `2 · Σ` internal heights with the root at most 1. The density grows
    /// with height, so moves that lower a node can be rejected, and it depends
    /// on the heights only, so each topology's probability is proportional to
    /// its rankings.
    fn log_target(tree: &Tree) -> f64 {
        if tree.height(tree.root()) > 1.0 {
            return f64::NEG_INFINITY;
        }
        2.0 * (tree.tips()..tree.nodes())
            .map(|n| tree.height(n))
            .sum::<f64>()
    }

    /// Total variation from the expected topology frequencies for five tips,
    /// alternating `kind` with uniform node-height moves.
    fn total_variation(kind: Move, steps: usize, seed: u64) -> f64 {
        let mut tree = Tree::new(
            vec![
                None,
                None,
                None,
                None,
                None,
                Some((0, 1)),
                Some((5, 2)),
                Some((6, 3)),
                Some((7, 4)),
            ],
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.2, 0.4, 0.6, 0.8],
        )
        .unwrap();
        let mut proposal = tree.clone();
        let mut random = Random::new(seed);
        let mut birth_death = BirthDeath {
            birth_difference: 1.0,
            relative_death: 0.5,
        };
        let mut current = log_target(&tree);
        let mut counts: HashMap<Vec<u32>, usize> = HashMap::new();
        for step in 0..steps {
            let chosen = if random.coin() {
                kind
            } else {
                Move::NodeHeight
            };
            proposal.clone_from(&tree);
            let tuning = chosen.initial_tuning().map(|t| t.min(0.3));
            if let Some(log_hastings) = chosen.propose(
                &mut proposal,
                &mut birth_death,
                &mut RateVariation::default(),
                1.0,
                tuning,
                &mut random,
            ) {
                let next = log_target(&proposal);
                let log_alpha = next - current + log_hastings;
                if log_alpha >= 0.0 || random.uniform() < log_alpha.exp() {
                    tree.clone_from(&proposal);
                    current = next;
                }
            }
            if step % 4 == 0 {
                *counts.entry(clades(&tree)).or_default() += 1;
            }
        }
        let total: usize = counts.values().sum();
        let observed: f64 = counts
            .iter()
            .map(|(c, &n)| (n as f64 / total as f64 - rankings(c) / 180.0).abs())
            .sum();
        let unobserved = 1.0 - counts.keys().map(|c| rankings(c) / 180.0).sum::<f64>();
        (observed + unobserved) / 2.0
    }

    /// The subtree slide mixes slowest; dropping either of its counts raises
    /// its distance to about 0.026 at 6.4 million steps, against 0.009.
    #[test]
    fn topology_moves_keep_a_height_tilted_target() {
        for (kind, steps, tolerance) in [
            (Move::SubtreeSlide, 6_400_000, 0.016),
            (Move::NarrowExchange, 1_600_000, 0.02),
            (Move::WideExchange, 1_600_000, 0.02),
            (Move::WilsonBalding, 1_600_000, 0.02),
        ] {
            let distance = total_variation(kind, steps, 20261028);
            assert!(distance < tolerance, "{kind:?}: total variation {distance}");
        }
    }
}
