//! Rooted binary trees with node heights.
//!
//! Tips are nodes `0..tips` and internal nodes `tips..2 · tips − 1`, so node
//! indices can be passed to `LikelihoodCore` directly. A parent is never lower
//! than its children; the branch above a node has length `parent − node`.

use std::fmt;

use crate::random::Random;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// The node count is not `2 · tips − 1` for at least two tips.
    Size { nodes: usize },
    /// A tip has children, an internal node lacks them, or a child index is
    /// out of range or shared.
    Shape { node: usize },
    /// The nodes do not form one tree.
    Disconnected,
    /// A height is not finite, a tip is below zero, or a child is above its parent.
    Height { node: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size { nodes } => write!(f, "{nodes} nodes cannot form a rooted binary tree"),
            Self::Shape { node } => write!(f, "node {node} has invalid children"),
            Self::Disconnected => write!(f, "the nodes do not form a single tree"),
            Self::Height { node } => write!(f, "node {node} has an invalid height"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, PartialEq)]
pub struct Tree {
    tips: usize,
    parents: Vec<Option<usize>>,
    children: Vec<Option<(usize, usize)>>,
    heights: Vec<f64>,
    root: usize,
}

impl Clone for Tree {
    fn clone(&self) -> Self {
        Self {
            tips: self.tips,
            parents: self.parents.clone(),
            children: self.children.clone(),
            heights: self.heights.clone(),
            root: self.root,
        }
    }

    /// Reuses the buffers, so the sampler can save a tree on every step
    /// without allocating.
    fn clone_from(&mut self, source: &Self) {
        self.tips = source.tips;
        self.parents.clone_from(&source.parents);
        self.children.clone_from(&source.children);
        self.heights.clone_from(&source.heights);
        self.root = source.root;
    }
}

impl Tree {
    pub fn new(children: Vec<Option<(usize, usize)>>, heights: Vec<f64>) -> Result<Self, Error> {
        let nodes = children.len();
        if nodes < 3 || nodes % 2 == 0 || heights.len() != nodes {
            return Err(Error::Size { nodes });
        }
        let tips = nodes.div_ceil(2);
        let mut parents = vec![None; nodes];
        for (node, pair) in children.iter().enumerate() {
            match (node < tips, *pair) {
                (true, None) => {}
                (false, Some((a, b))) => {
                    for child in [a, b] {
                        if child >= nodes || child == node || a == b {
                            return Err(Error::Shape { node });
                        }
                        if parents[child].replace(node).is_some() {
                            return Err(Error::Shape { node: child });
                        }
                    }
                }
                _ => return Err(Error::Shape { node }),
            }
        }
        let roots: Vec<usize> = (0..nodes).filter(|&n| parents[n].is_none()).collect();
        let [root] = roots[..] else {
            return Err(Error::Disconnected);
        };
        let tree = Self {
            tips,
            parents,
            children,
            heights,
            root,
        };
        if tree.postorder().len() != nodes {
            return Err(Error::Disconnected);
        }
        tree.check_heights()?;
        Ok(tree)
    }

    fn check_heights(&self) -> Result<(), Error> {
        for node in 0..self.nodes() {
            let height = self.heights[node];
            let below_parent = self.parents[node].is_none_or(|p| height <= self.heights[p]);
            if !height.is_finite() || height < 0.0 || !below_parent {
                return Err(Error::Height { node });
            }
        }
        Ok(())
    }

    /// Checks the invariants that `new` establishes.
    pub fn validate(&self) -> Result<(), Error> {
        Self::new(self.children.clone(), self.heights.clone()).and_then(|tree| {
            if tree.root == self.root && tree.parents == self.parents {
                Ok(())
            } else {
                Err(Error::Disconnected)
            }
        })
    }

    /// A random tree with every tip at height zero, in which each clade in
    /// `clades` is monophyletic. The clades must be nested or disjoint.
    pub fn random(tips: usize, clades: &[Vec<usize>], random: &mut Random) -> Self {
        assert!(tips >= 2, "a tree needs at least two tips");
        let mut ordered: Vec<&Vec<usize>> = clades.iter().collect();
        ordered.sort_by_key(|clade| clade.len());
        let mut children = vec![None; tips];
        let mut heights = vec![0.0; tips];
        let mut units: Vec<(usize, Vec<usize>)> = (0..tips).map(|t| (t, vec![t])).collect();
        let mut join = |pool: Vec<(usize, Vec<usize>)>, random: &mut Random| {
            let mut pool = pool;
            let mut time = pool.iter().map(|u| heights[u.0]).fold(0.0, f64::max);
            while pool.len() > 1 {
                let k = pool.len() as f64;
                time += random.exponential() / (k * (k - 1.0) / 2.0);
                let (a, mut taxa) = pool.swap_remove(random.below(pool.len()));
                let (b, other) = pool.swap_remove(random.below(pool.len()));
                taxa.extend(other);
                children.push(Some((a, b)));
                heights.push(time);
                pool.push((children.len() - 1, taxa));
            }
            pool.pop().expect("a joined group")
        };
        for clade in ordered {
            let (inside, outside) = units
                .into_iter()
                .partition(|(_, taxa)| taxa.iter().all(|t| clade.contains(t)));
            units = outside;
            units.push(join(inside, random));
        }
        join(units, random);
        Self::new(children, heights).expect("compatible clades give a valid tree")
    }

    pub fn tips(&self) -> usize {
        self.tips
    }

    pub fn nodes(&self) -> usize {
        self.children.len()
    }

    pub fn root(&self) -> usize {
        self.root
    }

    pub fn parent(&self, node: usize) -> Option<usize> {
        self.parents[node]
    }

    pub fn children(&self, node: usize) -> Option<(usize, usize)> {
        self.children[node]
    }

    pub fn height(&self, node: usize) -> f64 {
        self.heights[node]
    }

    /// The length of the branch above `node`; zero for the root.
    pub fn branch_length(&self, node: usize) -> f64 {
        self.parents[node].map_or(0.0, |p| self.heights[p] - self.heights[node])
    }

    pub fn tree_length(&self) -> f64 {
        (0..self.nodes()).map(|n| self.branch_length(n)).sum()
    }

    /// Every node, with both children before their parent.
    pub fn postorder(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.nodes());
        self.postorder_into(&mut order);
        order
    }

    /// `postorder` into a reused buffer. The walk follows parent links, so it
    /// needs no stack.
    pub fn postorder_into(&self, order: &mut Vec<usize>) {
        order.clear();
        let mut node = self.root;
        let mut previous = None;
        loop {
            match self.children[node] {
                Some((a, _)) if previous == self.parents[node] => {
                    previous = Some(node);
                    node = a;
                }
                Some((a, b)) if previous == Some(a) => {
                    previous = Some(node);
                    node = b;
                }
                _ => {
                    order.push(node);
                    let Some(parent) = self.parents[node] else {
                        break;
                    };
                    previous = Some(node);
                    node = parent;
                }
            }
        }
    }

    /// Whether some node has exactly the tips marked in `members` below it.
    pub fn is_monophyletic(&self, members: &[bool]) -> bool {
        let mut below = vec![(0, 0); self.nodes()];
        self.is_monophyletic_in(members, &self.postorder(), &mut below)
    }

    /// `is_monophyletic` with the postorder and one scratch entry per node supplied.
    pub(crate) fn is_monophyletic_in(
        &self,
        members: &[bool],
        order: &[usize],
        below: &mut [(usize, usize)],
    ) -> bool {
        let size = members.iter().filter(|&&m| m).count();
        for &node in order {
            below[node] = match self.children[node] {
                None => (1, usize::from(members[node])),
                Some((a, b)) => (below[a].0 + below[b].0, below[a].1 + below[b].1),
            };
            if below[node] == (size, size) {
                return true;
            }
        }
        false
    }

    /// Newick with branch lengths, naming each tip with `label`.
    pub fn to_newick(&self, label: impl Fn(usize) -> String) -> String {
        let mut text = String::new();
        self.write_newick(self.root, &label, &|_| None, &mut text);
        text.push(';');
        text
    }

    /// Time lengths with a rate annotation on every non-root branch.
    pub fn to_newick_with_rates(
        &self,
        label: impl Fn(usize) -> String,
        rate: impl Fn(usize) -> f64,
    ) -> String {
        let mut text = String::new();
        self.write_newick(self.root, &label, &|node| Some(rate(node)), &mut text);
        text.push(';');
        text
    }

    fn write_newick(
        &self,
        node: usize,
        label: &impl Fn(usize) -> String,
        rate: &impl Fn(usize) -> Option<f64>,
        text: &mut String,
    ) {
        match self.children[node] {
            None => text.push_str(&label(node)),
            Some((a, b)) => {
                text.push('(');
                self.write_newick(a, label, rate, text);
                text.push(',');
                self.write_newick(b, label, rate, text);
                text.push(')');
            }
        }
        if node != self.root {
            if let Some(rate) = rate(node) {
                text.push_str(&format!("[&rate={rate}]"));
            }
            text.push(':');
            text.push_str(&self.branch_length(node).to_string());
        }
    }

    pub(crate) fn set_height(&mut self, node: usize, height: f64) {
        self.heights[node] = height;
    }

    /// Makes `new` a child of `parent` in place of `old`.
    pub(crate) fn replace_child(&mut self, parent: usize, old: usize, new: usize) {
        let (a, b) = self.children[parent].expect("an internal parent");
        self.children[parent] = Some(if a == old { (new, b) } else { (a, new) });
        debug_assert!(a == old || b == old);
        self.parents[new] = Some(parent);
    }

    pub(crate) fn make_root(&mut self, node: usize) {
        self.parents[node] = None;
        self.root = node;
    }

    pub(crate) fn sibling(&self, node: usize) -> usize {
        let parent = self.parents[node].expect("a node below the root");
        let (a, b) = self.children[parent].expect("an internal parent");
        if a == node { b } else { a }
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, Tree};
    use crate::random::Random;

    fn example() -> Tree {
        // ((0, 1):3, 2):4 with heights 0, 0, 0, 1, 2.
        Tree::new(
            vec![None, None, None, Some((0, 1)), Some((3, 2))],
            vec![0.0, 0.0, 0.0, 1.0, 2.0],
        )
        .unwrap()
    }

    #[test]
    fn reports_branch_lengths_and_order() {
        let tree = example();
        assert_eq!(tree.root(), 4);
        assert_eq!(tree.branch_length(3), 1.0);
        assert_eq!(tree.branch_length(4), 0.0);
        assert_eq!(tree.tree_length(), 5.0);
        assert_eq!(tree.postorder(), vec![0, 1, 3, 2, 4]);
        assert_eq!(tree.to_newick(|t| format!("T{t}")), "((T0:1,T1:1):1,T2:2);");
    }

    #[test]
    fn rejects_malformed_trees() {
        let tip_children = vec![Some((1, 2)), None, None, None, Some((3, 0))];
        assert_eq!(
            Tree::new(tip_children, vec![0.0; 5]),
            Err(Error::Shape { node: 0 })
        );
        let shared = vec![None, None, None, Some((0, 1)), Some((0, 2))];
        assert_eq!(
            Tree::new(shared, vec![0.0; 5]),
            Err(Error::Shape { node: 0 })
        );
        let cycle = vec![None, None, None, Some((4, 0)), Some((3, 1))];
        assert!(Tree::new(cycle, vec![0.0, 0.0, 0.0, 1.0, 2.0]).is_err());
        let inverted = vec![0.0, 0.0, 0.0, 3.0, 2.0];
        assert_eq!(
            Tree::new(vec![None, None, None, Some((0, 1)), Some((3, 2))], inverted),
            Err(Error::Height { node: 3 })
        );
        assert_eq!(
            Tree::new(vec![None, None], vec![0.0; 2]),
            Err(Error::Size { nodes: 2 })
        );
    }

    #[test]
    fn checks_monophyly() {
        let tree = example();
        assert!(tree.is_monophyletic(&[true, true, false]));
        assert!(!tree.is_monophyletic(&[true, false, true]));
        assert!(tree.is_monophyletic(&[true, true, true]));
    }

    #[test]
    fn random_trees_respect_nested_clades() {
        let mut random = Random::new(3);
        let clades = vec![vec![0, 1, 2, 3, 4], vec![1, 2], vec![6, 7, 8]];
        for _ in 0..200 {
            let tree = Tree::random(10, &clades, &mut random);
            tree.validate().unwrap();
            assert_eq!(tree.tips(), 10);
            for clade in &clades {
                let members: Vec<bool> = (0..10).map(|t| clade.contains(&t)).collect();
                assert!(tree.is_monophyletic(&members), "{clade:?}");
            }
            assert!((0..10).all(|t| tree.height(t) == 0.0));
        }
    }
}
