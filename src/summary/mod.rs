//! Posterior tree summaries. Each pass holds one parsed tree; clade counts and
//! statistics for the selected target are retained between passes.
pub mod nexus;

use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

use nexus::{Clade, Reader, Tree};

#[derive(Debug)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug)]
pub enum Burnin {
    Percent(u8),
    Samples(usize),
}
impl Burnin {
    pub fn discard(self, total: usize) -> Result<usize> {
        let skip = match self {
            Self::Percent(p) if p < 100 => (total as u128 * p as u128 / 100) as usize,
            Self::Samples(n) => n,
            _ => return Err(Error("burn-in percent must be between 0 and 99".into())),
        };
        if skip >= total {
            return Err(Error("burn-in leaves no retained samples".into()));
        }
        Ok(skip)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heights {
    Keep,
    CommonAncestor,
}

#[derive(Clone, Default, Debug)]
pub struct Moments {
    pub count: usize,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
}
impl Moments {
    fn add(&mut self, x: f64) {
        self.count += 1;
        self.mean += (x - self.mean) / self.count as f64;
        if self.count == 1 {
            self.min = x;
            self.max = x;
        } else {
            self.min = self.min.min(x);
            self.max = self.max.max(x);
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct NodeSummary {
    pub posterior: f64,
    pub height: Moments,
    pub ancestor: Moments,
    pub rate: Moments,
    pub constrained: bool,
}

#[derive(Debug)]
pub struct Summary {
    pub taxa: Vec<String>,
    pub total: usize,
    pub discarded: usize,
    pub first_state: u64,
    pub last_state: u64,
    pub first_retained_state: u64,
    pub sampling_interval: Option<u64>,
    pub target: Tree,
    pub log_credibility: f64,
    pub heights: Heights,
    pub nodes: Vec<NodeSummary>,
    pub distinct_clades: usize,
}

fn open(path: &Path) -> Result<Reader<BufReader<File>>> {
    Reader::new(BufReader::with_capacity(128 * 1024, File::open(path)?))
}

fn visit(
    path: &Path,
    skip: usize,
    cancel: &dyn Fn() -> bool,
    mut f: impl FnMut(Tree) -> Result<()>,
) -> Result<Vec<String>> {
    let mut reader = open(path)?;
    let mut index = 0;
    while let Some(tree) = reader.next_tree()? {
        if cancel() {
            return Err(Error("tree summary cancelled".into()));
        }
        if index >= skip {
            f(tree)?;
        }
        index += 1;
    }
    Ok(reader.taxa)
}

pub fn summarize(
    path: &Path,
    burnin: Burnin,
    heights: Heights,
    constraints: &[Vec<String>],
    cancel: &dyn Fn() -> bool,
) -> Result<Summary> {
    let mut total = 0;
    let (mut first_state, mut last_state) = (0, 0);
    let mut interval = None;
    let mut regular = true;
    let taxa = visit(path, 0, cancel, |tree| {
        let state = tree.state.expect("sample reader returns sample states");
        if total == 0 {
            first_state = state;
        } else {
            let gap = state - last_state;
            if interval.is_some_and(|value| value != gap) {
                regular = false;
            }
            interval.get_or_insert(gap);
        }
        last_state = state;
        total += 1;
        Ok(())
    })?;
    let skip = burnin.discard(total)?;
    let retained = total - skip;
    let mut counts: HashMap<Clade, usize> = HashMap::new();
    let mut first_retained_state = None;
    visit(path, skip, cancel, |tree| {
        first_retained_state
            .get_or_insert(tree.state.expect("sample reader returns sample states"));
        for (node, clade) in tree.nodes.iter().zip(tree.clades()) {
            if node.children.is_some() {
                *counts.entry(clade).or_default() += 1;
            }
        }
        if counts.len() > 1_000_000
            || counts.len() * (taxa.len().div_ceil(64) * 8 + 64) > 128 * 1024 * 1024
        {
            return Err(Error(
                "clade count exceeds the summary memory budget".into(),
            ));
        }
        Ok(())
    })?;
    let mut target = None;
    let mut best = f64::NEG_INFINITY;
    visit(path, skip, cancel, |tree| {
        let masks = tree.clades();
        let mut clades: Vec<_> = tree
            .nodes
            .iter()
            .zip(&masks)
            .filter(|(n, _)| n.children.is_some())
            .map(|(_, m)| m)
            .collect();
        // Stable summation order makes scores independent of child rotation.
        clades.sort_unstable();
        let score: f64 = clades
            .iter()
            .map(|c| (counts[*c] as f64 / retained as f64).ln())
            .sum();
        if score > best {
            best = score;
            target = Some(tree);
        }
        Ok(())
    })?;
    let target = target.ok_or_else(|| Error("no MCC candidate".into()))?;
    let masks = target.clades();
    let mut summaries = vec![NodeSummary::default(); target.nodes.len()];
    for (i, mask) in masks.iter().enumerate() {
        summaries[i].posterior = counts
            .get(mask)
            .map_or(1.0, |&n| n as f64 / retained as f64);
    }
    for group in constraints {
        let mut mask = vec![0u64; taxa.len().div_ceil(64)];
        for taxon in group {
            let i = taxa.iter().position(|t| t == taxon).ok_or_else(|| {
                Error(format!(
                    "constraint taxon {taxon:?} is absent from the trees"
                ))
            })?;
            if mask[i / 64] & (1 << (i % 64)) != 0 {
                return Err(Error("duplicate constraint taxon".into()));
            }
            mask[i / 64] |= 1 << (i % 64);
        }
        if group.len() < 2 || counts.get(&mask) != Some(&retained) {
            return Err(Error(
                "recorded constraint is not present in every retained tree".into(),
            ));
        }
        let i = masks
            .iter()
            .position(|c| *c == mask)
            .ok_or_else(|| Error("constraint missing from MCC tree".into()))?;
        summaries[i].constrained = true;
    }
    visit(path, skip, cancel, |tree| {
        let current = tree.clades();
        for (i, mask) in masks.iter().enumerate() {
            let ancestor = tree.ancestor(mask, &current);
            let node = &tree.nodes[ancestor];
            summaries[i].ancestor.add(node.height);
            if current[ancestor] == *mask {
                summaries[i].height.add(node.height);
                if let Some(rate) = node.rate {
                    summaries[i].rate.add(rate);
                }
            }
        }
        Ok(())
    })?;
    Ok(Summary {
        taxa,
        total,
        discarded: skip,
        first_state,
        last_state,
        first_retained_state: first_retained_state.unwrap(),
        sampling_interval: if regular { interval } else { None },
        target,
        log_credibility: best,
        heights,
        nodes: summaries,
        distinct_clades: counts.len(),
    })
}

impl Summary {
    pub fn retained(&self) -> usize {
        self.total - self.discarded
    }
    pub fn write_tree(&self, mut out: impl Write) -> Result<()> {
        writeln!(
            out,
            "#NEXUS\n[Rooted MCC summary; posterior support is not a convergence diagnostic.]\nBegin trees;\nTranslate"
        )?;
        for (i, taxon) in self.taxa.iter().enumerate() {
            writeln!(
                out,
                "  {} {}{}",
                i + 1,
                if taxon
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c))
                {
                    taxon.clone()
                } else {
                    format!("'{}'", taxon.replace('\'', "''"))
                },
                if i + 1 == self.taxa.len() { ";" } else { "," }
            )?;
        }
        write!(out, "tree MCC = [&R] ")?;
        self.write_node(self.target.root, &mut out)?;
        writeln!(out, ";\nEnd;")?;
        Ok(())
    }
    fn height(&self, i: usize) -> f64 {
        match self.heights {
            Heights::Keep => self.target.nodes[i].height,
            Heights::CommonAncestor => self.nodes[i].ancestor.mean,
        }
    }
    fn write_node(&self, i: usize, out: &mut impl Write) -> Result<()> {
        let node = &self.target.nodes[i];
        let stats = &self.nodes[i];
        if let Some([a, b]) = node.children {
            write!(out, "(")?;
            self.write_node(a, out)?;
            write!(out, ",")?;
            self.write_node(b, out)?;
            write!(out, ")")?;
        } else {
            write!(out, "{}", node.tip.unwrap() + 1)?;
        }
        write!(
            out,
            "[&height={},height_mean={},height_range={{{},{}}},CAheight_mean={},CAheight_range={{{},{}}}",
            self.height(i),
            stats.height.mean,
            stats.height.min,
            stats.height.max,
            stats.ancestor.mean,
            stats.ancestor.min,
            stats.ancestor.max
        )?;
        if node.children.is_some() {
            write!(
                out,
                ",posterior={},constrained={}",
                stats.posterior, stats.constrained
            )?;
        }
        if stats.rate.count > 0 {
            write!(
                out,
                ",rate={},rate_range={{{},{}}}",
                stats.rate.mean, stats.rate.min, stats.rate.max
            )?;
        }
        write!(out, "]")?;
        if let Some(parent) = node.parent {
            let length = self.height(parent) - self.height(i);
            if length < -1e-12 {
                return Err(Error(
                    "summary heights would create a negative branch".into(),
                ));
            }
            write!(out, ":{}", length.max(0.0))?;
        }
        Ok(())
    }
}
