use std::collections::HashSet;

use super::lexer::{Kind, Token};
use super::{Error, Format, words};
use crate::data::Cell;

type Rows = (Vec<String>, Vec<Vec<Cell>>);

pub(super) fn read(
    tokens: &[Token],
    ntax: usize,
    nchar: usize,
    format: &Format,
    head: &Token,
) -> Result<Rows, Error> {
    if format.interleave {
        interleaved(tokens, ntax, nchar, format, head)
    } else {
        sequential(tokens, ntax, nchar, format, head)
    }
}

fn sequential(
    tokens: &[Token],
    ntax: usize,
    nchar: usize,
    format: &Format,
    head: &Token,
) -> Result<Rows, Error> {
    let tokens = words(tokens);
    let mut taxa = Vec::new();
    let mut rows = Vec::new();
    let mut names = HashSet::new();
    let mut i = 0;
    while i < tokens.len() {
        if taxa.len() == ntax {
            return Err(tokens[i].error("extra data after NTAX complete rows"));
        }
        let label = tokens[i];
        let name = label.label()?;
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(label.error(format!("duplicate taxon {name:?}")));
        }
        i += 1;
        let mut row = Vec::new();
        while row.len() < nchar {
            if i == tokens.len() {
                return Err(label.error(format!(
                    "incomplete row for {name:?}: expected {nchar} characters, found {}",
                    row.len()
                )));
            }
            let at = tokens[i];
            let cells = chunk(&tokens, &mut i, format).map_err(|mut e| {
                e.message = format!("taxon {name:?}, character {}: {}", row.len() + 1, e.message);
                e
            })?;
            row.extend(cells);
            if row.len() > nchar {
                return Err(at.error(format!("row for {name:?} exceeds NCHAR={nchar}")));
            }
        }
        taxa.push(name);
        rows.push(row);
    }
    if taxa.len() != ntax {
        return Err(head.error(format!("expected {ntax} taxa, found {}", taxa.len())));
    }
    Ok((taxa, rows))
}

fn interleaved(
    tokens: &[Token],
    ntax: usize,
    nchar: usize,
    format: &Format,
    head: &Token,
) -> Result<Rows, Error> {
    let lines: Vec<_> = tokens
        .split(|t| t.kind == Kind::Newline)
        .filter(|line| !line.is_empty())
        .collect();
    let mut taxa: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut names = HashSet::new();
    for (page, lines) in lines.chunks(ntax).enumerate() {
        if lines.len() != ntax {
            return Err(
                lines[0][0].error("incomplete interleaved page; expected NTAX labeled rows")
            );
        }
        let mut width = None;
        for (taxon, line) in lines.iter().enumerate() {
            let label = &line[0];
            let name = label.label()?;
            if page == 0 {
                if !names.insert(name.to_ascii_lowercase()) {
                    return Err(label.error(format!("duplicate taxon {name:?}")));
                }
                taxa.push(name.clone());
                rows.push(Vec::new());
            } else if !name.eq_ignore_ascii_case(&taxa[taxon]) {
                return Err(label.error(format!(
                    "interleaved pages must repeat taxon labels in order; expected {:?}",
                    taxa[taxon]
                )));
            }
            let tokens = words(&line[1..]);
            let mut i = 0;
            let mut cells = Vec::new();
            while i < tokens.len() {
                cells.extend(chunk(&tokens, &mut i, format)?);
            }
            if cells.is_empty() {
                return Err(label.error("empty interleaved row"));
            }
            if width.is_some_and(|n| n != cells.len()) {
                return Err(label.error("interleaved rows must have equal widths within a page"));
            }
            width = Some(cells.len());
            rows[taxon].extend(cells);
            if rows[taxon].len() > nchar {
                return Err(label.error(format!("row for {name:?} exceeds NCHAR={nchar}")));
            }
        }
    }
    if taxa.len() != ntax || rows.iter().any(|r| r.len() != nchar) {
        return Err(head.error("incomplete interleaved matrix; rows must match NTAX and NCHAR"));
    }
    Ok((taxa, rows))
}

fn chunk(tokens: &[&Token], i: &mut usize, format: &Format) -> Result<Vec<Cell>, Error> {
    let token = tokens[*i];
    *i += 1;
    if token.punct('{') || token.punct('(') {
        let close = if token.punct('{') { '}' } else { ')' };
        let mut states = Vec::new();
        loop {
            let next = tokens
                .get(*i)
                .ok_or_else(|| token.error("unterminated state set"))?;
            *i += 1;
            if next.punct(close) {
                break;
            }
            for cell in symbols(next, format)? {
                let Cell::State(state) = cell else {
                    return Err(next.error("state sets may contain only declared states"));
                };
                states.push(state);
            }
        }
        if states.is_empty() {
            return Err(token.error("empty state set"));
        }
        states.sort_unstable();
        states.dedup();
        let cell = if close == '}' {
            Cell::Uncertain(states)
        } else {
            Cell::Polymorphic(states)
        };
        Ok(vec![cell])
    } else {
        symbols(token, format)
    }
}

fn symbols(token: &Token, format: &Format) -> Result<Vec<Cell>, Error> {
    if !matches!(
        token.kind,
        Kind::Word | Kind::Punctuation('+') | Kind::Punctuation('-')
    ) {
        return Err(token.error("expected a state symbol or a well-formed state set"));
    }
    token
        .text
        .chars()
        .map(|symbol| {
            if format.equal(symbol, format.missing) {
                Ok(Cell::Missing)
            } else if format.gap.is_some_and(|g| format.equal(g, symbol)) {
                Ok(Cell::Gap)
            } else if let Some(state) = format.state(symbol) {
                Ok(Cell::State(state))
            } else {
                Err(token.error(format!("undeclared state symbol {symbol:?}")))
            }
        })
        .collect()
}
