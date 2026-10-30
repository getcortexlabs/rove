use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

use rove::diagnostics::{ESS_METHOD, Trace, rhat, statistics};
use rove::summary::nexus::{Clade, Reader};
use serde_json::{Value, json};

use super::sha256;

pub(super) const RHAT_METHOD: &str =
    "rank-normalized split R-hat, including folded ranks; average ties, discard odd middle draw";

pub(super) fn summarize(
    directories: &[PathBuf],
    burnin: usize,
    constraints: &[Vec<String>],
    dating: Option<&Value>,
    cancel: &dyn Fn() -> bool,
    validate_records: bool,
) -> Result<Value, Box<dyn Error>> {
    let mut traces = Vec::new();
    let mut sources = Vec::new();
    let mut cells = 0;
    for directory in directories {
        let path = directory.join("trace.log");
        if fs::metadata(&path)?.len() > 128 * 1024 * 1024 {
            return Err("trace exceeds 128 MiB".into());
        }
        let bytes = fs::read(&path)?;
        let trace = Trace::read(&bytes, cancel)?;
        if validate_records {
            let (_, status) = super::diagnose::source_record(&path, &trace, &bytes)?;
            if !matches!(status.as_str(), "completed" | "cancelled") {
                return Err("comparison needs finished or cancelled runs".into());
            }
        }
        cells += trace.states.len() * trace.columns.len();
        if cells > 32_000_000 {
            return Err("comparison exceeds 32 million numeric cells".into());
        }
        sources.push(json!({"directory": directory, "trace_sha256": sha256::hex(&bytes), "trees_sha256": sha256::file(&directory.join("trees.nexus"))?, "samples": trace.states.len()}));
        traces.push(trace);
    }
    let count = traces
        .iter()
        .map(|t| t.states.len())
        .min()
        .ok_or("no chains")?;
    let discarded = count * burnin / 100;
    let retained = count - discarded;
    let first = &traces[0];
    if traces.iter().any(|t| {
        t.states[..count] != first.states[..count]
            || t.columns
                .iter()
                .map(|c| &c.name)
                .ne(first.columns.iter().map(|c| &c.name))
    }) {
        return Err("chains must have matching sample states and trace columns".into());
    }
    let mut columns = Vec::new();
    for (index, column) in first.columns.iter().enumerate() {
        let values: Vec<_> = traces
            .iter()
            .map(|t| &t.columns[index].values[discarded..count])
            .collect();
        let per_chain = values
            .iter()
            .map(|v| statistics(v, cancel))
            .collect::<Result<Vec<_>, _>>()?;
        let diagnostic = if retained >= 4 && traces.len() >= 2 {
            rhat(&values)?
        } else {
            None
        };
        let reason = match diagnostic {
            Some(x) if !x.is_finite() => Some("zero_within_chain_variance"),
            Some(_) => None,
            None if retained < 4 => Some("too_short"),
            None if traces.len() < 2 => Some("one_chain"),
            None => Some("constant"),
        };
        let ess: Vec<_> = per_chain.iter().map(|s| s.ess).collect();
        columns.push(json!({"name": column.name, "rhat": diagnostic.filter(|x| x.is_finite()),
            "rhat_reason": reason, "ess": ess, "means": per_chain.iter().map(|s| s.mean).collect::<Vec<_>>(),
            "ess_reasons": per_chain.iter().map(|s| s.ess_reason).collect::<Vec<_>>(),
            "lag_limit_reached": per_chain.iter().map(|s| s.lag_limit_reached).collect::<Vec<_>>() }));
    }
    let mut taxa = Vec::new();
    let mut counts: BTreeMap<Clade, Vec<usize>> = BTreeMap::new();
    for (chain, directory) in directories.iter().enumerate() {
        let mut reader = Reader::new(BufReader::new(File::open(directory.join("trees.nexus"))?))?;
        // At a synchronized sample boundary the writer is flushed but the
        // NEXUS block is still open. Read precisely the common trace prefix.
        for sample in 0..count {
            if sample % 256 == 0 && cancel() {
                return Err("comparison cancelled".into());
            }
            let tree = reader
                .next_tree()?
                .ok_or("fewer trees than trace samples")?;
            if tree.state != Some(first.states[sample]) {
                return Err("tree and trace states disagree".into());
            }
            if sample == 0 {
                if chain == 0 {
                    taxa = reader.taxa.clone();
                } else if taxa != reader.taxa {
                    return Err("tree taxa differ across chains".into());
                }
            }
            if sample < discarded {
                continue;
            }
            for (node, mask) in tree.clades().into_iter().enumerate() {
                if node == tree.root || tree.nodes[node].tip.is_some() {
                    continue;
                }
                counts
                    .entry(mask)
                    .or_insert_with(|| vec![0; directories.len()])[chain] += 1;
            }
            if counts.len() > 250_000 {
                return Err("comparison exceeds 250,000 distinct clades".into());
            }
        }
    }
    let mut clades: Vec<_> = counts.into_iter().map(|(mask, counts)| {
        let members: Vec<_> = taxa.iter().enumerate().filter(|(i, _)| mask[i/64] & (1 << (i%64)) != 0).map(|(_, name)| name).collect();
        let constrained = constraints.iter().any(|c| c.len() == members.len() && members.iter().all(|n| c.iter().any(|s| s.replace('_', " ") == ***n)));
        let frequencies: Vec<_> = counts.iter().map(|&n| n as f64 / retained as f64).collect();
        let min = frequencies.iter().copied().fold(f64::INFINITY, f64::min);
        let max = frequencies.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        json!({"taxa": members, "frequencies": frequencies, "difference": max-min, "constrained": constrained,
            "constant_in_every_chain": counts.iter().all(|&n| n == 0 || n == retained)})
    }).collect();
    clades.sort_by(|a, b| {
        b["difference"]
            .as_f64()
            .unwrap()
            .total_cmp(&a["difference"].as_f64().unwrap())
            .then_with(|| a["taxa"].to_string().cmp(&b["taxa"].to_string()))
    });
    if let Some(config) = dating.filter(|d| !d.is_null()) {
        // Exclude only parameters fixed by the model, never a sampled parameter
        // that happens to be constant. Verify the full trace against that value.
        for (i, column) in columns.iter_mut().enumerate() {
            let name = first.columns[i].name.as_str();
            let expected = if name == "clock_rate" && config.get("clock_prior_mean").is_none() {
                config["clock_rate"].as_f64()
            } else if name == "dating_parameter_prior" && config.get("clock_prior_mean").is_none() {
                let d = super::dating::parse(config, &taxa)?;
                Some(
                    -d.ages
                        .iter()
                        .filter(|a| a.minimum < a.maximum)
                        .map(|a| (a.maximum - a.minimum).ln())
                        .sum::<f64>(),
                )
            } else if let Some(tip) = name
                .strip_prefix("tip_age_")
                .and_then(|s| s.parse::<usize>().ok())
            {
                let age = taxa.get(tip.wrapping_sub(1)).map(|t| &config["ages"][t]);
                age.and_then(|a| {
                    a.as_f64().or_else(|| {
                        let v = a.as_array()?;
                        (v.len() == 2 && v[0] == v[1])
                            .then(|| v[0].as_f64())
                            .flatten()
                    })
                })
            } else {
                None
            };
            if let Some(expected) = expected {
                if traces.iter().any(|t| {
                    t.columns[i]
                        .values
                        .iter()
                        .any(|v| (*v - expected).abs() > 1e-12 * expected.abs().max(1.0))
                }) {
                    return Err(format!(
                        "fixed parameter {name} differs from the recorded dating model"
                    )
                    .into());
                }
                column["fixed_by_model"] = json!(true);
            }
        }
    }
    let max_difference = clades
        .iter()
        .filter(|c| c["constrained"] == false)
        .filter_map(|c| c["difference"].as_f64())
        .fold(0.0, f64::max);
    Ok(
        json!({"schema_version": 1, "chain_count": directories.len(), "burnin_percent": burnin,
        "common_samples": count, "discarded_per_chain": discarded, "retained_per_chain": retained,
        "last_state": first.states[count-1], "sources": sources, "statistics": columns,
        "rhat_method": RHAT_METHOD, "ess_method": ESS_METHOD,
        "clades": clades, "max_unconstrained_clade_difference": max_difference,
        "topology_convergence_assessed": false,
        "notes": ["ESS is computed separately within each chain, not pooled bulk or tail ESS.",
            "R-hat and ESS thresholds are diagnostics, not proof of convergence.",
            "Clade frequencies compare the same retained prefix; constrained clades are not independent recovery.",
            "Agreement on constant clades cannot establish exploration of tree space."]}),
    )
}

pub(super) fn thresholds_met(summary: &Value, min_ess: f64, max_rhat: f64) -> bool {
    // At least 1,000 retained draws per chain prevents a tiny accidental match
    // from terminating a run. Unavailable diagnostics block it, except for
    // parameters independently verified as fixed by the model.
    summary["retained_per_chain"].as_u64().unwrap_or(0) >= 1000
        && summary["statistics"].as_array().is_some_and(|columns| {
            !columns.is_empty()
                && columns.iter().any(|s| s["fixed_by_model"] != true)
                && columns.iter().all(|s| {
                    if s["fixed_by_model"] == true {
                        return true;
                    }
                    s["rhat"].as_f64().is_some_and(|r| r <= max_rhat)
                        && s["ess"].as_array().is_some_and(|values| {
                            values
                                .iter()
                                .all(|v| v.as_f64().is_some_and(|v| v >= min_ess))
                        })
                        && s["lag_limit_reached"]
                            .as_array()
                            .is_some_and(|values| values.iter().all(|v| v == false))
                })
        })
}

pub(super) fn run(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let mut directories = Vec::new();
    let mut burnin = 10;
    let mut args = arguments.iter();
    while let Some(arg) = args.next() {
        if arg == "--burnin-percent" {
            burnin = args.next().ok_or("missing burn-in")?.parse()?;
        } else if arg.starts_with("--") {
            return Err(format!("unknown comparison option {arg}").into());
        } else {
            directories.push(PathBuf::from(arg));
        }
    }
    if !(2..=16).contains(&directories.len()) || burnin >= 100 {
        return Err(
            "compare needs 2 to 16 run directories and burn-in from 0 to 99 percent".into(),
        );
    }
    let records = directories
        .iter()
        .map(|d| -> Result<Value, Box<dyn Error>> {
            Ok(serde_json::from_slice(&fs::read(d.join("run.json"))?)?)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let first = &records[0];
    let mut seeds = std::collections::HashSet::new();
    for record in &records {
        if record["input"]["sha256"] != first["input"]["sha256"]
            || record["model"] != first["model"]
            || !seeds.insert(record["settings"]["seed"].as_u64().ok_or("missing seed")?)
        {
            return Err("comparison requires the same input and model, with distinct seeds".into());
        }
    }
    let constraints: Vec<Vec<String>> =
        serde_json::from_value(first["model"]["monophyly"].clone())?;
    let summary = summarize(
        &directories,
        burnin,
        &constraints,
        first["model"].get("dating"),
        &|| false,
        true,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopping_excludes_verified_fixed_parameters_but_not_stuck_parameters() {
        let mut summary = json!({"retained_per_chain":1000,"statistics":[
            {"rhat":1.005,"ess":[300,300],"lag_limit_reached":[false,false]},
            {"fixed_by_model":true,"rhat":null,"ess":[null,null],"lag_limit_reached":[false,false]}
        ]});
        assert!(thresholds_met(&summary, 200.0, 1.01));
        summary["statistics"][1]["fixed_by_model"] = json!(false);
        assert!(!thresholds_met(&summary, 200.0, 1.01));
        summary["statistics"][1]["fixed_by_model"] = json!(true);
        summary["statistics"][0]["fixed_by_model"] = json!(true);
        assert!(!thresholds_met(&summary, 200.0, 1.01));
    }

    #[test]
    fn stopping_requires_every_chain_and_statistic_to_be_assessable() {
        let mut summary = json!({"retained_per_chain": 1000,
            "statistics": [{"rhat": 1.005, "ess": [250.0, 220.0], "lag_limit_reached": [false, false]}]});
        assert!(thresholds_met(&summary, 200.0, 1.01));
        summary["statistics"][0]["ess"][1] = Value::Null;
        assert!(!thresholds_met(&summary, 200.0, 1.01));
        summary["statistics"][0]["ess"][1] = json!(220.0);
        summary["statistics"][0]["rhat"] = Value::Null;
        assert!(!thresholds_met(&summary, 200.0, 1.01));
        summary["statistics"][0]["rhat"] = json!(1.2);
        assert!(!thresholds_met(&summary, 200.0, 1.01));
        summary["statistics"][0]["rhat"] = json!(1.005);
        summary["statistics"][0]["lag_limit_reached"][1] = json!(true);
        assert!(!thresholds_met(&summary, 200.0, 1.01));
        summary["statistics"][0]["lag_limit_reached"][1] = json!(false);
        summary["retained_per_chain"] = json!(999);
        assert!(!thresholds_met(&summary, 200.0, 1.01));
    }
}
