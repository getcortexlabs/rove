//! A Metropolis–Hastings sampler for binary morphological characters on a
//! time tree.
//!
//! The model is Mk with one rate category, optionally conditioned on variable
//! characters (Mkv), under a strict clock with rate 1, so node heights are in
//! expected changes per character. The tree prior is the birth–death prior in
//! `speciation`, with Uniform(0, `birth_difference_upper`) on λ − μ and
//! Uniform(0, 1) on μ / λ. Monophyly constraints restrict the topology.

mod moves;
pub mod output;

use std::fmt;

use crate::LikelihoodCore;
use crate::Scaling;
use crate::ascertainment::{self, VariableCharacters};
use crate::data::Patterns;
use crate::model::Mk;
use crate::random::Random;
use crate::speciation::BirthDeath;
use crate::tree::Tree;

pub use moves::Move;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// A tree needs at least two taxa.
    TooFewTaxa(usize),
    /// Only binary characters are supported.
    States(usize),
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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewTaxa(taxa) => write!(f, "a tree needs at least two taxa, not {taxa}"),
            Self::States(states) => write!(
                f,
                "the sampler supports binary characters only, not {states} states"
            ),
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
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ascertainment(error) => Some(error),
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
}

impl Default for Settings {
    /// The weights and starting values of the strict-clock comparison with
    /// BEAST 2.7.8 recorded in the repository.
    fn default() -> Self {
        Self {
            conditioning: Conditioning::Variable,
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
        }
    }
}

/// Log density components of a state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Posterior {
    pub likelihood: f64,
    pub tree_prior: f64,
    pub parameter_prior: f64,
}

impl Posterior {
    pub fn prior(&self) -> f64 {
        self.tree_prior + self.parameter_prior
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
    fn new(patterns: &Patterns, conditioning: Conditioning, tree: &Tree) -> Result<Self, Error> {
        let model = Mk::uniform(2);
        let variable = match conditioning {
            Conditioning::Variable => {
                Some(VariableCharacters::new(patterns).map_err(Error::Ascertainment)?)
            }
            Conditioning::None => None,
        };
        let tips = variable
            .as_ref()
            .map_or(patterns.tip_partials(), VariableCharacters::tip_partials);
        let mut core =
            LikelihoodCore::mk(&model, tips[0].len() / 2, 1, tree.nodes(), Scaling::Always);
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
            likelihood
                .core
                .set_node_distance(node, 0, tree.branch_length(node));
            if let Some((a, b)) = tree.children(node) {
                likelihood.core.calculate_partials(a, b, node);
            }
        }
        Ok(likelihood)
    }

    /// Recomputes the branches that differ from `before`, and the partials of
    /// their ancestors.
    fn update(&mut self, tree: &Tree, before: &Tree) -> f64 {
        let dirty = &mut self.dirty;
        dirty.fill(false);
        for node in 0..tree.nodes() {
            if node == tree.root()
                || (tree.parent(node) == before.parent(node)
                    && tree.branch_length(node) == before.branch_length(node))
            {
                continue;
            }
            self.core.set_node_matrix_for_update(node);
            self.core
                .set_node_distance(node, 0, tree.branch_length(node));
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
        self.evaluate(tree.root())
    }

    /// −∞ when the value is not a number or the Mkv correction is undefined.
    fn evaluate(&mut self, root: usize) -> f64 {
        let value = match &self.variable {
            Some(variable) => {
                self.core.integrate_partials(root, &[1.0]);
                let lls = self.core.calculate_log_likelihoods(&self.frequencies);
                variable.log_likelihood(lls).unwrap_or(f64::NEG_INFINITY)
            }
            None => self
                .core
                .log_likelihood(root, &[1.0], &self.frequencies, &self.weights),
        };
        if value.is_nan() {
            f64::NEG_INFINITY
        } else {
            value
        }
    }
}

pub struct Sampler {
    tree: Tree,
    birth_death: BirthDeath,
    stored_tree: Tree,
    likelihood: Option<Likelihood>,
    patterns: Patterns,
    conditioning: Conditioning,
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
    pub fn new(patterns: &Patterns, settings: &Settings, seed: u64) -> Result<Self, Error> {
        let mut random = Random::new(seed);
        let tips = patterns.tip_partials().len();
        if tips < 2 {
            return Err(Error::TooFewTaxa(tips));
        }
        let tree = Tree::random(
            tips,
            &check_constraints(&settings.monophyly, tips)?,
            &mut random,
        );
        Self::from_tree(patterns, settings, tree, random)
    }

    /// A chain that starts from `tree`, whose tips must be at height zero.
    pub fn with_tree(
        patterns: &Patterns,
        settings: &Settings,
        tree: Tree,
        seed: u64,
    ) -> Result<Self, Error> {
        Self::from_tree(patterns, settings, tree, Random::new(seed))
    }

    fn from_tree(
        patterns: &Patterns,
        settings: &Settings,
        tree: Tree,
        random: Random,
    ) -> Result<Self, Error> {
        let states = patterns.symbols().len();
        if states != 2 {
            return Err(Error::States(states));
        }
        let tips = tree.tips();
        assert_eq!(tips, patterns.tip_partials().len(), "one tip per taxon");
        assert!(
            (0..tips).all(|t| tree.height(t) == 0.0),
            "tips must be at height zero"
        );
        let constraints = check_constraints(&settings.monophyly, tips)?
            .iter()
            .map(|clade| (0..tips).map(|t| clade.contains(&t)).collect())
            .collect();
        let valid = |(_, w): &(Move, f64)| w.is_finite() && *w >= 0.0;
        if !settings.weights.iter().all(valid) || settings.weights.iter().all(|(_, w)| *w == 0.0) {
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
        let likelihood = if settings.sample_from_prior {
            None
        } else {
            Some(Likelihood::new(patterns, settings.conditioning, &tree)?)
        };
        let mut sampler = Self {
            stored_tree: tree.clone(),
            tree,
            birth_death: settings.initial,
            likelihood,
            patterns: patterns.clone(),
            conditioning: settings.conditioning,
            posterior: Posterior {
                likelihood: 0.0,
                tree_prior: 0.0,
                parameter_prior: 0.0,
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
        if sampler.posterior.total() == f64::NEG_INFINITY || !sampler.satisfies_constraints() {
            return Err(Error::InitialState);
        }
        Ok(sampler)
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

    fn priors(&self) -> (f64, f64) {
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
        (self.birth_death.log_density(&self.tree), parameters)
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
        let (tree_prior, parameter_prior) = self.priors();
        let likelihood = match &self.likelihood {
            None => 0.0,
            Some(_) => Likelihood::new(&self.patterns, self.conditioning, &self.tree)?
                .evaluate(self.tree.root()),
        };
        Ok(Posterior {
            likelihood,
            tree_prior,
            parameter_prior,
        })
    }

    /// One Metropolis–Hastings step. Returns the move and whether it was accepted.
    pub fn step(&mut self) -> (Move, bool) {
        self.steps += 1;
        let index = self.choose();
        let kind = self.operators[index].kind;
        let stored_birth_death = self.birth_death;
        self.stored_tree.clone_from(&self.tree);
        if let Some(likelihood) = &mut self.likelihood {
            likelihood.core.store();
        }
        let proposal = kind.propose(
            &mut self.tree,
            &mut self.birth_death,
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
            let (tree_prior, parameter_prior) = self.priors();
            let likelihood = match &mut self.likelihood {
                _ if !kind.changes_tree() => self.posterior.likelihood,
                Some(l) if tree_prior + parameter_prior > f64::NEG_INFINITY => {
                    l.update(&self.tree, &self.stored_tree)
                }
                Some(_) => f64::NEG_INFINITY,
                None => 0.0,
            };
            let proposed = Posterior {
                likelihood,
                tree_prior,
                parameter_prior,
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
            if let Some(likelihood) = &mut self.likelihood {
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
        Sampler::new(&patterns(), settings, 1).unwrap()
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
                Sampler::new(&patterns(), &bad, 1).is_err(),
                "{kind:?} = {value}"
            );
        }
    }
}
