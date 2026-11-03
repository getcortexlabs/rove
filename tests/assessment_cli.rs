use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn call(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rove"))
        .args(args)
        .output()
        .unwrap()
}
fn good(output: Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn fixture(name: &str, chains: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("matrix.nex"), "#NEXUS BEGIN DATA; DIMENSIONS NTAX=4 NCHAR=4; FORMAT SYMBOLS=\"01\"; MATRIX A 0000 B 0011 C 1100 D 1111; END;").unwrap();
    good(call(&[
        "sample",
        root.join("matrix.nex").to_str().unwrap(),
        "--output",
        root.join("run").to_str().unwrap(),
        "--seed",
        "42",
        "--steps",
        "100",
        "--sample-every",
        "10",
        "--chains",
        chains,
    ]));
    root
}
fn assess(root: &Path, name: &str) -> Value {
    good(call(&[
        "assess",
        root.join("run").to_str().unwrap(),
        "--output",
        root.join(name).to_str().unwrap(),
    ]));
    read(root.join(name).join("assessment.json"))
}
fn status<'a>(report: &'a Value, id: &str) -> &'a Value {
    &report["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == id)
        .unwrap()["status"]
}

#[test]
fn review_distinguishes_finished_short_single_chain_from_convergence() {
    let root = fixture("assessment-single", "1");
    let report = assess(&root, "review");
    assert_eq!(status(&report, "completion"), "passed");
    assert_eq!(status(&report, "sampling"), "attention");
    assert_eq!(status(&report, "agreement"), "unavailable");
    assert_eq!(status(&report, "prediction"), "unavailable");
    assert_eq!(report["convergence_certified"], false);
    assert_eq!(report["original_matrix_status"], "matches");
    let log = root.join("run/trace.log");
    let original = fs::read_to_string(&log).unwrap();
    fs::write(
        &log,
        original
            .lines()
            .take(original.lines().count() - 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .unwrap();
    let bad = assess(&root, "truncated");
    assert_eq!(status(&bad, "completion"), "attention");
    assert!(
        bad["diagnostics"][0]["error"]
            .as_str()
            .unwrap()
            .contains("truncated")
    );
}

#[test]
fn chain_group_checks_child_identity_before_assessing_agreement() {
    let root = fixture("assessment-group", "2");
    let report = assess(&root, "review");
    assert_eq!(status(&report, "completion"), "passed");
    assert_eq!(status(&report, "agreement"), "attention");
    let file = root.join("run/chain-02/run.json");
    let mut record = read(&file);
    record["model"]["partitioning"] = Value::String("declared".into());
    fs::write(file, serde_json::to_vec(&record).unwrap()).unwrap();
    let incompatible = assess(&root, "mixed");
    assert_eq!(status(&incompatible, "agreement"), "unavailable");
    assert_eq!(status(&incompatible, "completion"), "attention");
}

#[test]
fn archive_preserves_sources_and_requires_explicit_matching_matrix() {
    let root = fixture("assessment-export", "1");
    let original = fs::read(root.join("run/trace.log")).unwrap();
    good(call(&[
        "export-run",
        root.join("run").to_str().unwrap(),
        "--output",
        root.join("package").to_str().unwrap(),
    ]));
    let manifest = read(root.join("package/manifest.json"));
    assert_eq!(manifest["matrix_included"], false);
    assert!(!root.join("package/matrix.nex").exists());
    assert_eq!(
        fs::read(root.join("package/run/trace.log")).unwrap(),
        original
    );
    assert_eq!(fs::read(root.join("run/trace.log")).unwrap(), original);
    assert!(
        manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["sha256"].as_str().unwrap().len() == 64)
    );
    good(call(&[
        "export-run",
        root.join("run").to_str().unwrap(),
        "--output",
        root.join("with-matrix").to_str().unwrap(),
        "--include-matrix",
        root.join("matrix.nex").to_str().unwrap(),
    ]));
    assert_eq!(
        fs::read(root.join("with-matrix/matrix.nex")).unwrap(),
        fs::read(root.join("matrix.nex")).unwrap()
    );
    fs::write(root.join("matrix.nex"), "changed").unwrap();
    let changed = assess(&root, "changed");
    assert_eq!(changed["original_matrix_status"], "changed");
    let failed = call(&[
        "export-run",
        root.join("run").to_str().unwrap(),
        "--output",
        root.join("rejected").to_str().unwrap(),
        "--include-matrix",
        root.join("matrix.nex").to_str().unwrap(),
    ]);
    assert!(!failed.status.success());
    assert!(!root.join("rejected").exists());
    assert!(
        !call(&[
            "export-run",
            root.join("run").to_str().unwrap(),
            "--output",
            root.join("package").to_str().unwrap()
        ])
        .status
        .success()
    );
    assert!(
        !call(&[
            "export-run",
            root.join("run").to_str().unwrap(),
            "--output",
            root.join("run/nested").to_str().unwrap()
        ])
        .status
        .success()
    );
}

#[test]
fn reports_require_matching_source_fingerprints() {
    let root = fixture("assessment-reports", "1");
    fs::create_dir(root.join("run/summaries")).unwrap();
    good(call(&[
        "summarize",
        root.join("run/trees.nexus").to_str().unwrap(),
        "--output",
        root.join("run/summaries/first").to_str().unwrap(),
        "--burnin-percent",
        "10",
        "--heights",
        "keep",
    ]));
    let report = assess(&root, "review");
    assert_eq!(status(&report, "trees"), "review_required");
    let file = root.join("run/summaries/first/summary.json");
    let mut summary = read(&file);
    summary["source"]["sha256"] = Value::String("0".repeat(64));
    fs::write(&file, serde_json::to_vec(&summary).unwrap()).unwrap();
    let changed = assess(&root, "changed");
    assert_eq!(status(&changed, "trees"), "unavailable");
}

#[cfg(unix)]
#[test]
fn exports_reject_symlinked_artifacts() {
    let root = fixture("assessment-symlinks", "1");
    fs::create_dir(root.join("run/predictions")).unwrap();
    std::os::unix::fs::symlink(root.join("matrix.nex"), root.join("run/predictions/linked"))
        .unwrap();
    assert!(
        !call(&[
            "export-run",
            root.join("run").to_str().unwrap(),
            "--output",
            root.join("package").to_str().unwrap()
        ])
        .status
        .success()
    );
    assert!(!root.join("package").exists());
}

#[test]
fn predictive_source_copies_respect_matrix_consent() {
    let root = fixture("assessment-predictive-matrix-consent", "1");
    fs::create_dir(root.join("run/predictions")).unwrap();
    good(call(&[
        "predict",
        root.join("run").to_str().unwrap(),
        "--matrix",
        root.join("matrix.nex").to_str().unwrap(),
        "--output",
        root.join("run/predictions/check").to_str().unwrap(),
        "--seed",
        "7",
        "--replicates",
        "2",
        "--burnin-percent",
        "10",
    ]));
    let original = root.join("run/predictions/check/source/matrix.nex");
    assert!(original.exists());
    for include in [false, true] {
        let name = if include { "included" } else { "excluded" };
        let package = root.join(name);
        let source = root.join("run");
        let matrix = root.join("matrix.nex");
        let mut args = vec![
            "export-run",
            source.to_str().unwrap(),
            "--output",
            package.to_str().unwrap(),
        ];
        if include {
            args.extend(["--include-matrix", matrix.to_str().unwrap()]);
        }
        good(call(&args));
        assert_eq!(package.join("matrix.nex").exists(), include);
        assert!(
            !package
                .join("run/predictions/check/source/matrix.nex")
                .exists()
        );
        assert!(
            package
                .join("run/predictions/check/matrices/replicate-00001.nex")
                .exists()
        );
        let manifest = read(package.join("manifest.json"));
        assert_eq!(
            manifest["omitted_source_matrices"][0],
            "predictions/check/source/matrix.nex"
        );
        assert_eq!(
            status(&read(package.join("assessment.json")), "prediction"),
            "review_required"
        );
    }
    assert_eq!(
        fs::read(original).unwrap(),
        fs::read(root.join("matrix.nex")).unwrap()
    );
}
