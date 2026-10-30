use std::error::Error;
use std::fs;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::{Value, json};

use super::{Json, Options, checkpoint, compare, events, run_chain, sha256};

enum Message {
    Boundary(usize, u64),
    Finished(usize, Result<bool, String>),
    Saved(usize, Value),
}

enum Instruction {
    Save,
    Continue(bool),
}

struct Boundaries {
    furthest: u64,
    requested: Option<u64>,
    saved_at: Instant,
}

pub(super) struct Worker {
    pub index: usize,
    boundaries: Arc<Mutex<Boundaries>>,
    initial: u64,
    limit: u64,
    sample_every: u64,
    checkpoint_every: Option<u64>,
    check_every: Option<u64>,
    report: Sender<Message>,
    resume: Receiver<Instruction>,
}
impl Worker {
    pub fn should_wait(&self, step: u64, pause: bool) -> Result<bool, Box<dyn Error>> {
        let mut state = self
            .boundaries
            .lock()
            .map_err(|_| "checkpoint coordinator failed")?;
        state.furthest = state.furthest.max(step);
        let timed_save =
            self.checkpoint_every.is_none() && state.saved_at.elapsed().as_secs() >= 30;
        if (pause || timed_save) && state.requested.is_none() {
            // Every worker registers its progress under this lock before
            // continuing. The target is beyond any boundary already passed.
            state.requested = Some(
                state
                    .furthest
                    .saturating_add(self.sample_every)
                    .min(self.limit),
            );
        }
        Ok(step == self.initial
            || step == self.limit
            || state.requested == Some(step)
            || self.checkpoint_every.is_some_and(|n| step % n == 0)
            || self.check_every.is_some_and(|n| step % n == 0))
    }

    pub fn check(
        &mut self,
        step: u64,
        mut save: impl FnMut() -> Result<Value, Box<dyn Error>>,
    ) -> Result<bool, Box<dyn Error>> {
        self.report.send(Message::Boundary(self.index, step))?;
        loop {
            match self.resume.recv()? {
                Instruction::Save => self.report.send(Message::Saved(self.index, save()?))?,
                Instruction::Continue(stop) => return Ok(stop),
            }
        }
    }
}

use checkpoint::write_json;

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
    let _guard = if options.resume.is_none() {
        fs::create_dir(&options.output).map_err(|e| {
            format!(
                "cannot create {} (it must not exist yet): {e}",
                options.output.display()
            )
        })?;
        Some(checkpoint::Lock::acquire(&options.output)?)
    } else {
        None
    };
    let previous_seconds = options
        .resume
        .as_ref()
        .and_then(|r| r.manifest["chains"].as_array())
        .map(|chains| {
            chains
                .iter()
                .filter_map(|c| c["elapsed_seconds"].as_f64())
                .fold(0.0, f64::max)
        })
        .unwrap_or(0.0);
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
    let initial = options
        .resume
        .as_ref()
        .and_then(|r| r.manifest["chains"][0]["step"].as_u64())
        .unwrap_or(0);
    let coordinated = Arc::new(Mutex::new(Boundaries {
        furthest: initial,
        requested: None,
        saved_at: Instant::now(),
    }));
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
            boundaries: coordinated.clone(),
            initial,
            limit: options.steps,
            sample_every: options.sample_every,
            checkpoint_every: options.checkpoint_every,
            check_every: options.stop_when_converged.then_some(every),
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
    let mut checks = options
        .resume
        .as_ref()
        .and_then(|r| r.manifest["checks"].as_array())
        .cloned()
        .unwrap_or_default();
    let mut consecutive = checks
        .last()
        .and_then(|v| v["consecutive"].as_u64())
        .unwrap_or(0);
    let resumed_step = options
        .resume
        .as_ref()
        .and_then(|r| r.manifest["chains"][0]["step"].as_u64());
    let mut saving = vec![None; options.chains];
    let mut stop_after_save = false;
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
                        let _ = resume.send(Instruction::Continue(true));
                    }
                }
            }
            Message::Saved(index, value) => {
                saving[index - 1] = Some(value);
                if saving.iter().all(Option::is_some) {
                    if !cancellation.requested() {
                        match checkpoint::commit(
                            &options,
                            arguments,
                            saving.iter().map(|v| v.clone().unwrap()).collect(),
                            &checks,
                        ) {
                            Ok(()) => {}
                            Err(error) => {
                                failures.push(error.to_string());
                                cancellation.cancel();
                            }
                        }
                    }
                    {
                        let mut state = coordinated
                            .lock()
                            .map_err(|_| "checkpoint coordinator failed")?;
                        state.saved_at = Instant::now();
                        state.requested = None;
                    }
                    for resume in &resumes {
                        let _ = resume.send(Instruction::Continue(
                            stop_after_save || cancellation.requested(),
                        ));
                    }
                    saving.fill(None);
                    boundaries.fill(None);
                }
            }
            Message::Boundary(index, step) => {
                boundaries[index - 1] = Some(step);
                if cancellation.requested() {
                    let _ = resumes[index - 1].send(Instruction::Continue(true));
                } else if boundaries.iter().all(|s| *s == Some(step)) {
                    let mut stop = consecutive >= 2 && options.stop_when_converged;
                    if options.stop_when_converged
                        && step > 0
                        && step < options.steps
                        && step % every == 0
                        && resumed_step != Some(step)
                    {
                        match compare::summarize(
                            &directories,
                            options.burnin,
                            &options.monophyly,
                            options.dating.as_ref(),
                            &|| cancellation.requested(),
                            false,
                        ) {
                            Ok(value) => {
                                let met = compare::thresholds_met(
                                    &value,
                                    options.min_ess,
                                    options.max_rhat,
                                );
                                consecutive = if met { consecutive + 1 } else { 0 };
                                stop = consecutive >= 2;
                                checks.push(json!({"step": step, "thresholds_met": met, "consecutive": consecutive,
                                    "min_ess": value["statistics"].as_array().unwrap().iter().flat_map(|s| s["ess"].as_array().unwrap().iter()).filter_map(Value::as_f64).reduce(f64::min),
                                    "max_rhat": value["statistics"].as_array().unwrap().iter().filter_map(|s| s["rhat"].as_f64()).reduce(f64::max)}));
                            }
                            Err(error) => {
                                if !cancellation.requested() {
                                    failures.push(error.to_string());
                                }
                                cancellation.cancel();
                            }
                        }
                    }
                    if stop {
                        stopped_at = Some(step);
                    }
                    let pause = cancellation.paused() && step < options.steps;
                    stop_after_save = stop || pause;
                    let timed = coordinated
                        .lock()
                        .map_err(|_| "checkpoint coordinator failed")?
                        .requested
                        == Some(step);
                    let save = step == 0
                        || timed
                        || options.checkpoint_every.is_some_and(|n| step % n == 0)
                        || stop
                        || pause
                        || step == options.steps;
                    for resume in &resumes {
                        let command = if save && !cancellation.requested() {
                            Instruction::Save
                        } else {
                            Instruction::Continue(stop_after_save || cancellation.requested())
                        };
                        let _ = resume.send(command);
                    }
                    if !save || cancellation.requested() {
                        boundaries.fill(None);
                    }
                }
            }
        }
    }
    for thread in threads {
        let _ = thread.join();
    }
    let sampling_seconds = previous_seconds + clock.elapsed().as_secs_f64();
    let reached_limit = directories.iter().all(|d| {
        fs::read(d.join("run.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .is_some_and(|v| v["completed_steps"].as_u64() == Some(options.steps))
    });
    if failures.is_empty() && !cancellation.requested() && (!cancellation.paused() || reached_limit)
    {
        match compare::summarize(
            &directories,
            options.burnin,
            &options.monophyly,
            options.dating.as_ref(),
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
    } else if cancellation.paused() && min_step < options.steps {
        "paused"
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
    group["wall_seconds"] = json!(previous_seconds + clock.elapsed().as_secs_f64());
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
                Json::Number(previous_seconds + clock.elapsed().as_secs_f64()),
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
