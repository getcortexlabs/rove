use super::{Commands, Error, Kind, Token, is_end, words};
use crate::data::Matrix;
use crate::model::CharacterOrdering;

pub(super) fn parse(
    commands: &mut Commands<'_>,
    block: &Token,
    data: &mut Matrix,
) -> Result<(), Error> {
    let mut default = CharacterOrdering::Unordered;
    let mut options_seen = false;
    let mut typeset = None;
    loop {
        let command = words(commands.required(block)?);
        let head = command[0];
        if is_end(&command)? {
            break;
        }
        if head.is("OPTIONS") && !options_seen {
            if command.len() != 4 || !command[1].is("DEFTYPE") || !command[2].punct('=') {
                return Err(
                    head.error("ASSUMPTIONS OPTIONS supports only DEFTYPE=ORD or DEFTYPE=UNORD")
                );
            }
            default = ordering(command[3])?;
            options_seen = true;
        } else if head.is("TYPESET") && typeset.is_none() {
            typeset = Some(parse_typeset(&command, data.character_count())?);
        } else {
            return Err(head.error("unsupported or repeated ASSUMPTIONS command; only OPTIONS DEFTYPE and one TYPESET are supported"));
        }
    }
    data.ordering.fill(default);
    if let Some(typeset) = typeset {
        for (destination, source) in data.ordering.iter_mut().zip(typeset) {
            if let Some(source) = source {
                *destination = source;
            }
        }
    }
    Ok(())
}

fn ordering(token: &Token) -> Result<CharacterOrdering, Error> {
    if token.is("ORD") {
        Ok(CharacterOrdering::Ordered)
    } else if token.is("UNORD") {
        Ok(CharacterOrdering::Unordered)
    } else {
        Err(token.error("only ORD and UNORD character types are supported"))
    }
}

fn parse_typeset(
    command: &[&Token],
    count: usize,
) -> Result<Vec<Option<CharacterOrdering>>, Error> {
    let head = command[0];
    let mut at = 1;
    if command.get(at).is_some_and(|t| t.punct('*')) {
        at += 1;
    }
    command
        .get(at)
        .ok_or_else(|| head.error("TYPESET requires a name"))?
        .label()?;
    at += 1;
    let mut vector = false;
    if command.get(at).is_some_and(|t| t.punct('(')) {
        let format = command
            .get(at + 1)
            .ok_or_else(|| head.error("missing TYPESET format"))?;
        vector = format.is("VECTOR");
        if (!vector && !format.is("STANDARD")) || !command.get(at + 2).is_some_and(|t| t.punct(')'))
        {
            return Err(format.error("TYPESET format must be STANDARD or VECTOR"));
        }
        at += 3;
    }
    if !command.get(at).is_some_and(|t| t.punct('=')) {
        return Err(head.error("TYPESET requires '=' followed by character types"));
    }
    at += 1;
    if at == command.len() {
        return Err(head.error("empty TYPESET"));
    }
    if vector {
        if command.len() - at != count {
            return Err(head.error("VECTOR TYPESET must specify one type per character"));
        }
        return command[at..]
            .iter()
            .map(|t| ordering(t).map(Some))
            .collect();
    }
    let mut result = vec![None; count];
    loop {
        let token = command
            .get(at)
            .ok_or_else(|| head.error("missing type after comma"))?;
        let kind = ordering(token)?;
        if !command.get(at + 1).is_some_and(|t| t.punct(':')) {
            return Err(token.error("expected ':' after character type"));
        }
        at += 2;
        let start = at;
        while at < command.len() && !command[at].punct(',') {
            at += 1;
        }
        for site in characters(&command[start..at], count, token)? {
            if result[site].replace(kind).is_some() {
                return Err(token.error(format!(
                    "character {} occurs more than once in TYPESET",
                    site + 1
                )));
            }
        }
        if at == command.len() {
            break;
        }
        at += 1;
    }
    Ok(result)
}

fn characters(tokens: &[&Token], count: usize, head: &Token) -> Result<Vec<usize>, Error> {
    if tokens.is_empty() {
        return Err(head.error("character list is empty"));
    }
    // The matrix lexer keeps '-' inside names. Split it only in numeric lists,
    // so both 1-3 and 1 - 3 retain the same meaning without changing taxon labels.
    let mut units = Vec::new();
    for &token in tokens {
        if token.kind == Kind::Word {
            for (i, part) in token.text.split('-').enumerate() {
                if i > 0 {
                    units.push(("-", token));
                }
                if !part.is_empty() {
                    units.push((part, token));
                }
            }
        } else {
            units.push((token.text.as_str(), token));
        }
    }
    let index = |text: &str, token: &Token| -> Result<usize, Error> {
        if token.kind == Kind::Quoted || token.kind == Kind::String {
            return Err(token.error("TYPESET character lists require numeric indices"));
        }
        if text == "." {
            return Ok(count - 1);
        }
        text.parse::<usize>()
            .ok()
            .filter(|&n| n > 0 && n <= count)
            .map(|n| n - 1)
            .ok_or_else(|| {
                token.error(
                    "character index must be between 1 and NCHAR; named sets are not supported",
                )
            })
    };
    let mut result = Vec::new();
    let mut at = 0;
    while at < units.len() {
        let (text, token) = units[at];
        at += 1;
        let all = token.is("ALL");
        let first = if all { 0 } else { index(text, token)? };
        let mut last = if all { count - 1 } else { first };
        if units.get(at).is_some_and(|&(text, _)| text == "-") {
            if all {
                return Err(token.error("ALL cannot be a range endpoint"));
            }
            let &(text, token) = units
                .get(at + 1)
                .ok_or_else(|| token.error("missing range endpoint"))?;
            last = index(text, token)?;
            at += 2;
        }
        if last < first {
            return Err(token.error("character range must be increasing"));
        }
        let mut stride = 1;
        if units.get(at).is_some_and(|&(_, t)| t.punct('\\')) {
            let &(text, token) = units
                .get(at + 1)
                .ok_or_else(|| token.error("missing range stride"))?;
            stride = text
                .parse::<usize>()
                .ok()
                .filter(|&n| n > 0)
                .ok_or_else(|| token.error("range stride must be a positive integer"))?;
            at += 2;
        }
        result.extend((first..=last).step_by(stride));
    }
    Ok(result)
}
