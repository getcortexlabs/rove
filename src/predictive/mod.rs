//! Posterior predictive character simulation and paired discrepancies.
//!
//! Model alphabets are frozen from the fitted data. Ascertainment applies to
//! complete latent characters before the original missing-data mask is restored.
mod statistics;
pub use statistics::{CharacterStatistics, Statistics, TailCount};

use std::collections::BTreeMap;
use std::fmt::{self, Write};

use crate::data::{Cell, CharacterSet, GapTreatment, Matrix, Patterns};
use crate::mcmc::Conditioning;
use crate::model::{CharacterOrdering, Mk, Ordered};
use crate::random::Random;
use crate::rates::RateVariation;
use crate::tree::Tree;
use crate::{LikelihoodCore, Scaling};

#[derive(Debug)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

pub struct Design {
    original: Matrix,
    sets: Vec<CharacterSet>,
    gaps: GapTreatment,
    conditioning: Conditioning,
    morphology_rate: f64,
}

impl Design {
    pub fn new(
        matrix: &Matrix,
        partitions: &[Patterns],
        gaps: GapTreatment,
        conditioning: Conditioning,
    ) -> Result<Self> {
        if matrix.data_type() == crate::data::DataType::Dna && conditioning != Conditioning::None {
            return Err(Error(
                "DNA JC69 requires unconditioned aligned sites".into(),
            ));
        }
        if matrix.taxa().len() < 2
            || matrix.taxa().len().saturating_mul(matrix.character_count()) > 8_000_000
        {
            return Err(Error(
                "predictive matrices require at least two taxa and at most eight million cells"
                    .into(),
            ));
        }
        // An uncertainty set is an observation, not a stochastic observation
        // mechanism. Copying it into a replicate would condition on the data twice.
        if matrix
            .rows()
            .iter()
            .flatten()
            .any(|c| matches!(c, Cell::Uncertain(_) | Cell::Polymorphic(_)))
        {
            return Err(Error("posterior prediction needs single states or missing cells; uncertainty and polymorphism need an explicit observation model".into()));
        }
        let mut groups: BTreeMap<Vec<char>, Vec<usize>> = BTreeMap::new();
        for p in partitions {
            for (&site, symbols) in p.sites().iter().zip(p.state_symbols()) {
                groups.entry(symbols.clone()).or_default().push(site);
            }
        }
        let sets = groups
            .into_iter()
            .map(|(symbols, mut sites)| {
                sites.sort_unstable();
                CharacterSet { sites, symbols }
            })
            .collect();
        let design = Self {
            original: matrix.clone(),
            sets,
            gaps,
            conditioning,
            morphology_rate: 1.0,
        };
        for p in design.compress(matrix)? {
            match conditioning.for_data(p.data_type()) {
                Conditioning::None => {}
                Conditioning::Variable => {
                    crate::ascertainment::VariableCharacters::new(&p)
                        .map_err(|e| Error(e.to_string()))?;
                }
                Conditioning::Informative => {
                    crate::ascertainment::validate_informative(&p)
                        .map_err(|e| Error(e.to_string()))?;
                }
            }
        }
        Ok(design)
    }

    pub fn set_morphology_rate(&mut self, rate: f64) -> Result<()> {
        if !rate.is_finite()
            || rate <= 0.0
            || (self.original.data_type() != crate::data::DataType::Mixed && rate != 1.0)
        {
            return Err(Error(
                "relative morphology rate requires mixed input and a positive finite value".into(),
            ));
        }
        self.morphology_rate = rate;
        Ok(())
    }

    fn rates_for(&self, rates: &RateVariation, data_type: crate::data::DataType) -> RateVariation {
        let mut result = rates.clone();
        if self.original.data_type() == crate::data::DataType::Mixed
            && data_type == crate::data::DataType::Standard
        {
            result.mean = Some(rates.mean.unwrap_or(1.0) * self.morphology_rate);
        }
        result
    }

    pub fn original(&self) -> &Matrix {
        &self.original
    }
    pub fn character_sets(&self) -> &[CharacterSet] {
        &self.sets
    }

    pub fn compress(&self, matrix: &Matrix) -> Result<Vec<Patterns>> {
        if matrix.blocks != self.original.blocks
            || matrix.data_type() != self.original.data_type()
            || matrix.taxa() != self.original.taxa()
            || matrix.symbols() != self.original.symbols()
            || matrix.ordering() != self.original.ordering()
            || matrix.character_count() != self.original.character_count()
        {
            return Err(Error(
                "predictive matrix does not match the fitted character design".into(),
            ));
        }
        if matrix
            .rows()
            .iter()
            .flatten()
            .any(|c| matches!(c, Cell::Uncertain(_) | Cell::Polymorphic(_)))
        {
            return Err(Error(
                "predictive matrices require single states or missing cells".into(),
            ));
        }
        matrix
            .compress_sets(&self.sets, self.gaps)
            .map_err(|e| Error(e.to_string()))
    }

    /// Rejection sampling redraws the rate category on every attempt. Holding a
    /// category fixed while rejecting changes the conditioned mixture weights.
    pub fn simulate(
        &self,
        tree: &Tree,
        rates: &RateVariation,
        seed: u64,
        max_attempts: usize,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Matrix> {
        validate(tree, rates, self.original.taxa().len())?;
        if max_attempts == 0 {
            return Err(Error("simulation attempt limit must be positive".into()));
        }
        let mut result = self.original.clone();
        let mut random = Random::new(seed);
        let mut states = vec![0; tree.nodes()];
        let order = tree.postorder();
        for partition in self.compress(&self.original)? {
            let k = partition.symbols().len();
            let rates = self.rates_for(rates, partition.data_type());
            let matrices = transitions(tree, &rates, k, partition.ordering())?;
            let mut counts = vec![0; k];
            for (&site, alphabet) in partition.sites().iter().zip(partition.state_symbols()) {
                let symbols: Vec<_> = alphabet
                    .iter()
                    .map(|s| self.original.symbols().iter().position(|c| c == s).unwrap())
                    .collect();
                let mut accepted = false;
                for attempt in 0..max_attempts {
                    if attempt % 128 == 0 && cancel() {
                        return Err(Error("posterior prediction cancelled".into()));
                    }
                    let category = random.below(rates.category_rates().len());
                    states[tree.root()] = random.below(k);
                    for &node in order.iter().rev() {
                        if let Some(parent) = tree.parent(node) {
                            let offset =
                                (category * tree.nodes() + node) * k * k + states[parent] * k;
                            states[node] = draw(&matrices[offset..offset + k], &mut random);
                        }
                    }
                    counts.fill(0);
                    for &state in &states[..tree.tips()] {
                        counts[state] += 1;
                    }
                    accepted = match self.conditioning.for_data(partition.data_type()) {
                        Conditioning::None => true,
                        Conditioning::Variable => counts.iter().filter(|&&n| n > 0).count() > 1,
                        Conditioning::Informative => {
                            counts.iter().filter(|&&n| n >= 2).count() >= 2
                        }
                    };
                    if accepted {
                        break;
                    }
                }
                if !accepted {
                    return Err(Error(format!(
                        "character {} exceeded {max_attempts} simulation attempts; ascertainment is too rare at this state",
                        site + 1
                    )));
                }
                for (tip, row) in result.rows.iter_mut().enumerate() {
                    if matches!(row[site], Cell::State(_)) {
                        row[site] = Cell::State(symbols[states[tip]]);
                    }
                }
            }
        }
        // Exercise the same parser and frozen-alphabet compression used for
        // exported replicates; losing a state must never shrink the model.
        let parsed =
            crate::alignment::parse(&to_input(&result)).map_err(|e| Error(e.to_string()))?;
        self.compress(&parsed)?;
        Ok(parsed)
    }

    pub fn statistics(
        &self,
        matrix: &Matrix,
        tree: &Tree,
        rates: &RateVariation,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Statistics> {
        validate(tree, rates, matrix.taxa().len())?;
        let parts = self.compress(matrix)?;
        let mut characters = vec![CharacterStatistics::default(); matrix.character_count()];
        for p in parts {
            let rates = self.rates_for(rates, p.data_type());
            if cancel() {
                return Err(Error("posterior prediction cancelled".into()));
            }
            let k = p.symbols().len();
            let categories = rates.category_rates().len();
            let patterns = p.weights().len();
            if tree
                .nodes()
                .saturating_mul(patterns.next_multiple_of(8))
                .saturating_mul(k)
                .saturating_mul(categories)
                > 8_000_000
            {
                return Err(Error(
                    "predictive likelihood workspace exceeds its memory budget".into(),
                ));
            }
            let mut core = match p.ordering() {
                CharacterOrdering::Unordered => LikelihoodCore::mk(
                    &Mk::uniform(k),
                    patterns,
                    categories,
                    tree.nodes(),
                    Scaling::Always,
                ),
                CharacterOrdering::Ordered => {
                    LikelihoodCore::ordered(k, patterns, categories, tree.nodes(), Scaling::Always)
                }
            };
            for (tip, partials) in p.tip_partials().iter().enumerate() {
                core.set_tip_partials(tip, partials);
            }
            for node in tree.postorder() {
                let distance = tree.branch_length(node) * rates.branch_rate(node, tree.root());
                for (c, &rate) in rates.category_rates().iter().enumerate() {
                    core.set_node_distance(
                        node,
                        c,
                        if rate == 0.0 { 0.0 } else { distance * rate },
                    );
                }
                if let Some((a, b)) = tree.children(node) {
                    core.calculate_partials(a, b, node);
                }
            }
            core.integrate_partials(tree.root(), rates.category_weights());
            let log_likelihoods = core.calculate_log_likelihoods(&vec![1.0 / k as f64; k]);
            let mut scores = Vec::with_capacity(patterns);
            for (pattern, &ll) in log_likelihoods.iter().enumerate() {
                if cancel() {
                    return Err(Error("posterior prediction cancelled".into()));
                }
                if !ll.is_finite() {
                    return Err(Error("nonfinite predictive character likelihood".into()));
                }
                let mut score = statistics::character(&p, pattern, tree);
                score.surprise = -ll;
                scores.push(score);
            }
            for (&site, &pattern) in p.sites().iter().zip(p.site_to_pattern()) {
                characters[site] = scores[pattern];
            }
        }
        Ok(Statistics { characters })
    }
}

fn validate(tree: &Tree, rates: &RateVariation, tips: usize) -> Result<()> {
    tree.validate().map_err(|e| Error(e.to_string()))?;
    if rates.mean.is_some_and(|x| !x.is_finite() || x <= 0.0) {
        return Err(Error(
            "global clock rate must be positive and finite".into(),
        ));
    }
    if tips != tree.tips()
        || rates
            .clock
            .as_ref()
            .is_some_and(|c| c.rates().len() != tree.nodes() - 1)
    {
        return Err(Error(
            "tree, branch rates and matrix dimensions disagree".into(),
        ));
    }
    Ok(())
}

fn transitions(
    tree: &Tree,
    rates: &RateVariation,
    k: usize,
    ordering: CharacterOrdering,
) -> Result<Vec<f64>> {
    let size = tree
        .nodes()
        .saturating_mul(rates.category_rates().len())
        .saturating_mul(k * k);
    if size > 16_000_000 {
        return Err(Error(
            "simulation transition matrices exceed 128 MiB".into(),
        ));
    }
    let mut matrices = vec![0.0; size];
    let mk = Mk::uniform(k);
    let mut ordered = Ordered::new(k);
    for (c, &rate) in rates.category_rates().iter().enumerate() {
        for node in 0..tree.nodes() {
            let distance = tree.branch_length(node) * rates.branch_rate(node, tree.root());
            let distance = if rate == 0.0 { 0.0 } else { distance * rate };
            let offset = (c * tree.nodes() + node) * k * k;
            let out = &mut matrices[offset..offset + k * k];
            match ordering {
                CharacterOrdering::Unordered => mk.transition_matrix(distance, out),
                CharacterOrdering::Ordered => ordered.transition_matrix(distance, out),
            }
        }
    }
    Ok(matrices)
}

fn draw(probabilities: &[f64], random: &mut Random) -> usize {
    let mut remaining = random.uniform();
    for (i, &p) in probabilities.iter().enumerate() {
        if remaining < p {
            return i;
        }
        remaining -= p;
    }
    probabilities.len() - 1
}

/// Compatibility entry point for `to_input`: NEXUS for homogeneous data,
/// a typed manifest for mixed data. Alphabets, labels, ordering and missing masks
/// are retained. Frozen model alphabets live in the predictive report and must
/// be reused when fitting exported replicates.
pub fn to_nexus(matrix: &Matrix) -> String {
    to_input(matrix)
}

/// NEXUS for homogeneous data, a self-contained typed manifest for mixed data.
pub fn to_input(matrix: &Matrix) -> String {
    if matrix.data_type() == crate::data::DataType::Mixed {
        let meta = matrix.mixed_metadata().expect("validated mixed matrix");
        let mut components = matrix.component_matrices();
        for (part, source) in components.iter_mut().zip(&meta.sources) {
            let kept: Vec<_> = part
                .taxa
                .iter()
                .enumerate()
                .filter(|(_, t)| !source.missing_taxa.contains(t))
                .map(|(i, _)| i)
                .collect();
            part.rows = kept.iter().map(|i| part.rows[*i].clone()).collect();
            part.taxa = kept.iter().map(|i| part.taxa[*i].clone()).collect();
        }
        return crate::mixed::manifest(
            ("morphology.nex", &to_input(&components[0])),
            ("dna.nex", &to_input(&components[1])),
            &meta.missing_taxa_policy,
        )
        .expect("validated mixed matrix serialization");
    }
    fn quote(s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }
    let mut out = format!(
        "#NEXUS\nBEGIN DATA;\nDIMENSIONS NTAX={} NCHAR={};\nFORMAT DATATYPE={} SYMBOLS=\"{}\" MISSING=? GAP=-;\n",
        matrix.taxa().len(),
        matrix.character_count(),
        if matrix.data_type() == crate::data::DataType::Dna {
            "DNA"
        } else {
            "STANDARD"
        },
        matrix.symbols().iter().collect::<String>()
    );
    let mut labels = Vec::new();
    for (site, label) in matrix.character_labels().iter().enumerate() {
        if let Some(label) = label {
            labels.push(format!("{} {}", site + 1, quote(label)));
        }
    }
    if !labels.is_empty() {
        writeln!(out, "CHARSTATELABELS {};", labels.join(", ")).unwrap();
    }
    // STATELABELS is independent of character names, which may be absent.
    let labels: Vec<_> = matrix
        .state_labels()
        .iter()
        .enumerate()
        .filter(|(_, v)| !v.is_empty())
        .map(|(i, v)| {
            format!(
                "{} {}",
                i + 1,
                v.iter().map(|s| quote(s)).collect::<Vec<_>>().join(" ")
            )
        })
        .collect();
    if !labels.is_empty() {
        writeln!(out, "STATELABELS {};", labels.join(", ")).unwrap();
    }
    out.push_str("MATRIX\n");
    for (taxon, row) in matrix.taxa().iter().zip(matrix.rows()) {
        write!(out, "{} ", quote(taxon)).unwrap();
        for cell in row {
            match cell {
                Cell::State(s) => out.push(matrix.symbols()[*s]),
                Cell::Missing => out.push('?'),
                Cell::Gap => out.push('-'),
                Cell::Uncertain(states) | Cell::Polymorphic(states) => {
                    let uncertain = matches!(cell, Cell::Uncertain(_));
                    out.push(if uncertain { '{' } else { '(' });
                    for &s in states {
                        out.push(matrix.symbols()[s]);
                    }
                    out.push(if uncertain { '}' } else { ')' });
                }
            }
        }
        out.push('\n');
    }
    out.push_str(";\nEND;\n");
    let ordered: Vec<_> = matrix
        .ordering()
        .iter()
        .enumerate()
        .filter(|(_, o)| **o == CharacterOrdering::Ordered)
        .map(|(i, _)| (i + 1).to_string())
        .collect();
    if !ordered.is_empty() {
        writeln!(
            out,
            "BEGIN ASSUMPTIONS;\nTYPESET * prediction = ORD: {};\nEND;",
            ordered.join(" ")
        )
        .unwrap();
    }
    out
}
