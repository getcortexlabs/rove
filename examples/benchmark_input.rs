//! Export the exact state partitions used by Rove and check archived BEAST states.
//! This is benchmark tooling; it is not part of the application interface.
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;

use rove::ascertainment::VariableCharacters;
use rove::data::{GapTreatment, Matrix, Patterns};
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::rates::{GammaRates, RateVariation, RelaxedClock};
use rove::speciation::BirthDeath;
use rove::{Backend, LikelihoodCore, Scaling, nexus, tree::Tree};
use serde_json::{Value, json};

fn export(data: &Matrix, partitions: &[Patterns]) -> Value {
    let parts: Vec<_> = partitions
        .iter()
        .map(|part| {
            let k = part.symbols().len();
            assert!(
                k <= 10,
                "the BEAST benchmark exporter supports up to ten states"
            );
            let mut ambiguities = BTreeSet::new();
            let sequences: Vec<String> = part
                .tip_partials()
                .iter()
                .map(|tip| {
                    part.site_to_pattern()
                        .iter()
                        .map(|&pattern| {
                            let allowed: String = (0..k)
                                .filter(|&s| tip[pattern * k + s] > 0.0)
                                .map(|s| char::from_digit(s as u32, 10).unwrap())
                                .collect();
                            match allowed.len() {
                                n if n == k => "?".to_owned(),
                                1 => allowed,
                                _ => {
                                    ambiguities.insert(allowed.clone());
                                    format!("{{{allowed}}}")
                                }
                            }
                        })
                        .collect()
                })
                .collect();
            json!({"states":k,"sites":part.sites(),"characters":part.sites().len(),
            "patterns":part.weights().len(),"pattern_weights":part.weights(),
            "state_symbols":part.state_symbols(),"sequences":sequences,
            "ambiguities":ambiguities.into_iter().collect::<Vec<_>>().join(" ")})
        })
        .collect();
    json!({"schema_version":1,"taxa":data.taxa(),"characters":data.character_count(),
        "partitioning":"observed","gaps":"missing","partitions":parts})
}

fn number(v: &Value) -> f64 {
    v.as_f64().expect("finite numeric snapshot field")
}
fn index(v: &Value) -> usize {
    v.as_u64().expect("integer snapshot field") as usize
}
fn numbers(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(number).collect()
}

fn check(
    data: &Matrix,
    partitions: &[Patterns],
    state: &Value,
    conditioning: Conditioning,
) -> Result<Value, Box<dyn Error>> {
    assert_eq!(
        partitions.len(),
        state["partitions"].as_array().unwrap().len()
    );
    let nodes = state["nodes"].as_array().unwrap();
    let mut order: Vec<_> = data
        .taxa()
        .iter()
        .map(|taxon| {
            nodes
                .iter()
                .position(|node| node["taxon"].as_str() == Some(taxon.as_str()))
                .unwrap()
        })
        .collect();
    fn visit(node: usize, nodes: &[Value], order: &mut Vec<usize>) {
        let children = nodes[node]["children"].as_array().unwrap();
        if children.is_empty() {
            return;
        }
        for child in children {
            visit(index(child), nodes, order);
        }
        order.push(node);
    }
    visit(index(&state["root"]), nodes, &mut order);
    let mut remap = vec![0; nodes.len()];
    for (new, &old) in order.iter().enumerate() {
        remap[old] = new;
    }
    let children = order
        .iter()
        .map(|&old| {
            let pair = nodes[old]["children"].as_array().unwrap();
            (!pair.is_empty()).then(|| (remap[index(&pair[0])], remap[index(&pair[1])]))
        })
        .collect();
    let heights = order.iter().map(|&i| number(&nodes[i]["height"])).collect();
    let tree = Tree::new(children, heights)?;
    let branch_rates: Vec<_> = order[..order.len() - 1]
        .iter()
        .map(|&i| number(&nodes[i]["branch_rate"]))
        .collect();
    let settings = Settings {
        conditioning,
        initial: BirthDeath {
            birth_difference: number(&state["birth_difference"]),
            relative_death: number(&state["relative_death"]),
        },
        ..Settings::reference()
    };
    let rates = RateVariation {
        gamma: Some(GammaRates::new(number(&state["gamma_shape"]))?),
        clock: Some(RelaxedClock::new(
            number(&state["clock_sigma"]),
            branch_rates.clone(),
        )?),
    };
    let categories = numbers(&state["category_rates"]);
    let category_weights = numbers(&state["category_weights"]);
    let mut results = Vec::new();
    for backend in [Backend::Portable, Backend::NATIVE] {
        let mut frozen = 0.0;
        let mut differences = Vec::new();
        for (part, expected) in partitions
            .iter()
            .zip(state["partitions"].as_array().unwrap())
        {
            assert_eq!(part.symbols().len(), index(&expected["states"]));
            let variable = (conditioning == Conditioning::Variable)
                .then(|| VariableCharacters::new(part).unwrap());
            let tips = variable
                .as_ref()
                .map_or(part.tip_partials(), VariableCharacters::tip_partials);
            let k = part.symbols().len();
            let mut core = LikelihoodCore::mk(
                &rove::model::Mk::uniform(k),
                tips[0].len() / k,
                categories.len(),
                tree.nodes(),
                Scaling::Always,
            );
            core.set_backend(backend);
            for (tip, values) in tips.iter().enumerate() {
                core.set_tip_partials(tip, values);
            }
            for node in tree.postorder() {
                if node != tree.root() {
                    for (c, rate) in categories.iter().enumerate() {
                        core.set_node_distance(
                            node,
                            c,
                            tree.branch_length(node) * branch_rates[node] * rate,
                        );
                    }
                }
                if let Some((a, b)) = tree.children(node) {
                    core.calculate_partials(a, b, node);
                }
            }
            core.integrate_partials(tree.root(), &category_weights);
            let logs = core.calculate_log_likelihoods(&vec![1.0 / k as f64; k]);
            let value = match variable {
                Some(correction) => correction.log_likelihood(logs)?,
                None => logs
                    .iter()
                    .zip(part.weights())
                    .map(|(l, w)| l * *w as f64)
                    .sum(),
            };
            let difference = value - number(&expected["log_likelihood"]);
            assert!(
                difference.abs() < 1e-7,
                "fixed-category likelihood mismatch: {difference}"
            );
            differences.push(difference);
            frozen += value;
        }
        let sampler = Sampler::with_rates(
            partitions,
            &Settings {
                backend,
                ..settings.clone()
            },
            tree.clone(),
            rates.clone(),
            1,
        )?;
        let posterior = sampler.posterior();
        let values = [
            posterior.likelihood,
            posterior.tree_prior,
            posterior.gamma_prior,
            posterior.clock_rate_prior,
            posterior.clock_sigma_prior,
            posterior.prior(),
            posterior.total(),
        ];
        let mut components = serde_json::Map::new();
        for (label, value) in [
            "likelihood",
            "birth_death",
            "gamma_prior",
            "clock_rate_prior",
            "clock_sigma_prior",
            "prior",
            "posterior",
        ]
        .into_iter()
        .zip(values)
        {
            let expected = number(&state["components"][label]);
            let difference = value - expected;
            let tolerance = if label == "likelihood" || label == "posterior" {
                2e-5
            } else {
                1e-8
            };
            assert!(
                difference.abs() < tolerance,
                "{label}: {difference} exceeds {tolerance}"
            );
            components.insert(label.into(), json!({"rove":value,"beast":expected,"difference":difference,"tolerance":tolerance}));
        }
        results.push(
            json!({"backend":backend.name(),"frozen_category_likelihood":frozen,
            "partition_differences":differences,"components":components}),
        );
    }
    Ok(json!({"schema_version":1,"checks":results}))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let mode = args
        .get(1)
        .ok_or("use export <matrix> or check <matrix> <snapshot> <variable|none>")?;
    let data = nexus::parse(&fs::read_to_string(
        args.get(2).ok_or("matrix path required")?,
    )?)?;
    let partitions = data.partition_by_states(GapTreatment::Missing)?;
    let result = match mode.as_str() {
        "export" => export(&data, &partitions),
        "check" => {
            let snapshot: Value = serde_json::from_str(&fs::read_to_string(
                args.get(3).ok_or("snapshot required")?,
            )?)?;
            let conditioning = match args.get(4).map(String::as_str) {
                Some("variable") => Conditioning::Variable,
                Some("none") => Conditioning::None,
                _ => return Err("conditioning must be variable or none".into()),
            };
            check(&data, &partitions, &snapshot, conditioning)?
        }
        _ => return Err("unknown command".into()),
    };
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
