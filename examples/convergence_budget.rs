//! Diagnose a declared prefix of four independent chains without exporting taxa.
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

use rove::diagnostics::{Trace, rhat, statistics};
use rove::summary::nexus::{Clade, Reader, Tree};
use serde_json::{Value, json};

const CHAINS: usize = 4;
const BATCHES: usize = 20;

fn diagnostic(values: &[&[f64]]) -> Result<Value, Box<dyn Error>> {
    let stats = values
        .iter()
        .map(|v| statistics(v, &|| false))
        .collect::<Result<Vec<_>, _>>()?;
    let r = rhat(values)?;
    let passed = r.is_some_and(|x| x.is_finite() && x <= 1.01)
        && stats
            .iter()
            .all(|s| s.ess.is_some_and(|x| x >= 100.0) && !s.lag_limit_reached);
    Ok(
        json!({"rhat": r.filter(|x| x.is_finite()), "passed": passed,
        "means": stats.iter().map(|s| s.mean).collect::<Vec<_>>(),
        "ess": stats.iter().map(|s| s.ess).collect::<Vec<_>>(),
        "ess_reasons": stats.iter().map(|s| s.ess_reason).collect::<Vec<_>>(),
        "lag_limit_reached": stats.iter().map(|s| s.lag_limit_reached).collect::<Vec<_>>() }),
    )
}

fn clades(tree: &Tree) -> Vec<Clade> {
    tree.clades()
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i != tree.root && tree.nodes[*i].tip.is_none())
        .map(|(_, c)| c)
        .collect()
}

fn batch_mcse(counts: &[usize; BATCHES], batch_size: usize) -> f64 {
    let frequencies = counts.map(|n| n as f64 / batch_size as f64);
    let mean = frequencies.iter().sum::<f64>() / BATCHES as f64;
    (frequencies.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / ((BATCHES - 1) * BATCHES) as f64)
        .sqrt()
}

#[derive(Default)]
struct Counts {
    total: [usize; CHAINS],
    batches: [[usize; BATCHES]; CHAINS],
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: convergence_budget <four-chain-directory> <through-step>".into());
    }
    let directory = PathBuf::from(&args[0]);
    let through: u64 = args[1].parse()?;
    let mut traces = Vec::new();
    let mut records = Vec::new();
    for chain in 1..=CHAINS {
        let path = directory.join(format!("chain-{chain:02}"));
        let record: Value = serde_json::from_slice(&fs::read(path.join("run.json"))?)?;
        if record["status"] != "completed"
            || record["completed_steps"].as_u64().unwrap_or(0) < through
        {
            return Err("expected completed chains covering the requested prefix".into());
        }
        traces.push(Trace::read(&fs::read(path.join("trace.log"))?, &|| false)?);
        records.push(record);
    }
    let seeds: HashSet<_> = records
        .iter()
        .map(|r| r["settings"]["seed"].as_u64())
        .collect();
    if seeds.len() != CHAINS
        || seeds.contains(&None)
        || records.iter().any(|r| {
            r["input"]["sha256"] != records[0]["input"]["sha256"]
                || r["model"] != records[0]["model"]
        })
        || records[0]["model"]["monophyly"] != json!([])
    {
        return Err("expected distinct seeds, identical targets and no constraints".into());
    }
    let count = traces[0].states.partition_point(|&s| s <= through);
    if count == 0 || traces[0].states[count - 1] != through {
        return Err("prefix must end at a recorded state".into());
    }
    for (trace, record) in traces.iter().zip(&records) {
        let interval = record["settings"]["sample_every"]
            .as_u64()
            .ok_or("missing sample interval")?;
        if interval == 0
            || trace.states.len() as u64
                != record["completed_steps"].as_u64().unwrap() / interval + 1
            || trace
                .states
                .iter()
                .enumerate()
                .any(|(i, &s)| s != i as u64 * interval)
            || trace.states[..count] != traces[0].states[..count]
            || trace
                .columns
                .iter()
                .map(|c| &c.name)
                .ne(traces[0].columns.iter().map(|c| &c.name))
        {
            return Err("incomplete or mismatched traces".into());
        }
    }
    let discarded = count / 10;
    let retained = count - discarded;
    if retained < 1000 {
        return Err("need at least 1,000 retained draws per chain".into());
    }
    let mut columns = Vec::new();
    for (i, column) in traces[0].columns.iter().enumerate() {
        let values: Vec<_> = traces
            .iter()
            .map(|t| &t.columns[i].values[discarded..count])
            .collect();
        let mut d = diagnostic(&values)?;
        d["name"] = json!(column.name);
        columns.push(d);
    }
    let mut taxa = Vec::new();
    let mut reference = HashSet::new();
    let mut ids: HashMap<Clade, usize> = HashMap::new();
    let mut counts: Vec<Counts> = Vec::new();
    let mut rf = Vec::new();
    let mut unique = Vec::new();
    let batch_size = retained / BATCHES;
    for (chain, trace) in traces.iter().enumerate() {
        let path = directory.join(format!("chain-{:02}/trees.nexus", chain + 1));
        let mut reader = Reader::new(BufReader::new(File::open(path)?))?;
        let mut distances = Vec::new();
        let mut topologies = HashSet::new();
        for sample in 0..count {
            let tree = reader
                .next_tree()?
                .ok_or("tree file ends before trace prefix")?;
            if tree.state != Some(trace.states[sample]) {
                return Err("tree and trace states differ".into());
            }
            if sample == 0 {
                if chain == 0 {
                    taxa.clone_from(&reader.taxa);
                    reference.extend(clades(&tree));
                } else if taxa != reader.taxa {
                    return Err("tree taxa differ across chains".into());
                }
            }
            if sample < discarded {
                continue;
            }
            let masks = clades(&tree);
            let shared = masks.iter().filter(|m| reference.contains(*m)).count();
            distances.push((masks.len() + reference.len() - 2 * shared) as f64);
            let mut topology = Vec::with_capacity(masks.len());
            for mask in masks {
                let id = *ids.entry(mask).or_insert_with(|| {
                    counts.push(Counts::default());
                    counts.len() - 1
                });
                counts[id].total[chain] += 1;
                let batch = (sample - discarded) / batch_size;
                if batch < BATCHES {
                    counts[id].batches[chain][batch] += 1;
                }
                topology.push(id);
            }
            if counts.len() > 250_000 {
                return Err("more than 250,000 distinct clades".into());
            }
            topology.sort_unstable();
            topologies.insert(topology);
        }
        unique.push(topologies.len());
        rf.push(distances);
    }
    let rf = diagnostic(&rf.iter().map(Vec::as_slice).collect::<Vec<_>>())?;
    let mut relevant = 0;
    let mut spread_failures = 0;
    let mut uncertainty_failures = 0;
    let mut maximum_spread: f64 = 0.0;
    let mut maximum_mcse = [0.0_f64; CHAINS];
    let mut constant = 0;
    for c in &counts {
        if c.total.iter().sum::<usize>() as f64 / ((CHAINS * retained) as f64) < 0.01 {
            continue;
        }
        relevant += 1;
        let frequencies = c.total.map(|n| n as f64 / retained as f64);
        let spread = frequencies
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
            - frequencies.iter().copied().fold(f64::INFINITY, f64::min);
        maximum_spread = maximum_spread.max(spread);
        spread_failures += usize::from(spread > 0.05);
        let errors = c.batches.map(|b| batch_mcse(&b, batch_size));
        for (maximum, error) in maximum_mcse.iter_mut().zip(errors) {
            *maximum = maximum.max(error);
        }
        uncertainty_failures += usize::from(errors.iter().any(|&e| e > 0.0125));
        constant += usize::from(c.total.iter().all(|&n| n == 0 || n == retained));
    }
    let scalar_pass = columns.iter().all(|c| c["passed"] == true);
    let clade_pass = relevant > 0 && spread_failures == 0 && uncertainty_failures == 0;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "through_step": through, "burnin_percent": 10, "retained_per_chain": retained,
            "input_sha256": records[0]["input"]["sha256"],
            "seeds": records.iter().map(|r| &r["settings"]["seed"]).collect::<Vec<_>>(),
            "scalar_pass": scalar_pass, "clade_pass": clade_pass,
            "all_pass": scalar_pass && clade_pass && rf["passed"] == true,
            "statistics": columns, "rf": rf, "unique_topologies": unique,
            "rf_reference": "initial tree, state zero of chain 1; fixed before posterior sampling, not a true tree",
            "clades": {"distinct": counts.len(), "pooled_frequency_at_least_0_01": relevant,
                "max_frequency_spread": maximum_spread, "spread_failures": spread_failures,
                "max_batch_mcse_per_chain": maximum_mcse, "mcse_failures": uncertainty_failures,
                "constant_in_every_chain": constant, "batches": BATCHES, "batch_size": batch_size,
                "omitted_tail_draws_for_batches": retained - batch_size * BATCHES},
            "notes": ["ESS/time is descriptive when diagnostics fail.",
                "Constant clades do not establish exploration of tree space.",
                "Batch MCSE assumes sufficiently long batches; these diagnostics do not prove convergence."]
        }))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_and_separated_chains_do_not_pass() {
        let a = vec![1.0; 1000];
        let b = vec![2.0; 1000];
        assert_eq!(diagnostic(&[&a, &a, &a, &a]).unwrap()["passed"], false);
        assert_eq!(diagnostic(&[&a, &a, &b, &b]).unwrap()["passed"], false);
    }

    #[test]
    fn batch_error_detects_a_shift_hidden_by_the_overall_frequency() {
        let flat = [50; BATCHES];
        let mut shifted = [0; BATCHES];
        shifted[BATCHES / 2..].fill(100);
        assert_eq!(batch_mcse(&flat, 100), 0.0);
        assert!((batch_mcse(&shifted, 100) - (0.25_f64 / 19.0).sqrt()).abs() < 1e-12);
    }
}
