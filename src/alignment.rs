//! Aligned DNA import. FASTA identifiers are the first whitespace-delimited
//! token after `>`; underscores become spaces as in Rove's NEXUS reader.
//! Descriptions do not become taxon names. No alignment,
//! trimming, state recoding or substitution-model inference is performed.
use std::collections::HashSet;

use crate::data::{Cell, DataType, Matrix};
use crate::model::CharacterOrdering;
use crate::nexus::{self, Error};

pub fn is_fasta(input: &str) -> bool {
    input
        .trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with('>')
}

pub fn parse(input: &str) -> Result<Matrix, Error> {
    let input = input.trim_start_matches('\u{feff}');
    if is_fasta(input) {
        fasta(input)
    } else {
        nexus::parse(input)
    }
}

/// IUPAC ambiguity is a set of possible states, not polymorphism or a vector
/// of normalized probabilities. N permits all four states.
pub(crate) fn dna_cell(symbol: char) -> Option<Cell> {
    let states: &[usize] = match symbol.to_ascii_uppercase() {
        'A' => &[0],
        'C' => &[1],
        'G' => &[2],
        'T' => &[3],
        'R' => &[0, 2],
        'Y' => &[1, 3],
        'S' => &[1, 2],
        'W' => &[0, 3],
        'K' => &[2, 3],
        'M' => &[0, 1],
        'B' => &[1, 2, 3],
        'D' => &[0, 2, 3],
        'H' => &[0, 1, 3],
        'V' => &[0, 1, 2],
        'N' => &[0, 1, 2, 3],
        _ => return None,
    };
    Some(if states.len() == 1 {
        Cell::State(states[0])
    } else {
        Cell::Uncertain(states.to_vec())
    })
}

fn fasta(input: &str) -> Result<Matrix, Error> {
    let mut taxa = Vec::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut seen = HashSet::new();
    let mut length = None;
    let error = |line, column, message: String| Error {
        line,
        column,
        message,
    };
    for (index, line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(header) = line.strip_prefix('>') {
            if rows.last().is_some_and(Vec::is_empty) {
                return Err(error(line_number, 1, "FASTA sequence is empty".into()));
            }
            if let Some(row) = rows.last() {
                let expected = *length.get_or_insert(row.len());
                if row.len() != expected {
                    return Err(error(
                        line_number,
                        1,
                        "FASTA sequences must already be aligned to the same length".into(),
                    ));
                }
            }
            let name = header.split_whitespace().next().ok_or_else(|| {
                error(
                    line_number,
                    1,
                    "FASTA needs a nonempty sequence identifier".into(),
                )
            })?;
            // Match Rove's existing NEXUS/BEAST naming convention.
            let name = name.replace('_', " ");
            if name.trim().is_empty() {
                return Err(error(
                    line_number,
                    1,
                    "FASTA identifier is empty after NEXUS name normalization".into(),
                ));
            }
            if !seen.insert(name.to_ascii_lowercase()) {
                return Err(error(
                    line_number,
                    1,
                    format!("duplicate FASTA identifier {name:?}"),
                ));
            }
            taxa.push(name.to_owned());
            rows.push(Vec::new());
        } else {
            let row = rows.last_mut().ok_or_else(|| {
                error(
                    line_number,
                    1,
                    "FASTA sequence appears before its header".into(),
                )
            })?;
            for (column, symbol) in line.chars().enumerate() {
                if symbol.is_ascii_whitespace() {
                    continue;
                }
                row.push(match symbol {
                    '?' => Cell::Missing, '-' => Cell::Gap,
                    _ => dna_cell(symbol).ok_or_else(|| error(line_number, column+1, format!("unsupported DNA symbol {symbol:?}; use A/C/G/T, IUPAC ambiguity, ? or -")))?,
                });
            }
        }
    }
    let nchar = rows.first().map_or(0, Vec::len);
    if nchar == 0 || rows.iter().any(|r| r.len() != nchar) {
        return Err(error(
            input.lines().count().max(1),
            1,
            "FASTA sequences must be nonempty and aligned to the same length".into(),
        ));
    }
    Ok(Matrix {
        data_type: DataType::Dna,
        taxa,
        rows,
        symbols: vec!['A', 'C', 'G', 'T'],
        character_labels: vec![None; nchar],
        state_labels: vec![Vec::new(); nchar],
        ordering: vec![CharacterOrdering::Unordered; nchar],
    })
}
