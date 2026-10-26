use rove::data::{Cell, GapTreatment, Matrix, Patterns};
use rove::model::Mk;
use rove::reference::Problem;
use rove::{LikelihoodCore, Scaling, nexus};

#[rustfmt::skip]
#[path = "reference_state.rs"]
mod reference_state;

pub struct Tree {
    pub children: Vec<Option<(usize, usize)>>,
    pub distances: Vec<f64>,
    parents: Vec<Option<usize>>,
}

impl Tree {
    fn new(children: Vec<Option<(usize, usize)>>, distances: Vec<f64>) -> Self {
        assert_eq!(children.len(), distances.len());
        let mut parents = vec![None; children.len()];
        for (parent, pair) in children.iter().enumerate() {
            if let Some((a, b)) = *pair {
                assert!(a < parent && b < parent && a != b);
                for child in [a, b] {
                    assert!(parents[child].replace(parent).is_none());
                }
            }
        }
        assert!(parents[..parents.len() - 1].iter().all(Option::is_some));
        Self {
            children,
            distances,
            parents,
        }
    }

    fn generated(tips: usize, ladder: bool) -> Self {
        let mut children = vec![None; tips];
        if ladder {
            let mut left = 0;
            for right in 1..tips {
                let parent = children.len();
                children.push(Some((left, right)));
                left = parent;
            }
        } else {
            fn join(start: usize, end: usize, children: &mut Vec<Option<(usize, usize)>>) -> usize {
                if end - start == 1 {
                    return start;
                }
                let middle = (start + end) / 2;
                let left = join(start, middle, children);
                let right = join(middle, end, children);
                let parent = children.len();
                children.push(Some((left, right)));
                parent
            }
            join(0, tips, &mut children);
        }
        let mut distances = vec![0.1; children.len()];
        *distances.last_mut().unwrap() = 0.0;
        Self::new(children, distances)
    }

    pub fn root(&self) -> usize {
        self.children.len() - 1
    }

    pub fn depth(&self, mut node: usize) -> usize {
        let mut depth = 0;
        while let Some(parent) = self.parents[node] {
            depth += 1;
            node = parent;
        }
        depth
    }
}

pub struct Workload {
    pub name: &'static str,
    pub input: String,
    pub data: Matrix,
    pub patterns: Patterns,
    pub tree: Tree,
    pub shape: &'static str,
    pub rates: Vec<f64>,
    pub category_weights: Vec<f64>,
    pub weights: Vec<f64>,
    pub model: Mk,
    pub tip_branch: usize,
    pub internal_branch: usize,
    expected: Option<f64>,
}

impl Workload {
    fn new(
        name: &'static str,
        input: String,
        tree: Tree,
        shape: &'static str,
        rates: Vec<f64>,
    ) -> Self {
        let data = nexus::parse(&input).expect("valid benchmark matrix");
        let sites = 0..data.character_count();
        let patterns = data
            .compress(std::slice::from_ref(&sites), GapTreatment::Reject)
            .unwrap()
            .remove(0);
        let weights = patterns.weights().iter().map(|&w| w as f64).collect();
        let model = Mk::uniform(data.symbols().len());
        assert_eq!(tree.children.len(), data.taxa().len() * 2 - 1);
        let tip_branch = (0..data.taxa().len())
            .max_by_key(|&tip| tree.depth(tip))
            .unwrap();
        let (a, b) = tree.children[tree.root()].unwrap();
        let internal_branch = if tree.children[a].is_some() { a } else { b };
        assert!(tree.children[internal_branch].is_some());
        let category_weights = vec![1.0 / rates.len() as f64; rates.len()];
        Self {
            name,
            input,
            data,
            patterns,
            tree,
            shape,
            rates,
            category_weights,
            weights,
            model,
            tip_branch,
            internal_branch,
            expected: None,
        }
    }

    pub fn missing_cells(&self) -> usize {
        self.data
            .rows()
            .iter()
            .flatten()
            .filter(|c| **c == Cell::Missing)
            .count()
    }

    /// Allocate and populate the core. Pruning is measured separately.
    pub fn engine(&self) -> Engine<'_> {
        let mut core = LikelihoodCore::new(
            self.model.states(),
            self.weights.len(),
            self.rates.len(),
            self.tree.children.len(),
            Scaling::Always,
        );
        for (tip, values) in self.patterns.tip_partials().iter().enumerate() {
            core.set_tip_partials(tip, values);
        }
        let matrix = vec![0.0; self.model.states() * self.model.states()];
        let mut engine = Engine {
            workload: self,
            core,
            matrix,
            alternate: false,
        };
        for node in 0..self.tree.root() {
            engine.set_matrix(node, self.tree.distances[node]);
        }
        engine
    }

    /// Check every timed numerical path before collecting any measurements.
    pub fn validate(&self) -> f64 {
        let mut engine = self.engine();
        let initial = engine.full();
        close(initial, engine.reference());
        if let Some(expected) = self.expected {
            close(initial, expected);
        }
        for branch in [self.tip_branch, self.internal_branch] {
            close(engine.partial(branch), engine.reference());
            for reject in [true, false, true, false] {
                let before = engine.score();
                let proposed = engine.proposal(branch, reject);
                assert!(proposed.is_finite());
                if reject {
                    close(engine.score(), before);
                } else {
                    close(proposed, engine.reference());
                }
                close(engine.score(), engine.reference());
            }
        }
        close(engine.full_with_matrices(), engine.reference());
        initial
    }
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite() && expected.is_finite());
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

pub struct Engine<'a> {
    workload: &'a Workload,
    pub core: LikelihoodCore,
    matrix: Vec<f64>,
    alternate: bool,
}

impl Engine<'_> {
    fn set_matrix(&mut self, node: usize, distance: f64) {
        for (category, rate) in self.workload.rates.iter().enumerate() {
            self.workload
                .model
                .transition_matrix(distance * rate, &mut self.matrix);
            self.core.set_node_matrix(node, category, &self.matrix);
        }
    }

    pub fn score(&mut self) -> f64 {
        self.core.log_likelihood(
            self.workload.tree.root(),
            &self.workload.category_weights,
            self.workload.model.frequencies(),
            &self.workload.weights,
        )
    }

    pub fn full(&mut self) -> f64 {
        for (parent, pair) in self.workload.tree.children.iter().enumerate() {
            if let Some((a, b)) = *pair {
                self.core.calculate_partials(a, b, parent);
            }
        }
        self.score()
    }

    fn factor(&mut self) -> f64 {
        self.alternate = !self.alternate;
        if self.alternate { 1.1 } else { 0.9 }
    }

    pub fn full_with_matrices(&mut self) -> f64 {
        let factor = self.factor();
        for node in 0..self.workload.tree.root() {
            self.set_matrix(node, self.workload.tree.distances[node] * factor);
        }
        self.full()
    }

    fn update(&mut self, branch: usize, switch_buffers: bool) -> f64 {
        let distance = self.workload.tree.distances[branch] * self.factor();
        if switch_buffers {
            self.core.set_node_matrix_for_update(branch);
        }
        self.set_matrix(branch, distance);
        let mut node = branch;
        while let Some(parent) = self.workload.tree.parents[node] {
            if switch_buffers {
                self.core.set_node_partials_for_update(parent);
            }
            let (a, b) = self.workload.tree.children[parent].unwrap();
            self.core.calculate_partials(a, b, parent);
            node = parent;
        }
        self.score()
    }

    pub fn partial(&mut self, branch: usize) -> f64 {
        self.update(branch, false)
    }

    /// A forced outcome isolates buffer bookkeeping; this is not an MCMC sampler.
    pub fn proposal(&mut self, branch: usize, reject: bool) -> f64 {
        self.core.store();
        let proposed = self.update(branch, true);
        if reject {
            self.core.restore();
        }
        proposed
    }

    fn reference(&self) -> f64 {
        let matrices: Vec<Vec<f64>> = (0..self.core.node_count())
            .map(|node| {
                (0..self.workload.rates.len())
                    .flat_map(|c| self.core.node_matrix(node, c).iter().copied())
                    .collect()
            })
            .collect();
        Problem {
            children: &self.workload.tree.children,
            root: self.workload.tree.root(),
            tip_partials: self.workload.patterns.tip_partials(),
            matrices: &matrices,
            category_weights: &self.workload.category_weights,
            frequencies: self.workload.model.frequencies(),
            pattern_weights: &self.workload.weights,
        }
        .log_likelihood()
    }
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }
}

fn synthetic(
    taxa: usize,
    characters: usize,
    states: usize,
    missing_percent: u64,
    repeat: usize,
) -> String {
    assert_eq!(characters % repeat, 0);
    let mut rng = SplitMix64(20261026);
    let symbols: String = (0..states).map(|s| char::from(b'0' + s as u8)).collect();
    let mut text = format!(
        "#NEXUS\nBEGIN DATA; DIMENSIONS NTAX={taxa} NCHAR={characters};\nFORMAT SYMBOLS=\"{symbols}\" MISSING=?;\nMATRIX\n"
    );
    for tip in 1..=taxa {
        text.push_str(&format!("T{tip} "));
        for _ in 0..characters / repeat {
            let state = char::from(b'0' + (rng.next() % states as u64) as u8);
            let missing = rng.next() % 100 < missing_percent;
            for _ in 0..repeat {
                text.push(if missing { '?' } else { state });
            }
        }
        text.push('\n');
    }
    text.push_str("; END;\n");
    text
}

pub fn suite() -> Vec<Workload> {
    let mut reference = Workload::new(
        "reference_20x50",
        include_str!("../../fixtures/reference/20x50/matrix.nex").into(),
        Tree::new(
            reference_state::CHILDREN.to_vec(),
            reference_state::DISTANCES.to_vec(),
        ),
        "reference",
        reference_state::RATES.to_vec(),
    );
    assert_eq!(reference.data.taxa(), reference_state::TAXA);
    reference.model = Mk::with_frequencies(reference_state::FREQUENCIES.to_vec());
    reference.category_weights = reference_state::CATEGORY_WEIGHTS.to_vec();
    reference.expected = Some(reference_state::EXPECTED);
    let mut cases = vec![
        reference,
        Workload::new(
            "morphology_90x250",
            include_str!("../../fixtures/benchmarks/morphology_90x250.nex").into(),
            Tree::generated(90, false),
            "balanced",
            vec![0.25, 0.75, 1.25, 1.75],
        ),
    ];
    for (name, tips, characters, states, categories, missing, repeat, ladder) in [
        ("binary_one_rate", 90, 250, 2, 1, 0, 1, false),
        ("binary_four_rates", 90, 250, 2, 4, 0, 1, false),
        ("ternary_four_rates", 90, 250, 3, 4, 0, 1, false),
        ("eight_states", 90, 250, 8, 4, 0, 1, false),
        ("missing_20_percent", 90, 250, 2, 4, 20, 1, false),
        ("repeated_columns", 90, 250, 2, 4, 0, 10, false),
        ("ladder", 90, 250, 2, 4, 0, 1, true),
        ("small_unpadded", 5, 16, 2, 4, 0, 1, false),
        ("small_padded", 5, 17, 2, 4, 0, 1, false),
    ] {
        let rates = if categories == 1 {
            vec![1.0]
        } else {
            vec![0.25, 0.75, 1.25, 1.75]
        };
        let input = if tips == 5 {
            let rows: String = (0..tips)
                .map(|tip| {
                    let row: String = (0..characters)
                        .map(|site| if site & (1 << tip) == 0 { '0' } else { '1' })
                        .collect();
                    format!("T{} {row}\n", tip + 1)
                })
                .collect();
            format!("#NEXUS BEGIN DATA; DIMENSIONS NTAX=5 NCHAR={characters}; MATRIX\n{rows}; END;")
        } else {
            synthetic(tips, characters, states, missing, repeat)
        };
        cases.push(Workload::new(
            name,
            input,
            Tree::generated(tips, ladder),
            if ladder { "ladder" } else { "balanced" },
            rates,
        ));
    }
    cases
}
