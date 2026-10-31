//! Single-chain phase sampling for benchmark builds, without debugger access.
//!
//! This measures wall-clock phase occupancy, not hardware CPU counters. The
//! observer and phase stores perturb execution; always pair with a normal build.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::json;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Phase {
    Other,
    Tree,
    LikelihoodUpdate,
    Node,
    Rescale,
    Root,
    Priors,
    Random,
    Output,
}

const NAMES: [&str; 9] = [
    "other",
    "tree_and_bookkeeping",
    "likelihood_update",
    "node_partials",
    "rescaling",
    "root_likelihood",
    "priors",
    "random",
    "output",
];

#[repr(align(64))]
struct State {
    phase: AtomicU8,
    step: AtomicU64,
    stop: AtomicBool,
}
static STATE: State = State {
    phase: AtomicU8::new(0),
    step: AtomicU64::new(0),
    stop: AtomicBool::new(false),
};

pub struct Guard(u8);

#[inline]
pub fn enter(phase: Phase) -> Guard {
    let previous = STATE.phase.load(Ordering::Relaxed);
    STATE.phase.store(phase as u8, Ordering::Relaxed);
    Guard(previous)
}

impl Drop for Guard {
    #[inline]
    fn drop(&mut self) {
        STATE.phase.store(self.0, Ordering::Relaxed);
    }
}

pub fn step(value: u64) {
    STATE.step.store(value, Ordering::Relaxed);
}

pub struct Session {
    output: PathBuf,
    observer: JoinHandle<serde_json::Value>,
}

impl Session {
    pub fn start(output: PathBuf) -> Self {
        STATE.stop.store(false, Ordering::Relaxed);
        STATE.step.store(0, Ordering::Relaxed);
        STATE.phase.store(0, Ordering::Relaxed);
        let observer = thread::spawn(|| {
            let mut counts = [0_u64; NAMES.len()];
            let mut first = None;
            let mut last = None;
            let mut max_gap: f64 = 0.0;
            let start = Instant::now();
            while !STATE.stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_micros(1000));
                if STATE.step.load(Ordering::Relaxed) < 100_000 {
                    continue;
                }
                let now = start.elapsed().as_secs_f64();
                if let Some(previous) = last {
                    max_gap = max_gap.max(now - previous);
                }
                first.get_or_insert(now);
                last = Some(now);
                counts[STATE.phase.load(Ordering::Relaxed) as usize] += 1;
            }
            json!({"method": "single-chain wall-clock phase sampling; nominal 1 ms sleep",
                "warmup_steps": 100_000, "counts": NAMES.iter().zip(counts).collect::<std::collections::BTreeMap<_, _>>(),
                "samples": counts.iter().sum::<u64>(), "first_sample_seconds": first,
                "last_sample_seconds": last, "maximum_gap_seconds": max_gap,
                "observed_steps": STATE.step.load(Ordering::Relaxed)})
        });
        Self { output, observer }
    }

    pub fn finish(self) -> Result<(), Box<dyn std::error::Error>> {
        STATE.stop.store(true, Ordering::Relaxed);
        let value = self
            .observer
            .join()
            .map_err(|_| "profile observer panicked")?;
        // create_new keeps a previous measurement from being silently replaced.
        use std::io::Write;
        let mut out = std::fs::File::create_new(self.output)?;
        writeln!(out, "{}", serde_json::to_string_pretty(&value)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_scopes_restore_the_enclosing_phase() {
        STATE.phase.store(Phase::Other as u8, Ordering::Relaxed);
        {
            let _tree = enter(Phase::Tree);
            {
                let _rng = enter(Phase::Random);
            }
            assert_eq!(STATE.phase.load(Ordering::Relaxed), Phase::Tree as u8);
        }
        assert_eq!(STATE.phase.load(Ordering::Relaxed), Phase::Other as u8);
    }
}
