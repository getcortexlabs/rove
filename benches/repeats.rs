#[path = "support/allocations.rs"]
mod allocations;
#[path = "support/timing.rs"]
mod timing;
#[path = "support/workload.rs"]
#[allow(dead_code)]
mod workload;

use std::time::Duration;

use rove::{
    Backend,
    data::{Cell, GapTreatment},
    nexus,
    random::Random,
    reference::Problem,
};
use serde_json::{Value, json};
use workload::{Engine, Path, Workload};

#[global_allocator]
static ALLOCATOR: allocations::CountingAllocator = allocations::CountingAllocator;

fn cases() -> Vec<Workload> {
    let mut cases = workload::suite();
    for (base, name, noise, missing) in [
        ("reference_20x50", "reference_noise10", 10, 0),
        ("reference_20x50", "reference_noise30", 30, 0),
        ("reference_20x50", "reference_missing20", 0, 20),
        ("morphology_90x250", "morphology_noise10", 10, 0),
        ("morphology_90x250", "morphology_noise30", 30, 0),
        ("morphology_90x250", "morphology_missing20", 0, 20),
    ] {
        let mut case = workload::suite()
            .into_iter()
            .find(|c| c.name == base)
            .unwrap();
        let mut random = Random::new(2026103017);
        let mut text = format!(
            "#NEXUS BEGIN DATA; DIMENSIONS NTAX={} NCHAR={}; FORMAT SYMBOLS=\"{}\" MISSING=?; MATRIX\n",
            case.data.taxa().len(),
            case.data.character_count(),
            case.data.symbols().iter().collect::<String>()
        );
        for (taxon, row) in case.data.taxa().iter().zip(case.data.rows()) {
            text.push_str(taxon);
            text.push(' ');
            for cell in row {
                let symbol = if random.below(100) < missing {
                    '?'
                } else {
                    match cell {
                        Cell::State(s) => {
                            let changed = if random.below(100) < noise {
                                (s + 1) % case.model.states()
                            } else {
                                *s
                            };
                            case.data.symbols()[changed]
                        }
                        Cell::Missing => '?',
                        _ => panic!("degradation fixtures use single states or missing cells"),
                    }
                };
                text.push(symbol);
            }
            text.push('\n');
        }
        text.push_str("; END;");
        case.name = name;
        case.data = nexus::parse(&text).unwrap();
        case.input = text;
        case.patterns = case
            .data
            .compress(
                std::slice::from_ref(&(0..case.data.character_count())),
                GapTreatment::Reject,
            )
            .unwrap()
            .remove(0);
        case.weights = case.patterns.weights().iter().map(|&w| w as f64).collect();
        cases.push(case);
    }
    cases
}

/// Exchange two tips in different subtrees. Internal edges and the root stay
/// fixed; only the union of their ancestor paths needs new partials.
struct Experiment<'a> {
    engine: Engine<'a>,
    case: &'a Workload,
    dirty: Vec<usize>,
    swapped: bool,
}

impl<'a> Experiment<'a> {
    fn new(case: &'a Workload, backend: Backend) -> Self {
        let mut dirty = vec![false; case.tree.children.len()];
        let last = case.data.taxa().len() - 1;
        for (node, pair) in case.tree.children.iter().enumerate() {
            if let Some((a, b)) = pair {
                dirty[node] =
                    *a == 0 || *b == 0 || *a == last || *b == last || dirty[*a] || dirty[*b];
            }
        }
        Self {
            engine: case.engine(Path::Mk, backend),
            case,
            dirty: dirty
                .iter()
                .enumerate()
                .filter_map(|(n, &d)| d.then_some(n))
                .collect(),
            swapped: false,
        }
    }

    fn children(&self, parent: usize, swapped: bool) -> (usize, usize) {
        let (a, b) = self.case.tree.children[parent].unwrap();
        let last = self.case.data.taxa().len() - 1;
        let map = |n| {
            if !swapped {
                n
            } else if n == 0 {
                last
            } else if n == last {
                0
            } else {
                n
            }
        };
        (map(a), map(b))
    }

    fn topology(&mut self, reject: bool) -> f64 {
        self.engine.core.store();
        let proposed = !self.swapped;
        for &parent in &self.dirty {
            let (a, b) = self.children(parent, proposed);
            self.engine.core.set_node_partials_for_update(parent);
            self.engine.core.calculate_partials(a, b, parent);
        }
        let value = self.engine.score();
        if reject {
            self.engine.core.restore();
        } else {
            self.swapped = proposed;
        }
        value
    }

    fn reference(&self, swapped: bool) -> f64 {
        let children: Vec<_> = self
            .case
            .tree
            .children
            .iter()
            .enumerate()
            .map(|(n, c)| c.map(|_| self.children(n, swapped)))
            .collect();
        let k = self.case.model.states();
        let matrices: Vec<Vec<f64>> = (0..children.len())
            .map(|n| {
                let mut matrix = vec![0.0; k * k * self.case.rates.len()];
                for (c, row) in matrix.chunks_exact_mut(k * k).enumerate() {
                    self.case
                        .model
                        .transition_matrix(self.engine.core.node_distance(n, c), row);
                }
                matrix
            })
            .collect();
        let problem = Problem {
            children: &children,
            root: self.case.tree.root(),
            tip_partials: self.case.patterns.tip_partials(),
            matrices: &matrices,
            category_weights: &self.case.category_weights,
            frequencies: self.case.model.frequencies(),
            pattern_weights: &self.case.weights,
        };
        if self.case.variable.is_some() {
            problem.variable_log_likelihood()
        } else {
            problem.log_likelihood()
        }
    }

    fn validate(&mut self) {
        let initial = self.engine.full();
        assert!((initial - self.reference(false)).abs() < 1e-9);
        for reject in [true, false, true, false] {
            let before = self.engine.score();
            let expected = self.reference(!self.swapped);
            let proposed = self.topology(reject);
            assert!((expected - proposed).abs() < 1e-9);
            if reject {
                assert_eq!(before.to_bits(), self.engine.score().to_bits());
            }
            assert!((self.engine.score() - self.reference(self.swapped)).abs() < 1e-9);
        }
    }
}

fn stats(engine: &Engine<'_>) -> Value {
    #[cfg(feature = "experimental-repeats")]
    if let Some(s) = engine.core.repeat_stats() {
        return json!({"node_updates":s.node_updates,"index_rebuilds":s.index_rebuilds,"packed_updates":s.packed_updates,
            "dense_pattern_lanes":s.dense_pattern_lanes,"evaluated_pattern_lanes":s.evaluated_pattern_lanes,"allocated_bytes":s.allocated_bytes});
    }
    let _ = engine;
    Value::Null
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = timing::Settings {
        samples: 5,
        sample_time: Duration::from_millis(5),
        warmup: Duration::from_millis(10),
    };
    let reverse = std::env::args().any(|a| a == "--reverse");
    let check = std::env::args().any(|a| a == "--check");
    let mut backends = workload::backends();
    if reverse {
        backends.reverse();
    }
    let mut rows = Vec::new();
    for case in cases() {
        for &backend in &backends {
            let mut validated = Experiment::new(&case, backend);
            validated.validate();
            let mut first = case.engine(Path::Mk, backend);
            let ll = first.full();
            let index_stats = stats(&first);
            if check {
                continue;
            }
            for metric in [
                "core_setup",
                "setup_first",
                "full_cached",
                "tip_update",
                "topology_accept",
                "topology_reject",
            ] {
                let mut engine = Experiment::new(&case, backend);
                engine.engine.full();
                let before = stats(&engine.engine);
                let observation = timing::measure(&settings, || match metric {
                    "core_setup" => std::hint::black_box(case.engine(Path::Mk, backend))
                        .core
                        .node_count() as f64,
                    "setup_first" => case.engine(Path::Mk, backend).full(),
                    "full_cached" => engine.engine.full(),
                    "tip_update" => engine.engine.partial(case.tip_branch),
                    "topology_accept" => engine.topology(false),
                    "topology_reject" => engine.topology(true),
                    _ => unreachable!(),
                });
                rows.push(json!({"case":case.name,"backend":backend.name(),"metric":metric,
                    "taxa":case.data.taxa().len(),"characters":case.data.character_count(),"patterns":case.weights.len(),
                    "core_patterns":case.core_patterns(),"states":case.model.states(),"categories":case.rates.len(),"shape":case.shape,
                    "missing_cells":case.missing_cells(),"dirty_nodes":engine.dirty.len(),"baseline_log_likelihood":ll,
                    "index_after_first_prune":index_stats,"index_before_timing":before,"index_after_timing":stats(&engine.engine),
                    "allocation_calls":observation.counts.allocations,"reallocation_calls":observation.counts.reallocations,
                    "requested_bytes":observation.counts.bytes,"deallocation_calls":observation.counts.deallocations,
                    "iterations":observation.iterations,"elapsed_ns":observation.elapsed.iter().map(|d|d.as_nanos() as u64).collect::<Vec<_>>()}));
            }
        }
        eprintln!("Checked and measured {}", case.name);
    }
    println!(
        "{}",
        json!({"experimental_repeats":cfg!(feature="experimental-repeats"),"rows":rows})
    );
    Ok(())
}
