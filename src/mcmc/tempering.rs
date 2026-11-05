//! Replica exchange for `p(x) L(x)^beta`, with an untempered prior.
//!
//! Temperature slots retain their RNGs, proposal counters and adaptive tuning.
//! Accepted exchanges move the inference state and its likelihood buffers.
use std::error::Error;
use std::io::{self, Cursor, Read, Write};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};

use super::{Sampler, Settings};
use crate::checkpoint::{invalid, read_f64, read_u64, write_f64, write_u64};
use crate::data::Patterns;
use crate::random::Random;

const MAGIC: &[u8; 8] = b"ROVEMC01";
const SWAP_SEED_MASK: u64 = 0xd1b5_4a32_d192_ed03;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SwapCount {
    pub attempted: u64,
    pub accepted: u64,
}

pub struct Ensemble {
    chains: Vec<Sampler>,
    powers: Vec<f64>,
    swap_every: u64,
    random: Random,
    swaps: Vec<SwapCount>,
    labels: Vec<usize>,
    // Per state label: 0 = has not visited cold; 1 = cold visited;
    // 2 = hottest visited after cold, awaiting the return to cold.
    journeys: Vec<u8>,
    round_trips: Vec<u64>,
}

fn validate(powers: &[f64], every: u64) -> io::Result<()> {
    if powers.is_empty()
        || powers.len() > 16
        || powers[0] != 1.0
        || powers.iter().any(|p| !p.is_finite() || *p <= 0.0)
        || powers.windows(2).any(|p| p[0] <= p[1])
        || every == 0
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "use 1 to 16 strictly decreasing positive likelihood powers starting at 1, and a positive swap interval",
        ));
    }
    Ok(())
}

/// Symmetric neighbour proposal: both copies of the untempered prior cancel.
pub fn swap_log_ratio(beta_i: f64, beta_j: f64, log_l_i: f64, log_l_j: f64) -> f64 {
    (beta_i - beta_j) * (log_l_j - log_l_i)
}

impl Ensemble {
    pub fn new(
        patterns: impl AsRef<[Patterns]>,
        settings: &Settings,
        powers: &[f64],
        swap_every: u64,
        seed: u64,
    ) -> Result<Self, Box<dyn Error>> {
        validate(powers, swap_every)?;
        if settings.likelihood_power != 1.0 {
            return Err("the ensemble's base settings must have likelihood power one".into());
        }
        let chains = powers
            .iter()
            .enumerate()
            .map(|(i, &power)| {
                let mut slot = settings.clone();
                slot.likelihood_power = power;
                Sampler::new(patterns.as_ref(), &slot, seed.wrapping_add(i as u64))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut journeys = vec![0; powers.len()];
        journeys[0] = 1;
        Ok(Self {
            chains,
            powers: powers.to_vec(),
            swap_every,
            random: Random::new(seed ^ SWAP_SEED_MASK),
            swaps: vec![SwapCount::default(); powers.len() - 1],
            labels: (0..powers.len()).collect(),
            journeys,
            round_trips: vec![0; powers.len()],
        })
    }

    pub fn cold(&self) -> &Sampler {
        &self.chains[0]
    }

    pub fn chains(&self) -> &[Sampler] {
        &self.chains
    }

    pub fn powers(&self) -> &[f64] {
        &self.powers
    }

    pub fn swap_every(&self) -> u64 {
        self.swap_every
    }

    pub fn swaps(&self) -> &[SwapCount] {
        &self.swaps
    }

    pub fn state_labels(&self) -> &[usize] {
        &self.labels
    }

    pub fn round_trips(&self) -> &[u64] {
        &self.round_trips
    }

    /// Advance all slots to an absolute step. One persistent worker per slot
    /// runs each batch; completion order never changes the swap RNG stream.
    /// The visitor runs at the initial state and at sampling/exchange boundaries.
    /// Returning false stops cleanly. On a worker error, recover a saved checkpoint.
    pub fn run(
        &mut self,
        until: u64,
        sample_every: u64,
        parallel: bool,
        mut visit: impl FnMut(&Self) -> io::Result<bool>,
    ) -> io::Result<()> {
        if sample_every == 0 || until < self.cold().steps() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid run boundary",
            ));
        }
        if !visit(self)? {
            return Ok(());
        }
        let pool = if parallel && self.chains.len() > 1 {
            Some(Pool::new(self.chains.len())?)
        } else {
            None
        };
        while self.cold().steps() < until {
            let step = self.cold().steps();
            let count = (until - step)
                .min(sample_every - step % sample_every)
                .min(self.swap_every - step % self.swap_every)
                .min(1024);
            if let Some(pool) = &pool {
                for (worker, chain) in pool.workers.iter().zip(self.chains.drain(..)) {
                    worker
                        .request
                        .as_ref()
                        .unwrap()
                        .send((chain, count))
                        .map_err(|_| io::Error::other("coupled-chain worker disconnected"))?;
                }
                for worker in &pool.workers {
                    self.chains.push(
                        worker
                            .reply
                            .recv()
                            .map_err(|_| io::Error::other("coupled-chain worker disconnected"))?
                            .map_err(io::Error::other)?,
                    );
                }
            } else {
                for chain in &mut self.chains {
                    for _ in 0..count {
                        chain.step();
                    }
                }
            }
            if self.cold().steps() % self.swap_every == 0 && self.chains.len() > 1 {
                self.exchange();
            }
            if !visit(self)? {
                break;
            }
        }
        Ok(())
    }

    fn exchange(&mut self) {
        let i = (self.random.uniform() * self.swaps.len() as f64) as usize;
        let ratio = swap_log_ratio(
            self.powers[i],
            self.powers[i + 1],
            self.chains[i].posterior.likelihood,
            self.chains[i + 1].posterior.likelihood,
        );
        let counter = &mut self.swaps[i];
        counter.attempted += 1;
        if ratio >= 0.0 || self.random.uniform().ln() < ratio {
            counter.accepted += 1;
            let (left, right) = self.chains.split_at_mut(i + 1);
            swap_state(&mut left[i], &mut right[0]);
            self.labels.swap(i, i + 1);
        }
        self.track_journeys();
    }

    fn track_journeys(&mut self) {
        let cold = self.labels[0];
        if self.journeys[cold] == 2 {
            self.round_trips[cold] += 1;
        }
        self.journeys[cold] = 1;
        let hot = *self.labels.last().unwrap();
        if self.journeys[hot] == 1 {
            self.journeys[hot] = 2;
        }
    }

    pub fn checkpoint(&self, settings: &Settings) -> io::Result<Vec<u8>> {
        if settings.likelihood_power != 1.0 || self.chains.len() != self.powers.len() {
            return Err(invalid());
        }
        let mut out = Vec::new();
        out.write_all(MAGIC)?;
        write_u64(&mut out, self.chains.len() as u64)?;
        write_u64(&mut out, self.swap_every)?;
        for value in self.random.state() {
            write_u64(&mut out, value)?;
        }
        for (i, chain) in self.chains.iter().enumerate() {
            write_f64(&mut out, self.powers[i])?;
            write_u64(&mut out, self.labels[i] as u64)?;
            write_u64(&mut out, self.journeys[i] as u64)?;
            write_u64(&mut out, self.round_trips[i])?;
            let mut slot = settings.clone();
            slot.likelihood_power = self.powers[i];
            let bytes = chain.checkpoint(&slot)?;
            write_u64(&mut out, bytes.len() as u64)?;
            out.write_all(&bytes)?;
        }
        for edge in &self.swaps {
            write_u64(&mut out, edge.attempted)?;
            write_u64(&mut out, edge.accepted)?;
        }
        Ok(out)
    }

    /// The caller verifies the original executable and input fingerprints.
    pub fn resume(
        patterns: impl AsRef<[Patterns]>,
        settings: &Settings,
        bytes: &[u8],
    ) -> io::Result<Self> {
        if settings.likelihood_power != 1.0 {
            return Err(invalid());
        }
        let mut input = Cursor::new(bytes);
        let mut magic = [0; 8];
        input.read_exact(&mut magic)?;
        let n = read_u64(&mut input)? as usize;
        if &magic != MAGIC || !(1..=16).contains(&n) {
            return Err(invalid());
        }
        let swap_every = read_u64(&mut input)?;
        let mut rng = [0; 4];
        for value in &mut rng {
            *value = read_u64(&mut input)?;
        }
        let random = Random::from_state(rng).ok_or_else(invalid)?;
        let mut result = Self {
            chains: Vec::with_capacity(n),
            powers: Vec::with_capacity(n),
            swap_every,
            random,
            swaps: Vec::with_capacity(n - 1),
            labels: Vec::with_capacity(n),
            journeys: Vec::with_capacity(n),
            round_trips: Vec::with_capacity(n),
        };
        for _ in 0..n {
            let power = read_f64(&mut input)?;
            let label = read_u64(&mut input)?;
            let journey = read_u64(&mut input)?;
            let trips = read_u64(&mut input)?;
            let size = read_u64(&mut input)?;
            if label >= n as u64 || journey > 2 || size > bytes.len() as u64 - input.position() {
                return Err(invalid());
            }
            let start = input.position() as usize;
            let mut slot = settings.clone();
            slot.likelihood_power = power;
            result.chains.push(Sampler::resume(
                patterns.as_ref(),
                &slot,
                &bytes[start..start + size as usize],
            )?);
            input.set_position(start as u64 + size);
            result.powers.push(power);
            result.labels.push(label as usize);
            result.journeys.push(journey as u8);
            result.round_trips.push(trips);
        }
        validate(&result.powers, swap_every)?;
        let step = result.cold().steps();
        let mut labels = result.labels.clone();
        labels.sort_unstable();
        if labels != (0..n).collect::<Vec<_>>() || result.chains.iter().any(|c| c.steps() != step) {
            return Err(invalid());
        }
        let mut attempts = 0_u64;
        for _ in 1..n {
            let attempted = read_u64(&mut input)?;
            let accepted = read_u64(&mut input)?;
            if accepted > attempted {
                return Err(invalid());
            }
            attempts = attempts.checked_add(attempted).ok_or_else(invalid)?;
            result.swaps.push(SwapCount {
                attempted,
                accepted,
            });
        }
        if input.position() != bytes.len() as u64 || (n > 1 && attempts != step / swap_every) {
            return Err(invalid());
        }
        Ok(result)
    }
}

fn swap_state(a: &mut Sampler, b: &mut Sampler) {
    use std::mem::swap;
    swap(&mut a.tree, &mut b.tree);
    swap(&mut a.rates, &mut b.rates);
    swap(&mut a.birth_death, &mut b.birth_death);
    swap(&mut a.fbd, &mut b.fbd);
    swap(&mut a.posterior, &mut b.posterior);
    swap(&mut a.likelihoods, &mut b.likelihoods);
    swap(&mut a.stored_tree, &mut b.stored_tree);
    swap(&mut a.stored_rates, &mut b.stored_rates);
}

struct Worker {
    request: Option<SyncSender<(Sampler, u64)>>,
    reply: Receiver<Result<Sampler, &'static str>>,
    thread: Option<JoinHandle<()>>,
}

struct Pool {
    workers: Vec<Worker>,
}

impl Pool {
    fn new(count: usize) -> io::Result<Self> {
        let mut pool = Self {
            workers: Vec::with_capacity(count),
        };
        for i in 0..count {
            let (send, jobs) = mpsc::sync_channel::<(Sampler, u64)>(1);
            let (finished, reply) = mpsc::sync_channel(1);
            let thread = thread::Builder::new()
                .name(format!("rove-temperature-{i}"))
                .spawn(move || {
                    while let Ok((mut chain, steps)) = jobs.recv() {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            for _ in 0..steps {
                                chain.step();
                            }
                            chain
                        }))
                        .map_err(|_| "coupled-chain worker panicked");
                        let failed = result.is_err();
                        if finished.send(result).is_err() || failed {
                            break;
                        }
                    }
                })?;
            pool.workers.push(Worker {
                request: Some(send),
                reply,
                thread: Some(thread),
            });
        }
        Ok(pool)
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            worker.request.take();
        }
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GapTreatment;
    use crate::dating::{Dating, FossilizedBirthDeath, ProcessPriors, TipAge};
    use crate::mcmc::{Conditioning, Move};
    use crate::tree::Tree;

    fn patterns() -> Vec<Patterns> {
        crate::nexus::parse(include_str!("../../fixtures/reference/20x50/matrix.nex"))
            .unwrap()
            .partition_by_states(GapTreatment::Missing)
            .unwrap()
    }

    #[test]
    fn a_single_power_one_slot_is_the_original_chain() {
        let p = patterns();
        let settings = Settings::reference();
        let mut original = Sampler::new(&p, &settings, 71).unwrap();
        let mut ensemble = Ensemble::new(&p, &settings, &[1.0], 17, 71).unwrap();
        for _ in 0..2003 {
            original.step();
        }
        ensemble.run(2003, 13, true, |_| Ok(true)).unwrap();
        assert_eq!(
            original.checkpoint(&settings).unwrap(),
            ensemble.cold().checkpoint(&settings).unwrap()
        );
        assert!(ensemble.swaps().is_empty());
        assert_eq!(ensemble.round_trips(), &[0]);
    }

    #[test]
    fn swaps_balance_the_product_target_with_the_prior_unheated() {
        for (li, lj, pi, pj) in [(-2.0, -8.0, -100.0, 3.0), (-60.0, -1.0, 7.0, -30.0)] {
            for (bi, bj) in [(1.0, 0.3), (0.9, 0.5), (1.0, 1.0)] {
                let forward = swap_log_ratio(bi, bj, li, lj);
                let reverse = swap_log_ratio(bi, bj, lj, li);
                let before = pi + bi * li + pj + bj * lj;
                let after = pj + bi * lj + pi + bj * li;
                assert!((forward - (after - before)).abs() < 1e-12);
                assert!((before + forward.min(0.0) - after - reverse.min(0.0)).abs() < 1e-12);
            }
        }
    }

    fn assert_recomputed(ensemble: &Ensemble) {
        for c in ensemble.chains() {
            let fresh = c.recompute().unwrap();
            assert!((fresh.likelihood - c.posterior().likelihood).abs() < 1e-8);
            assert!((fresh.prior() - c.posterior().prior()).abs() < 1e-8);
        }
    }

    #[test]
    fn parallel_and_serial_execution_and_restart_are_bit_identical() {
        let p = patterns();
        let mut s = Settings::reference();
        s.monophyly = vec![vec![0, 1]];
        s.tuning_delay = 0;
        let powers = [1.0, 0.9, 0.7, 0.4];
        let mut serial = Ensemble::new(&p, &s, &powers, 17, 100).unwrap();
        let mut parallel = Ensemble::new(&p, &s, &powers, 17, 100).unwrap();
        serial
            .run(4003, 13, false, |e| {
                assert_recomputed(e);
                Ok(true)
            })
            .unwrap();
        parallel.run(1937, 13, true, |_| Ok(true)).unwrap();
        let saved = parallel.checkpoint(&s).unwrap();
        let mut resumed = Ensemble::resume(&p, &s, &saved).unwrap();
        assert_eq!(saved, resumed.checkpoint(&s).unwrap());
        resumed.run(4003, 13, true, |_| Ok(true)).unwrap();
        assert_eq!(
            serial.checkpoint(&s).unwrap(),
            resumed.checkpoint(&s).unwrap()
        );
        assert!(serial.swaps.iter().map(|s| s.accepted).sum::<u64>() > 20);
        assert_eq!(
            serial.swaps.iter().map(|s| s.attempted).sum::<u64>(),
            4003 / 17
        );
        assert!(Ensemble::resume(&p, &s, &saved[..saved.len() - 1]).is_err());
        let mut wrong = s.clone();
        wrong.conditioning = Conditioning::None;
        assert!(Ensemble::resume(&p, &wrong, &saved).is_err());
    }

    #[test]
    fn inference_state_moves_but_randomness_and_tuning_stay_at_the_temperature() {
        let p = patterns();
        let s = Settings::reference();
        let mut e = Ensemble::new(&p, &s, &[1.0, 0.6], 10, 29).unwrap();
        e.run(113, 13, false, |_| Ok(true)).unwrap();
        let rng = e.cold().random.state();
        let operators = e.cold().operators.clone();
        let posterior = e.chains[1].posterior;
        let (a, b) = e.chains.split_at_mut(1);
        swap_state(&mut a[0], &mut b[0]);
        assert_eq!(e.cold().random.state(), rng);
        assert_eq!(e.cold().operators, operators);
        assert_eq!(e.cold().posterior.total(), posterior.total());
        assert_eq!(e.cold().likelihood_power(), 1.0);
        assert_recomputed(&e);
        e.run(157, 13, false, |_| Ok(true)).unwrap();
        assert_recomputed(&e);
    }

    #[test]
    fn exchanges_preserve_partitioned_ordered_and_informative_likelihoods() {
        for (matrix, conditioning) in [
            (
                include_str!("../../fixtures/validation/multistate/matrix.nex"),
                Conditioning::Variable,
            ),
            (
                include_str!("../../fixtures/validation/ordered/matrix.nex"),
                Conditioning::Variable,
            ),
            (
                include_str!("../../fixtures/validation/informative/matrix.nex"),
                Conditioning::Informative,
            ),
        ] {
            let p = crate::nexus::parse(matrix)
                .unwrap()
                .partition_by_states(GapTreatment::Missing)
                .unwrap();
            let mut s = Settings::reference();
            s.conditioning = conditioning;
            let mut e = Ensemble::new(&p, &s, &[1.0, 0.9, 0.7, 0.4], 13, 741).unwrap();
            e.run(203, 17, true, |e| {
                assert_recomputed(e);
                Ok(true)
            })
            .unwrap();
            let mut resumed = Ensemble::resume(&p, &s, &e.checkpoint(&s).unwrap()).unwrap();
            e.run(1001, 17, true, |e| {
                assert_recomputed(e);
                Ok(true)
            })
            .unwrap();
            resumed.run(1001, 17, false, |_| Ok(true)).unwrap();
            assert_eq!(e.checkpoint(&s).unwrap(), resumed.checkpoint(&s).unwrap());
            assert!(e.swaps().iter().any(|edge| edge.accepted > 0));
        }
    }

    #[test]
    fn round_trips_require_cold_then_hot_then_cold() {
        let mut e =
            Ensemble::new(patterns(), &Settings::default(), &[1.0, 0.5, 0.1], 10, 1).unwrap();
        e.labels.swap(0, 1);
        e.track_journeys();
        e.labels.swap(1, 2);
        e.track_journeys();
        assert_eq!(e.round_trips(), &[0, 0, 0]);
        e.labels.swap(1, 2);
        e.track_journeys();
        e.labels.swap(0, 1);
        e.track_journeys();
        assert_eq!(e.round_trips(), &[1, 0, 0]);
        e.track_journeys();
        assert_eq!(e.round_trips(), &[1, 0, 0]);
    }

    #[test]
    fn dated_process_parameters_and_tip_ages_survive_swaps() {
        let matrix = crate::nexus::parse("#NEXUS BEGIN DATA; DIMENSIONS NTAX=3 NCHAR=4; FORMAT SYMBOLS=\"01\"; MATRIX A 0011 B 1100 C 0101; END;").unwrap();
        let p = matrix.partition_by_states(GapTreatment::Missing).unwrap();
        let mut s = Settings {
            conditioning: Conditioning::None,
            ..Settings::reference()
        };
        s.set_dating(Dating {
            process: FossilizedBirthDeath {
                birth: 0.8,
                death: 0.3,
                fossil_sampling: 0.2,
                present_sampling: 0.7,
                origin: 4.0,
            },
            process_priors: Some(ProcessPriors {
                birth_rate_mean: Some(1.0),
                death_rate_mean: Some(0.5),
                fossil_sampling_rate_mean: Some(0.3),
                present_sampling_beta: Some((2.0, 3.0)),
                origin_excess_mean: Some(2.0),
            }),
            ages: vec![
                TipAge {
                    minimum: 0.0,
                    maximum: 0.0,
                },
                TipAge {
                    minimum: 0.6,
                    maximum: 0.8,
                },
                TipAge {
                    minimum: 0.0,
                    maximum: 0.0,
                },
            ],
            clock_rate: 0.1,
            clock_prior_mean: Some(0.1),
        });
        let mut e = Ensemble::new(&p, &s, &[1.0, 0.7, 0.3], 11, 79).unwrap();
        e.run(2000, 25, true, |e| {
            assert_recomputed(e);
            Ok(true)
        })
        .unwrap();
        let saved = e.checkpoint(&s).unwrap();
        let mut resumed = Ensemble::resume(&p, &s, &saved).unwrap();
        e.run(2500, 25, false, |_| Ok(true)).unwrap();
        resumed.run(2500, 25, true, |_| Ok(true)).unwrap();
        assert_eq!(e.checkpoint(&s).unwrap(), resumed.checkpoint(&s).unwrap());
        assert_recomputed(&resumed);
    }

    #[test]
    fn cold_height_distribution_matches_numerical_quadrature() {
        let matrix = crate::nexus::parse("#NEXUS BEGIN DATA; DIMENSIONS NTAX=3 NCHAR=7; FORMAT SYMBOLS=\"01\"; MATRIX A 0011010 B 0011101 C 1100110; END;").unwrap();
        let p = matrix.partition_by_states(GapTreatment::Missing).unwrap();
        let s = Settings {
            conditioning: Conditioning::None,
            weights: vec![(Move::NodeHeight, 1.0)],
            tuning: false,
            ..Settings::default()
        };
        let tree = |h| {
            Tree::new(
                vec![None, None, None, Some((0, 1)), Some((3, 2))],
                vec![0.0, 0.0, 0.0, h, 1.0],
            )
            .unwrap()
        };
        let mut weight = 0.0;
        let mut moment = 0.0;
        for cell in 0..4000 {
            let h = (cell as f64 + 0.5) / 4000.0;
            let c = Sampler::with_tree(&p, &s, tree(h), 1).unwrap();
            let w = c.posterior.total().exp();
            weight += w;
            moment += w * h;
        }
        let expected = moment / weight;
        let mut e = Ensemble::new(&p, &s, &[1.0, 0.6, 0.2], 20, 103).unwrap();
        for (i, chain) in e.chains.iter_mut().enumerate() {
            let mut slot = s.clone();
            slot.likelihood_power = e.powers[i];
            *chain = Sampler::with_tree(&p, &slot, tree(0.5), 103 + i as u64).unwrap();
        }
        let mut values = Vec::new();
        e.run(210_000, 10, false, |e| {
            if e.cold().steps() > 10_000 && e.cold().steps() % 10 == 0 {
                values.push(e.cold().tree.height(3));
            }
            Ok(true)
        })
        .unwrap();
        let stats = crate::diagnostics::statistics(&values, &|| false).unwrap();
        let mean = stats.mean;
        let variance =
            values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
        let se = (variance / stats.ess.unwrap()).sqrt();
        assert!(stats.ess.unwrap() > 1000.0);
        assert!(
            (mean - expected).abs() < 5.0 * se,
            "{mean} vs {expected}, SE {se}"
        );
        assert!(e.round_trips.iter().sum::<u64>() > 100);
    }

    #[test]
    fn invalid_ladders_are_rejected_before_sampling() {
        for powers in [
            vec![],
            vec![0.9],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
            vec![1.0, f64::NAN],
            vec![1.0, 0.3, 0.4],
        ] {
            assert!(Ensemble::new(patterns(), &Settings::default(), &powers, 100, 1).is_err());
        }
        assert!(Ensemble::new(patterns(), &Settings::default(), &[1.0], 0, 1).is_err());
    }
}
