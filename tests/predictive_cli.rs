use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn scratch(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}
fn rove(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rove"))
        .args(args)
        .output()
        .unwrap()
}
fn good(result: Output) -> Output {
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    result
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn fit(path: &Path, model: &str, coding: &str, ordered: bool, missing: bool) {
    let rows = if ordered {
        "A 002000 B 012020 C 220121 D 220321"
    } else if missing {
        "A 0000 B 00?1 C 111- D 1110"
    } else {
        "A 0000 B 0011 C 1100 D 1111"
    };
    let text = format!(
        "#NEXUS BEGIN DATA; DIMENSIONS NTAX=4 NCHAR={}; FORMAT SYMBOLS=\"{}\" MISSING=? GAP=-; MATRIX {rows}; END; {}",
        if ordered { 6 } else { 4 },
        if ordered { "0123" } else { "01" },
        if ordered {
            "BEGIN ASSUMPTIONS; TYPESET * fitted = ORD: 2-3 5; END;"
        } else {
            ""
        }
    );
    fs::write(path.join("input.nex"), text).unwrap();
    good(rove(&[
        "sample",
        path.join("input.nex").to_str().unwrap(),
        "--output",
        path.join("run").to_str().unwrap(),
        "--model",
        model,
        "--conditioning",
        coding,
        "--gaps",
        "missing",
        "--seed",
        "123",
        "--steps",
        "200",
        "--sample-every",
        "10",
    ]));
}
fn predict(path: &Path, name: &str) -> Output {
    rove(&[
        "predict",
        path.join("run").to_str().unwrap(),
        "--matrix",
        path.join("input.nex").to_str().unwrap(),
        "--output",
        path.join(name).to_str().unwrap(),
        "--seed",
        "555",
        "--burnin-percent",
        "25",
        "--replicates",
        "6",
    ])
}

#[test]
fn completed_runs_preserve_their_models_and_replay_paired_replicates() {
    for (i, (model, coding, ordered, missing)) in [
        ("strict", "none", false, false),
        ("reference", "variable", false, false),
        ("reference", "informative", false, false),
        ("reference", "variable", true, false),
        ("strict", "variable", false, true),
    ]
    .into_iter()
    .enumerate()
    {
        let path = scratch(&format!("predictive-cli-{i}"));
        fit(&path, model, coding, ordered, missing);
        let source: Vec<_> = ["trace.log", "trees.nexus", "run.json"]
            .map(|n| fs::read(path.join("run").join(n)).unwrap())
            .into();
        good(predict(&path, "prediction"));
        good(predict(&path, "replay"));
        let report = read(&path.join("prediction/predictive.json"));
        assert_eq!(report["status"], "completed");
        assert_eq!(report["source"]["discarded_samples"], 5);
        assert_eq!(report["source"]["retained_samples"], 16);
        assert_eq!(report["replicates"], 6);
        assert_eq!(report["convergence_assessed"], false);
        assert_eq!(report["global"]["parsimony_length"]["valid_pairs"], 6);
        assert!(
            report["source"]["max_reconstructed_likelihood_difference"]
                .as_f64()
                .unwrap()
                < 1e-7
        );
        for name in ["replicates.jsonl", "characters.tsv"] {
            assert_eq!(
                fs::read(path.join("prediction").join(name)).unwrap(),
                fs::read(path.join("replay").join(name)).unwrap()
            );
        }
        for line in fs::read_to_string(path.join("prediction/replicates.jsonl"))
            .unwrap()
            .lines()
        {
            let record: Value = serde_json::from_str(line).unwrap();
            assert!(record["source_sample_index"].as_u64().unwrap() >= 5);
            assert_eq!(
                record["state"].as_u64().unwrap(),
                record["source_sample_index"].as_u64().unwrap() * 10
            );
            let filename = record["matrix"].as_str().unwrap();
            let a = fs::read_to_string(path.join("prediction").join(filename)).unwrap();
            assert_eq!(
                a,
                fs::read_to_string(path.join("replay").join(filename)).unwrap()
            );
            let matrix = rove::nexus::parse(&a).unwrap();
            assert_eq!(matrix.taxa().len(), 4);
            assert_eq!(matrix.character_count(), if ordered { 6 } else { 4 });
        }
        for (name, bytes) in ["trace.log", "trees.nexus", "run.json"]
            .into_iter()
            .zip(source)
        {
            assert_eq!(fs::read(path.join("run").join(name)).unwrap(), bytes);
        }
        let before = fs::read(path.join("prediction/predictive.json")).unwrap();
        assert!(!predict(&path, "prediction").status.success());
        assert_eq!(
            before,
            fs::read(path.join("prediction/predictive.json")).unwrap()
        );
    }
}

#[test]
fn mismatched_inputs_trace_states_likelihoods_and_incomplete_runs_fail_without_results() {
    let path = scratch("predictive-cli-rejections");
    fit(&path, "reference", "variable", false, false);
    let original = fs::read(path.join("input.nex")).unwrap();
    fs::write(
        path.join("input.nex"),
        [original.as_slice(), b"\n"].concat(),
    )
    .unwrap();
    let result = predict(&path, "wrong-matrix");
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("fingerprint"));
    fs::write(path.join("input.nex"), original).unwrap();
    let record = read(&path.join("run/run.json"));
    for (name, mut bad) in [("partial", record.clone()), ("prior", record.clone())] {
        if name == "partial" {
            bad["status"] = "cancelled".into();
        } else {
            bad["model"]["sample_from_prior"] = true.into();
        }
        fs::write(path.join("run/run.json"), serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(!predict(&path, name).status.success());
        assert!(!path.join(name).exists());
    }
    fs::write(
        path.join("run/run.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
    let trace = fs::read_to_string(path.join("run/trace.log")).unwrap();
    let bad = trace
        .lines()
        .map(|line| {
            if line.starts_with('#') || line.starts_with("Sample") {
                return line.to_string();
            }
            let mut fields: Vec<_> = line.split('\t').map(str::to_string).collect();
            fields[2] = (fields[2].parse::<f64>().unwrap() + 1.0).to_string();
            fields.join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(path.join("run/trace.log"), bad).unwrap();
    let result = predict(&path, "wrong-likelihood");
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("likelihood"));
    assert!(!path.join("wrong-likelihood").exists());
    fs::write(path.join("run/trace.log"), trace).unwrap();
    let trees = fs::read_to_string(path.join("run/trees.nexus")).unwrap();
    fs::write(
        path.join("run/trees.nexus"),
        trees.replace("STATE_100", "STATE_101"),
    )
    .unwrap();
    assert!(!predict(&path, "wrong-state").status.success());
    assert!(!path.join("wrong-state").exists());
    fs::write(
        path.join("run/trees.nexus"),
        trees.trim_end().strip_suffix("End;").unwrap(),
    )
    .unwrap();
    assert!(!predict(&path, "truncated").status.success());
    assert!(!path.join("truncated").exists());
}

#[test]
fn closing_the_owner_cancels_prediction_without_publishing_partial_output() {
    let path = scratch("predictive-owner-close");
    fit(&path, "strict", "variable", false, false);
    let mut child = Command::new(env!("CARGO_BIN_EXE_rove"))
        .args([
            "predict",
            path.join("run").to_str().unwrap(),
            "--matrix",
            path.join("input.nex").to_str().unwrap(),
            "--output",
            path.join("prediction").to_str().unwrap(),
            "--seed",
            "3",
            "--burnin-percent",
            "25",
            "--replicates",
            "10000",
            "--cancel-on-stdin-close",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdin.take());
    let result = child.wait_with_output().unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cancelled"));
    assert!(!path.join("prediction").exists());
}
