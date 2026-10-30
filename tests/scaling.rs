use std::collections::VecDeque;

use rove::{Backend, LikelihoodCore, Scaling, model::Mk, reference::Problem};
use serde_json::{Value, json};

fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

#[test]
fn power_scaling_matches_unscaled_decimal_pruning() {
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/validation/scaling/exact.json")).unwrap();
    let mut report = Vec::new();
    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let k = case["states"].as_u64().unwrap() as usize;
        let count = k + 3;
        let tips = case["tips"].as_u64().unwrap() as usize;
        let nodes = 2 * tips - 1;
        let rates = numbers(&case["rates"]);
        let weights = numbers(&case["category_weights"]);
        let model = Mk::with_frequencies(numbers(&case["frequencies"])).unwrap();
        let distance = case["distance"].as_f64().unwrap();
        let mut children = vec![None; nodes];
        let mut available: VecDeque<_> = (0..tips).collect();
        for (node, pair) in children.iter_mut().enumerate().skip(tips) {
            let a = available.pop_front().unwrap();
            let b = available.pop_front().unwrap();
            *pair = Some((a, b));
            if case["ladder"].as_bool().unwrap() {
                available.push_front(node);
            } else {
                available.push_back(node);
            }
        }
        let states: Vec<Vec<u32>> = (0..tips)
            .map(|tip| {
                (0..3)
                    .map(|p| {
                        if (tip + 2 * p) % 7 == 0 {
                            k as u32
                        } else {
                            ((tip + p) % k) as u32
                        }
                    })
                    .chain(0..k as u32)
                    .collect()
            })
            .collect();
        let tip_partials: Vec<Vec<f64>> = states
            .iter()
            .map(|row| {
                row.iter()
                    .flat_map(|&s| (0..k).map(move |i| f64::from(s == k as u32 || s == i as u32)))
                    .collect()
            })
            .collect();
        let mut matrix = vec![0.0; rates.len() * k * k];
        for (c, &rate) in rates.iter().enumerate() {
            model.transition_matrix(distance * rate, &mut matrix[c * k * k..(c + 1) * k * k]);
        }
        let expected: Vec<f64> = case["pattern_log_likelihoods"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().parse().unwrap())
            .collect();
        let exact_mk = case["mk"].as_str().unwrap().parse::<f64>().unwrap();
        let exact_variable = case["log_variable"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap();
        let exact_mkv = case["mkv"].as_str().unwrap().parse::<f64>().unwrap();
        let tolerance = if tips > 100 { 2e-10 } else { 2e-13 };
        let mut backend_values = None;
        for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
            let mut core = LikelihoodCore::mk(&model, count, rates.len(), nodes, Scaling::Always);
            core.set_backend(backend);
            for (tip, row) in states.iter().enumerate() {
                core.set_tip_states(tip, row);
            }
            for node in 0..nodes - 1 {
                for (category, &rate) in rates.iter().enumerate() {
                    core.set_node_distance(node, category, distance * rate);
                }
            }
            for (parent, &pair) in children.iter().enumerate() {
                if let Some((a, b)) = pair {
                    core.calculate_partials(a, b, parent);
                }
            }
            core.integrate_partials(nodes - 1, &weights);
            let logs = core.calculate_log_likelihoods(model.frequencies());
            for (&actual, &expected) in logs.iter().zip(&expected) {
                assert!(
                    (actual - expected).abs() < tolerance,
                    "case {index}, {backend:?}: {actual} != {expected}"
                );
            }
            let mk: f64 = logs[..3].iter().sum();
            let constant: f64 = logs[3..].iter().map(|x| x.exp()).sum();
            let variable = (-constant).ln_1p();
            let mkv = mk - 3.0 * variable;
            // Conditioning divides by a tiny variable-site probability on
            // 1e-9 branches; account for that amplification, not just the logs.
            let correction_tolerance = 72.0 * f64::EPSILON / exact_variable.exp();
            assert!((mkv - exact_mkv).abs() < (3.0 * tolerance).max(correction_tolerance));
            let bits: Vec<_> = logs.iter().map(|x| x.to_bits()).collect();
            if let Some(previous) = &backend_values {
                assert_eq!(&bits, previous);
            }
            backend_values = Some(bits);
            report.push(json!({"case":index,"backend":backend.name(), "pattern_logs":logs,
                "mk":mk,"mkv":mkv,"mk_error":mk-exact_mk,"mkv_error":mkv-exact_mkv,
                "maximum_pattern_error":logs.iter().zip(&expected).map(|(a,b)| (a-b).abs()).fold(0.0,f64::max)}));
        }
        if tips < 100 {
            let matrices = vec![matrix; nodes];
            let mut pattern_weights = vec![0.0; count];
            pattern_weights[..3].fill(1.0);
            let reference = Problem {
                children: &children,
                root: nodes - 1,
                tip_partials: &tip_partials,
                matrices: &matrices,
                category_weights: &weights,
                frequencies: model.frequencies(),
                pattern_weights: &pattern_weights,
            }
            .log_likelihood();
            assert!((reference - exact_mk).abs() < 1e-11);
        } else {
            let mut core = LikelihoodCore::mk(&model, count, rates.len(), nodes, Scaling::None);
            for (tip, row) in states.iter().enumerate() {
                core.set_tip_states(tip, row);
            }
            for node in 0..nodes - 1 {
                for (c, &rate) in rates.iter().enumerate() {
                    core.set_node_distance(node, c, distance * rate);
                }
            }
            for (parent, &pair) in children.iter().enumerate() {
                if let Some((a, b)) = pair {
                    core.calculate_partials(a, b, parent);
                }
            }
            core.integrate_partials(nodes - 1, &weights);
            assert!(
                core.calculate_log_likelihoods(model.frequencies())[..3]
                    .iter()
                    .all(|&ll| ll == f64::NEG_INFINITY)
            );
        }
    }
    if let Ok(path) = std::env::var("ROVE_SCALING_REPORT") {
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
}
