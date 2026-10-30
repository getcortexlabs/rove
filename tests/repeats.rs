#![cfg(feature = "experimental-repeats")]

use rove::{Backend, LikelihoodCore, Scaling, model::Mk};

#[path = "support/repeat_matrix.rs"]
mod repeat_matrix;

fn compare(a: &mut LikelihoodCore, b: &mut LikelihoodCore, root: usize, model: &Mk) {
    let layout = a.layout();
    let weights = vec![1.0 / layout.categories as f64; layout.categories];
    a.integrate_partials(root, &weights);
    b.integrate_partials(root, &weights);
    for (&x, &y) in a
        .calculate_log_likelihoods(model.frequencies())
        .iter()
        .zip(b.calculate_log_likelihoods(model.frequencies()))
    {
        assert!(
            x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()),
            "{x} != {y}"
        );
    }
    for node in 0..a.node_count() {
        for (&x, &y) in a.node_partials(node).iter().zip(b.node_partials(node)) {
            assert!(
                x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()),
                "node {node}: {x} != {y}"
            );
        }
    }
}

#[test]
fn packed_patterns_match_dense_buffers_through_topology_and_rate_changes() {
    let original = [(0, 1, 6), (2, 3, 7), (4, 5, 8), (6, 7, 9), (9, 8, 10)];
    let rearranged = [(0, 2, 6), (1, 4, 7), (3, 5, 8), (8, 7, 10), (6, 10, 9)];
    for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
        for k in [2, 3, 4, 8] {
            let total = (1..=k).sum::<usize>() as f64;
            let model = Mk::with_frequencies((1..=k).map(|x| x as f64 / total).collect()).unwrap();
            for categories in [1, 4] {
                for scaling in [Scaling::None, Scaling::Always] {
                    let mut packed = LikelihoodCore::mk(&model, 65, categories, 11, scaling);
                    let mut dense = LikelihoodCore::mk(&model, 65, categories, 11, scaling);
                    dense.disable_site_repeats();
                    for core in [&mut packed, &mut dense] {
                        core.set_backend(backend);
                        for tip in 0..6 {
                            let values: Vec<f64> = (0..65)
                                .flat_map(|p| {
                                    let state = (p / 3 + tip / 2) % k;
                                    (0..k).map(move |s| {
                                        f64::from(
                                            (p + tip) % 13 == 0
                                                || s == state
                                                || (p % 11 == 0 && s == (state + 1) % k),
                                        )
                                    })
                                })
                                .collect();
                            core.set_tip_partials(tip, &values);
                        }
                        for node in 0..11 {
                            for c in 0..categories {
                                core.set_node_distance(node, c, 0.01 * (node + c + 1) as f64);
                            }
                        }
                        for &(a, b, p) in &original {
                            core.calculate_partials(a, b, p);
                        }
                    }
                    compare(&mut packed, &mut dense, 10, &model);
                    let rebuilds = packed.repeat_stats().unwrap().index_rebuilds;
                    // A new distance and category rates must leave repeat IDs valid.
                    for core in [&mut packed, &mut dense] {
                        core.store();
                        core.set_node_matrix_for_update(0);
                        for c in 0..categories {
                            core.set_node_distance(0, c, 1e-9 * (c + 1) as f64);
                        }
                        for &(a, b, p) in &original {
                            core.set_node_partials_for_update(p);
                            core.calculate_partials(a, b, p);
                        }
                    }
                    compare(&mut packed, &mut dense, 10, &model);
                    assert_eq!(packed.repeat_stats().unwrap().index_rebuilds, rebuilds);
                    for core in [&mut packed, &mut dense] {
                        core.restore();
                    }
                    compare(&mut packed, &mut dense, 10, &model);
                    for reject in [true, false, true, false] {
                        for core in [&mut packed, &mut dense] {
                            core.store();
                            for &(a, b, p) in &rearranged {
                                core.set_node_partials_for_update(p);
                                core.calculate_partials(a, b, p);
                            }
                        }
                        compare(&mut packed, &mut dense, 9, &model);
                        if reject {
                            for core in [&mut packed, &mut dense] {
                                core.unstore();
                            }
                            compare(&mut packed, &mut dense, 10, &model);
                        }
                        // Rebuild the original tree before the next proposal.
                        for core in [&mut packed, &mut dense] {
                            for &(a, b, p) in &original {
                                core.calculate_partials(a, b, p);
                            }
                        }
                        compare(&mut packed, &mut dense, 10, &model);
                    }
                    assert!(packed.repeat_stats().unwrap().packed_updates > 0);
                }
            }
        }
    }
}

#[test]
fn equal_counts_do_not_hide_different_repeat_classes() {
    let model = Mk::uniform(2);
    let mut packed = LikelihoodCore::mk(&model, 128, 1, 7, Scaling::Always);
    let mut dense = LikelihoodCore::mk(&model, 128, 1, 7, Scaling::Always);
    dense.disable_site_repeats();
    for core in [&mut packed, &mut dense] {
        for tip in 0..4 {
            let states: Vec<_> = (0..128).map(|p| ((p >> (tip % 3)) & 1) as u32).collect();
            core.set_tip_states(tip, &states);
        }
        for node in 0..6 {
            core.set_node_distance(node, 0, 0.1);
        }
        for (a, b, p) in [(0, 1, 4), (2, 3, 5), (4, 5, 6)] {
            core.calculate_partials(a, b, p);
        }
    }
    compare(&mut packed, &mut dense, 6, &model);
    let rebuilds = packed.repeat_stats().unwrap().index_rebuilds;
    for core in [&mut packed, &mut dense] {
        // The cherry still has four classes, but their memberships change.
        let states: Vec<_> = (0..128).map(|p| ((p >> 2) & 1) as u32).collect();
        core.set_tip_states(1, &states);
        core.calculate_partials(0, 1, 4);
        core.calculate_partials(4, 5, 6);
    }
    compare(&mut packed, &mut dense, 6, &model);
    assert!(packed.repeat_stats().unwrap().index_rebuilds > rebuilds);
}

#[test]
fn packed_deep_tree_preserves_cumulative_scaling() {
    let model = Mk::uniform(4);
    let tips = 1024;
    for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
        let mut packed = LikelihoodCore::mk(&model, 129, 4, 2 * tips - 1, Scaling::Always);
        let mut dense = LikelihoodCore::mk(&model, 129, 4, 2 * tips - 1, Scaling::Always);
        dense.disable_site_repeats();
        for core in [&mut packed, &mut dense] {
            core.set_backend(backend);
            for tip in 0..tips {
                let states: Vec<_> = (0..129).map(|p| ((p + tip) % 4) as u32).collect();
                core.set_tip_states(tip, &states);
            }
            for node in 0..2 * tips - 1 {
                for category in 0..4 {
                    core.set_node_distance(node, category, 0.1 * (category + 1) as f64);
                }
            }
            let mut root = 0;
            for tip in 1..tips {
                let parent = tips + tip - 1;
                core.calculate_partials(root, tip, parent);
                root = parent;
            }
        }
        compare(&mut packed, &mut dense, 2 * tips - 2, &model);
        assert!(
            packed
                .pattern_log_likelihoods()
                .iter()
                .all(|v| v.is_finite() && *v < -1000.0)
        );
        assert_eq!(
            packed.repeat_stats().unwrap().packed_updates,
            (tips - 1) as u64
        );
    }
}

#[test]
fn partitioned_sampler_rebuilds_repeat_indices_after_resume() {
    use rove::{
        data::GapTreatment,
        mcmc::{Conditioning, Sampler, Settings},
        nexus,
    };

    let partitions = nexus::parse(&repeat_matrix::matrix())
        .unwrap()
        .partition_by_states(GapTreatment::Reject)
        .unwrap();
    assert_eq!(partitions.len(), 2);
    assert!(partitions.iter().all(|p| p.weights().len() == 128));
    for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
        for conditioning in [Conditioning::None, Conditioning::Variable] {
            let settings = Settings {
                backend,
                conditioning,
                tuning_delay: 100,
                ..Settings::reference()
            };
            let mut original = Sampler::new(&partitions, &settings, 17).unwrap();
            for _ in 0..200 {
                original.step();
            }
            let bytes = original.checkpoint(&settings).unwrap();
            let mut resumed = Sampler::resume(&partitions, &settings, &bytes).unwrap();
            for _ in 0..500 {
                original.step();
                resumed.step();
            }
            assert_eq!(
                original.checkpoint(&settings).unwrap(),
                resumed.checkpoint(&settings).unwrap()
            );
        }
    }
}
