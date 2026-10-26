//! A strict subset of morphological NEXUS. See the NEXUS guide for supported syntax.

mod lexer;
mod matrix;

use std::collections::HashSet;
use std::fmt;

use crate::data::Matrix;
use lexer::{Kind, Token};

/// A source location uses one-based lines and columns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for Error {}

/// Read one STANDARD matrix without invoking BEAST or inferring a model alphabet
/// from observed states. Unsupported analysis directives produce an error.
pub fn parse(input: &str) -> Result<Matrix, Error> {
    let tokens = lexer::lex(input)?;
    let mut commands = Commands {
        tokens: &tokens,
        index: 0,
    };
    commands.skip_newlines();
    let header = commands.tokens.get(commands.index).ok_or(Error {
        line: 1,
        column: 1,
        message: "expected #NEXUS".into(),
    })?;
    if !header.is("#NEXUS") {
        return Err(header.error("expected #NEXUS"));
    }
    commands.index += 1;
    let mut taxa = None;
    let mut matrix = None;
    while let Some(command) = commands.next()? {
        let start = words(command);
        if start.len() != 2 || !start[0].is("BEGIN") {
            return Err(start[0].error("expected BEGIN followed by a block name"));
        }
        let block = start[1];
        if block.is("TAXA") {
            if taxa.is_some() || matrix.is_some() {
                return Err(block.error("only one TAXA block, before the matrix, is supported"));
            }
            taxa = Some(parse_taxa(&mut commands, block)?);
        } else if block.is("DATA") || block.is("CHARACTERS") {
            if matrix.is_some() {
                return Err(block.error("only one DATA or CHARACTERS block is supported"));
            }
            matrix = Some(parse_data(&mut commands, block, taxa.as_deref())?);
        } else if block.is("TREES") {
            loop {
                let command = commands.required(block)?;
                if is_end(&words(command))? {
                    break;
                }
            }
        } else {
            return Err(block.error(format!(
                "unsupported block {}; model directives and partitions cannot be ignored",
                block.text
            )));
        }
    }
    matrix.ok_or_else(|| header.error("no DATA or CHARACTERS matrix found"))
}

struct Commands<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Commands<'a> {
    fn skip_newlines(&mut self) {
        while self
            .tokens
            .get(self.index)
            .is_some_and(|t| t.kind == Kind::Newline)
        {
            self.index += 1;
        }
    }

    fn next(&mut self) -> Result<Option<&'a [Token]>, Error> {
        self.skip_newlines();
        let start = self.index;
        let Some(token) = self.tokens.get(start) else {
            return Ok(None);
        };
        while let Some(t) = self.tokens.get(self.index) {
            self.index += 1;
            if t.punct(';') {
                if self.index == start + 1 {
                    return Err(token.error("empty command"));
                }
                return Ok(Some(&self.tokens[start..self.index - 1]));
            }
        }
        Err(token.error("unterminated command; expected ';'"))
    }

    fn required(&mut self, block: &Token) -> Result<&'a [Token], Error> {
        self.next()?
            .ok_or_else(|| block.error("unterminated block; expected END;"))
    }
}

fn words(tokens: &[Token]) -> Vec<&Token> {
    tokens.iter().filter(|t| t.kind != Kind::Newline).collect()
}

fn is_end(words: &[&Token]) -> Result<bool, Error> {
    let end = words[0].is("END") || words[0].is("ENDBLOCK");
    if end && words.len() != 1 {
        return Err(words[0].error("unexpected arguments after END"));
    }
    Ok(end)
}

fn unique_names(tokens: &[&Token]) -> Result<Vec<String>, Error> {
    let mut seen = HashSet::new();
    tokens
        .iter()
        .map(|token| {
            let name = token.label()?;
            if !seen.insert(name.to_ascii_lowercase()) {
                return Err(token.error(format!("duplicate taxon {name:?}")));
            }
            Ok(name)
        })
        .collect()
}

#[derive(Default)]
struct Dimensions {
    taxa: Option<usize>,
    characters: Option<usize>,
    new_taxa: bool,
}

fn dimensions(tokens: &[&Token]) -> Result<Dimensions, Error> {
    let mut result = Dimensions::default();
    let mut i = 1;
    while i < tokens.len() {
        let key = tokens[i];
        i += 1;
        if key.is("NEWTAXA") && !result.new_taxa {
            result.new_taxa = true;
            continue;
        }
        let slot = if key.is("NTAX") {
            &mut result.taxa
        } else if key.is("NCHAR") {
            &mut result.characters
        } else {
            return Err(key.error("unsupported DIMENSIONS option"));
        };
        if slot.is_some() {
            return Err(key.error("duplicate dimension"));
        }
        let value = assignment(tokens, &mut i, key)?;
        let size = value
            .text
            .parse::<usize>()
            .ok()
            .filter(|&n| n > 0)
            .ok_or_else(|| value.error("dimension must be a positive integer"))?;
        *slot = Some(size);
    }
    Ok(result)
}

fn assignment<'a>(tokens: &[&'a Token], i: &mut usize, key: &Token) -> Result<&'a Token, Error> {
    if !tokens.get(*i).is_some_and(|t| t.punct('=')) {
        return Err(key.error("expected '=' and a value"));
    }
    let value = tokens
        .get(*i + 1)
        .ok_or_else(|| key.error("missing value after '='"))?;
    *i += 2;
    Ok(value)
}

fn parse_taxa(commands: &mut Commands<'_>, block: &Token) -> Result<Vec<String>, Error> {
    let mut count = None;
    let mut labels = None;
    loop {
        let command = words(commands.required(block)?);
        let head = command[0];
        if is_end(&command)? {
            break;
        }
        if head.is("DIMENSIONS") && count.is_none() && labels.is_none() {
            let dimensions = dimensions(&command)?;
            if dimensions.characters.is_some() || dimensions.new_taxa {
                return Err(head.error("TAXA dimensions support only NTAX"));
            }
            count = dimensions.taxa;
            if count.is_none() {
                return Err(head.error("TAXA requires NTAX"));
            }
        } else if head.is("TAXLABELS") && labels.is_none() && count.is_some() {
            labels = Some(unique_names(&command[1..])?);
        } else {
            return Err(head.error("unsupported, repeated or misplaced TAXA command"));
        }
    }
    let labels = labels.ok_or_else(|| block.error("TAXA requires DIMENSIONS and TAXLABELS"))?;
    if Some(labels.len()) != count {
        return Err(block.error("TAXLABELS count does not match NTAX"));
    }
    Ok(labels)
}

struct Format {
    symbols: Vec<char>,
    missing: char,
    gap: Option<char>,
    respect_case: bool,
    interleave: bool,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            symbols: vec!['0', '1'],
            missing: '?',
            gap: None,
            respect_case: false,
            interleave: false,
        }
    }
}

impl Format {
    fn equal(&self, a: char, b: char) -> bool {
        a == b || (!self.respect_case && a.eq_ignore_ascii_case(&b))
    }

    fn state(&self, symbol: char) -> Option<usize> {
        self.symbols.iter().position(|&s| self.equal(s, symbol))
    }
}

fn valid_symbol(c: char) -> bool {
    c.is_ascii_graphic() && !"()[]{}/\\,;:=*'\"`<>~^".contains(c)
}

fn format(tokens: &[&Token]) -> Result<Format, Error> {
    let mut result = Format::default();
    let mut seen = HashSet::new();
    let mut i = 1;
    while i < tokens.len() {
        let key = tokens[i];
        i += 1;
        if !seen.insert(key.text.to_ascii_uppercase()) {
            return Err(key.error("duplicate FORMAT option"));
        }
        if key.is("DATATYPE") {
            if !assignment(tokens, &mut i, key)?.is("STANDARD") {
                return Err(key.error("only DATATYPE=STANDARD is supported"));
            }
        } else if key.is("RESPECTCASE") {
            result.respect_case = true;
        } else if key.is("LABELS") || key.is("NOTOKENS") {
        } else if key.is("INTERLEAVE") {
            result.interleave = true;
            if tokens.get(i).is_some_and(|t| t.punct('=')) {
                let value = assignment(tokens, &mut i, key)?;
                if value.is("YES") {
                    result.interleave = true;
                } else if value.is("NO") {
                    result.interleave = false;
                } else {
                    return Err(value.error("INTERLEAVE expects YES or NO"));
                }
            }
        } else if key.is("SYMBOLS") {
            let value = assignment(tokens, &mut i, key)?;
            if value.kind != Kind::String {
                return Err(value.error("SYMBOLS requires double quotes"));
            }
            result.symbols = value.text.chars().filter(|c| !c.is_whitespace()).collect();
        } else if key.is("MISSING") || key.is("GAP") {
            let value = assignment(tokens, &mut i, key)?;
            let mut chars = value.text.chars();
            let symbol = chars
                .next()
                .filter(|&c| valid_symbol(c))
                .filter(|_| chars.next().is_none())
                .ok_or_else(|| value.error("expected one ASCII missing or gap symbol"))?;
            if key.is("MISSING") {
                result.missing = symbol;
            } else {
                result.gap = Some(symbol);
            }
        } else {
            return Err(key.error(format!("unsupported FORMAT option {}", key.text)));
        }
    }
    if result.symbols.len() < 2 || result.symbols.iter().any(|&s| !valid_symbol(s)) {
        return Err(tokens[0]
            .error("SYMBOLS must contain at least two valid single-character ASCII states"));
    }
    for (i, &state) in result.symbols.iter().enumerate() {
        if result.symbols[..i].iter().any(|&s| result.equal(s, state))
            || result.equal(state, result.missing)
            || result.gap.is_some_and(|g| result.equal(state, g))
        {
            return Err(tokens[0].error("state, missing and gap symbols must be distinct"));
        }
    }
    if result.gap.is_some_and(|g| result.equal(g, result.missing)) {
        return Err(tokens[0].error("missing and gap symbols must be distinct"));
    }
    Ok(result)
}

fn parse_data(
    commands: &mut Commands<'_>,
    block: &Token,
    external_taxa: Option<&[String]>,
) -> Result<Matrix, Error> {
    let mut dims = None;
    let mut fmt = Format::default();
    let mut seen = HashSet::new();
    let mut taxa = external_taxa.map(<[String]>::to_vec);
    let mut matrix = None;
    let mut label_commands = Vec::new();
    loop {
        let raw = commands.required(block)?;
        let command = words(raw);
        let head = command[0];
        if is_end(&command)? {
            break;
        }
        if matrix.is_some() {
            return Err(head.error("commands after MATRIX are unsupported"));
        }
        if !seen.insert(head.text.to_ascii_uppercase()) {
            return Err(head.error("duplicate command"));
        }
        if head.is("DIMENSIONS") {
            dims = Some(dimensions(&command)?);
        } else if head.is("FORMAT") {
            fmt = format(&command)?;
        } else if head.is("TAXLABELS") {
            if taxa.is_some() {
                return Err(head.error("taxa have already been declared"));
            }
            taxa = Some(unique_names(&command[1..])?);
        } else if head.is("CHARLABELS") || head.is("STATELABELS") || head.is("CHARSTATELABELS") {
            label_commands.push(command);
        } else if head.is("MATRIX") {
            let dimensions = dims
                .as_ref()
                .ok_or_else(|| head.error("DIMENSIONS must precede MATRIX"))?;
            let nchar = dimensions
                .characters
                .ok_or_else(|| head.error("NCHAR is required"))?;
            let ntax = dimensions
                .taxa
                .or_else(|| taxa.as_ref().map(Vec::len))
                .filter(|&count| count > 0)
                .ok_or_else(|| {
                    head.error("a positive NTAX or a nonempty taxon declaration is required")
                })?;
            if taxa.as_ref().is_some_and(|t| t.len() != ntax) {
                return Err(head.error("TAXLABELS count does not match NTAX"));
            }
            if dimensions.new_taxa && dimensions.taxa.is_none() {
                return Err(head.error("NEWTAXA requires NTAX"));
            }
            let (names, rows) = matrix::read(&raw[1..], ntax, nchar, &fmt, head)?;
            let (names, rows) = if let Some(labels) = taxa.take() {
                let mut ordered = Vec::new();
                for name in &labels {
                    let index = names
                        .iter()
                        .position(|s| s.eq_ignore_ascii_case(name))
                        .ok_or_else(|| {
                            head.error(format!("declared taxon {name:?} is absent from MATRIX"))
                        })?;
                    ordered.push(rows[index].clone());
                }
                (labels, ordered)
            } else {
                (names, rows)
            };
            let mut data = Matrix {
                taxa: names,
                rows,
                symbols: fmt.symbols.clone(),
                character_labels: vec![None; nchar],
                state_labels: vec![Vec::new(); nchar],
            };
            apply_labels(&mut data, &label_commands)?;
            matrix = Some(data);
        } else {
            return Err(head.error(format!("unsupported matrix command {}", head.text)));
        }
    }
    matrix.ok_or_else(|| block.error("missing MATRIX command"))
}

fn apply_labels(data: &mut Matrix, commands: &[Vec<&Token>]) -> Result<(), Error> {
    let mut character_seen = HashSet::new();
    let mut states_seen = HashSet::new();
    for command in commands {
        let head = command[0];
        if head.is("CHARLABELS") {
            if command.len() - 1 != data.character_count() {
                return Err(head.error("CHARLABELS count must match NCHAR"));
            }
            for (site, token) in command[1..].iter().enumerate() {
                if !character_seen.insert(site) {
                    return Err(token.error("character label already defined"));
                }
                data.character_labels[site] = Some(token.label()?);
            }
            continue;
        }
        for entry in command[1..].split(|t| t.punct(',')) {
            let first = entry
                .first()
                .ok_or_else(|| head.error("empty state-label entry"))?;
            let site = first
                .text
                .parse::<usize>()
                .ok()
                .filter(|&n| n > 0 && n <= data.character_count())
                .ok_or_else(|| first.error("character label index is outside 1..NCHAR"))?
                - 1;
            let mut i = 1;
            if head.is("CHARSTATELABELS") {
                if entry.get(i).is_some_and(|t| !t.punct('/')) {
                    if !character_seen.insert(site) {
                        return Err(first.error("character label already defined"));
                    }
                    data.character_labels[site] = Some(entry[i].label()?);
                    i += 1;
                }
                if i == entry.len() {
                    continue;
                }
                if !entry[i].punct('/') {
                    return Err(entry[i].error("expected '/' before state labels"));
                }
                i += 1;
            }
            if !states_seen.insert(site) {
                return Err(first.error("state labels already defined"));
            }
            let labels = entry[i..]
                .iter()
                .map(|t| t.label())
                .collect::<Result<Vec<_>, _>>()?;
            if labels.len() > data.symbols.len() {
                return Err(first.error("more state labels than declared symbols"));
            }
            data.state_labels[site] = labels;
        }
    }
    Ok(())
}
