//! A Metropolis–Hastings sampler for partitioned morphological characters on a
//! time tree.
//!
//! The model is Mk, optionally conditioned on variable characters (Mkv),
//! with one rate or four gamma categories, and a strict or lognormal relaxed
//! clock. The clock-rate and mutation-rate multipliers are fixed at one.
//! The tree prior is the birth–death prior in
//! `speciation`, with Uniform(0, `birth_difference_upper`) on λ − μ and
//! Uniform(0, 1) on μ / λ. Monophyly constraints restrict the topology.

mod checkpoint;
mod coupled;
mod moves;
pub mod output;

use std::fmt;

use crate::ascertainment::{self, VariableCharacters};
use crate::data::Patterns;
use crate::model::Mk;
use crate::random::Random;
use crate::rates::{self, GammaRates, RateVariation, RelaxedClock};
use crate::speciation::BirthDeath;
use crate::tree::Tree;
use crate::{Backend, LikelihoodCore, Scaling};

pub use moves::Move;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// A tree needs at least two taxa.
    TooFewTaxa(usize),
    /// Mk needs at least two states.
    States(usize),
    /// Partitions must be nonempty and contain the same taxa in the same order.
    Partitions,
    Ascertainment(ascertainment::Error),
    /// A monophyly constraint has fewer than two tips, an unknown tip, or
    /// overlaps another without nesting. The index is the constraint's.
    Constraint(usize),
    /// A move weight is negative or not finite, or no move has weight.
    Weights,
    /// A tuning value is given for a move without one, or is out of range:
    /// slide sizes must be positive and scale factors between 0 and 1.
    Tuning(Move),
    /// The initial state has no posterior density.
    InitialState,
    /// The likelihood backend is not part of this build.
    Backend(Backend),
    Rates(rates::Error),
    /// A rate move is selected without its corresponding model component.
    RateMove(Move),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rates(error) => error.fmt(f),
            Self::RateMove(kind) => write!(f, "{} needs its rate model to be enabled", kind.name()),
            Self::TooFewTaxa(taxa) => write!(f, "a tree needs at least two taxa, not {taxa}"),
            Self::States(states) => write!(f, "Mk needs at least two states, not {states}"),
            Self::Partitions => write!(f, "partitions must be nonempty and contain the same taxa"),
            Self::Ascertainment(error) => error.fmt(f),
            Self::Constraint(index) => write!(
                f,
                "monophyly constraint {} needs at least two known taxa and must nest within or exclude every other constraint",
                index + 1
            ),
            Self::Weights => write!(
                f,
                "move weights must be finite, non-negative and not all zero"
            ),
            Self::Tuning(kind) => write!(
                f,
                "invalid tuning value for {}: slide sizes must be positive and scale factors between 0 and 1",
                kind.name()
            ),
            Self::InitialState => write!(f, "the initial state has zero posterior probability"),
            Self::Backend(backend) => write!(
                f,
                "the {} backend is not available on this target",
                backend.name()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ascertainment(error) => Some(error),
            Self::Rates(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conditioning {
    /// Mk: the likelihood of the observed characters.
    None,
    /// Mkv: conditioned on every character being variable.
    Variable,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub conditioning: Conditioning,
    /// Four midpoint gamma categories, starting at this shape; None uses one rate.
    pub gamma_shape: Option<f64>,
    /// Explicit lognormal branch rates, starting at 0.5 with this sigma.
    /// None uses a strict clock. Both clock-rate multipliers are fixed at one.
    pub clock_sigma: Option<f64>,
    /// Ignore the data: the chain samples the prior.
    pub sample_from_prior: bool,
    /// Tip sets that must stay monophyletic.
    pub monophyly: Vec<Vec<usize>>,
    pub birth_difference_upper: f64,
    pub initial: BirthDeath,
    /// Relative selection weights; a move with weight zero is never chosen.
    pub weights: Vec<(Move, f64)>,
    /// Starting slide sizes or scale factors, in place of each move's default.
    pub tuning_values: Vec<(Move, f64)>,
    /// After `tuning_delay` tunable proposals, move slide sizes and scale
    /// factors toward an acceptance rate of 0.234; otherwise keep them fixed.
    pub tuning: bool,
    pub tuning_delay: u64,
    /// The likelihood core's backend; every backend gives the same results.
    pub backend: Backend,
}

impl Default for Settings {
    /// The weights and starting values of the strict-clock comparison with
    /// BEAST 2.7.8 recorded in the repository.
    fn default() -> Self {
        Self {
            conditioning: Conditioning::Variable,
            gamma_shape: None,
            clock_sigma: None,
            sample_from_prior: false,
            monophyly: Vec::new(),
            birth_difference_upper: 1000.0,
            initial: BirthDeath {
                birth_difference: 1.0,
                relative_death: 0.5,
            },
            weights: vec![
                (Move::NodeHeight, 30.0),
                (Move::SubtreeSlide, 15.0),
                (Move::NarrowExchange, 15.0),
                (Move::WideExchange, 3.0),
                (Move::WilsonBalding, 3.0),
                (Move::RootScale, 3.0),
                (Move::TreeScale, 3.0),
                (Move::BirthDifferenceScale, 3.0),
                (Move::RelativeDeathScale, 3.0),
            ],
            tuning_values: Vec::new(),
            tuning: true,
            tuning_delay: 10_000,
            backend: Backend::NATIVE,
        }
    }
}

impl Settings {
    /// The reference model, with classical tree moves and three scale moves
    /// instead of ORC's adaptive mixtures. Monophyly must be set explicitly.
    pub fn reference() -> Self {
        let mut settings = Self {
            gamma_shape: Some(1.0),
            clock_sigma: Some(0.2),
            ..Self::default()
        };
        settings.weights.extend([
            (Move::GammaShapeScale, 0.1),
            (Move::ClockSigmaScale, 3.0),
            (Move::BranchRateScale, 20.0),
        ]);
        settings
    }
}

/// Log density components of a state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Posterior {
    pub likelihood: f64,
    pub tree_prior: f64,
    pub parameter_prior: f64,
    pub gamma_prior: f64,
    pub clock_rate_prior: f64,
    pub clock_sigma_prior: f64,
}

impl Posterior {
    pub fn prior(&self) -> f64 {
        self.tree_prior
            + self.parameter_prior
            + self.gamma_prior
            + self.clock_rate_prior
            + self.clock_sigma_prior
    }

    pub fn total(&self) -> f64 {
        self.likelihood + self.prior()
    }
}

/// Selection weight, tuning and outcome counts of one move.
#[derive(Clone, Debug, PartialEq)]
pub struct Operator {
    pub kind: Move,
    pub weight: f64,
    pub initial_tuning: Option<f64>,
    pub tuning: Option<f64>,
    pub accepted: u64,
    pub rejected: u64,
    /// Proposals that were invalid or broke a constraint.
    pub failed: u64,
    /// Proposals since tuning started.
    pub tuned: u64,
}

struct Likelihood {
    core: LikelihoodCore,
    variable: Option<VariableCharacters>,
    weights: Vec<f64>,
    frequencies: Vec<f64>,
    dirty: Vec<bool>,
    order: Vec<usize>,
}

impl Likelihood {
    fn new(
        patterns: &Patterns,
        conditioning: Conditioning,
        backend: Backend,
        tree: &Tree,
        rates: &RateVariation,
    ) -> Result<Self, Error> {
        let states = patterns.symbols().len();
        let model = Mk::uniform(states);
        let variable = match conditioning {
            Conditioning::Variable => {
                Some(VariableCharacters::new(patterns).map_err(Error::Ascertainment)?)
            }
            Conditioning::None => None,
        };
        let tips = variable
            .as_ref()
            .map_or(patterns.tip_partials(), VariableCharacters::tip_partials);
        let mut core = LikelihoodCore::mk(
            &model,
            tips[0].len() / states,
            rates.category_rates().len(),
            tree.nodes(),
            Scaling::Always,
        );
        core.set_backend(backend);
        for (tip, partials) in tips.iter().enumerate() {
            core.set_tip_partials(tip, partials);
        }
        let mut likelihood = Self {
            core,
            variable,
            weights: patterns.weights().iter().map(|&w| w as f64).collect(),
            frequencies: model.frequencies().to_vec(),
            dirty: vec![false; tree.nodes()],
            order: Vec::with_capacity(tree.nodes()),
        };
        for node in tree.postorder() {
            let distance = tree.branch_length(node) * rates.branch_rate(node, tree.root());
            for (category, &rate) in rates.category_rates().iter().enumerate() {
                likelihood.core.set_node_distance(
                    node,
                    category,
                    category_distance(distance, rate),
                );
            }
            if let Some((a, b)) = tree.children(node) {
                likelihood.core.calculate_partials(a, b, node);
            }
        }
        Ok(likelihood)
    }

    /// Recomputes the branches that differ from `before`, and the partials of
    /// their ancestors.
    fn update(&mut self, tree: &Tree, before: &Tree, rates: &RateVariation) -> f64 {
        let dirty = &mut self.dirty;
        dirty.fill(false);
        for node in 0..tree.nodes() {
            if node == tree.root() {
                continue;
            }
            let distance = tree.branch_length(node) * rates.branch_rate(node, tree.root());
            let unchanged =
                rates.category_rates().iter().enumerate().all(|(c, r)| {
                    self.core.node_distance(node, c) == category_distance(distance, *r)
                });
            if tree.parent(node) == before.parent(node) && unchanged {
                continue;
            }
            self.core.set_node_matrix_for_update(node);
            for (category, &rate) in rates.category_rates().iter().enumerate() {
                self.core
                    .set_node_distance(node, category, category_distance(distance, rate));
            }
            let mut above = tree.parent(node);
            while let Some(p) = above.filter(|&p| !dirty[p]) {
                dirty[p] = true;
                above = tree.parent(p);
            }
        }
        tree.postorder_into(&mut self.order);
        for &node in &self.order {
            if let (true, Some((a, b))) = (self.dirty[node], tree.children(node)) {
                self.core.set_node_partials_for_update(node);
                self.core.calculate_partials(a, b, node);
            }
        }
        self.evaluate(tree.root(), rates)
    }

    /// −∞ when the value is not a number or the Mkv correction is undefined.
    fn evaluate(&mut self, root: usize, rates: &RateVariation) -> f64 {
        let value = match &self.variable {
            Some(variable) => {
                self.core.integrate_partials(root, rates.category_weights());
                let lls = self.core.calculate_log_likelihoods(&self.frequencies);
                variable.log_likelihood(lls).unwrap_or(f64::NEG_INFINITY)
            }
            None => self.core.log_likelihood(
                root,
                rates.category_weights(),
                &self.frequencies,
                &self.weights,
            ),
        };
        if value.is_nan() {
            f64::NEG_INFINITY
        } else {
            value
        }
    }
}

// A zero-rate category stays constant even when the product of a finite
// branch length and a finite clock rate overflows to infinity.
fn category_distance(distance: f64, rate: f64) -> f64 {
    if rate == 0.0 { 0.0 } else { distance * rate }
}

pub struct Sampler {
    tree: Tree,
    rates: RateVariation,
    stored_rates: RateVariation,
    birth_death: BirthDeath,
    stored_tree: Tree,
    likelihoods: Vec<Likelihood>,
    patterns: Vec<Patterns>,
    conditioning: Conditioning,
    backend: Backend,
    posterior: Posterior,
    operators: Vec<Operator>,
    constraints: Vec<Vec<bool>>,
    order: Vec<usize>,
    below: Vec<(usize, usize)>,
    birth_difference_upper: f64,
    tuning: bool,
    tuning_delay: u64,
    tunable_proposals: u64,
    random: Random,
    steps: u64,
}

impl Sampler {
    /// A chain from a random tree that satisfies the constraints.
    pub fn new(
        patterns: impl AsRef<[Patterns]>,
        settings: &Settings,
        seed: u64,
    ) -> Result<Self, Error> {
        let patterns = patterns.as_ref();
        let mut random = Random::new(seed);
        let tips = patterns
            .first()
            .ok_or(Error::Partitions)?
            .tip_partials()
            .len();
        if tips < 2 {
            return Err(Error::TooFewTaxa(tips));
        }
        let tree = Tree::random(
            tips,
            &check_constraints(&settings.monophyly, tips)?,
            &mut random,
        );
        Self::from_tree(patterns, settings, tree, None, random)
    }

    /// A chain that starts from `tree`, whose tips must be at height zero.
    pub fn with_tree(
        patterns: impl AsRef<[Patterns]>,
        settings: &Settings,
        tree: Tree,
        seed: u64,
    ) -> Result<Self, Error> {
        Self::from_tree(patterns.as_ref(), settings, tree, None, Random::new(seed))
    }

    /// A fixed rate state, for resuming model calculations and numerical
    /// comparisons. Rate-component presence must match the settings.
    pub fn with_rates(
        patterns: impl AsRef<[Patterns]>,
        settings: &Settings,
        tree: Tree,
        rates: RateVariation,
        seed: u64,
    ) -> Result<Self, Error> {
        Self::from_tree(
            patterns.as_ref(),
            settings,
            tree,
            Some(rates),
            Random::new(seed),
        )
    }

    fn from_tree(
        patterns: &[Patterns],
        settings: &Settings,
        tree: Tree,
        given_rates: Option<RateVariation>,
        random: Random,
    ) -> Result<Self, Error> {
        let tips = tree.tips();
        if patterns.is_empty() || patterns.iter().any(|p| p.tip_partials().len() != tips) {
            return Err(Error::Partitions);
        }
        if let Some(p) = patterns.iter().find(|p| p.symbols().len() < 2) {
            return Err(Error::States(p.symbols().len()));
        }
        assert!(
            (0..tips).all(|t| tree.height(t) == 0.0),
            "tips must be at height zero"
        );
        let rates = match given_rates {
            Some(rates) => rates,
            None => RateVariation {
                gamma: settings
                    .gamma_shape
                    .map(GammaRates::new)
                    .transpose()
                    .map_err(Error::Rates)?,
                clock: settings
                    .clock_sigma
                    .map(|sigma| RelaxedClock::new(sigma, vec![0.5; tree.nodes() - 1]))
                    .transpose()
                    .map_err(Error::Rates)?,
            },
        };
        if rates.gamma.is_some() != settings.gamma_shape.is_some() {
            return Err(Error::Rates(rates::Error::Shape));
        }
        if rates.clock.is_some() != settings.clock_sigma.is_some()
            || rates
                .clock
                .as_ref()
                .is_some_and(|c| c.rates().len() != tree.nodes() - 1)
        {
            return Err(Error::Rates(rates::Error::BranchRates));
        }
        for &(kind, weight) in &settings.weights {
            if weight > 0.0
                && ((kind == Move::GammaShapeScale && rates.gamma.is_none())
                    || (matches!(
                        kind,
                        Move::ClockSigmaScale
                            | Move::BranchRateScale
                            | Move::NodeTimeRates
                            | Move::RootTimeRates
                            | Move::ClockSigmaRates
                    ) && rates.clock.is_none()))
            {
                return Err(Error::RateMove(kind));
            }
        }
        let constraints = check_constraints(&settings.monophyly, tips)?
            .iter()
            .map(|clade| (0..tips).map(|t| clade.contains(&t)).collect())
            .collect();
        let valid = |(_, w): &(Move, f64)| w.is_finite() && *w >= 0.0;
        let total_weight: f64 = settings.weights.iter().map(|(_, w)| w).sum();
        if !settings.weights.iter().all(valid) || !total_weight.is_finite() || total_weight <= 0.0 {
            return Err(Error::Weights);
        }
        for &(kind, value) in &settings.tuning_values {
            let in_range = match kind {
                Move::SubtreeSlide => value > 0.0,
                _ => value > 0.0 && value < 1.0,
            };
            if kind.initial_tuning().is_none() || !value.is_finite() || !in_range {
                return Err(Error::Tuning(kind));
            }
        }
        let operators = settings
            .weights
            .iter()
            .map(|&(kind, weight)| {
                let given = settings
                    .tuning_values
                    .iter()
                    .rev()
                    .find(|(k, _)| *k == kind);
                let initial = given.map(|&(_, value)| value).or(kind.initial_tuning());
                Operator {
                    kind,
                    weight,
                    initial_tuning: initial,
                    tuning: initial,
                    accepted: 0,
                    rejected: 0,
                    failed: 0,
                    tuned: 0,
                }
            })
            .collect();
        if !settings.backend.is_available() {
            return Err(Error::Backend(settings.backend));
        }
        let likelihoods = if settings.sample_from_prior {
            Vec::new()
        } else {
            patterns
                .iter()
                .map(|p| Likelihood::new(p, settings.conditioning, settings.backend, &tree, &rates))
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut sampler = Self {
            stored_tree: tree.clone(),
            stored_rates: rates.clone(),
            rates,
            tree,
            birth_death: settings.initial,
            likelihoods,
            patterns: patterns.to_vec(),
            conditioning: settings.conditioning,
            backend: settings.backend,
            posterior: Posterior {
                likelihood: 0.0,
                tree_prior: 0.0,
                parameter_prior: 0.0,
                gamma_prior: 0.0,
                clock_rate_prior: 0.0,
                clock_sigma_prior: 0.0,
            },
            operators,
            constraints,
            order: Vec::with_capacity(tips * 2),
            below: vec![(0, 0); tips * 2 - 1],
            birth_difference_upper: settings.birth_difference_upper,
            tuning: settings.tuning,
            tuning_delay: settings.tuning_delay,
            tunable_proposals: 0,
            random,
            steps: 0,
        };
        sampler.posterior = sampler.recompute()?;
        if !sampler.posterior.total().is_finite() || !sampler.satisfies_constraints() {
            return Err(Error::InitialState);
        }
        Ok(sampler)
    }

    pub fn rates(&self) -> &RateVariation {
        &self.rates
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    pub fn birth_death(&self) -> BirthDeath {
        self.birth_death
    }

    pub fn posterior(&self) -> Posterior {
        self.posterior
    }

    pub fn operators(&self) -> &[Operator] {
        &self.operators
    }

    pub fn steps(&self) -> u64 {
        self.steps
    }

    fn priors(&self) -> Posterior {
        let BirthDeath {
            birth_difference: r,
            relative_death: a,
        } = self.birth_death;
        let upper = self.birth_difference_upper;
        let parameters = if (0.0..=upper).contains(&r) && (0.0..=1.0).contains(&a) {
            -upper.ln()
        } else {
            f64::NEG_INFINITY
        };
        Posterior {
            likelihood: 0.0,
            tree_prior: self.birth_death.log_density(&self.tree),
            parameter_prior: parameters,
            gamma_prior: self.rates.gamma.as_ref().map_or(0.0, GammaRates::log_prior),
            clock_rate_prior: self
                .rates
                .clock
                .as_ref()
                .map_or(0.0, RelaxedClock::log_rate_prior),
            clock_sigma_prior: self
                .rates
                .clock
                .as_ref()
                .map_or(0.0, RelaxedClock::log_sigma_prior),
        }
    }

    fn satisfies_constraints(&mut self) -> bool {
        if self.constraints.is_empty() {
            return true;
        }
        self.tree.postorder_into(&mut self.order);
        let (tree, order, below) = (&self.tree, &self.order, &mut self.below);
        self.constraints
            .iter()
            .all(|clade| tree.is_monophyletic_in(clade, order, below))
    }

    /// Every posterior component evaluated from scratch, with a new core.
    pub fn recompute(&self) -> Result<Posterior, Error> {
        let mut posterior = self.priors();
        if !self.likelihoods.is_empty() {
            for patterns in &self.patterns {
                posterior.likelihood += Likelihood::new(
                    patterns,
                    self.conditioning,
                    self.backend,
                    &self.tree,
                    &self.rates,
                )?
                .evaluate(self.tree.root(), &self.rates);
            }
        }
        Ok(posterior)
    }

    /// One Metropolis–Hastings step. Returns the move and whether it was accepted.
    pub fn step(&mut self) -> (Move, bool) {
        self.steps += 1;
        let index = self.choose();
        let kind = self.operators[index].kind;
        let stored_birth_death = self.birth_death;
        self.stored_tree.clone_from(&self.tree);
        self.stored_rates.clone_from(&self.rates);
        for likelihood in &mut self.likelihoods {
            likelihood.core.store();
        }
        let proposal = kind.propose(
            &mut self.tree,
            &mut self.birth_death,
            &mut self.rates,
            self.birth_difference_upper,
            self.operators[index].tuning,
            &mut self.random,
        );
        let valid = match proposal {
            Some(_) if kind.changes_topology() && !self.satisfies_constraints() => None,
            other => other,
        };
        let mut log_alpha = f64::NEG_INFINITY;
        let mut accepted = false;
        if let Some(log_hastings) = valid {
            let mut proposed = self.priors();
            proposed.likelihood = if !kind.changes_likelihood() {
                self.posterior.likelihood
            } else if self.likelihoods.is_empty() {
                0.0
            } else if proposed.prior().is_finite() {
                self.likelihoods
                    .iter_mut()
                    .map(|l| l.update(&self.tree, &self.stored_tree, &self.rates))
                    .sum()
            } else {
                f64::NEG_INFINITY
            };
            log_alpha = proposed.total() - self.posterior.total() + log_hastings;
            accepted = log_alpha >= 0.0
                || (log_alpha > f64::NEG_INFINITY && self.random.uniform() < log_alpha.exp());
            if accepted {
                self.posterior = proposed;
            }
        }
        let operator = &mut self.operators[index];
        match (valid.is_some(), accepted) {
            (_, true) => operator.accepted += 1,
            (true, false) => operator.rejected += 1,
            (false, _) => operator.failed += 1,
        }
        if !accepted {
            std::mem::swap(&mut self.tree, &mut self.stored_tree);
            self.birth_death = stored_birth_death;
            std::mem::swap(&mut self.rates, &mut self.stored_rates);
            for likelihood in &mut self.likelihoods {
                likelihood.core.restore();
            }
        }
        self.tune(index, log_alpha);
        (kind, accepted)
    }

    fn choose(&mut self) -> usize {
        let total: f64 = self.operators.iter().map(|o| o.weight).sum();
        let mut target = self.random.uniform() * total;
        for (index, operator) in self.operators.iter().enumerate() {
            if target < operator.weight {
                return index;
            }
            target -= operator.weight;
        }
        self.operators
            .iter()
            .rposition(|o| o.weight > 0.0)
            .expect("a weighted move")
    }

    /// BEAST's update, `OperatorSchedule.calcDelta` followed by the operator's
    /// `optimize`, with BEAST's optional square-root transform: the step is
    /// `(α − 0.234) / √(n + 1)` after the move's `n`-th proposal since tuning
    /// started. BEAST's default divides by `n + 1` instead. A failed proposal
    /// counts as `α = 0`; for a proposal its operator rejects, BEAST reuses the
    /// previous step's α.
    pub(crate) fn tune(&mut self, index: usize, log_alpha: f64) {
        let operator = &mut self.operators[index];
        let Some(tuning) = operator.tuning.filter(|_| self.tuning) else {
            return;
        };
        if self.tunable_proposals < self.tuning_delay {
            self.tunable_proposals += 1;
            return;
        }
        operator.tuned += 1;
        if log_alpha.is_nan() {
            return;
        }
        let count = (operator.tuned as f64 + 1.0).sqrt();
        let delta = (log_alpha.min(0.0).exp() - 0.234) / count;
        operator.tuning = Some(operator.kind.adapt(tuning, delta));
    }
}

/// Sorted, distinct tip indices for each constraint, if they are valid and
/// pairwise nested or disjoint.
fn check_constraints(constraints: &[Vec<usize>], tips: usize) -> Result<Vec<Vec<usize>>, Error> {
    let mut clades = Vec::with_capacity(constraints.len());
    for (index, clade) in constraints.iter().enumerate() {
        let mut sorted = clade.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() < 2 || sorted.len() != clade.len() || sorted.iter().any(|&t| t >= tips) {
            return Err(Error::Constraint(index));
        }
        clades.push(sorted);
    }
    for (i, a) in clades.iter().enumerate() {
        for b in &clades[..i] {
            let shared = a.iter().filter(|t| b.contains(t)).count();
            if shared != 0 && shared != a.len() && shared != b.len() {
                return Err(Error::Constraint(i));
            }
        }
    }
    Ok(clades)
}

#[cfg(test)]
mod tests {
    use super::{Move, Sampler, Settings};
    use crate::data::{GapTreatment, Patterns};
    use crate::nexus;

    fn patterns() -> Patterns {
        let text =
            "#NEXUS\nBEGIN DATA; DIMENSIONS NTAX=4 NCHAR=2; MATRIX a 01 b 10 c 01 d 10; END;";
        nexus::parse(text)
            .unwrap()
            .compress(std::slice::from_ref(&(0..2)), GapTreatment::Reject)
            .unwrap()
            .remove(0)
    }

    fn sampler(settings: &Settings) -> Sampler {
        Sampler::new(patterns(), settings, 1).unwrap()
    }

    #[test]
    fn tuning_follows_the_acceptance_of_each_proposal() {
        let settings = Settings {
            tuning_delay: 0,
            ..Settings::default()
        };
        let mut chain = sampler(&settings);
        let slide = chain
            .operators
            .iter()
            .position(|o| o.kind == Move::SubtreeSlide)
            .unwrap();
        let scale = chain
            .operators
            .iter()
            .position(|o| o.kind == Move::TreeScale)
            .unwrap();
        // The first tuned proposal divides by √2: an accepted slide grows by
        // e^((1 − 0.234)/√2); a failed scale move raises its factor.
        chain.tune(slide, 0.0);
        let grown = chain.operators[slide].tuning.unwrap();
        assert!(
            (grown - ((1.0 - 0.234) / 2f64.sqrt()).exp()).abs() < 1e-12,
            "{grown}"
        );
        chain.tune(scale, f64::NEG_INFINITY);
        let factor = chain.operators[scale].tuning.unwrap();
        let expected = 1.0 / (((-0.234) / 2f64.sqrt() + (1.0 / 0.75 - 1.0f64).ln()).exp() + 1.0);
        assert!(
            (factor - expected).abs() < 1e-12 && factor > 0.75,
            "{factor}"
        );
        for _ in 0..50 {
            chain.tune(slide, f64::NEG_INFINITY);
            chain.tune(scale, 0.0);
        }
        assert!(chain.operators[slide].tuning.unwrap() < grown);
        assert!(chain.operators[scale].tuning.unwrap() < factor);
        chain.tune(slide, f64::NAN);
        assert_eq!(chain.operators[slide].tuned, 52);
    }

    #[test]
    fn fixed_tuning_values_are_used_and_checked() {
        let settings = Settings {
            tuning: false,
            tuning_values: vec![(Move::SubtreeSlide, 0.002), (Move::TreeScale, 0.3)],
            ..Settings::default()
        };
        let mut chain = sampler(&settings);
        for _ in 0..2000 {
            chain.step();
        }
        let tuning = |kind| {
            chain
                .operators
                .iter()
                .find(|o| o.kind == kind)
                .unwrap()
                .tuning
        };
        assert_eq!(tuning(Move::SubtreeSlide), Some(0.002));
        assert_eq!(tuning(Move::TreeScale), Some(0.3));
        assert_eq!(tuning(Move::RootScale), Some(0.75));
        for (kind, value) in [
            (Move::TreeScale, 1.0),
            (Move::SubtreeSlide, 0.0),
            (Move::NodeHeight, 0.5),
        ] {
            let bad = Settings {
                tuning_values: vec![(kind, value)],
                ..Settings::default()
            };
            assert!(
                Sampler::new(patterns(), &bad, 1).is_err(),
                "{kind:?} = {value}"
            );
        }
    }
}
