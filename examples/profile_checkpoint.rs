//! Profile a copy of a frozen terminal state. Never append to its source run.
#[path = "../src/bin/rove/sha256.rs"]
mod sha256;

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use rove::data::GapTreatment;
use rove::mcmc::{Conditioning, Sampler, Settings};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: profile_checkpoint <verified-job.json> <new-output-directory>".into());
    }
    let job: Value = serde_json::from_slice(&fs::read(&args[0])?)?;
    let input = PathBuf::from(job["input"].as_str().ok_or("missing input")?);
    let state = PathBuf::from(job["state"].as_str().ok_or("missing state")?);
    if sha256::file(&input)? != job["input_sha256"] || sha256::file(&state)? != job["state_sha256"]
    {
        return Err("input or state fingerprint changed".into());
    }
    let initial = job["initial_step"].as_u64().ok_or("missing initial step")?;
    let steps = job["steps"].as_u64().ok_or("missing continuation length")?;
    if initial < 100_000 || steps == 0 || initial.checked_add(steps).is_none() {
        return Err("expected a post-delay state and a bounded continuation".into());
    }
    let conditioning = match job["conditioning"].as_str() {
        Some("none") => Conditioning::None,
        Some("variable") => Conditioning::Variable,
        _ => return Err("conditioning must be none or variable".into()),
    };
    let matrix = rove::nexus::parse(&fs::read_to_string(input)?)?;
    let patterns = matrix.partition_by_states(GapTreatment::Missing)?;
    let settings = Settings {
        conditioning,
        ..Settings::reference()
    };
    let bytes = fs::read(state)?;
    // This benchmark deliberately compares two binaries with identical engine
    // sources. The driver checks that provenance; round-tripping additionally
    // verifies every saved float, buffer, tuning counter and RNG word.
    let mut sampler = Sampler::resume(&patterns, &settings, &bytes)?;
    if sampler.steps() != initial || sampler.checkpoint(&settings)? != bytes {
        return Err("terminal state did not round-trip exactly".into());
    }
    let out = PathBuf::from(&args[1]);
    fs::create_dir(&out)?;
    #[cfg(feature = "profile-sampler")]
    let profiler = rove::profiling::Session::start(out.join("profile.json"));
    #[cfg(feature = "profile-sampler")]
    rove::profiling::step(initial);
    let start = Instant::now();
    for _ in 0..steps {
        sampler.step();
        #[cfg(feature = "profile-sampler")]
        if sampler.steps() % 1000 == 0 {
            rove::profiling::step(sampler.steps());
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    #[cfg(feature = "profile-sampler")]
    profiler.finish()?;
    // No log/checkpoint I/O runs inside the measured loop. Hashing the final
    // complete state establishes identity without publishing private trees.
    let final_state_sha256 = sha256::hex(&sampler.checkpoint(&settings)?);
    let result = json!({"initial_step": initial, "final_step": sampler.steps(),
        "steps": steps, "sampling_seconds": seconds, "final_state_sha256": final_state_sha256,
        "input_sha256": job["input_sha256"], "source_state_sha256": job["state_sha256"],
        "state_round_trip_identical": true, "output_in_timed_loop": false,
        "instrumented": cfg!(feature = "profile-sampler")});
    fs::write(out.join("result.json"), serde_json::to_vec_pretty(&result)?)?;
    Ok(())
}
