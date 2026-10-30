//! Experimental subtree site repeats. Equality is structural, never a
//! floating-point tolerance: a parent's class is a pair of child classes.
//! Full pattern buffers are retained; representatives are packed for the
//! existing Mk kernels, then expanded for the ordinary core API.

use super::{LikelihoodCore, Scaling, Transition, mk, scaling};
use crate::layout::Layout;

const EMPTY: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, Default)]
pub struct RepeatStats {
    pub node_updates: u64,
    pub index_rebuilds: u64,
    pub packed_updates: u64,
    pub dense_pattern_lanes: u64,
    pub evaluated_pattern_lanes: u64,
    pub allocated_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Dependency {
    node: usize,
    version: u64,
}

#[derive(Default)]
struct Index {
    ids: Vec<u32>,
    representatives: Vec<u32>,
    count: usize,
    version: u64,
    children: Option<[Dependency; 2]>,
}

impl Index {
    fn new(patterns: usize) -> Self {
        Self {
            ids: vec![0; patterns],
            representatives: Vec::with_capacity(patterns),
            count: patterns,
            ..Self::default()
        }
    }

    fn id(&self, pattern: usize) -> u32 {
        if self.count == self.ids.len() {
            pattern as u32
        } else {
            self.ids[pattern]
        }
    }
}

/// Shared open-addressed scratch table. Full keys (and tip vectors for tip
/// hashes) are checked, so collisions affect cost, never equality.
struct Table {
    keys: Vec<u64>,
    values: Vec<u32>,
}

impl Table {
    fn new(patterns: usize) -> Self {
        let size = (2 * patterns).next_power_of_two();
        Self {
            keys: vec![0; size],
            values: vec![EMPTY; size],
        }
    }

    fn clear(&mut self) {
        self.values.fill(EMPTY);
    }

    fn slot(&self, key: u64) -> usize {
        (key.wrapping_mul(0x9e3779b97f4a7c15) >> (64 - self.keys.len().ilog2())) as usize
    }
}

pub(super) struct Repeats {
    indices: Vec<Index>,
    current: Vec<u8>,
    stored: Vec<u8>,
    version: u64,
    table: Table,
    first: Vec<f64>,
    second: Vec<f64>,
    output: Vec<f64>,
    exponents: Vec<i64>,
    stats: RepeatStats,
}

impl Repeats {
    pub fn new(layout: Layout, nodes: usize) -> Self {
        assert!(
            layout.patterns < u32::MAX as usize,
            "too many patterns for repeat indices"
        );
        let mut repeats = Self {
            indices: (0..2 * nodes)
                .map(|_| Index::new(layout.patterns))
                .collect(),
            current: vec![0; nodes],
            stored: vec![0; nodes],
            version: 0,
            table: Table::new(layout.patterns),
            first: vec![0.0; layout.partials_len()],
            second: vec![0.0; layout.partials_len()],
            output: vec![0.0; layout.partials_len()],
            exponents: vec![0; layout.stride],
            stats: RepeatStats::default(),
        };
        repeats.stats.allocated_bytes = repeats.indices.capacity() * size_of::<Index>()
            + repeats
                .indices
                .iter()
                .map(|i| (i.ids.capacity() + i.representatives.capacity()) * size_of::<u32>())
                .sum::<usize>()
            + repeats.current.capacity()
            + repeats.stored.capacity()
            + repeats.table.keys.capacity() * size_of::<u64>()
            + repeats.table.values.capacity() * size_of::<u32>()
            + (repeats.first.capacity() + repeats.second.capacity() + repeats.output.capacity())
                * size_of::<f64>()
            + repeats.exponents.capacity() * size_of::<i64>();
        repeats
    }

    pub fn stats(&self) -> RepeatStats {
        self.stats
    }

    fn buffer(&self, node: usize) -> usize {
        2 * node + self.current[node] as usize
    }

    fn version(&mut self) -> u64 {
        self.version = self
            .version
            .checked_add(1)
            .expect("repeat version overflow");
        self.version
    }

    pub fn set_tip(&mut self, node: usize, partials: &[f64], layout: Layout) {
        self.table.clear();
        let mut index = std::mem::take(&mut self.indices[2 * node]);
        index.representatives.clear();
        for p in 0..layout.patterns {
            let mut hash = 0u64;
            for s in 0..layout.states {
                hash = (hash ^ partials[layout.offset(0, s) + p].to_bits())
                    .rotate_left(13)
                    .wrapping_mul(0x9e3779b97f4a7c15);
            }
            let mut slot = self.table.slot(hash);
            let id = loop {
                let id = self.table.values[slot];
                if id == EMPTY {
                    let id = index.representatives.len() as u32;
                    index.representatives.push(p as u32);
                    self.table.keys[slot] = hash;
                    self.table.values[slot] = id;
                    break id;
                }
                let r = index.representatives[id as usize] as usize;
                if self.table.keys[slot] == hash
                    && (0..layout.states).all(|s| {
                        partials[layout.offset(0, s) + p].to_bits()
                            == partials[layout.offset(0, s) + r].to_bits()
                    })
                {
                    break id;
                }
                slot = (slot + 1) & (self.table.keys.len() - 1);
            };
            index.ids[p] = id;
        }
        index.count = index.representatives.len();
        index.version = if index.count == layout.patterns {
            0
        } else {
            self.version()
        };
        index.children = None;
        let other = &mut self.indices[2 * node + 1];
        other.ids.copy_from_slice(&index.ids);
        other.representatives.clear();
        other
            .representatives
            .extend_from_slice(&index.representatives);
        other.count = index.count;
        other.version = index.version;
        other.children = None;
        self.indices[2 * node] = index;
    }

    /// Branch lengths, rates and partials-buffer indices do not affect the
    /// equivalence classes. Versions track only changes to the repeat index.
    fn prepare(&mut self, first: usize, second: usize, parent: usize, patterns: usize) {
        let dependencies = [first, second].map(|node| Dependency {
            node,
            version: self.indices[self.buffer(node)].version,
        });
        let current = self.buffer(parent);
        if self.indices[current].children == Some(dependencies) {
            return;
        }
        let target = current ^ 1;
        let mut index = std::mem::take(&mut self.indices[target]);
        index.children = Some(dependencies);
        index.representatives.clear();
        let a = &self.indices[self.buffer(first)];
        let b = &self.indices[self.buffer(second)];
        if a.count == patterns || b.count == patterns {
            // A child with no repeats makes every pair unique, regardless of
            // its sibling. Identity maps need no table construction or fill.
            index.count = patterns;
            index.version = 0;
        } else {
            self.stats.index_rebuilds += 1;
            self.table.clear();
            for p in 0..patterns {
                let key = (u64::from(a.id(p)) << 32) | u64::from(b.id(p));
                let mut slot = self.table.slot(key);
                while self.table.values[slot] != EMPTY && self.table.keys[slot] != key {
                    slot = (slot + 1) & (self.table.keys.len() - 1);
                }
                if self.table.values[slot] == EMPTY {
                    self.table.keys[slot] = key;
                    self.table.values[slot] = index.representatives.len() as u32;
                    index.representatives.push(p as u32);
                }
                index.ids[p] = self.table.values[slot];
            }
            index.count = index.representatives.len();
            index.version = if index.count == patterns {
                0
            } else {
                self.version()
            };
        }
        self.indices[target] = index;
        self.current[parent] ^= 1;
    }

    pub fn calculate(
        &mut self,
        core: &mut LikelihoodCore,
        a: usize,
        b: usize,
        parent: usize,
    ) -> bool {
        let l = core.layout;
        self.stats.node_updates += 1;
        self.stats.dense_pattern_lanes += l.stride as u64;
        self.prepare(a, b, parent, l.patterns);
        let index = &self.indices[self.buffer(parent)];
        let packed = Layout::new(l.states, l.categories, index.count);
        // Packing both children and expanding the result costs more than the
        // arithmetic saved when few patterns repeat. Retain the dense kernel
        // unless at least half of its padded lanes can be omitted.
        if packed.stride > l.stride / 2 {
            self.stats.evaluated_pattern_lanes += l.stride as u64;
            return false;
        }
        self.stats.evaluated_pattern_lanes += packed.stride as u64;
        self.stats.packed_updates += 1;
        let a_buffer = core.partials_buffer(a);
        let b_buffer = core.partials_buffer(b);
        for (source, dest) in [
            (&core.partials[a_buffer], &mut self.first),
            (&core.partials[b_buffer], &mut self.second),
        ] {
            for c in 0..l.categories {
                for s in 0..l.states {
                    let o = packed.offset(c, s);
                    for (p, &r) in index.representatives.iter().enumerate() {
                        dest[o + p] = source[l.offset(c, s) + r as usize];
                    }
                    dest[o + index.count..o + packed.stride].fill(0.0);
                }
            }
        }
        let children = [
            mk::Child {
                partials: &self.first,
                transition: &core.transitions[core.transitions_buffer(a)],
            },
            mk::Child {
                partials: &self.second,
                transition: &core.transitions[core.transitions_buffer(b)],
            },
        ];
        let Transition::Mk(model) = &core.transition else {
            unreachable!("repeat pruning requires Mk");
        };
        let scale = &mut core.scale[..packed.stride];
        let maxima = match core.backend {
            #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
            super::Backend::Neon => {
                super::neon::mk(
                    &packed,
                    model.frequencies(),
                    children,
                    &mut self.output,
                    (core.scaling == Scaling::Always).then_some(&mut *scale),
                );
                true
            }
            _ => {
                mk::prune(
                    packed,
                    model.frequencies(),
                    children,
                    &mut self.output,
                    &mut core.acc1[..packed.stride],
                    &mut core.acc2[..packed.stride],
                );
                false
            }
        };
        if core.scaling == Scaling::Always {
            if !maxima {
                scale.fill(0.0);
                for row in self.output[..packed.partials_len()].chunks_exact(packed.stride) {
                    for (m, x) in scale.iter_mut().zip(row) {
                        *m = m.max(*x);
                    }
                }
            }
            scaling::factors(scale, &mut self.exponents[..packed.stride], core.backend);
            for row in self.output[..packed.partials_len()].chunks_exact_mut(packed.stride) {
                for (x, inv) in row.iter_mut().zip(&*scale) {
                    *x *= inv;
                }
            }
            let out = core.partials_buffer(parent);
            let mut exponents = std::mem::take(&mut core.exponents[out]);
            for (p, total) in exponents[..l.patterns].iter_mut().enumerate() {
                *total = self.exponents[index.ids[p] as usize]
                    + core.exponents[a_buffer][p]
                    + core.exponents[b_buffer][p];
            }
            exponents[l.patterns..].fill(0);
            core.exponents[out] = exponents;
        }
        let out = core.partials_buffer(parent);
        for c in 0..l.categories {
            for s in 0..l.states {
                let row = &mut core.partials[out][l.offset(c, s)..l.offset(c, s) + l.stride];
                for (p, x) in row[..l.patterns].iter_mut().enumerate() {
                    *x = self.output[packed.offset(c, s) + index.ids[p] as usize];
                }
                row[l.patterns..].fill(0.0);
            }
        }
        true
    }

    pub fn store(&mut self) {
        self.stored.copy_from_slice(&self.current);
    }
    pub fn restore(&mut self) {
        std::mem::swap(&mut self.stored, &mut self.current);
    }
    pub fn unstore(&mut self) {
        self.current.copy_from_slice(&self.stored);
    }
}
