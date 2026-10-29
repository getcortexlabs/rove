use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MATRIX: &str = "#NEXUS
BEGIN DATA;
  DIMENSIONS NTAX=5 NCHAR=6;
  FORMAT SYMBOLS=\"01\" MISSING=? GAP=-;
  MATRIX
    A 010110
    B 011010
    C 101-01
    D 10?101
    E_e 110011
  ;
END;
";

/// A fresh directory for one test, under Cargo's target directory.
fn scratch(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn rove(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rove"))
        .args(arguments)
        .output()
        .unwrap()
}

fn error(output: &Output) -> String {
    assert!(!output.status.success(), "unexpected success");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_run_writes_matching_traces_trees_and_metadata() {
    let directory = scratch("run");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let output = directory.join("out");
    let result = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--seed",
        "3",
        "--steps",
        "2000",
        "--sample-every",
        "100",
        "--gaps",
        "missing",
        "--monophyly",
        "A,B",
        "--monophyly",
        "A,B,E_e",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let trace = fs::read_to_string(output.join("trace.log")).unwrap();
    let rows: Vec<&str> = trace.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(rows[0].split('\t').count(), 9);
    assert_eq!(rows.len(), 1 + 21);
    let samples: Vec<u64> = rows[1..]
        .iter()
        .map(|row| row.split('\t').next().unwrap().parse().unwrap())
        .collect();
    assert_eq!(samples, (0..=2000).step_by(100).collect::<Vec<u64>>());
    assert!(rows[1..].iter().all(|row| {
        row.split('\t')
            .all(|value| value.parse::<f64>().is_ok_and(f64::is_finite))
    }));

    let trees = fs::read_to_string(output.join("trees.nexus")).unwrap();
    assert!(trees.starts_with("#NEXUS"));
    assert!(trees.contains("\t\t5 E_e\n;"));
    assert_eq!(
        trees
            .lines()
            .filter(|l| l.starts_with("tree STATE_"))
            .count(),
        21
    );
    assert!(trees.trim_end().ends_with("End;"));
    for line in trees.lines().filter(|l| l.starts_with("tree STATE_")) {
        assert!(
            line.contains("(1:") && line.contains(",2:")
                || line.contains("(2:") && line.contains(",1:"),
            "{line}"
        );
    }

    let metadata = fs::read_to_string(output.join("run.json")).unwrap();
    for field in [
        "\"sha256\"",
        "\"executable_sha256\"",
        "\"seed\": 3",
        "\"monophyly\"",
        "\"wilson_balding\"",
        "\"tuning\": true",
    ] {
        assert!(metadata.contains(field), "{field} missing from run.json");
    }

    let again = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--seed",
        "3",
        "--steps",
        "100",
        "--sample-every",
        "100",
        "--gaps",
        "missing",
    ]);
    assert!(error(&again).contains("must not exist"));
}

#[test]
fn reference_model_writes_rate_parameters_priors_and_annotated_branches() {
    let directory = scratch("reference-model");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let output = directory.join("out");
    let result = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--model",
        "reference",
        "--output",
        output.to_str().unwrap(),
        "--seed",
        "3",
        "--steps",
        "2000",
        "--sample-every",
        "100",
        "--gaps",
        "missing",
        "--monophyly",
        "A,B",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let trace = fs::read_to_string(output.join("trace.log")).unwrap();
    let rows: Vec<_> = trace.lines().filter(|l| !l.starts_with('#')).collect();
    let columns: Vec<_> = rows[0].split('\t').collect();
    assert_eq!(rows.len(), 22);
    assert_eq!(columns.len(), 15);
    let index = |name| columns.iter().position(|&c| c == name).unwrap();
    for row in &rows[1..] {
        let values: Vec<f64> = row.split('\t').map(|v| v.parse().unwrap()).collect();
        assert_eq!(values.len(), columns.len());
        assert!(values.iter().all(|v| v.is_finite()));
        let prior = values[index("birth_death")] - 1000.0f64.ln()
            + values[index("gamma_prior")]
            + values[index("clock_rate_prior")]
            + values[index("clock_sigma_prior")];
        assert!((prior - values[index("prior")]).abs() < 1e-11);
        assert!(
            (values[index("prior")] + values[index("likelihood")] - values[index("posterior")])
                .abs()
                < 1e-11
        );
    }
    let trees = fs::read_to_string(output.join("trees.nexus")).unwrap();
    for line in trees.lines().filter(|l| l.starts_with("tree STATE_")) {
        assert_eq!(line.matches("[&rate=").count(), 8);
    }
    let metadata = fs::read_to_string(output.join("run.json")).unwrap();
    for field in [
        "\"preset\": \"reference\"",
        "\"gamma_shape_prior\"",
        "\"clock_sigma_prior\"",
        "\"branch_rate_parameters\"",
        "\"rate_mapping\"",
        "\"gamma_shape_scale\"",
        "\"clock_sigma_scale\"",
        "\"branch_rate_scale\"",
    ] {
        assert!(metadata.contains(field), "missing {field}");
    }
}

#[test]
fn unsupported_inputs_and_settings_are_rejected() {
    let directory = scratch("errors");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let ternary = directory.join("ternary.nex");
    fs::write(
        &ternary,
        MATRIX
            .replace("\"01\"", "\"012\"")
            .replace("A 010110", "A 010112"),
    )
    .unwrap();
    let path = matrix.to_str().unwrap();
    let out = directory.join("never").to_str().unwrap().to_string();
    let base = [
        "--output",
        &out,
        "--seed",
        "1",
        "--steps",
        "10",
        "--sample-every",
        "5",
    ];
    let run = |extra: &[&str], input: &str| {
        let mut arguments = vec!["sample", input];
        arguments.extend(base);
        arguments.extend(extra);
        error(&rove(&arguments))
    };
    assert!(
        run(&[], path).contains("gap"),
        "gaps must be chosen explicitly"
    );
    assert!(run(&["--gaps", "missing", "--monophyly", "A,Z"], path).contains("unknown taxon"));
    assert!(run(&["--gaps", "missing", "--monophyly", "A"], path).contains("constraint 1"));
    assert!(
        run(&["--gaps", "missing", "--clock", "relaxed"], path)
            .contains("unsupported option --clock")
    );
    assert!(
        run(&["--gaps", "missing"], ternary.to_str().unwrap()).contains("binary characters only")
    );
    let uneven = rove(&[
        "sample",
        path,
        "--output",
        &out,
        "--seed",
        "1",
        "--steps",
        "10",
        "--sample-every",
        "3",
    ]);
    assert!(error(&uneven).contains("multiple of --sample-every"));
    let missing = rove(&[
        "sample",
        path,
        "--output",
        &out,
        "--steps",
        "10",
        "--sample-every",
        "5",
    ]);
    assert!(error(&missing).contains("--seed is required"));
    assert!(!Path::new(&out).exists());
}

#[test]
fn fixed_tuning_is_recorded_and_closing_standard_error_does_not_stop_a_run() {
    use std::process::Stdio;
    let directory = scratch("fixed");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let output = directory.join("out");
    let mut child = Command::new(env!("CARGO_BIN_EXE_rove"))
        .args([
            "sample",
            matrix.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--seed",
            "4",
            "--steps",
            "3000",
            "--sample-every",
            "1000",
            "--print-every",
            "1",
            "--gaps",
            "missing",
            "--no-tuning",
            "--tuning",
            "subtree_slide=0.02",
        ])
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stderr.take());
    assert!(child.wait().unwrap().success());
    let metadata = fs::read_to_string(output.join("run.json")).unwrap();
    assert!(metadata.contains("\"tuning\": false"));
    let slide = metadata.split("\"subtree_slide\"").nth(1).unwrap();
    assert!(slide.contains("\"initial_tuning\": 0.02") && slide.contains("\"final_tuning\": 0.02"));
}

#[test]
fn labels_repeated_options_and_degenerate_inputs_are_rejected() {
    let directory = scratch("labels");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let quoted = directory.join("quoted.nex");
    fs::write(&quoted, MATRIX.replace("E_e", "'E, e'")).unwrap();
    let single = directory.join("single.nex");
    fs::write(
        &single,
        "#NEXUS\nBEGIN DATA; DIMENSIONS NTAX=1 NCHAR=2; MATRIX A 01; END;\n",
    )
    .unwrap();
    let out = directory.join("never").to_str().unwrap().to_string();
    let run = |input: &Path, extra: &[&str]| {
        let mut arguments = vec![
            "sample",
            input.to_str().unwrap(),
            "--output",
            &out,
            "--seed",
            "1",
            "--steps",
            "10",
            "--sample-every",
            "5",
            "--gaps",
            "missing",
        ];
        arguments.extend(extra);
        error(&rove(&arguments))
    };
    assert!(run(&quoted, &[]).contains("cannot be written so that BEAST reads it back"));
    let message = run(&single, &["--conditioning", "none"]);
    assert!(message.contains("at least two taxa"), "{message}");
    assert!(run(&matrix, &["--seed", "2"]).contains("--seed is given more than once"));
    assert!(
        run(&matrix, &["--tuning", "tree_scale=1.5"])
            .contains("invalid tuning value for tree_scale")
    );
    assert!(run(&matrix, &["--tuning", "spin=0.1"]).contains("--tuning needs <move>=<value>"));
    assert!(!Path::new(&out).exists());
}

#[test]
fn the_backend_is_recorded_and_does_not_change_the_samples() {
    let directory = scratch("backend");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let run = |backend: &str| {
        let output = directory.join(backend);
        let result = rove(&[
            "sample",
            matrix.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--seed",
            "5",
            "--steps",
            "2000",
            "--sample-every",
            "100",
            "--gaps",
            "missing",
            "--backend",
            backend,
        ]);
        (output, result)
    };
    let (portable, result) = run("portable");
    assert!(result.status.success());
    let samples = |output: &Path| {
        let trace = fs::read_to_string(output.join("trace.log")).unwrap();
        let trees = fs::read_to_string(output.join("trees.nexus")).unwrap();
        let keep = |text: &str, comment: &str| {
            text.lines()
                .filter(|line| !line.starts_with(comment))
                .collect::<Vec<_>>()
                .join("\n")
        };
        (keep(&trace, "#"), keep(&trees, "[command"))
    };
    let record = fs::read_to_string(portable.join("run.json")).unwrap();
    assert!(record.contains("\"backend\": \"portable\""), "{record}");
    let (neon, result) = run("neon");
    if rove::Backend::Neon.is_available() {
        assert!(result.status.success());
        assert_eq!(samples(&neon), samples(&portable));
    } else {
        assert!(error(&result).contains("the neon backend is not available"));
        assert!(!neon.exists());
    }
    let (_, result) = run("avx2");
    assert!(error(&result).contains("--backend must be portable or neon"));
}
