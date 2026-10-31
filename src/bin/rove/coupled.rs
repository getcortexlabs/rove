//! One independent MC³ ensemble; only its power-one samples are posterior draws.
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rove::mcmc::output::{TraceWriter, TreeWriter};
use rove::mcmc::tempering::Ensemble;
use serde_json::{Value, json};

use super::{
    Configured, Json, Options, checkpoint, configure, events, input_json, model_json, moves_json,
    parse_options, progress, sha256,
};

const TARGET: &str = "prior(x) * likelihood(x)^beta; prior untempered; only beta=1 is logged";

fn value(v: Json) -> Value {
    let mut text = String::new();
    v.write(&mut text, 0);
    serde_json::from_str(&text).expect("run metadata is valid JSON")
}

fn parse(arguments: &[String]) -> Result<(Options, Vec<f64>, u64), Box<dyn Error>> {
    let mut common = vec!["sample".to_string()];
    let mut powers = None;
    let mut interval = None;
    let mut iter = arguments.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--likelihood-powers" => {
                if powers.is_some() {
                    return Err("--likelihood-powers is repeated".into());
                }
                powers = Some(
                    iter.next()
                        .ok_or("--likelihood-powers needs a list")?
                        .split(',')
                        .map(str::parse::<f64>)
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            "--swap-every" => {
                if interval.is_some() {
                    return Err("--swap-every is repeated".into());
                }
                interval = Some(
                    iter.next()
                        .ok_or("--swap-every needs an integer")?
                        .parse::<u64>()?,
                );
            }
            "--chains"
            | "--stop-when-converged"
            | "--min-ess"
            | "--max-rhat"
            | "--check-every"
            | "--burnin-percent" => {
                return Err(format!("{arg} is not an ensemble option; compare independently seeded cold runs with rove compare").into());
            }
            _ => common.push(arg.clone()),
        }
    }
    let options = parse_options(&common)?;
    Ok((
        options,
        powers.ok_or("--likelihood-powers is required")?,
        interval.unwrap_or(1000),
    ))
}

fn canonical_command(
    arguments: &[String],
    options: &Options,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut result = vec!["coupled".into()];
    let mut args = arguments.iter().skip(1);
    while let Some(arg) = args.next() {
        if !arg.starts_with("--") {
            result.push(options.matrix.canonicalize()?.display().to_string());
        } else if matches!(
            arg.as_str(),
            "--no-tuning" | "--sample-from-prior" | "--events-json" | "--cancel-on-stdin-close"
        ) {
            result.push(arg.clone());
        } else {
            let text = args.next().ok_or("missing option value")?;
            if matches!(arg.as_str(), "--dating" | "--dating-json") {
                result.extend([
                    "--dating-json".into(),
                    serde_json::to_string(&options.dating)?,
                ]);
                continue;
            }
            let text = match arg.as_str() {
                "--output" => options.output.display().to_string(),
                _ => text.clone(),
            };
            result.extend([arg.clone(), text]);
        }
    }
    Ok(result)
}

fn diagnostic(ensemble: &Ensemble) -> Value {
    json!({"target": TARGET, "likelihood_powers": ensemble.powers(),
        "swap_every": ensemble.swap_every(), "proposal": "one uniformly selected adjacent pair per exchange boundary",
        "acceptance": "log alpha = (beta_i - beta_j) * (log_likelihood_j - log_likelihood_i)",
        "state_labels_at_powers": ensemble.state_labels(),
        "cold_hot_cold_round_trips_per_state": ensemble.round_trips(),
        "round_trip_definition": "state visits cold, then hottest, then cold; initial hot-to-cold transit alone does not count",
        "swaps": ensemble.swaps().iter().enumerate().map(|(i,s)| json!({"lower_index":i,"upper_index":i+1,"attempted":s.attempted,"accepted":s.accepted})).collect::<Vec<_>>(),
        "operators_per_power": ensemble.chains().iter().map(|s| value(moves_json(s))).collect::<Vec<_>>()})
}

fn load(
    arguments: &[String],
    executable: &str,
) -> Result<(Vec<String>, Value, checkpoint::Lock), Box<dyn Error>> {
    if arguments.get(1).map(String::as_str) != Some("--resume")
        || arguments.len() < 3
        || arguments[3..]
            .iter()
            .any(|a| !matches!(a.as_str(), "--events-json" | "--cancel-on-stdin-close"))
    {
        return Err(
            "use rove coupled --resume <directory> with optional event/cancellation flags".into(),
        );
    }
    let directory = Path::new(&arguments[2]).canonicalize()?;
    let guard = checkpoint::Lock::acquire(&directory)?;
    let path = directory.join("coupling.json");
    if fs::metadata(&path)?.len() > 1024 * 1024 {
        return Err("coupling manifest exceeds 1 MiB".into());
    }
    let mut manifest: Value = serde_json::from_slice(&fs::read(path)?)?;
    let digest = manifest
        .as_object_mut()
        .ok_or("invalid coupling manifest")?
        .remove("manifest_sha256")
        .ok_or("missing manifest fingerprint")?;
    if sha256::hex(&serde_json::to_vec(&manifest)?) != digest
        || manifest["kind"] != "coupled_checkpoint"
        || manifest["schema_version"] != 1
        || manifest["executable_sha256"] != executable
    {
        return Err("coupling checkpoint changed or belongs to another executable".into());
    }
    let record: Value = serde_json::from_slice(&fs::read(directory.join("run.json"))?)?;
    if record["status"] == "completed" {
        return Err("this coupled run already completed".into());
    }
    let mut command: Vec<String> = serde_json::from_value(manifest["arguments"].clone())?;
    // Ownership of stdin and event consumers belongs to this invocation.
    command.retain(|a| !matches!(a.as_str(), "--events-json" | "--cancel-on-stdin-close"));
    let (options, _, _) = parse(&command)?;
    if options.output.canonicalize()? != directory {
        return Err("checkpoint directory differs from its command".into());
    }
    for flag in &arguments[3..] {
        if !command.contains(flag) {
            command.push(flag.clone());
        }
    }
    Ok((command, manifest, guard))
}

pub(super) fn run(arguments: &[String]) -> Result<bool, Box<dyn Error>> {
    if cfg!(feature = "profile-sampler") {
        return Err("profile-sampler builds cannot profile concurrent temperature slots".into());
    }
    let executable = sha256::file(&std::env::current_exe()?)?;
    let (command, saved, mut guard) = if arguments.iter().any(|a| a == "--resume") {
        let (command, saved, lock) = load(arguments, &executable)?;
        (command, Some(saved), Some(lock))
    } else {
        (arguments.to_vec(), None, None)
    };
    let (mut options, powers, swap_every) = parse(&command)?;
    options.output = std::path::absolute(&options.output)?;
    options.executable_sha256 = Some(executable.clone());
    let Configured {
        input,
        data,
        patterns,
        settings,
    } = configure(&mut options)?;
    let setting_hash = sha256::hex(format!("{settings:?}").as_bytes());
    let command = canonical_command(&command, &options)?;
    let mut ensemble = if let Some(saved) = &saved {
        if saved["input_sha256"] != input || saved["settings_sha256"] != setting_hash {
            return Err("coupled checkpoint input or model changed".into());
        }
        let step = saved["step"].as_u64().ok_or("missing checkpoint step")?;
        let state = options.output.join(format!("ensemble-{step}.bin"));
        if fs::metadata(&state)?.len() > 1024 * 1024 * 1024 {
            return Err("coupled checkpoint exceeds 1 GiB".into());
        }
        let bytes = fs::read(&state)?;
        if sha256::hex(&bytes) != saved["state_sha256"] {
            return Err("coupled checkpoint state changed".into());
        }
        let restored = Ensemble::resume(&patterns, &settings, &bytes)?;
        if restored.cold().steps() != step
            || step > options.steps
            || step % options.sample_every != 0
            || restored.powers() != powers
            || restored.swap_every() != swap_every
        {
            return Err("coupled checkpoint state differs from its schedule".into());
        }
        // Verify both prefixes before truncating either append-only output.
        checkpoint::verify_prefix(&options.output.join("trace.log"), &saved["trace"])?;
        checkpoint::verify_prefix(&options.output.join("trees.nexus"), &saved["trees"])?;
        restored
    } else {
        Ensemble::new(&patterns, &settings, &powers, swap_every, options.seed)?
    };
    if saved.is_none() {
        if let Some(parent) = options.output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir(&options.output)?;
        guard = Some(checkpoint::Lock::acquire(&options.output)?);
    }
    let _guard = guard;
    let cancellation = events::Cancellation::watch_stdin(options.cancel_on_stdin_close);
    let mut comments = vec![
        format!("rove {}", env!("CARGO_PKG_VERSION")),
        format!("command {}", serde_json::to_string(&command)?),
        format!("input sha256 {input}"),
        format!("executable sha256 {executable}"),
        TARGET.into(),
    ];
    if let Some(config) = &options.dating {
        comments.push(format!("time unit {}; ages before present; branch lengths in time; global clock in trace.clock_rate; branch rate annotations are relative multipliers", config["time_unit"].as_str().unwrap()));
    }
    let trace_path = options.output.join("trace.log");
    let tree_path = options.output.join("trees.nexus");
    let mut trace = if let Some(saved) = &saved {
        TraceWriter::resume_sampler(
            checkpoint::Output::resume(&trace_path, &saved["trace"])?,
            ensemble.cold(),
        )
    } else {
        TraceWriter::with_sampler(
            checkpoint::Output::create(&trace_path)?,
            &comments,
            ensemble.cold(),
        )?
    };
    let mut trees = if let Some(saved) = &saved {
        TreeWriter::resume(checkpoint::Output::resume(&tree_path, &saved["trees"])?)
    } else {
        TreeWriter::new(
            checkpoint::Output::create(&tree_path)?,
            data.taxa(),
            &comments,
        )?
    };
    let initial = ensemble.cold().steps();
    let started = saved
        .as_ref()
        .and_then(|s| s["started_unix_seconds"].as_u64())
        .unwrap_or(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());
    let previous = saved
        .as_ref()
        .and_then(|s| s["elapsed_seconds"].as_f64())
        .unwrap_or(0.0);
    let clock = Instant::now();
    let mut last_saved = Instant::now();
    let mut last_checkpoint = saved.as_ref().and_then(|s| s["step"].as_u64());
    let mut record = json!({"kind":"coupled_chains", "schema_version":1,"status":"partial",
        "completed_steps":initial,"rove_version":env!("CARGO_PKG_VERSION"),"executable_sha256":executable,
        "platform":format!("{}-{}",std::env::consts::ARCH,std::env::consts::OS),
        "command":std::iter::once("rove").chain(command.iter().map(String::as_str)).collect::<Vec<_>>(),
        "input":value(input_json(&options,&input,&data,&patterns)),
        "model":value(model_json(&options,&settings,&patterns,data.taxa())),
        "settings":{"seed":options.seed,"steps":options.steps,"sample_every":options.sample_every,
            "initial_gamma_shape":settings.gamma_shape,"initial_clock_sigma":settings.clock_sigma,
            "initial_branch_rate":if options.reference_model { Some(0.5) } else { None },
            "initial_birth_difference_rate":settings.initial.birth_difference,
            "initial_relative_death_rate":settings.initial.relative_death,
            "initial_tree":"random, with every monophyly constraint satisfied",
            "tuning_target_acceptance":0.234,
            "tuning":settings.tuning,"tuning_delay":settings.tuning_delay,"tuning_schedule":super::TUNING_SCHEDULE,
            "backend":settings.backend.name()},"started_unix_seconds":started,
        "coupling":diagnostic(&ensemble),"worker_count":powers.len(),
        "seed_method":"slot seed = base + zero-based slot, modulo 2^64; swap seed = base XOR 0xd1b54a32d192ed03",
        "independent_cold_chains":1});
    checkpoint::write_json(&options.output.join("run.json"), &record)?;
    if saved.is_none() {
        trace.write(0, ensemble.cold())?;
        trees.write(0, ensemble.cold())?;
    }
    if options.events_json {
        events::state(
            "started",
            initial,
            options.steps,
            previous,
            &input,
            ensemble.cold(),
            None,
            None,
        )?;
    }
    let print_every = options.print_every.unwrap_or((options.steps / 20).max(1));
    let mut printed = initial;
    let result = ensemble.run(options.steps, options.sample_every, true, |e| {
        let step = e.cold().steps();
        let elapsed = previous + clock.elapsed().as_secs_f64();
        if step > initial && step % options.sample_every == 0 {
            trace.write(step,e.cold())?; trees.write(step,e.cold())?;
        }
        if step.saturating_sub(printed) >= print_every {
            if options.events_json {
                events::state("progress",step,options.steps,elapsed,&input,e.cold(),None,None)?;
            } else {
                progress(format!("coupled {step}/{}; cold log posterior {:.4}; {:.1}s",options.steps,e.cold().posterior().total(),elapsed));
            }
            printed = step;
        }
        let at_sample = step % options.sample_every == 0;
        let save = at_sample && ((step == 0 && saved.is_none()) || step == options.steps
            || cancellation.paused() || options.checkpoint_every.map_or(last_saved.elapsed().as_secs() >= 30, |n| step > initial && step % n == 0));
        if save {
            let bytes = e.checkpoint(&settings)?;
            checkpoint::atomic(&options.output.join(format!("ensemble-{step}.bin")), &bytes)?;
            let mut manifest = json!({"schema_version":1,"kind":"coupled_checkpoint","arguments":command,
                "executable_sha256":executable,"input_sha256":input,"settings_sha256":setting_hash,
                "step":step,"state_sha256":sha256::hex(&bytes),"trace":trace.inner_mut().record().map_err(|e| io::Error::other(e.to_string()))?,
                "trees":trees.inner_mut().record().map_err(|e| io::Error::other(e.to_string()))?,"started_unix_seconds":started,"elapsed_seconds":elapsed});
            manifest["manifest_sha256"] = json!(sha256::hex(&serde_json::to_vec(&manifest)?));
            checkpoint::write_json(&options.output.join("coupling.json"),&manifest).map_err(|e| io::Error::other(e.to_string()))?;
            if let Some(old) = last_checkpoint.replace(step).filter(|old| *old != step) {
                // The new manifest is durable; an obsolete state is no longer
                // needed for recovery. A cleanup failure leaves it harmlessly.
                let _ = fs::remove_file(options.output.join(format!("ensemble-{old}.bin")));
            }
            last_saved = Instant::now();
        }
        Ok(!cancellation.requested() && !(at_sample && cancellation.paused()))
    });
    if let Err(error) = result {
        record["status"] = json!("failed");
        record["error"] = json!(error.to_string());
        checkpoint::write_json(&options.output.join("run.json"), &record)?;
        return Err(error.into());
    }
    trace.finish()?;
    trees.finish()?;
    let step = ensemble.cold().steps();
    let status = if step == options.steps {
        "completed"
    } else if cancellation.paused() {
        "paused"
    } else {
        "cancelled"
    };
    let elapsed = previous + clock.elapsed().as_secs_f64();
    record["status"] = json!(status);
    record["completed_steps"] = json!(step);
    record["termination_reason"] = json!(if status == "completed" {
        "step_limit"
    } else {
        status
    });
    record["wall_seconds"] = json!(elapsed);
    record["coupling"] = diagnostic(&ensemble);
    record["moves"] = value(moves_json(ensemble.cold()));
    checkpoint::write_json(&options.output.join("run.json"), &record)?;
    if options.events_json {
        events::state(
            status,
            step,
            options.steps,
            elapsed,
            &input,
            ensemble.cold(),
            None,
            record["termination_reason"].as_str(),
        )?;
    }
    progress(format!(
        "{status}: one cold chain, {} temperature slots, {step} steps in {elapsed:.1}s; wrote {}",
        powers.len(),
        options.output.display()
    ));
    Ok(status == "completed")
}
