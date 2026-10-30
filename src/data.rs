//! Morphological observations and lossless compression within model partitions.

use std::collections::{BTreeMap, HashMap};
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
    StateCount { site: usize },
    ExcludedState { site: usize, symbol: char },
    Gap { taxon: String, site: usize },
    Polymorphism { taxon: String, site: usize },
}

impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPartitions => write!(
                f,
                "partitions must have at least two distinct declared symbols and nonempty, sorted character sets covering every character once"
            ),
            Self::StateCount { site } => write!(
                f,
                "character {} has fewer than two observed states; specify its possible states explicitly or use declared-alphabet partitioning",
                site + 1
            ),
            Self::ExcludedState { site, symbol } => write!(
                f,
                "character {} contains state {symbol:?}, outside its partition's alphabet",
                site + 1
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
    sites: Vec<usize>,
    symbols: Vec<char>,
    state_symbols: Vec<Vec<char>>,
    weights: Vec<usize>,
    site_to_pattern: Vec<usize>,
    tip_partials: Vec<Vec<f64>>,
}

/// An explicit model alphabet and a sorted list of zero-based character indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterSet {
    pub sites: Vec<usize>,
    pub symbols: Vec<char>,
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

        self.compress_sets(
            &partitions
                .iter()
                .map(|sites| CharacterSet {
                    sites: sites.clone().collect(),
                    symbols: self.symbols.clone(),
                })
                .collect::<Vec<_>>(),
            gaps,
        )
    }

    /// Group characters by distinct observed or explicitly possible states. State labels
    /// are unordered: {0, 2, 3} becomes a three-state model, not a four-state one.
    /// Missing cells never add states; explicit uncertainty sets contribute
    /// their allowed states, following BEAST 2 MM's import convention.
    pub fn partition_by_states(
        &self,
        gaps: GapTreatment,
    ) -> Result<Vec<Patterns>, CompressionError> {
        // Mk needs at least two states, so a binary declared alphabet leaves
        // no state-count ambiguity even when a character is constant or missing.
        if self.symbols.len() == 2 {
            return self.compress(std::slice::from_ref(&(0..self.character_count())), gaps);
        }
        self.validate_observations(gaps)?;
        let mut groups = BTreeMap::<usize, Vec<(usize, Vec<usize>)>>::new();
        for site in 0..self.character_count() {
            let mut seen = vec![false; self.symbols.len()];
            for row in &self.rows {
                match &row[site] {
                    Cell::State(state) => seen[*state] = true,
                    Cell::Uncertain(states) => {
                        for &state in states {
                            seen[state] = true;
                        }
                    }
                    _ => {}
                }
            }
            let states: Vec<_> = (0..seen.len()).filter(|&s| seen[s]).collect();
            if states.len() < 2 {
                return Err(CompressionError::StateCount { site });
            }
            groups.entry(states.len()).or_default().push((site, states));
        }
        Ok(groups
            .values()
            .map(|sites| self.compress_mapped(sites))
            .collect())
    }

    /// Compress explicit, possibly non-contiguous character sets. Their alphabets
    /// include any unobserved possible states. Every character must occur once.
    pub fn compress_sets(
        &self,
        partitions: &[CharacterSet],
        gaps: GapTreatment,
    ) -> Result<Vec<Patterns>, CompressionError> {
        self.validate_observations(gaps)?;
        let mut covered = vec![false; self.character_count()];
        let mut mapped = Vec::new();
        for partition in partitions {
            let states = partition
                .symbols
                .iter()
                .map(|s| self.symbols.iter().position(|v| v == s))
                .collect::<Option<Vec<_>>>()
                .ok_or(CompressionError::InvalidPartitions)?;
            if states.len() < 2
                || states
                    .iter()
                    .enumerate()
                    .any(|(i, s)| states[..i].contains(s))
                || partition.sites.is_empty()
                || partition.sites.windows(2).any(|s| s[0] >= s[1])
            {
                return Err(CompressionError::InvalidPartitions);
            }
            let mut sites = Vec::new();
            for &site in &partition.sites {
                if site >= covered.len() || covered[site] {
                    return Err(CompressionError::InvalidPartitions);
                }
                covered[site] = true;
                for row in &self.rows {
                    let allowed: &[usize] = match &row[site] {
                        Cell::State(state) => std::slice::from_ref(state),
                        Cell::Uncertain(states) => states,
                        _ => &[],
                    };
                    if let Some(&state) = allowed.iter().find(|s| !states.contains(s)) {
                        return Err(CompressionError::ExcludedState {
                            site,
                            symbol: self.symbols[state],
                        });
                    }
                }
                sites.push((site, states.clone()));
            }
            mapped.push(sites);
        }
        if covered.iter().any(|&v| !v) || partitions.is_empty() {
            return Err(CompressionError::InvalidPartitions);
        }
        Ok(mapped
            .iter()
            .map(|sites| self.compress_mapped(sites))
            .collect())
    }

    fn validate_observations(&self, gaps: GapTreatment) -> Result<(), CompressionError> {
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

        Ok(())
    }

    fn compress_mapped(&self, sites: &[(usize, Vec<usize>)]) -> Patterns {
        let mut indices = HashMap::new();
        let mut representatives = Vec::new();
        let mut weights = Vec::new();
        let mut site_to_pattern = Vec::new();
        for (site, states) in sites {
            let column: Vec<_> = self
                .rows
                .iter()
                .map(|row| match &row[*site] {
                    Cell::State(state) => Cell::State(
                        states
                            .iter()
                            .position(|s| s == state)
                            .expect("validated state"),
                    ),
                    Cell::Uncertain(allowed) => {
                        let mut mapped: Vec<_> = allowed
                            .iter()
                            .map(|state| {
                                states
                                    .iter()
                                    .position(|s| s == state)
                                    .expect("validated state")
                            })
                            .collect();
                        mapped.sort_unstable();
                        Cell::Uncertain(mapped)
                    }
                    cell => cell.clone(),
                })
                .collect();
            let pattern = *indices.entry(column.clone()).or_insert_with(|| {
                representatives.push(column);
                weights.push(0);
                weights.len() - 1
            });
            weights[pattern] += 1;
            site_to_pattern.push(pattern);
        }

        let states = sites[0].1.len();
        let tip_partials = self
            .rows
            .iter()
            .enumerate()
            .map(|(taxon, _)| {
                let mut partials = Vec::new();
                for column in &representatives {
                    let start = partials.len();
                    partials.resize(start + states, 0.0);
                    match &column[taxon] {
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
            sites: sites.iter().map(|(site, _)| *site).collect(),
            symbols: sites[0].1.iter().map(|&s| self.symbols[s]).collect(),
            state_symbols: sites
                .iter()
                .map(|(_, states)| states.iter().map(|&s| self.symbols[s]).collect())
                .collect(),
            weights,
            site_to_pattern,
            tip_partials,
        }
    }
}

impl Patterns {
    pub fn sites(&self) -> &[usize] {
        &self.sites
    }

    /// Model labels from the first character. See `state_symbols` for each
    /// character's original labels after automatic recoding.
    pub fn symbols(&self) -> &[char] {
        &self.symbols
    }

    pub fn weights(&self) -> &[usize] {
        &self.weights
    }

    /// Original symbols in model-state order for each entry in `sites`.
    pub fn state_symbols(&self) -> &[Vec<char>] {
        &self.state_symbols
    }

    /// One pattern index per entry in `sites`.
    pub fn site_to_pattern(&self) -> &[usize] {
        &self.site_to_pattern
    }

    /// Pattern-major partials for each taxon, ready for `set_tip_partials`.
    pub fn tip_partials(&self) -> &[Vec<f64>] {
        &self.tip_partials
    }
}

impl AsRef<[Patterns]> for Patterns {
    fn as_ref(&self) -> &[Patterns] {
        std::slice::from_ref(self)
    }
}
