use std::error::Error;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rove::diagnostics::{ESS_METHOD, INTERVAL_METHOD, Trace, statistics};
use rove::summary::Burnin;
use serde_json::{Value, json};

use super::{events::Cancellation, sha256, summarize};
const MAX_BYTES: u64 = 128 * 1024 * 1024;

fn integer(record: &Value, field: &str) -> Result<u64, Box<dyn Error>> {
    record[field]
        .as_u64()
        .ok_or_else(|| format!("invalid run {field}").into())
}

fn source_record(
    path: &Path,
    trace: &Trace,
    bytes: &[u8],
) -> Result<(Value, String), Box<dyn Error>> {
    let file = path.with_file_name("run.json");
    if !file.exists() {
        return Ok((Value::Null, "unverified".into()));
    }
    let record = summarize::record(&file)?;
    let status = record["status"]
        .as_str()
        .ok_or("missing run status")?
        .to_owned();
    if !matches!(
        status.as_str(),
        "completed" | "cancelled" | "failed" | "partial"
    ) {
        return Err("unsupported run status".into());
    }
    let every = integer(&record["settings"], "sample_every")?;
    let requested = integer(&record["settings"], "steps")?;
    let completed = integer(&record, "completed_steps")?;
    if every == 0
        || completed > requested
        || (status == "completed" && completed != requested)
        || trace.states[0] != 0
        || trace
            .states
            .iter()
            .enumerate()
            .any(|(i, &s)| (i as u128 * every as u128) != s as u128)
        || (status == "completed"
            && (completed as u128 / every as u128 + 1 != trace.states.len() as u128
                || completed % every != 0))
        || trace.states.last().is_some_and(|&s| s > completed)
    {
        return Err("trace sample states disagree with the recorded run; the file may be truncated or belong to another run".into());
    }
    for line in std::str::from_utf8(bytes)?
        .lines()
        .take_while(|s| s.starts_with('#'))
    {
        if let Some(hash) = line.strip_prefix("# input sha256 ") {
            if record["input"]["sha256"] != hash {
                return Err("trace input fingerprint disagrees with the run record".into());
            }
        }
    }
    Ok((record, status))
}

pub(super) fn inspect(
    path: &Path,
    burnin: Burnin,
    cancel: &dyn Fn() -> bool,
) -> Result<Value, Box<dyn Error>> {
    let path = path.canonicalize()?;
    let before = fs::metadata(&path)?;
    if !before.is_file() || before.len() > MAX_BYTES {
        return Err("trace must be a regular file no larger than 128 MiB".into());
    }
    let mut bytes = Vec::with_capacity(before.len() as usize + 1);
    File::open(&path)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let after = fs::metadata(&path)?;
    if bytes.len() as u64 > MAX_BYTES
        || before.len() != after.len()
        || before.modified()? != after.modified()?
    {
        return Err("trace changed while reading; retry after the writer finishes".into());
    }
    let trace = Trace::read(&bytes, cancel)?;
    let (run_record, completion) = source_record(&path, &trace, &bytes)?;
    let discarded = burnin.discard(trace.states.len())?;
    let mut columns = Vec::with_capacity(trace.columns.len());
    for (index, column) in trace.columns.iter().enumerate() {
        if cancel() {
            return Err("trace diagnostics cancelled".into());
        }
        let stats = statistics(&column.values[discarded..], cancel)?;
        let points: Vec<_> = trace
            .plot(index, discarded)
            .into_iter()
            .map(|p| json!({"state": p.state, "value": p.value, "retained": p.retained}))
            .collect();
        columns.push(json!({
            "name": column.name, "mean": stats.mean, "sd": stats.sd, "interval": stats.interval,
            "ess": stats.ess, "ess_reason": stats.ess_reason, "last_lag": stats.last_lag,
            "lag_limit_reached": stats.lag_limit_reached, "points": points,
        }));
    }
    let burnin = match burnin {
        Burnin::Percent(value) => json!({"unit":"percent", "value":value}),
        Burnin::Samples(value) => json!({"unit":"samples", "value":value}),
    };
    Ok(json!({
        "schema_version": 1, "kind": "within_chain_diagnostics", "rove_version": env!("CARGO_PKG_VERSION"),
        "created_unix_seconds": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64(),
        "source": {"path":path, "sha256":sha256::hex(&bytes), "bytes":bytes.len(), "completion":completion, "run_record":run_record},
        "burnin": burnin, "total_samples":trace.states.len(), "discarded_samples":discarded,
        "retained_samples":trace.states.len()-discarded, "first_state":trace.states[0],
        "first_retained_state":trace.states[discarded], "last_state":trace.states.last(),
        "sample_every":trace.sample_every, "ess_method":ESS_METHOD, "interval_method":INTERVAL_METHOD,
        "plot_method":"first, minimum, maximum and last per bucket; at most 1028 points per column; burn-in boundary retained",
        "statistics_use_all_retained_samples":true, "convergence_assessed":false, "topology_assessed":false,
        "rhat":Value::Null, "columns":columns,
    }))
}

struct Pending(Option<PathBuf>);
impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = fs::remove_dir_all(path);
        }
    }
}

pub fn run(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let Some(input) = arguments.first() else {
        return Err(
            "usage: rove diagnose <trace.log> --burnin-percent <0..99> [--output <new-directory>]"
                .into(),
        );
    };
    let mut burnin = None;
    let mut output = None;
    let mut watch = false;
    let mut args = arguments[1..].iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--burnin-percent" if burnin.is_none() => {
                burnin = Some(Burnin::Percent(
                    args.next().ok_or("missing burn-in percent")?.parse()?,
                ))
            }
            "--burnin-samples" if burnin.is_none() => {
                burnin = Some(Burnin::Samples(
                    args.next().ok_or("missing burn-in samples")?.parse()?,
                ))
            }
            "--output" if output.is_none() => {
                output = Some(PathBuf::from(
                    args.next().ok_or("missing output directory")?,
                ))
            }
            "--cancel-on-stdin-close" if !watch => watch = true,
            _ => return Err(format!("unknown or repeated diagnostic option {arg}").into()),
        }
    }
    if output.as_ref().is_some_and(|p| p.exists()) {
        return Err("diagnostic output directory already exists".into());
    }
    let cancellation = Cancellation::watch_stdin(watch);
    let result = inspect(
        Path::new(input),
        burnin.ok_or("choose --burnin-percent or --burnin-samples")?,
        &|| cancellation.requested(),
    )?;
    if cancellation.requested() {
        return Err("trace diagnostics cancelled".into());
    }
    let text = serde_json::to_string(&result)?;
    if let Some(directory) = output {
        fs::create_dir(&directory)?;
        let mut guard = Pending(Some(directory.clone()));
        fs::write(directory.join("diagnostics.json.tmp"), text)?;
        fs::rename(
            directory.join("diagnostics.json.tmp"),
            directory.join("diagnostics.json"),
        )?;
        if cancellation.requested() {
            return Err("trace diagnostics cancelled".into());
        }
        guard.0 = None;
        println!("{}", directory.display());
    } else {
        println!("{text}");
    }
    Ok(())
}
