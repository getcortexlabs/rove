use std::io::{self, Cursor, Read, Write};

use super::{Sampler, Settings};
use crate::checkpoint::{invalid, read_f64, read_u64, write_f64, write_u64};
use crate::data::Patterns;
use crate::random::Random;
use crate::rates::{GammaRates, RateVariation, RelaxedClock};
use crate::tree::Tree;

const MAGIC: &[u8; 8] = b"ROVECP03";

impl Sampler {
    /// Saves a between-proposals state, including the live likelihood buffers.
    /// Float bits are preserved; this format is local to the current executable.
    pub fn checkpoint(&self, settings: &Settings) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();
        out.write_all(MAGIC)?;
        // A caller cannot accidentally resume with different model settings.
        let signature = format!("{settings:?}");
        write_u64(&mut out, signature.len() as u64)?;
        out.write_all(signature.as_bytes())?;
        write_partitions(&mut out, &self.patterns)?;
        write_u64(&mut out, self.steps)?;
        write_u64(&mut out, self.tunable_proposals)?;
        for value in self.random.state() {
            write_u64(&mut out, value)?;
        }
        for node in 0..self.tree.nodes() {
            let children = self.tree.children(node).unwrap_or((usize::MAX, usize::MAX));
            write_u64(&mut out, children.0 as u64)?;
            write_u64(&mut out, children.1 as u64)?;
            write_f64(&mut out, self.tree.height(node))?;
        }
        if let Some(gamma) = &self.rates.gamma {
            write_f64(&mut out, gamma.shape())?;
        }
        if let Some(clock) = &self.rates.clock {
            write_f64(&mut out, clock.sigma())?;
            for &rate in clock.rates() {
                write_f64(&mut out, rate)?;
            }
        }
        for value in [
            self.birth_death.birth_difference,
            self.birth_death.relative_death,
            self.posterior.likelihood,
            self.posterior.tree_prior,
            self.posterior.parameter_prior,
            self.posterior.gamma_prior,
            self.posterior.clock_rate_prior,
            self.posterior.clock_sigma_prior,
        ] {
            write_f64(&mut out, value)?;
        }
        for operator in &self.operators {
            if let Some(tuning) = operator.tuning {
                write_f64(&mut out, tuning)?;
            }
            for count in [
                operator.accepted,
                operator.rejected,
                operator.failed,
                operator.tuned,
            ] {
                write_u64(&mut out, count)?;
            }
        }
        for likelihood in &self.likelihoods {
            likelihood.core.save(&mut out)?;
        }
        Ok(out)
    }

    /// Restores with the same patterns and settings. The caller must verify
    /// the executable and input fingerprints before accepting a checkpoint.
    pub fn resume(
        patterns: impl AsRef<[Patterns]>,
        settings: &Settings,
        bytes: &[u8],
    ) -> io::Result<Self> {
        let patterns = patterns.as_ref();
        let mut input = Cursor::new(bytes);
        let mut magic = [0; 8];
        input.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(invalid());
        }
        let signature = format!("{settings:?}");
        if read_u64(&mut input)? != signature.len() as u64 {
            return Err(invalid());
        }
        let mut saved = vec![0; signature.len()];
        input.read_exact(&mut saved)?;
        if saved != signature.as_bytes() {
            return Err(invalid());
        }
        let mut partitions = Vec::new();
        write_partitions(&mut partitions, patterns)?;
        let mut saved = vec![0; partitions.len()];
        input.read_exact(&mut saved)?;
        if saved != partitions {
            return Err(invalid());
        }
        let steps = read_u64(&mut input)?;
        let tunable_proposals = read_u64(&mut input)?;
        let mut state = [0; 4];
        for value in &mut state {
            *value = read_u64(&mut input)?;
        }
        let random = Random::from_state(state).ok_or_else(invalid)?;
        let tips = patterns.first().ok_or_else(invalid)?.tip_partials().len();
        let nodes = tips
            .checked_mul(2)
            .and_then(|n| n.checked_sub(1))
            .ok_or_else(invalid)?;
        let mut children = Vec::with_capacity(nodes);
        let mut heights = Vec::with_capacity(nodes);
        for node in 0..nodes {
            let a = read_u64(&mut input)?;
            let b = read_u64(&mut input)?;
            if node < tips {
                if a != u64::MAX || b != u64::MAX {
                    return Err(invalid());
                }
                children.push(None);
            } else {
                if a >= nodes as u64 || b >= nodes as u64 {
                    return Err(invalid());
                }
                children.push(Some((a as usize, b as usize)));
            }
            heights.push(read_f64(&mut input)?);
        }
        if heights[..tips].iter().any(|&h| h != 0.0) {
            return Err(invalid());
        }
        let tree = Tree::new(children, heights).map_err(|_| invalid())?;
        let gamma = if settings.gamma_shape.is_some() {
            Some(GammaRates::new(read_f64(&mut input)?).map_err(|_| invalid())?)
        } else {
            None
        };
        let clock = if settings.clock_sigma.is_some() {
            let sigma = read_f64(&mut input)?;
            let rates = (0..nodes - 1)
                .map(|_| read_f64(&mut input))
                .collect::<io::Result<Vec<_>>>()?;
            Some(RelaxedClock::new(sigma, rates).map_err(|_| invalid())?)
        } else {
            None
        };
        let mut initial = settings.clone();
        initial.initial.birth_difference = read_f64(&mut input)?;
        initial.initial.relative_death = read_f64(&mut input)?;
        let mut sampler = Self::from_tree(
            patterns,
            &initial,
            tree,
            Some(RateVariation { gamma, clock }),
            random,
        )
        .map_err(|_| invalid())?;
        for value in [
            &mut sampler.posterior.likelihood,
            &mut sampler.posterior.tree_prior,
            &mut sampler.posterior.parameter_prior,
            &mut sampler.posterior.gamma_prior,
            &mut sampler.posterior.clock_rate_prior,
            &mut sampler.posterior.clock_sigma_prior,
        ] {
            *value = read_f64(&mut input)?;
            if !value.is_finite() {
                return Err(invalid());
            }
        }
        let mut proposals = 0u64;
        for operator in &mut sampler.operators {
            if let Some(tuning) = &mut operator.tuning {
                *tuning = read_f64(&mut input)?;
                if !tuning.is_finite() || *tuning <= 0.0 {
                    return Err(invalid());
                }
            }
            operator.accepted = read_u64(&mut input)?;
            operator.rejected = read_u64(&mut input)?;
            operator.failed = read_u64(&mut input)?;
            operator.tuned = read_u64(&mut input)?;
            for count in [operator.accepted, operator.rejected, operator.failed] {
                proposals = proposals.checked_add(count).ok_or_else(invalid)?;
            }
            if operator.tuned > steps {
                return Err(invalid());
            }
        }
        if proposals != steps
            || tunable_proposals > settings.tuning_delay
            || tunable_proposals > steps
        {
            return Err(invalid());
        }
        for likelihood in &mut sampler.likelihoods {
            likelihood.core.load(&mut input)?;
        }
        if input.position() != bytes.len() as u64 {
            return Err(invalid());
        }
        sampler.steps = steps;
        sampler.tunable_proposals = tunable_proposals;
        Ok(sampler)
    }
}

fn write_partitions(out: &mut impl Write, patterns: &[Patterns]) -> io::Result<()> {
    write_u64(out, patterns.len() as u64)?;
    for partition in patterns {
        write_u64(out, partition.symbols().len() as u64)?;
        write_u64(out, partition.sites().len() as u64)?;
        write_u64(out, partition.weights().len() as u64)?;
        write_u64(out, partition.tip_partials().len() as u64)?;
        for ((&site, symbols), &pattern) in partition
            .sites()
            .iter()
            .zip(partition.state_symbols())
            .zip(partition.site_to_pattern())
        {
            write_u64(out, site as u64)?;
            write_u64(out, pattern as u64)?;
            for &symbol in symbols {
                write_u64(out, symbol as u64)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GapTreatment;
    use crate::mcmc::Move;

    #[test]
    fn resumed_proposals_are_identical_with_live_buffers_and_tuning() {
        let data = crate::nexus::parse("#NEXUS BEGIN DATA; DIMENSIONS NTAX=5 NCHAR=4; MATRIX a 0111 b 0011 c 0001 d 1000 e 1010; END;").unwrap();
        let patterns = data
            .compress(std::slice::from_ref(&(0..4)), GapTreatment::Reject)
            .unwrap()
            .remove(0);
        for reference in [false, true] {
            for prior in [false, true] {
                let mut settings = if reference {
                    Settings::reference()
                } else {
                    Settings::default()
                };
                settings.sample_from_prior = prior;
                settings.tuning_delay = 10;
                if reference {
                    settings.weights.extend([
                        (Move::NodeTimeRates, 10.0),
                        (Move::RootTimeRates, 10.0),
                        (Move::ClockSigmaRates, 10.0),
                    ]);
                }
                let mut chain = Sampler::new(&patterns, &settings, 42).unwrap();
                for boundary in [0, 1, 9, 31, 501, 4000] {
                    while chain.steps() < boundary {
                        chain.step();
                    }
                    let bytes = chain.checkpoint(&settings).unwrap();
                    let mut resumed = Sampler::resume(&patterns, &settings, &bytes).unwrap();
                    for _ in 0..100 {
                        assert_eq!(chain.step(), resumed.step());
                        assert_eq!(chain.tree(), resumed.tree());
                        assert_eq!(chain.rates(), resumed.rates());
                        assert_eq!(chain.posterior(), resumed.posterior());
                        assert_eq!(chain.operators(), resumed.operators());
                    }
                    assert_eq!(
                        chain.checkpoint(&settings).unwrap(),
                        resumed.checkpoint(&settings).unwrap()
                    );
                    for length in [0, 8, bytes.len() - 1] {
                        assert!(Sampler::resume(&patterns, &settings, &bytes[..length]).is_err());
                    }
                }
            }
        }
    }
}
