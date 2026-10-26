use std::hint::black_box;
use std::time::{Duration, Instant};

use super::allocations::{self, Counts};

pub struct Settings {
    pub samples: usize,
    pub sample_time: Duration,
    pub warmup: Duration,
}

pub struct Measurement {
    pub counts: Counts,
    pub iterations: usize,
    pub elapsed: Vec<Duration>,
}

fn batch(operation: &mut impl FnMut() -> f64, iterations: usize) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(operation());
    }
    start.elapsed()
}

pub fn measure(settings: &Settings, mut operation: impl FnMut() -> f64) -> Measurement {
    let counts = allocations::observe(&mut operation);
    let start = Instant::now();
    while start.elapsed() < settings.warmup {
        black_box(operation());
    }

    // Use one calibrated batch size for every sample of this metric. Timer
    // reads and output formatting stay outside the per-operation loop.
    let mut iterations = 1;
    loop {
        if batch(&mut operation, iterations) >= settings.sample_time {
            break;
        }
        iterations = iterations
            .checked_mul(2)
            .expect("benchmark calibration overflow");
    }
    let elapsed = (0..settings.samples)
        .map(|_| batch(&mut operation, iterations))
        .collect();
    Measurement {
        counts,
        iterations,
        elapsed,
    }
}
