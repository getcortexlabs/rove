//! Cost of the informative-event probability alone; not an inference benchmark.
#[path = "support/allocations.rs"]
mod allocations;

use rove::ascertainment::InformativeProbability;
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling};
use std::hint::black_box;
use std::time::Instant;

#[global_allocator]
static ALLOCATOR: allocations::CountingAllocator = allocations::CountingAllocator;

fn main() {
    let mut results = Vec::new();
    for tips in [90usize, 500] {
        for states in [2, 4] {
            let mut active: std::collections::VecDeque<_> = (0..tips).collect();
            let mut children = vec![None; tips];
            let mut parents = vec![None; 2 * tips - 1];
            while active.len() > 1 {
                let a = active.pop_front().unwrap();
                let b = active.pop_front().unwrap();
                let node = children.len();
                parents[a] = Some(node);
                parents[b] = Some(node);
                children.push(Some((a, b)));
                active.push_back(node);
            }
            let root = children.len() - 1;
            let full: Vec<_> = (tips..children.len()).collect();
            let mut path = Vec::new();
            let mut node = 0;
            while let Some(parent) = parents[node] {
                path.push(parent);
                node = parent;
            }
            let mut core =
                LikelihoodCore::mk(&Mk::uniform(states), 1, 4, children.len(), Scaling::Always);
            for node in 0..children.len() {
                for category in 0..4 {
                    core.set_node_distance(node, category, 0.01 * (category + 1) as f64);
                }
            }
            let mut initialized = None;
            let setup = allocations::observe(|| {
                initialized = Some(InformativeProbability::new(states, tips, 4));
                0.0
            });
            let mut correction = initialized.unwrap();
            for &node in &full {
                let (a, b) = children[node].unwrap();
                correction.calculate_partials(a, b, node, &core);
            }
            for (name, nodes) in [("full", &full), ("one_tip", &path)] {
                let mut update = |i: usize| {
                    core.set_node_distance(0, 0, 0.002 + (i % 37) as f64 * 1e-6);
                    correction.store();
                    for &node in nodes {
                        let (a, b) = children[node].unwrap();
                        correction.set_node_partials_for_update(node);
                        correction.calculate_partials(a, b, node, &core);
                    }
                    let value = correction.log_probability(root, &[0.25; 4]).unwrap();
                    correction.restore();
                    black_box(value)
                };
                for i in 0..1000 {
                    update(i);
                }
                let mut samples = Vec::new();
                for _ in 0..9 {
                    let start = Instant::now();
                    for i in 0..1000 {
                        update(i);
                    }
                    samples.push(start.elapsed().as_nanos() as f64 / 1000.0);
                }
                let counts = allocations::observe(|| {
                    for i in 0..1000 {
                        update(i);
                    }
                    0.0
                });
                assert_eq!(
                    counts.allocations + counts.reallocations + counts.deallocations,
                    0
                );
                results.push(serde_json::json!({
                    "tips": tips, "states": states, "categories": 4, "update": name,
                    "updated_nodes": nodes.len(), "setup_allocated_bytes": setup.bytes,
                    "allocations_per_1000_updates": counts.allocations,
                    "nanoseconds_per_update": samples,
                }));
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
}
