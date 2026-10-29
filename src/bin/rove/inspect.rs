use std::error::Error;
use std::fs;
use std::io::Write;

use rove::ascertainment::VariableCharacters;
use rove::data::{Cell, GapTreatment, Matrix};
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::nexus;

use super::{Json, is_plain_label, sha256};

const USAGE: &str = "usage: rove inspect <matrix.nex> [--gaps reject|missing]";
const PREVIEW_TAXA: usize = 200;
const PREVIEW_CHARACTERS: usize = 120;

fn cell_text(cell: &Cell, symbols: &[char]) -> String {
    match cell {
        Cell::State(state) => symbols[*state].to_string(),
        Cell::Missing => "?".into(),
        Cell::Gap => "-".into(),
        Cell::Uncertain(states) | Cell::Polymorphic(states) => {
            let states: String = states.iter().map(|&i| symbols[i]).collect();
            if matches!(cell, Cell::Uncertain(_)) {
                format!("{{{states}}}")
            } else {
                format!("({states})")
            }
        }
    }
}

fn report(data: &Matrix, bytes: &[u8], gaps: GapTreatment) -> Json {
    let mut missing = 0;
    let mut gap_count = 0;
    let mut uncertain = 0;
    let mut polymorphic = 0;
    for cell in data.rows().iter().flatten() {
        match cell {
            Cell::Missing => missing += 1,
            Cell::Gap => gap_count += 1,
            Cell::Uncertain(_) => uncertain += 1,
            Cell::Polymorphic(_) => polymorphic += 1,
            Cell::State(_) => {}
        }
    }
    let sites = 0..data.character_count();
    let compressed = data.compress(std::slice::from_ref(&sites), gaps);
    let (patterns, mut sampler_error, variable_error) = match compressed {
        Ok(partitions) => {
            let patterns = &partitions[0];
            let settings = Settings {
                conditioning: Conditioning::None,
                ..Settings::default()
            };
            let sampler_error = Sampler::new(patterns, &settings, 1)
                .err()
                .map(|e| e.to_string());
            let variable_error = VariableCharacters::new(patterns)
                .err()
                .map(|e| e.to_string());
            (
                Json::Integer(patterns.weights().len() as u64),
                sampler_error,
                variable_error,
            )
        }
        Err(error) => (Json::Null, Some(error.to_string()), None),
    };
    if let Some(name) = data.taxa().iter().find(|name| !is_plain_label(name)) {
        sampler_error = Some(format!(
            "taxon label {name:?} cannot be written to the sampled tree format"
        ));
    }
    let optional = |value: Option<String>| value.map_or(Json::Null, Json::text);
    Json::Object(vec![
        ("schema_version", Json::Integer(1)),
        ("input_sha256", Json::text(sha256::hex(bytes))),
        ("taxon_count", Json::Integer(data.taxa().len() as u64)),
        (
            "character_count",
            Json::Integer(data.character_count() as u64),
        ),
        ("pattern_count", patterns),
        ("missing_count", Json::Integer(missing)),
        ("gap_count", Json::Integer(gap_count)),
        ("uncertain_count", Json::Integer(uncertain)),
        ("polymorphic_count", Json::Integer(polymorphic)),
        ("sampler_error", optional(sampler_error)),
        ("variable_error", optional(variable_error)),
        (
            "taxon_names",
            Json::List(data.taxa().iter().map(Json::text).collect()),
        ),
        (
            "symbols",
            Json::List(
                data.symbols()
                    .iter()
                    .map(|s| Json::text(s.to_string()))
                    .collect(),
            ),
        ),
        (
            "taxa",
            Json::List(
                data.taxa()
                    .iter()
                    .take(PREVIEW_TAXA)
                    .map(Json::text)
                    .collect(),
            ),
        ),
        (
            "character_labels",
            Json::List(
                data.character_labels()
                    .iter()
                    .take(PREVIEW_CHARACTERS)
                    .map(|s| optional(s.clone()))
                    .collect(),
            ),
        ),
        (
            "cells",
            Json::List(
                data.rows()
                    .iter()
                    .take(PREVIEW_TAXA)
                    .map(|row| {
                        Json::List(
                            row.iter()
                                .take(PREVIEW_CHARACTERS)
                                .map(|cell| Json::text(cell_text(cell, data.symbols())))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
        ),
    ])
}

pub(super) fn run(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let gaps = match arguments {
        [_] => GapTreatment::Reject,
        [_, flag, value] if flag == "--gaps" && value == "reject" => GapTreatment::Reject,
        [_, flag, value] if flag == "--gaps" && value == "missing" => GapTreatment::Missing,
        _ => return Err(USAGE.into()),
    };
    let bytes = fs::read(&arguments[0])?;
    let data = nexus::parse(std::str::from_utf8(&bytes)?)?;
    let mut text = String::new();
    report(&data, &bytes, gaps).write(&mut text, 0);
    writeln!(std::io::stdout().lock(), "{text}")?;
    Ok(())
}
