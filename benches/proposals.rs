#[path = "support/allocations.rs"]
mod allocations;

use std::error::Error;
use std::time::Instant;

use rove::mcmc::{Conditioning, Move, Sampler, Settings};
use rove::{data::GapTreatment, nexus, tree::Tree};
use serde_json::json;

#[global_allocator]
static ALLOCATOR: allocations::CountingAllocator = allocations::CountingAllocator;

fn clades(tree: &Tree) -> Vec<u64> {
    assert!(tree.tips() <= 64);
    let mut masks = vec![0; tree.nodes()];
    for node in tree.postorder() {
        masks[node] = match tree.children(node) {
            None => 1 << node,
            Some((a, b)) => masks[a] | masks[b],
        };
    }
    let mut clades = masks[tree.tips()..].to_vec();
    clades.sort_unstable();
    clades
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    let path = args
        .first()
        .ok_or("provide a matrix path, conditioning and seed")?;
    let conditioning = match args.get(1).map(String::as_str) {
        Some("none") => Conditioning::None,
        Some("variable") => Conditioning::Variable,
        _ => return Err("conditioning must be none or variable".into()),
    };
    let seed: u64 = args.get(2).ok_or("provide a seed")?.parse()?;
    let matrix = nexus::parse(&std::fs::read_to_string(path)?)?;
    let patterns = matrix
        .compress(
            std::slice::from_ref(&(0..matrix.character_count())),
            GapTreatment::Reject,
        )?
        .remove(0);
    let mut settings = Settings {
        conditioning,
        monophyly: vec![(0..19).collect()],
        ..Settings::reference()
    };
    for argument in &args[3..] {
        let (name, value) = argument.split_once('=').ok_or("expected move=weight")?;
        let kind = Move::ALL
            .into_iter()
            .find(|m| m.name() == name)
            .ok_or("unknown move")?;
        if let Some(entry) = settings.weights.iter_mut().find(|(m, _)| *m == kind) {
            entry.1 = value.parse()?;
        } else {
            settings.weights.push((kind, value.parse()?));
        }
    }
    let mut chain = Sampler::new(&patterns, &settings, seed)?;
    for _ in 0..2_000_000 {
        chain.step();
    }
    let before = chain.operators().to_vec();
    let count = Move::ALL.len();
    let mut nanoseconds = vec![0u128; count];
    let mut calls = vec![0u64; count];
    let mut changes = vec![0u64; count];
    let mut jumps = vec![[0.0; 3]; count];
    let mut previous_clades = clades(chain.tree());
    for _ in 0..500_000 {
        let previous = [
            chain.posterior().total(),
            chain.rates().clock.as_ref().unwrap().sigma().ln(),
            chain.tree().height(chain.tree().root()).ln(),
        ];
        let start = Instant::now();
        let (kind, accepted) = chain.step();
        let elapsed = start.elapsed().as_nanos();
        let i = Move::ALL.iter().position(|m| *m == kind).unwrap();
        nanoseconds[i] += elapsed;
        calls[i] += 1;
        if accepted {
            let next = [
                chain.posterior().total(),
                chain.rates().clock.as_ref().unwrap().sigma().ln(),
                chain.tree().height(chain.tree().root()).ln(),
            ];
            for k in 0..3 {
                jumps[i][k] += (next[k] - previous[k]).powi(2);
            }
            if kind.changes_topology() {
                let next = clades(chain.tree());
                changes[i] += u64::from(next != previous_clades);
                previous_clades = next;
            }
        }
    }
    let operators = chain.operators().to_vec();
    let mut allocation_calls = vec![0u64; count];
    let mut allocation_counts = vec![[0usize; 4]; count];
    for _ in 0..100_000 {
        let mut kind = Move::NodeHeight;
        let observation = allocations::observe(|| {
            kind = chain.step().0;
            chain.posterior().total()
        });
        let i = Move::ALL.iter().position(|m| *m == kind).unwrap();
        allocation_calls[i] += 1;
        for (total, value) in allocation_counts[i].iter_mut().zip([
            observation.allocations,
            observation.reallocations,
            observation.bytes,
            observation.deallocations,
        ]) {
            *total += value;
        }
    }
    let rows: Vec<_> = operators
        .iter()
        .zip(&before)
        .map(|(after, before)| {
            let i = Move::ALL.iter().position(|m| *m == after.kind).unwrap();
            json!({"move":after.kind.name(),"weight":after.weight,"calls":calls[i],
            "accepted":after.accepted-before.accepted,"rejected":after.rejected-before.rejected,
            "failed":after.failed-before.failed,"tuning":after.tuning,
            "step_nanoseconds":nanoseconds[i] as u64,"topology_changes":changes[i],
            "squared_jumps_posterior_log_sigma_log_height":jumps[i],
            "allocation_calls":allocation_calls[i],"allocations":allocation_counts[i][0],
            "reallocations":allocation_counts[i][1],"allocated_bytes":allocation_counts[i][2],
            "deallocations":allocation_counts[i][3]})
        })
        .collect();
    println!(
        "{}",
        json!({"matrix":path,"seed":seed,"warmup_steps":2_000_000,
        "timed_steps":500_000,"allocation_steps":100_000,"backend":settings.backend.name(),
        "method":"step wall time includes proposal, density, cache handling and adaptation; allocations measured in a separate untimed pass; topology comparisons outside timed regions",
        "operators":rows})
    );
    Ok(())
}
