use serde_json::Value;
use std::{fs, path::Path, process::Command};
const BIN: &str = env!("CARGO_BIN_EXE_rove");
#[test]
fn combine_is_explicit_self_contained_and_refuses_invalid_models_before_output() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("mixed-cli");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let morph = root.join("morph.nex");
    let dna = root.join("dna.fasta");
    let manifest = root.join("input.mixed.json");
    let morphology = include_str!("../fixtures/validation/mixed/morphology.nex");
    fs::write(&morph, morphology).unwrap();
    fs::write(
        &dna,
        include_str!("../fixtures/validation/dna/complete.fasta"),
    )
    .unwrap();
    let combine = |output: &Path| {
        Command::new(BIN)
            .arg("combine")
            .arg("--morphology")
            .arg(&morph)
            .arg("--dna")
            .arg(&dna)
            .arg("--output")
            .arg(output)
            .output()
            .unwrap()
    };
    assert!(combine(&manifest).status.success());
    assert!(!combine(&morph).status.success());
    assert_eq!(fs::read_to_string(&morph).unwrap(), morphology);
    fs::remove_file(&morph).unwrap();
    fs::remove_file(&dna).unwrap();
    let inspect = Command::new(BIN)
        .arg("inspect")
        .arg(&manifest)
        .output()
        .unwrap();
    assert!(inspect.status.success());
    let report: Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(report["data_type"], "mixed");
    assert_eq!(report["mixed"]["sources"][1]["name"], "dna.fasta");
    assert!(report["variable_error"].is_null());
    let sample = |name: &str, extra: &[&str]| {
        Command::new(BIN)
            .arg("sample")
            .arg(&manifest)
            .arg("--output")
            .arg(root.join(name))
            .args(["--seed", "42", "--steps", "400", "--sample-every", "20"])
            .args(extra)
            .output()
            .unwrap()
    };
    for args in [
        vec![],
        vec!["--morphology-rate", "0"],
        vec!["--morphology-rate", "NaN"],
        vec!["--morphology-rate", "2", "--morphology-rate-log-sd", "1"],
        vec![
            "--morphology-rate",
            "2",
            "--weight",
            "morphology_rate_scale=1",
        ],
    ] {
        let outcome = sample("invalid", &args);
        assert!(!outcome.status.success(), "{args:?}");
        assert!(!root.join("invalid").exists());
    }
    let run = sample(
        "run",
        &[
            "--morphology-rate",
            "estimate",
            "--morphology-rate-log-sd",
            "0.7",
        ],
    );
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let record: Value =
        serde_json::from_slice(&fs::read(root.join("run/run.json")).unwrap()).unwrap();
    assert_eq!(
        record["model"]["mixed_model"]["morphology_rate_prior"]["log_sd"],
        0.7
    );
    assert_eq!(record["input"]["mixed"], report["mixed"]);
    let prediction = root.join("run/predictions/check");
    fs::create_dir_all(prediction.parent().unwrap()).unwrap();
    let outcome = Command::new(BIN)
        .arg("predict")
        .arg(root.join("run"))
        .arg("--matrix")
        .arg(&manifest)
        .arg("--output")
        .arg(&prediction)
        .args([
            "--burnin-percent",
            "10",
            "--seed",
            "21",
            "--replicates",
            "2",
        ])
        .output()
        .unwrap();
    assert!(
        outcome.status.success(),
        "{}",
        String::from_utf8_lossy(&outcome.stderr)
    );
    for include in [false, true] {
        let output = root.join(format!("export-{include}"));
        let mut command = Command::new(BIN);
        command
            .arg("export-run")
            .arg(root.join("run"))
            .arg("--output")
            .arg(&output);
        if include {
            command.arg("--include-matrix").arg(&manifest);
        }
        let outcome = command.output().unwrap();
        assert!(
            outcome.status.success(),
            "{}",
            String::from_utf8_lossy(&outcome.stderr)
        );
        assert_eq!(output.join("matrix.mixed.json").exists(), include);
        assert!(
            !output
                .join("run/predictions/check/source/matrix.mixed.json")
                .exists()
        );
        assert!(
            output
                .join("run/predictions/check/matrices/replicate-00001.mixed.json")
                .exists()
        );
        if include {
            assert_eq!(
                fs::read(output.join("matrix.mixed.json")).unwrap(),
                fs::read(&manifest).unwrap()
            );
        }
    }
}
