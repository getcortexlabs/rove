//! Evidence review and local reproducibility packages. No convergence certificate.
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rove::summary::Burnin;
use serde_json::{Value, json};

use super::{compare, diagnose, events::Cancellation, sha256, summarize};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Output(PathBuf, bool);
impl Drop for Output {
    fn drop(&mut self) {
        if !self.1 {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

fn write(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn check(cancel: &dyn Fn() -> bool) -> Result<()> {
    if cancel() {
        Err("assessment/export cancelled".into())
    } else {
        Ok(())
    }
}

fn inventory(root: &Path, cancel: &dyn Fn() -> bool) -> Result<BTreeMap<PathBuf, String>> {
    fn visit(
        root: &Path,
        relative: &Path,
        depth: usize,
        out: &mut BTreeMap<PathBuf, String>,
        cancel: &dyn Fn() -> bool,
    ) -> Result<()> {
        check(cancel)?;
        if depth > 16 || out.len() >= 100_000 {
            return Err("run archive exceeds file/depth limit".into());
        }
        let mut path = root.join(relative);
        // Native CLI imports intentionally link their top-level results folder.
        // Materialize that declared source; reject links inside its artifacts.
        if depth == 0
            && relative == Path::new("results")
            && root.join("setup.rove.json").is_file()
            && root.join("session.json").is_file()
            && fs::symlink_metadata(&path)?.file_type().is_symlink()
        {
            path = path.canonicalize()?;
            if !path.is_dir() {
                return Err("imported results must reference a directory".into());
            }
        }
        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            return Err("run archive contains a symbolic link".into());
        }
        if meta.is_file() {
            out.insert(relative.to_owned(), sha256::file(&path)?);
        } else if meta.is_dir() {
            for child in fs::read_dir(path)? {
                visit(
                    root,
                    &relative.join(child?.file_name()),
                    depth + 1,
                    out,
                    cancel,
                )?;
            }
        } else {
            return Err("run archive contains a non-regular file".into());
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    // Export only Rove-owned records and derived-result folders, not neighboring research files.
    for child in fs::read_dir(root)? {
        let child = child?;
        let name = child.file_name();
        let text = name.to_str().ok_or("non-UTF-8 run filename")?;
        if matches!(
            text,
            "results"
                | "run.json"
                | "trace.log"
                | "trees.nexus"
                | "setup.rove.json"
                | "session.json"
                | "engine.stderr.log"
                | "events.jsonl"
                | "dating.json"
                | "comparison.json"
                | "checkpoint.json"
                | "diagnostics"
                | "comparisons"
                | "predictions"
                | "summaries"
                | "marginal"
        ) || (text.starts_with("checkpoint-") && text.ends_with(".bin"))
            || (text.starts_with("chain-") && text[6..].parse::<usize>().is_ok())
        {
            visit(root, Path::new(&name), 0, &mut files, cancel)?;
        }
    }
    Ok(files)
}

fn section(id: &str, status: &str, message: &str) -> Value {
    json!({"id":id, "status":status, "message":message})
}

fn assess(
    root: &Path,
    burnin: u8,
    files: &BTreeMap<PathBuf, String>,
    cancel: &dyn Fn() -> bool,
) -> Result<Value> {
    let engine = if root.join("results/run.json").exists() {
        root.join("results")
    } else {
        root.to_owned()
    };
    let record = summarize::record(&engine.join("run.json"))?;
    let grouped = record["kind"] == "independent_chains";
    let directories = if grouped {
        let n = record["chain_count"]
            .as_u64()
            .filter(|n| (2..=16).contains(n))
            .ok_or("invalid chain count")?;
        (1..=n)
            .map(|i| engine.join(format!("chain-{i:02}")))
            .collect::<Vec<_>>()
    } else {
        vec![engine.clone()]
    };
    let mut sections = Vec::new();
    let mut diagnostics = Vec::new();
    let mut completed = record["status"] == "completed";
    if root.join("session.json").exists() {
        let session = summarize::record(&root.join("session.json"))?;
        completed &= session["status"] == "completed" && session["exitCode"] == 0;
    }
    for directory in &directories {
        match diagnose::inspect(
            &directory.join("trace.log"),
            Burnin::Percent(burnin),
            cancel,
        ) {
            Ok(mut value) => {
                completed &= value["source"]["completion"] == "completed";
                if let Some(columns) = value["columns"].as_array_mut() {
                    for column in columns {
                        column.as_object_mut().unwrap().remove("points");
                    }
                }
                diagnostics.push(value);
            }
            Err(error) => {
                check(cancel)?;
                completed = false;
                diagnostics.push(json!({"source":{"path":directory.join("trace.log")},"error":error.to_string()}));
            }
        }
    }
    let comparison = (|| -> Result<Value> {
        let records = compare::compatible_records(&directories)?;
        let first = &records[0];
        if grouped {
            let children = record["chains"]
                .as_array()
                .ok_or("missing group chain records")?;
            if children.len() != directories.len() {
                return Err("group chain count mismatch".into());
            }
            for (i, child) in children.iter().enumerate() {
                if child["index"] != i + 1
                    || child["directory"] != format!("chain-{:02}", i + 1)
                    || child["seed"] != records[i]["settings"]["seed"]
                    || records[i]["input"]["sha256"] != record["input"]["sha256"]
                    || records[i]["settings"]["steps"] != record["settings"]["steps"]
                    || records[i]["settings"]["sample_every"] != record["settings"]["sample_every"]
                {
                    return Err("group metadata disagrees with a chain".into());
                }
            }
        }
        let constraints: Vec<Vec<String>> =
            serde_json::from_value(first["model"]["monophyly"].clone())?;
        compare::summarize(
            &directories,
            burnin as usize,
            &constraints,
            first["model"].get("dating"),
            cancel,
            false,
        )
    })();
    let (comparison, consistent) = match comparison {
        Ok(value) => (value, true),
        Err(error) => {
            check(cancel)?;
            (json!({"error":error.to_string()}), false)
        }
    };
    completed &= consistent && diagnostics.iter().all(|d| d.get("error").is_none());
    sections.push(section("completion", if completed {"passed"} else {"attention"},
        if completed {"Recorded steps and sample files agree. Completion is not convergence."} else {"The run is unfinished, incomplete or inconsistent. Review the source records and errors."}));
    let enough = comparison["retained_per_chain"].as_u64().unwrap_or(0) >= 1000;
    let scalar_pass = completed
        && enough
        && comparison["statistics"].as_array().is_some_and(|columns| {
            columns.iter().any(|v| v["fixed_by_model"] != true)
                && columns.iter().all(|v| {
                    v["fixed_by_model"] == true
                        || (v["ess"].as_array().is_some_and(|values| {
                            values
                                .iter()
                                .all(|x| x.as_f64().is_some_and(|x| x >= 200.0))
                        }) && v["lag_limit_reached"]
                            .as_array()
                            .is_some_and(|values| values.iter().all(|x| x == false)))
                })
        });
    sections.push(section("sampling", if !consistent {"unavailable"} else if scalar_pass {"passed"} else {"attention"},
        "Review every sampled scalar: at least 1,000 retained draws and ESS 200 per chain, with no lag truncation. Verified model-fixed values are excluded; stuck sampled values are not."));
    sections.push(section("agreement", if !consistent || directories.len() == 1 {"unavailable"} else if completed && compare::thresholds_met(&comparison, 200.0, 1.01) {"passed"} else {"attention"},
        "Between-chain assessment requires independent seeds, matching input/model, ESS 200 per chain, at least 1,000 retained draws and rank-normalized split R-hat at most 1.01. These diagnostics do not prove convergence."));
    let mut summaries = Vec::new();
    let mut predictions = Vec::new();
    for relative in files.keys() {
        check(cancel)?;
        let name = relative.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !matches!(name, "summary.json" | "predictive.json") {
            continue;
        }
        let value = match summarize::record(&root.join(relative)) {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Presence is not adequacy: only list reports whose recorded source hashes match this snapshot.
        let matched = directories.iter().any(|directory| {
            let prefix = directory.strip_prefix(root).unwrap();
            let trees = files.get(&prefix.join("trees.nexus"));
            if name == "summary.json" {
                value["status"] == "completed"
                    && trees.is_some_and(|h| value["source"]["sha256"] == *h)
            } else {
                value["status"] == "completed"
                    && value["method"] == "posterior-predictive-paired-v1"
                    && trees.is_some_and(|h| value["source"]["trees_sha256"] == *h)
                    && files
                        .get(&prefix.join("trace.log"))
                        .is_some_and(|h| value["source"]["trace_sha256"] == *h)
                    && files
                        .get(&prefix.join("run.json"))
                        .is_some_and(|h| value["source"]["run_sha256"] == *h)
            }
        });
        if matched {
            if name == "summary.json" {
                summaries.push(relative);
            } else {
                predictions.push(relative);
            }
        }
    }
    sections.push(section("trees", if summaries.is_empty() {"unavailable"} else {"review_required"},
        "Review tree summaries and unconstrained clade frequencies. High support or agreement on constant clades does not establish exploration of tree space."));
    sections.push(section("prediction", if predictions.is_empty() {"unavailable"} else {"review_required"},
        "Review paired predictive discrepancies, undefined pairs and simulation uncertainty. A saved check alone does not establish model adequacy."));
    let input = if grouped {
        summarize::record(&directories[0].join("run.json"))
            .ok()
            .map(|v| v["input"].clone())
            .unwrap_or(Value::Null)
    } else {
        record["input"].clone()
    };
    let input_status = input["path"]
        .as_str()
        .map(|p| match sha256::file(Path::new(p)) {
            Ok(h) if input["sha256"] == h => "matches",
            Ok(_) => "changed",
            Err(_) => "unavailable",
        })
        .unwrap_or("unavailable");
    Ok(
        json!({"schema_version":1,"kind":"run_assessment","rove_version":env!("CARGO_PKG_VERSION"),
        "created_unix_seconds":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        "source":root,"chain_count":directories.len(),"burnin_percent":burnin,
        "input":input,"original_matrix_status":input_status,"run_record":record,
        "criteria":{"minimum_retained_per_chain":1000,"min_ess_per_chain":200,"max_rhat":1.01},
        "sections":sections,"diagnostics":diagnostics,"comparison":comparison,
        "tree_reports":summaries,"predictive_reports":predictions,"convergence_certified":false,
        "source_files":files.iter().map(|(p,h)|json!({"path":p,"sha256":h})).collect::<Vec<_>>()}),
    )
}

pub(super) fn run(arguments: &[String], export: bool) -> Result<()> {
    let mut input = None;
    let mut output = None;
    let mut matrix = None;
    let mut burnin = 10u8;
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
                    args.next().ok_or("missing output directory")?,
                ))
            }
            "--include-matrix" if export => {
                matrix = Some(PathBuf::from(args.next().ok_or("missing matrix path")?))
            }
            "--burnin-percent" => burnin = args.next().ok_or("missing burn-in")?.parse()?,
            "--cancel-on-stdin-close" => cancel_on_close = true,
            path if !path.starts_with('-') && input.is_none() => input = Some(PathBuf::from(path)),
            _ => return Err(format!("unsupported assessment/export argument {arg}").into()),
        }
    }
    if burnin >= 100 {
        return Err("burn-in must be between 0 and 99 percent".into());
    }
    let root = input.ok_or("missing run directory")?.canonicalize()?;
    let output = output.ok_or("missing --output new directory")?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let output = parent
        .canonicalize()?
        .join(output.file_name().ok_or("invalid output directory")?);
    if export
        && (output.starts_with(&root)
            || root
                .join("results")
                .canonicalize()
                .is_ok_and(|p| output.starts_with(p)))
    {
        return Err("export destination must be outside the source run".into());
    }
    if output.exists() {
        return Err("output directory already exists".into());
    }
    let cancellation = Cancellation::watch_stdin(cancel_on_close);
    let cancel = || cancellation.requested();
    let before = inventory(&root, &cancel)?;
    let report = assess(&root, burnin, &before, &cancel)?;
    fs::create_dir(&output)?;
    let mut guard = Output(output.clone(), false);
    write(&output.join("assessment.json"), &report)?;
    if export {
        let mut entries = Vec::new();
        let mut omitted_source_matrices = Vec::new();
        for (relative, hash) in &before {
            check(&cancel)?;
            // Predictive checks retain an exact original in source/matrix.*.
            // Matrix consent applies to that copy too, including foreign checks.
            // Include only the explicitly supplied, verified matrix at package root.
            if relative.file_name().is_some_and(|n| {
                n == "matrix.nex" || n == "matrix.fasta" || n == "matrix.mixed.json"
            }) && relative
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|n| n == "source")
            {
                omitted_source_matrices.push(relative);
                continue;
            }
            let target = output.join("run").join(relative);
            fs::create_dir_all(target.parent().unwrap())?;
            fs::copy(root.join(relative), &target)?;
            if sha256::file(&target)? != *hash {
                return Err("source changed while exporting".into());
            }
            entries.push(json!({"path":Path::new("run").join(relative),"sha256":hash,"bytes":fs::metadata(&target)?.len()}));
        }
        if let Some(source) = matrix.as_ref() {
            let hash = sha256::file(source)?;
            if report["input"]["sha256"] != hash {
                return Err("included matrix does not match the run fingerprint".into());
            }
            let raw = fs::read(source)?;
            let name = if report["run_record"]["model"]["data_type"] == "mixed" {
                "matrix.mixed.json"
            } else if rove::alignment::is_fasta(std::str::from_utf8(&raw)?) {
                "matrix.fasta"
            } else {
                "matrix.nex"
            };
            drop(raw);
            let target = output.join(name);
            fs::copy(source, &target)?;
            if sha256::file(&target)? != hash {
                return Err("matrix changed while exporting".into());
            }
            entries.push(json!({"path":name,"sha256":hash,"bytes":fs::metadata(target)?.len()}));
        }
        entries.push(json!({"path":"assessment.json","sha256":sha256::file(&output.join("assessment.json"))?,"bytes":fs::metadata(output.join("assessment.json"))?.len()}));
        write(
            &output.join("manifest.json"),
            &json!({"schema_version":1,"kind":"rove_reproducibility_package",
            "created_unix_seconds":report["created_unix_seconds"],"rove_version":env!("CARGO_PKG_VERSION"),
            "export_executable_sha256":sha256::file(&std::env::current_exe()?)?,"source":root,
            "matrix_included":matrix.is_some(),"files":entries,
            "omitted_source_matrices":omitted_source_matrices,
            "exact_resume":"Requires a valid checkpoint, the original executable fingerprint and original model/input identity. This package does not include the executable. Paths in source records are preserved; review paths before reuse.",
            "scope":"Known Rove records, raw samples and derived-result folders; previous assessments, neighboring files and external comparison sources are not copied. Export does not certify convergence."}),
        )?;
    }
    if inventory(&root, &cancel)? != before {
        return Err(
            "run files changed during assessment/export; retry after the run finishes".into(),
        );
    }
    check(&cancel)?;
    guard.1 = true;
    Ok(())
}
