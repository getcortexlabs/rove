use std::error::Error;
use std::fs;
use std::io::Write;

use rove::ascertainment::VariableCharacters;
use rove::data::{Cell, GapTreatment, Matrix};
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::nexus;

use super::partitions::{self, Partitioning};
use super::{Json, is_plain_label, sha256};

const USAGE: &str =
    "usage: rove inspect <matrix.nex> [--gaps reject|missing] [--partitioning observed|declared]";
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

fn report(data: &Matrix, bytes: &[u8], gaps: GapTreatment, partitioning: Partitioning) -> Json {
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
    let compressed = partitioning.compress(data, gaps);
    let description = compressed
        .as_ref()
        .map_or(Json::Null, |p| partitions::describe(p));
    let (patterns, mut sampler_error, variable_error) = match compressed {
        Ok(partitions) => {
            let settings = Settings {
                conditioning: Conditioning::None,
                ..Settings::default()
            };
            let sampler_error = Sampler::new(&partitions, &settings, 1)
                .err()
                .map(|e| e.to_string());
            let variable_error = partitions
                .iter()
                .find_map(|p| VariableCharacters::new(p).err().map(|e| e.to_string()));
            (
                Json::Integer(partitions.iter().map(|p| p.weights().len() as u64).sum()),
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
        ("partitioning", Json::text(partitioning.name())),
        ("partitions", description),
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
    if arguments.is_empty() {
        return Err(USAGE.into());
    }
    let mut gaps = GapTreatment::Reject;
    let mut partitioning = Partitioning::Observed;
    let mut seen = std::collections::HashSet::new();
    for pair in arguments[1..].chunks(2) {
        if pair.len() != 2 || !seen.insert(&pair[0]) {
            return Err(USAGE.into());
        }
        match (pair[0].as_str(), pair[1].as_str()) {
            ("--gaps", "reject") => gaps = GapTreatment::Reject,
            ("--gaps", "missing") => gaps = GapTreatment::Missing,
            ("--partitioning", value) => partitioning = Partitioning::parse(value)?,
            _ => return Err(USAGE.into()),
        }
    }
    let bytes = fs::read(&arguments[0])?;
    let data = nexus::parse(std::str::from_utf8(&bytes)?)?;
    let mut text = String::new();
    report(&data, &bytes, gaps, partitioning).write(&mut text, 0);
    writeln!(std::io::stdout().lock(), "{text}")?;
    Ok(())
}
