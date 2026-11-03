use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_rove");

#[test]
fn dna_defaults_are_recorded_and_morphological_options_are_rejected() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("dna-cli");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let matrix = root.join("matrix.fasta");
    fs::write(
        &matrix,
        include_str!("../fixtures/validation/dna/matrix.fasta"),
    )
    .unwrap();
    let sample = |name: &str, extra: &[&str]| {
        Command::new(BIN)
            .arg("sample")
            .arg(&matrix)
            .arg("--output")
            .arg(root.join(name))
            .args([
                "--seed",
                "42",
                "--steps",
                "200",
                "--sample-every",
                "10",
                "--gaps",
                "missing",
            ])
            .args(extra)
            .output()
            .unwrap()
    };
    let valid = sample("run", &["--monophyly", "Alpha_one,Beta_two"]);
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let record: Value =
        serde_json::from_slice(&fs::read(root.join("run/run.json")).unwrap()).unwrap();
    assert_eq!(record["model"]["data_type"], "dna");
    assert_eq!(record["model"]["substitution_model"], "jc69");
    assert_eq!(record["model"]["conditioning"], "none (JC69)");
    assert_eq!(record["model"]["partitioning"], "declared");
    assert_eq!(record["model"]["partitions"][0]["states"], 4);
    assert_eq!(record["model"]["monophyly"][0][0], "Alpha one");
    for options in [
        ["--conditioning", "variable"],
        ["--conditioning", "informative"],
        ["--partitioning", "observed"],
        ["--partitioning", "labels"],
    ] {
        let rejected = sample("invalid", &options);
        assert!(!rejected.status.success(), "{options:?}");
        assert!(!root.join("invalid").exists());
    }
    let report = Command::new(BIN)
        .arg("inspect")
        .arg(&matrix)
        .args(["--gaps", "missing"])
        .output()
        .unwrap();
    assert!(report.status.success());
    let report: Value = serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(report["data_type"], "dna");
    assert_eq!(report["input_format"], "fasta");
    assert!(report["sampler_error"].is_null());

    let destination = root.join("export");
    let export = Command::new(BIN)
        .arg("export-run")
        .arg(root.join("run"))
        .arg("--output")
        .arg(&destination)
        .arg("--include-matrix")
        .arg(&matrix)
        .output()
        .unwrap();
    assert!(
        export.status.success(),
        "{}",
        String::from_utf8_lossy(&export.stderr)
    );
    assert_eq!(
        fs::read(destination.join("matrix.fasta")).unwrap(),
        fs::read(matrix).unwrap()
    );
    let exported: Value =
        serde_json::from_slice(&fs::read(destination.join("run/run.json")).unwrap()).unwrap();
    assert_eq!(exported["model"], record["model"]);
}

#[test]
fn fasta_predictive_source_copies_respect_export_inclusion() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("dna-predict-export");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let matrix = root.join("complete.fasta");
    fs::write(
        &matrix,
        include_str!("../fixtures/validation/dna/complete.fasta"),
    )
    .unwrap();
    let run = root.join("run");
    let sample = Command::new(BIN)
        .arg("sample")
        .arg(&matrix)
        .arg("--output")
        .arg(&run)
        .args(["--seed", "42", "--steps", "200", "--sample-every", "10"])
        .output()
        .unwrap();
    assert!(
        sample.status.success(),
        "{}",
        String::from_utf8_lossy(&sample.stderr)
    );
    fs::create_dir(run.join("predictions")).unwrap();
    let prediction = run.join("predictions/check");
    let check = Command::new(BIN)
        .arg("predict")
        .arg(&run)
        .arg("--matrix")
        .arg(&matrix)
        .arg("--output")
        .arg(&prediction)
        .args(["--replicates", "2", "--seed", "3", "--burnin-percent", "10"])
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert_eq!(
        fs::read(prediction.join("source/matrix.fasta")).unwrap(),
        fs::read(&matrix).unwrap()
    );
    for include in [false, true] {
        let destination = root.join(format!("export-{include}"));
        let mut command = Command::new(BIN);
        command
            .arg("export-run")
            .arg(&run)
            .arg("--output")
            .arg(&destination);
        if include {
            command.arg("--include-matrix").arg(&matrix);
        }
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(destination.join("matrix.fasta").exists(), include);
        assert!(
            !destination
                .join("run/predictions/check/source/matrix.fasta")
                .exists()
        );
        assert!(
            destination
                .join("run/predictions/check/matrices/replicate-00001.nex")
                .exists()
        );
    }
    assert!(prediction.join("source/matrix.fasta").exists());
}
