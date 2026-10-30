use std::error::Error;
use std::io::{self, Write};
use std::sync::Barrier;
use std::time::Instant;

use rove::data::{GapTreatment, Patterns};
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::{Backend, nexus};
use serde_json::{Value, json};

use super::{events::Cancellation, sha256};

const INPUT: &str = include_str!("../../../fixtures/benchmarks/morphology_90x250.nex");
const PROTOCOL: &str = include_str!("../../../fixtures/benchmarks/mac-v1.json");

fn emit(value: Value) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{value}")?;
    out.flush()
}

fn measure(
    patterns: &[Patterns],
    chains: usize,
    warmup: u64,
    steps: u64,
    seed: u64,
    cancel: &Cancellation,
) -> Result<Option<Value>, Box<dyn Error>> {
    let settings = Settings {
        conditioning: Conditioning::None,
        ..Settings::reference()
    };
    let samplers = (0..chains)
        .map(|chain| Sampler::new(patterns, &settings, seed + chain as u64))
        .collect::<Result<Vec<_>, _>>()?;
    let ready = Barrier::new(chains + 1);
    let start = Barrier::new(chains + 1);
    let (seconds, checksums) = std::thread::scope(|scope| {
        let handles: Vec<_> = samplers
            .into_iter()
            .map(|mut sampler| {
                let (ready, start) = (&ready, &start);
                scope.spawn(move || {
                    for step in 0..warmup {
                        if step % 128 == 0 && (cancel.requested() || cancel.paused()) {
                            break;
                        }
                        sampler.step();
                    }
                    // Even a cancelled warm-up reaches both barriers to release the owner.
                    ready.wait();
                    start.wait();
                    for step in 0..steps {
                        if step % 128 == 0 && (cancel.requested() || cancel.paused()) {
                            break;
                        }
                        sampler.step();
                    }
                    let posterior = sampler.posterior();
                    let values = [posterior.likelihood, posterior.prior(), posterior.total()];
                    assert!(values.iter().all(|v| v.is_finite()));
                    let bytes: Vec<_> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
                    sha256::hex(&bytes)
                })
            })
            .collect();
        ready.wait();
        let began = Instant::now();
        start.wait();
        let checksums: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        (began.elapsed().as_secs_f64(), checksums)
    });
    if cancel.requested() || cancel.paused() {
        return Ok(None);
    }
    Ok(Some(
        json!({"chains": chains, "seconds": seconds, "checksums": checksums}),
    ))
}

pub(super) fn run(arguments: &[String]) -> Result<bool, Box<dyn Error>> {
    if !arguments.is_empty() && arguments != ["--cancel-on-stdin-close"] {
        return Err("usage: rove benchmark [--cancel-on-stdin-close]".into());
    }
    let cancel = Cancellation::watch_stdin(!arguments.is_empty());
    let protocol: Value = serde_json::from_str(PROTOCOL)?;
    if protocol["inputSHA256"] != sha256::hex(INPUT.as_bytes()) {
        return Err("benchmark input does not match its protocol".into());
    }
    let patterns = nexus::parse(INPUT)?.partition_by_states(GapTreatment::Reject)?;
    let engine = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "sha256": sha256::file(&std::env::current_exe()?)?,
        "architecture": std::env::consts::ARCH,
        "os": std::env::consts::OS,
        "optimized": !cfg!(debug_assertions),
        "experimentalRepeats": cfg!(feature = "experimental-repeats"),
        "backend": Backend::NATIVE.name()
    });
    emit(
        json!({"event":"started", "protocol":protocol, "engine":engine,
        "protocolSHA256":sha256::hex(PROTOCOL.as_bytes())}),
    )?;
    for chains in protocol["chains"]
        .as_array()
        .ok_or("invalid benchmark protocol")?
    {
        for repetition in 0..protocol["repetitions"].as_u64().unwrap() {
            emit(json!({"event":"warming", "chains":chains, "repetition":repetition}))?;
            let sample = measure(
                &patterns,
                chains.as_u64().unwrap() as usize,
                protocol["warmupSteps"].as_u64().unwrap(),
                protocol["measuredSteps"].as_u64().unwrap(),
                protocol["seed"].as_u64().unwrap(),
                &cancel,
            )?;
            let Some(mut sample) = sample else {
                emit(json!({"event":"cancelled"}))?;
                return Ok(false);
            };
            sample["repetition"] = json!(repetition);
            emit(json!({"event":"measurement", "sample":sample}))?;
        }
    }
    emit(json!({"event":"completed"}))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workloads_repeat_the_same_fixed_work_and_cancel_without_a_sample() {
        let patterns = nexus::parse(INPUT)
            .unwrap()
            .partition_by_states(GapTreatment::Reject)
            .unwrap();
        let cancel = Cancellation::watch_stdin(false);
        let a = measure(&patterns, 1, 5, 20, 42, &cancel).unwrap().unwrap();
        let b = measure(&patterns, 4, 5, 20, 42, &cancel).unwrap().unwrap();
        let c = measure(&patterns, 4, 5, 20, 42, &cancel).unwrap().unwrap();
        assert_eq!(a["checksums"][0], b["checksums"][0]);
        assert_eq!(b["checksums"], c["checksums"]);
        assert!(a["seconds"].as_f64().unwrap() > 0.0);
        cancel.cancel();
        assert!(measure(&patterns, 4, 5, 20, 42, &cancel).unwrap().is_none());
    }
}
