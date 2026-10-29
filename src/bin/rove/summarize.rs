use std::collections::HashSet;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rove::summary::{self, Burnin, Heights};
use serde_json::{Value, json};

use super::{events, sha256};

pub const USAGE: &str = "rove summarize <trees.nexus|samples.trees> --output <new-directory> \
    (--burnin-percent <0..99> | --burnin-samples <n>) --heights keep|ca [--cancel-on-stdin-close]";

struct Output {
    directory: PathBuf,
    committed: bool,
}
impl Drop for Output {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.directory);
        }
    }
}

fn record(path: &Path) -> Result<Value, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > 2 * 1024 * 1024 {
        return Err("invalid or oversized source record".into());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub fn run(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let mut input = None;
    let mut output = None;
    let mut burnin = None;
    let mut heights = None;
    let mut cancel_on_close = false;
    let mut seen = HashSet::new();
    let mut args = arguments.iter();
    while let Some(arg) = args.next() {
        if arg.starts_with("--") && !seen.insert(arg) {
            return Err(format!("duplicate option {arg}").into());
        }
        match arg.as_str() {
            "--output" => {
                output = Some(PathBuf::from(
                    args.next().ok_or("--output needs a directory")?,
                ))
            }
            "--burnin-percent" | "--burnin-samples" => {
                if burnin.is_some() {
                    return Err("choose one burn-in unit".into());
                }
                let value = args.next().ok_or("burn-in needs a value")?;
                burnin = Some(if arg == "--burnin-percent" {
                    let p: u8 = value.parse()?;
                    if p >= 100 {
                        return Err("burn-in percent must be between 0 and 99".into());
                    }
                    Burnin::Percent(p)
                } else {
                    Burnin::Samples(value.parse()?)
                });
            }
            "--heights" => {
                heights = Some(match args.next().map(String::as_str) {
                    Some("keep") => Heights::Keep,
                    Some("ca") => Heights::CommonAncestor,
                    _ => return Err("--heights must be keep or ca".into()),
                })
            }
            "--cancel-on-stdin-close" => cancel_on_close = true,
            path if !path.starts_with('-') && input.is_none() => input = Some(PathBuf::from(path)),
            _ => return Err(format!("unsupported argument {arg:?}; {USAGE}").into()),
        }
    }
    let input = fs::canonicalize(input.ok_or(USAGE)?)?;
    let output = output.ok_or(USAGE)?;
    let burnin = burnin.ok_or(USAGE)?;
    let heights = heights.ok_or(USAGE)?;
    if !fs::metadata(&input)?.is_file() {
        return Err("source must be a regular tree file".into());
    }
    let parent = input.parent().ok_or("source has no parent directory")?;
    let run_path = parent.join("run.json");
    let metadata = if run_path.exists() {
        let value = record(&run_path)?;
        if value["status"] != "completed" || value["completed_steps"] != value["settings"]["steps"]
        {
            return Err("source run is not completed; partial runs cannot be summarized".into());
        }
        Some(value)
    } else {
        None
    };
    let session_path = parent.parent().map(|p| p.join("session.json"));
    if parent.file_name().is_some_and(|n| n == "results") {
        if let Some(path) = session_path.filter(|p| p.exists()) {
            let value = record(&path)?;
            if value["status"] != "completed" || value["exitCode"] != 0 {
                return Err("source session has no confirmed successful finish".into());
            }
        }
    }
    let constraints: Vec<Vec<String>> = match metadata.as_ref() {
        Some(value) => serde_json::from_value(value["model"]["monophyly"].clone())?,
        None => Vec::new(),
    };
    let cancellation = events::Cancellation::watch_stdin(cancel_on_close);
    let started = Instant::now();
    fs::create_dir(&output)?;
    let mut cleanup = Output {
        directory: output.clone(),
        committed: false,
    };
    // This immutable snapshot is also the raw .trees export. Never overwrite the source.
    let mut source = File::open(&input)?;
    let mut copy = BufWriter::new(File::create(output.join("samples.trees"))?);
    let mut digest = sha256::Digest::new();
    let mut buffer = vec![0; 128 * 1024];
    loop {
        if cancellation.requested() {
            return Err("tree summary cancelled".into());
        }
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        copy.write_all(&buffer[..count])?;
    }
    copy.flush()?;
    drop(copy);
    let input_hash = digest.finish();
    let summary = summary::summarize(
        &output.join("samples.trees"),
        burnin,
        heights,
        &constraints,
        &|| cancellation.requested(),
    )?;
    if let Some(value) = &metadata {
        let steps = value["settings"]["steps"]
            .as_u64()
            .ok_or("invalid source steps")?;
        let interval = value["settings"]["sample_every"]
            .as_u64()
            .ok_or("invalid source sampling interval")?;
        if interval == 0
            || summary.first_state != 0
            || summary.last_state != steps
            || summary.sampling_interval != Some(interval)
            || summary.total as u64 != steps / interval + 1
            || steps % interval != 0
        {
            return Err("tree samples disagree with the source run record".into());
        }
    }
    let mut tree_file = BufWriter::new(File::create(output.join("mcc.tree"))?);
    summary.write_tree(&mut tree_file)?;
    tree_file.flush()?;
    let (unit, amount) = match burnin {
        Burnin::Percent(p) => ("percent", p as usize),
        Burnin::Samples(n) => ("samples", n),
    };
    let manifest = json!({
        "schema_version": 1, "status": "completed", "method": "MCC",
        "rove_version": env!("CARGO_PKG_VERSION"),
        "executable_sha256": sha256::file(&std::env::current_exe()?)?,
        "annotated_tree_sha256": sha256::file(&output.join("mcc.tree"))?, "command": std::env::args().collect::<Vec<_>>(),
        "created_unix_seconds": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        "source": {"path": input, "sha256": input_hash, "completion": if metadata.is_some() {"recorded_completed"} else {"unverified"},
            "run_record": metadata, "constraints": constraints},
        "burnin": {"unit": unit, "value": amount, "rounding": "floor(total_samples * percent / 100); STATE_0 counts as a sample"},
        "total_samples": summary.total, "discarded_samples": summary.discarded, "retained_samples": summary.retained(),
        "first_state": summary.first_state, "first_retained_state": summary.first_retained_state, "last_state": summary.last_state,
        "selected_state": summary.target.state, "log_clade_credibility": summary.log_credibility,
        "scoring": "sum of log clade frequencies over internal nodes; fixed clade order; first retained candidate wins exact ties",
        "height_policy": if heights == Heights::Keep {"keep"} else {"ca"},
        "height_summary": "conditional means and ranges; CA means and ranges use the MRCA in every retained tree",
        "rate_summary": "conditional arithmetic mean and range when the clade exists; no rate for the root",
        "branch_units": "source time lengths; branch rate is a separate multiplier; no calendar calibration inferred",
        "intervals": "ranges are observed minima and maxima, not HPD or credible intervals",
        "taxa": summary.taxa, "distinct_clades": summary.distinct_clades,
        "annotated_tree": "mcc.tree", "raw_trees": "samples.trees",
        "wall_seconds": started.elapsed().as_secs_f64(), "convergence_assessed": false,
    });
    fs::write(
        output.join("summary.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    cleanup.committed = true;
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}
