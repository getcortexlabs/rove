//! Morphological observations and lossless compression within model partitions.

use std::collections::HashMap;
use std::fmt;
use std::ops::Range;

/// State indices refer to the matrix's declared alphabet, including unobserved states.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Cell {
    State(usize),
    Uncertain(Vec<usize>),
    Polymorphic(Vec<usize>),
    Missing,
    Gap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Matrix {
    pub(crate) taxa: Vec<String>,
    pub(crate) symbols: Vec<char>,
    pub(crate) rows: Vec<Vec<Cell>>,
    pub(crate) character_labels: Vec<Option<String>>,
    pub(crate) state_labels: Vec<Vec<String>>,
}

/// Gaps can mean inapplicability, so treating them as missing requires a choice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GapTreatment {
    Reject,
    Missing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompressionError {
    InvalidPartitions,
    Gap { taxon: String, site: usize },
    Polymorphism { taxon: String, site: usize },
}

impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPartitions => write!(
                f,
                "partitions must be nonempty, consecutive ranges covering every character once"
            ),
            Self::Gap { taxon, site } => write!(
                f,
                "gap at taxon {taxon:?}, character {}: choose an explicit gap treatment",
                site + 1
            ),
            Self::Polymorphism { taxon, site } => write!(
                f,
                "polymorphism at taxon {taxon:?}, character {}: the single-state Mk observation model does not support polymorphism",
                site + 1
            ),
        }
    }
}

impl std::error::Error for CompressionError {}

/// Observed patterns from one partition, numbered by first occurrence.
#[derive(Clone, Debug, PartialEq)]
pub struct Patterns {
    sites: Range<usize>,
    symbols: Vec<char>,
    weights: Vec<usize>,
    site_to_pattern: Vec<usize>,
    tip_partials: Vec<Vec<f64>>,
}

impl Matrix {
    pub fn taxa(&self) -> &[String] {
        &self.taxa
    }

    pub fn symbols(&self) -> &[char] {
        &self.symbols
    }

    pub fn character_count(&self) -> usize {
        self.character_labels.len()
    }

    /// Taxon-major observations, in the order returned by `taxa`.
    pub fn rows(&self) -> &[Vec<Cell>] {
        &self.rows
    }

    pub fn character_labels(&self) -> &[Option<String>] {
        &self.character_labels
    }

    /// Descriptive labels by character, in declared symbol order. May be absent
    /// or shorter than the alphabet; labels never change the model's state count.
    pub fn state_labels(&self) -> &[Vec<String>] {
        &self.state_labels
    }

    /// Compress exact observations within explicit, contiguous model partitions.
    ///
    /// Ranges are zero-based and must cover the matrix in order without overlaps
    /// or omissions. Each partition uses the full declared alphabet. Identical
    /// columns in different partitions remain separate. No ascertainment columns
    /// are inserted, and constant or missing-only observed columns are retained.
    ///
    /// Uncertainty contributes 1 for each allowed state (not normalized
    /// probabilities); missing data contributes 1 for every state. Polymorphism
    /// is preserved on import but rejected here rather than treated as uncertainty.
    pub fn compress(
        &self,
        partitions: &[Range<usize>],
        gaps: GapTreatment,
    ) -> Result<Vec<Patterns>, CompressionError> {
        let mut end = 0;
        for sites in partitions {
            if sites.start != end || sites.is_empty() || sites.end > self.character_count() {
                return Err(CompressionError::InvalidPartitions);
            }
            end = sites.end;
        }
        if end != self.character_count() {
            return Err(CompressionError::InvalidPartitions);
        }

        // Validate before allocating partials so unsupported observations cannot
        // produce a partially usable likelihood input.
        for (taxon, row) in self.taxa.iter().zip(&self.rows) {
            for (site, cell) in row.iter().enumerate() {
                match cell {
                    Cell::Polymorphic(_) => {
                        return Err(CompressionError::Polymorphism {
                            taxon: taxon.clone(),
                            site,
                        });
                    }
                    Cell::Gap if gaps == GapTreatment::Reject => {
                        return Err(CompressionError::Gap {
                            taxon: taxon.clone(),
                            site,
                        });
                    }
                    _ => {}
                }
            }
        }

        Ok(partitions
            .iter()
            .map(|sites| self.compress_partition(sites))
            .collect())
    }

    fn compress_partition(&self, sites: &Range<usize>) -> Patterns {
        let mut indices = HashMap::new();
        let mut representatives = Vec::new();
        let mut weights = Vec::new();
        let mut site_to_pattern = Vec::new();
        for site in sites.clone() {
            let column: Vec<_> = self.rows.iter().map(|row| &row[site]).collect();
            let pattern = *indices.entry(column).or_insert_with(|| {
                representatives.push(site);
                weights.push(0);
                weights.len() - 1
            });
            weights[pattern] += 1;
            site_to_pattern.push(pattern);
        }

        let states = self.symbols.len();
        let tip_partials = self
            .rows
            .iter()
            .map(|row| {
                let mut partials = Vec::new();
                for &site in &representatives {
                    let start = partials.len();
                    partials.resize(start + states, 0.0);
                    match &row[site] {
                        Cell::State(state) => partials[start + state] = 1.0,
                        Cell::Uncertain(allowed) => {
                            for &state in allowed {
                                partials[start + state] = 1.0;
                            }
                        }
                        Cell::Missing | Cell::Gap => partials[start..].fill(1.0),
                        Cell::Polymorphic(_) => unreachable!("observations were validated"),
                    }
                }
                partials
            })
            .collect();
        Patterns {
            sites: sites.clone(),
            symbols: self.symbols.clone(),
            weights,
            site_to_pattern,
            tip_partials,
        }
    }
}

impl Patterns {
    pub fn sites(&self) -> Range<usize> {
        self.sites.clone()
    }

    pub fn symbols(&self) -> &[char] {
        &self.symbols
    }

    pub fn weights(&self) -> &[usize] {
        &self.weights
    }

    /// One pattern index per original site, relative to this partition's start.
    pub fn site_to_pattern(&self) -> &[usize] {
        &self.site_to_pattern
    }

    /// Pattern-major partials for each taxon, ready for `set_tip_partials`.
    pub fn tip_partials(&self) -> &[Vec<f64>] {
        &self.tip_partials
    }
}
