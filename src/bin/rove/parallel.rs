use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use serde_json::{Value, json};

use super::{Json, Options, compare, events, run_chain, sha256};

enum Message {
    Boundary(usize, u64),
    Finished(usize, Result<bool, String>),
}

pub(super) struct Worker {
    pub index: usize,
    every: Option<u64>,
    report: Sender<Message>,
    resume: Receiver<bool>,
}
impl Worker {
    pub fn is_boundary(&self, step: u64) -> bool {
        self.every.is_some_and(|n| step % n == 0)
    }
    pub fn check(&mut self, step: u64) -> Result<bool, Box<dyn Error>> {
        self.report.send(Message::Boundary(self.index, step))?;
        Ok(self.resume.recv()?)
    }
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

pub(super) fn run(
    arguments: &[String],
    mut options: Options,
    cancellation: events::Cancellation,
) -> Result<bool, Box<dyn Error>> {
    let bytes = fs::read(&options.matrix)?;
    let fingerprint = sha256::hex(&bytes);
    if options
        .expected_input
        .as_ref()
        .is_some_and(|h| *h != fingerprint)
    {
        return Err("the matrix fingerprint changed; inspect and review the input again".into());
    }
    options.expected_input = Some(fingerprint.clone());
    if let Some(parent) = options
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&options.output).map_err(|e| {
        format!(
            "cannot create {} (it must not exist yet): {e}",
            options.output.display()
        )
    })?;
    let clock = Instant::now();
    let directories: Vec<_> = (1..=options.chains)
        .map(|i| options.output.join(format!("chain-{i:02}")))
        .collect();
    let seeds: Vec<_> = (0..options.chains)
        .map(|i| options.seed.wrapping_add(i as u64))
        .collect();
    let every = options
        .check_every
        .unwrap_or(options.sample_every.saturating_mul(1000));
    let mut group = json!({"schema_version": 1, "kind": "independent_chains", "status": "partial",
        "chain_count": options.chains, "input": {"sha256": fingerprint},
        "settings": {"seed": options.seed, "steps": options.steps, "sample_every": options.sample_every},
        "seed_method": "base seed + zero-based chain index, modulo 2^64",
        "diagnostics": {"burnin_percent": options.burnin, "stop_when_converged": options.stop_when_converged,
            "min_ess_per_chain": options.min_ess, "max_rhat": options.max_rhat, "check_every": every,
            "minimum_retained_per_chain": 1000, "required_consecutive_checks": 2,
            "stopping_scope": "all traced scalar statistics; does not certify topology convergence"},
        "chains": seeds.iter().enumerate().map(|(i, seed)| json!({"index": i+1, "seed": seed, "directory": format!("chain-{:02}", i+1)})).collect::<Vec<_>>()});
    write_json(&options.output.join("run.json"), &group)?;
    let (send, receive) = mpsc::channel();
    let mut resumes = Vec::new();
    let mut threads = Vec::new();
    for index in 1..=options.chains {
        let (resume, waiting) = mpsc::channel();
        resumes.push(resume);
        let mut worker_options = options.clone();
        worker_options.output = directories[index - 1].clone();
        worker_options.seed = seeds[index - 1];
        worker_options.chains = 1;
        // Keep a replayable single-chain command in each child's provenance.
        let mut command = Vec::new();
        let mut iter = arguments.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--chains" | "--burnin-percent" | "--min-ess" | "--max-rhat" | "--check-every" => {
                    iter.next();
                }
                "--stop-when-converged" => {}
                "--seed" => {
                    iter.next();
                    command.extend([arg.clone(), worker_options.seed.to_string()]);
                }
                "--output" => {
                    iter.next();
                    command.extend([arg.clone(), worker_options.output.display().to_string()]);
                }
                _ => command.push(arg.clone()),
            }
        }
        let report = send.clone();
        let cancel = cancellation.clone();
        let worker = Worker {
            index,
            every: options.stop_when_converged.then_some(every),
            report: report.clone(),
            resume: waiting,
        };
        let spawn = std::thread::Builder::new()
            .name(format!("rove-chain-{index}"))
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_chain(&command, worker_options, &cancel, Some(worker))
                        .map_err(|e| e.to_string())
                }))
                .unwrap_or_else(|_| Err("chain worker panicked".into()));
                let _ = report.send(Message::Finished(index, result));
            });
        match spawn {
            Ok(thread) => threads.push(thread),
            Err(error) => {
                cancellation.cancel();
                let _ = send.send(Message::Finished(index, Err(error.to_string())));
            }
        }
    }
    drop(send);
    let mut finished = 0;
    let mut boundaries = vec![None; options.chains];
    let mut failures = Vec::new();
    let mut consecutive = 0;
    let mut checks = Vec::new();
    let mut stopped_at = None;
    let mut summary = None;
    while finished < options.chains {
        let message = receive.recv()?;
        match message {
            Message::Finished(index, result) => {
                finished += 1;
                if let Err(error) = result {
                    failures.push(format!("chain {index}: {error}"));
                    cancellation.cancel();
                }
                if cancellation.requested() {
                    for resume in &resumes {
                        let _ = resume.send(true);
                    }
                }
            }
            Message::Boundary(index, step) => {
                boundaries[index - 1] = Some(step);
                if cancellation.requested() {
                    let _ = resumes[index - 1].send(true);
                } else if boundaries.iter().all(|s| *s == Some(step)) {
                    match compare::summarize(
                        &directories,
                        options.burnin,
                        &options.monophyly,
                        &|| cancellation.requested(),
                        false,
                    ) {
                        Ok(value) => {
                            let met =
                                compare::thresholds_met(&value, options.min_ess, options.max_rhat);
                            consecutive = if met { consecutive + 1 } else { 0 };
                            let stop = consecutive >= 2;
                            checks.push(json!({"step": step, "thresholds_met": met, "consecutive": consecutive,
                                "min_ess": value["statistics"].as_array().unwrap().iter().flat_map(|s| s["ess"].as_array().unwrap().iter()).filter_map(Value::as_f64).reduce(f64::min),
                                "max_rhat": value["statistics"].as_array().unwrap().iter().filter_map(|s| s["rhat"].as_f64()).reduce(f64::max)}));
                            if stop {
                                stopped_at = Some(step);
                            }
                            for resume in &resumes {
                                let _ = resume.send(stop);
                            }
                        }
                        Err(error) => {
                            if !cancellation.requested() {
                                failures.push(error.to_string());
                            }
                            cancellation.cancel();
                            for resume in &resumes {
                                let _ = resume.send(true);
                            }
                        }
                    }
                    boundaries.fill(None);
                }
            }
        }
    }
    for thread in threads {
        let _ = thread.join();
    }
    let sampling_seconds = clock.elapsed().as_secs_f64();
    if failures.is_empty() && !cancellation.requested() {
        match compare::summarize(
            &directories,
            options.burnin,
            &options.monophyly,
            &|| cancellation.requested(),
            false,
        ) {
            Ok(value) => summary = Some(value),
            Err(error) => {
                if !cancellation.requested() {
                    failures.push(error.to_string());
                }
            }
        }
    }
    if let Some(value) = &summary {
        if let Err(error) = write_json(&options.output.join("comparison.json"), value) {
            failures.push(error.to_string());
        }
    }
    // Only common configuration belongs to the group. Final states and tuned
    // operators remain in each member's record.
    if let Ok(bytes) = fs::read(directories[0].join("run.json")) {
        if let Ok(Value::Object(record)) = serde_json::from_slice(&bytes) {
            for (key, value) in record {
                if matches!(
                    key.as_str(),
                    "input"
                        | "model"
                        | "settings"
                        | "rove_version"
                        | "executable_sha256"
                        | "platform"
                        | "started_unix_seconds"
                ) {
                    group[&key] = value;
                }
            }
        }
    }
    let mut min_step = options.steps;
    for (i, dir) in directories.iter().enumerate() {
        let record: Value = fs::read(dir.join("run.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or(Value::Null);
        min_step = min_step.min(record["completed_steps"].as_u64().unwrap_or(0));
        group["chains"][i]["status"] = record["status"].clone();
        group["chains"][i]["completed_steps"] = record["completed_steps"].clone();
    }
    let status = if !failures.is_empty() {
        "failed"
    } else if cancellation.requested() {
        "cancelled"
    } else {
        "completed"
    };
    let reason = if status != "completed" {
        status
    } else if stopped_at.is_some() {
        "diagnostic_thresholds"
    } else {
        "step_limit"
    };
    group["status"] = json!(status);
    group["input"]["gaps"] = json!(match options.gaps {
        rove::data::GapTreatment::Reject => "reject",
        rove::data::GapTreatment::Missing => "missing",
    });
    group["completed_steps"] = json!(min_step);
    group["termination_reason"] = json!(reason);
    group["wall_seconds"] = json!(clock.elapsed().as_secs_f64());
    group["sampling_wall_seconds"] = json!(sampling_seconds);
    group["command"] = json!(
        std::iter::once("rove")
            .chain(arguments.iter().map(String::as_str))
            .collect::<Vec<_>>()
    );
    group["checks"] = json!(checks);
    group["errors"] = json!(failures);
    write_json(&options.output.join("run.json"), &group)?;
    if !failures.is_empty() {
        return Err(failures.join("; ").into());
    }
    if options.events_json {
        events::emit(vec![
            ("schema_version", Json::Integer(1)),
            ("event", Json::text(format!("group_{status}"))),
            ("chain_count", Json::Integer(options.chains as u64)),
            ("step", Json::Integer(min_step)),
            ("total_steps", Json::Integer(options.steps)),
            ("input_sha256", Json::text(fingerprint)),
            (
                "elapsed_seconds",
                Json::Number(clock.elapsed().as_secs_f64()),
            ),
            ("termination_reason", Json::text(reason)),
        ])?;
    }
    super::progress(format!(
        "{status}: {} independent chains in {:.1} s; wrote {}",
        options.chains,
        clock.elapsed().as_secs_f64(),
        options.output.display()
    ));
    Ok(status == "completed")
}
