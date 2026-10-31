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
fn events_do_not_change_the_sampled_trace_or_trees() {
    let directory = scratch("sample-events");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let plain = directory.join("plain");
    let streamed = directory.join("streamed");
    let run = |path: &Path, events: bool| {
        let mut args = vec![
            "sample",
            matrix.to_str().unwrap(),
            "--output",
            path.to_str().unwrap(),
            "--seed",
            "42",
            "--steps",
            "40",
            "--sample-every",
            "10",
            "--gaps",
            "missing",
            "--conditioning",
            "none",
            "--model",
            "reference",
            "--print-every",
            "10",
        ];
        if events {
            args.push("--events-json");
        }
        let result = rove(&args);
        assert!(result.status.success(), "{}", error_text(&result));
        result
    };
    run(&plain, false);
    let result = run(&streamed, true);
    let events = String::from_utf8(result.stdout).unwrap();
    let lines: Vec<_> = events.lines().collect();
    assert_eq!(lines.len(), 6);
    assert!(lines[0].contains("\"event\":\"started\""));
    assert!(lines[5].contains("\"event\":\"completed\""));
    assert!(lines[5].contains("\"step\":40,"));
    assert!(
        lines
            .iter()
            .all(|line| line.starts_with("{\"schema_version\":1,") && line.ends_with('}'))
    );
    for name in ["trace.log", "trees.nexus"] {
        let read = |path: &Path| {
            fs::read_to_string(path.join(name))
                .unwrap()
                .lines()
                .filter(|line| !line.starts_with("# command") && !line.starts_with("[command"))
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(read(&plain), read(&streamed));
    }
}

fn error_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn informative_coding_checks_input_and_records_the_selected_model() {
    let directory = scratch("informative-cli");
    let matrix = directory.join("matrix.nex");
    fs::write(
        &matrix,
        include_str!("../fixtures/validation/informative/matrix.nex"),
    )
    .unwrap();
    let inspect = |path: &Path| {
        let result = rove(&["inspect", path.to_str().unwrap()]);
        assert!(result.status.success(), "{}", error_text(&result));
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()
    };
    assert!(inspect(&matrix)["informative_error"].is_null());
    let run = |input: &Path, output: &Path| {
        rove(&[
            "sample",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--conditioning",
            "informative",
            "--steps",
            "100",
            "--sample-every",
            "10",
            "--seed",
            "42",
            "--model",
            "reference",
        ])
    };
    let output = directory.join("run");
    let result = run(&matrix, &output);
    assert!(result.status.success(), "{}", error_text(&result));
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("run.json")).unwrap()).unwrap();
    assert!(
        metadata
            .to_string()
            .contains("parsimony-informative characters (Mkparsinf)")
    );

    // Original site 4 becomes the first site of the three-state partition.
    let invalid = directory.join("invalid.nex");
    fs::write(&invalid, "#NEXUS BEGIN DATA; DIMENSIONS NTAX=5 NCHAR=4; FORMAT SYMBOLS=\"012\"; MATRIX a 0010 b 0010 c 1100 d 1101 e 0102; END;").unwrap();
    let report = inspect(&invalid);
    assert!(report["variable_error"].is_null());
    assert!(
        report["informative_error"]
            .as_str()
            .unwrap()
            .contains("character 4")
    );
    let rejected = directory.join("rejected");
    let result = run(&invalid, &rejected);
    assert!(error(&result).contains("character 4"));
    assert!(!rejected.exists());
}

#[test]
fn a_changed_fingerprint_fails_before_creating_outputs() {
    let directory = scratch("sample-fingerprint");
    let matrix = directory.join("matrix.nex");
    let output = directory.join("output");
    fs::write(&matrix, MATRIX).unwrap();
    let result = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--seed",
        "1",
        "--steps",
        "20",
        "--sample-every",
        "10",
        "--gaps",
        "missing",
        "--expect-input-sha256",
        &"0".repeat(64),
        "--events-json",
    ]);
    assert!(error(&result).contains("fingerprint changed"));
    let event = String::from_utf8(result.stdout).unwrap();
    assert_eq!(event.lines().count(), 1);
    assert!(event.contains("\"event\":\"failed\""));
    assert!(!output.exists());
}

#[test]
fn closing_the_owner_pipe_cancels_and_finishes_partial_files() {
    use std::io::Read;
    use std::process::Stdio;
    use std::time::{Duration, Instant};
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let directory = scratch("sample-cancel");
    let matrix = directory.join("matrix.nex");
    let output = directory.join("output");
    fs::write(&matrix, MATRIX).unwrap();
    let events_path = directory.join("events.jsonl");
    let events = fs::File::create(&events_path).unwrap();
    let mut child = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_rove"))
            .args([
                "sample",
                matrix.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
                "--seed",
                "1",
                "--steps",
                "1000000000",
                "--sample-every",
                "100",
                "--gaps",
                "missing",
                "--conditioning",
                "none",
                "--events-json",
                "--cancel-on-stdin-close",
            ])
            .stdin(Stdio::piped())
            .stdout(events)
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !fs::read_to_string(&events_path)
        .unwrap()
        .contains("\"event\":\"started\"")
    {
        assert!(Instant::now() < deadline, "engine did not start");
        assert!(child.0.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(child.0.stdin.take());
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "engine ignored owner EOF");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(status.code(), Some(130));
    let mut text = String::new();
    fs::File::open(&events_path)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    assert!(
        text.lines()
            .last()
            .unwrap()
            .contains("\"event\":\"cancelled\"")
    );
    assert!(
        fs::read_to_string(output.join("trees.nexus"))
            .unwrap()
            .trim_end()
            .ends_with("End;")
    );
    assert!(
        fs::read_to_string(output.join("run.json"))
            .unwrap()
            .contains("\"status\": \"cancelled\"")
    );
}

#[test]
fn inspection_preserves_observations_and_requires_an_explicit_gap_policy() {
    let directory = scratch("inspect-gaps");
    let matrix = directory.join("matrix with spaces.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let result = rove(&["inspect", matrix.to_str().unwrap()]);
    assert!(result.status.success());
    let report = String::from_utf8(result.stdout).unwrap();
    assert!(report.contains("\"missing_count\": 1"));
    assert!(report.contains("\"gap_count\": 1"));
    assert!(report.contains("\"pattern_count\": null"));
    assert!(report.contains("choose an explicit gap treatment"));
    assert!(report.contains("\"-\""));
    let accepted = rove(&["inspect", matrix.to_str().unwrap(), "--gaps", "missing"]);
    assert!(accepted.status.success());
    let report = String::from_utf8(accepted.stdout).unwrap();
    assert!(report.contains("\"sampler_error\": null"));
    assert!(!report.contains("\"pattern_count\": null"));
    assert!(
        report.contains("\"-\""),
        "preview must preserve a gap's original meaning"
    );
    assert_eq!(
        fs::read_dir(directory).unwrap().count(),
        1,
        "inspection writes no run files"
    );
}

#[test]
fn inspection_separates_import_support_from_sampler_support() {
    let directory = scratch("inspect-states");
    let matrix = directory.join("multistate.nex");
    fs::write(&matrix, "#NEXUS\nBEGIN DATA; DIMENSIONS NTAX=3 NCHAR=2; FORMAT DATATYPE=STANDARD SYMBOLS=\"012\"; MATRIX\nA 01\nB 12\nC 20\n; END;").unwrap();
    let result = rove(&["inspect", matrix.to_str().unwrap()]);
    assert!(result.status.success());
    let report = String::from_utf8(result.stdout).unwrap();
    assert!(report.contains("\"taxon_count\": 3"));
    assert!(report.contains("\"pattern_count\": 2"));
    assert!(report.contains("\"sampler_error\": null"));
    fs::write(&matrix, "#NEXUS\nBEGIN DATA; DIMENSIONS NTAX=3 NCHAR=2; FORMAT DATATYPE=STANDARD SYMBOLS=\"01\"; MATRIX\nA 00\nB 01\nC 01\n; END;").unwrap();
    let result = rove(&["inspect", matrix.to_str().unwrap()]);
    let report = String::from_utf8(result.stdout).unwrap();
    assert!(report.contains("\"sampler_error\": null"));
    assert!(
        !report.contains("\"variable_error\": null"),
        "constant columns cannot use Mkv"
    );
}

#[test]
fn inspection_refuses_bad_input_without_emitting_partial_json() {
    let directory = scratch("inspect-errors");
    let matrix = directory.join("broken.nex");
    fs::write(&matrix, "#NEXUS BEGIN DATA; DIMENSIONS NTAX=5 NCHAR=6;").unwrap();
    let result = rove(&["inspect", matrix.to_str().unwrap()]);
    assert!(error(&result).contains("unterminated"));
    assert!(result.stdout.is_empty());
    assert!(error(&rove(&["inspect"])).contains("usage:"));
    assert!(
        error(&rove(&[
            "inspect",
            matrix.to_str().unwrap(),
            "--gaps",
            "ignore"
        ]))
        .contains("usage:")
    );
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
fn custom_weights_select_and_record_coupled_moves() {
    let directory = scratch("custom-weights");
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
        "27",
        "--steps",
        "2000",
        "--sample-every",
        "100",
        "--gaps",
        "missing",
        "--conditioning",
        "none",
        "--weight",
        "node_height=0",
        "--weight",
        "node_time_rates=30",
        "--weight",
        "root_time_rates=3",
        "--weight",
        "clock_sigma_rates=3",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output.join("run.json")).unwrap()).unwrap();
    let moves = metadata["moves"].as_array().unwrap();
    let disabled = moves.iter().find(|m| m["name"] == "node_height").unwrap();
    assert_eq!(disabled["weight"], 0.0);
    for counter in ["accepted", "rejected", "failed"] {
        assert_eq!(disabled[counter], 0);
    }
    for (name, weight) in [
        ("node_time_rates", 30.0),
        ("root_time_rates", 3.0),
        ("clock_sigma_rates", 3.0),
    ] {
        let record = moves.iter().find(|m| m["name"] == name).unwrap();
        assert_eq!(record["weight"], weight);
        assert!(record["accepted"].as_u64().unwrap() > 0);
        assert!(record["rejected"].as_u64().unwrap() > 0);
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
        run(
            &["--gaps", "missing", "--partitioning", "unknown"],
            ternary.to_str().unwrap()
        )
        .contains("--partitioning must be")
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
    assert!(run(&matrix, &["--weight", "spin=1"]).contains("--weight needs <move>=<value>"));
    for value in ["node_height=-1", "node_height=NaN", "node_height=inf"] {
        assert!(run(&matrix, &["--weight", value]).contains("move weights must be"));
    }
    assert!(
        run(
            &matrix,
            &[
                "--weight",
                "node_height=1e308",
                "--weight",
                "tree_scale=1e308"
            ]
        )
        .contains("move weights must be")
    );
    assert!(
        run(&matrix, &["--weight", "node_time_rates=1"])
            .contains("node_time_rates needs its rate model")
    );
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

#[test]
fn parallel_chains_replay_individually_and_keep_separate_events() {
    let directory = scratch("parallel-identity");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let output = directory.join("group");
    let result = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--seed",
        "18446744073709551615",
        "--steps",
        "400",
        "--sample-every",
        "10",
        "--gaps",
        "missing",
        "--chains",
        "2",
        "--events-json",
    ]);
    assert!(result.status.success(), "{}", error_text(&result));
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("run.json")).unwrap()).unwrap();
    assert_eq!(metadata["status"], "completed");
    assert_eq!(metadata["chains"][0]["seed"], u64::MAX);
    assert_eq!(metadata["chains"][1]["seed"], 0);
    let events: Vec<serde_json::Value> = String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(events.last().unwrap()["event"], "group_completed");
    for index in 1..=2 {
        let child = output.join(format!("chain-{index:02}"));
        let record: serde_json::Value =
            serde_json::from_slice(&fs::read(child.join("run.json")).unwrap()).unwrap();
        let direct = directory.join(format!("direct-{index}"));
        let mut args: Vec<String> = serde_json::from_value(record["command"].clone()).unwrap();
        args.remove(0);
        let position = args.iter().position(|a| a == "--output").unwrap();
        args[position + 1] = direct.display().to_string();
        let result = rove(&args.iter().map(String::as_str).collect::<Vec<_>>());
        assert!(result.status.success(), "{}", error_text(&result));
        for name in ["trace.log", "trees.nexus"] {
            let body = |p: &Path| {
                fs::read_to_string(p.join(name))
                    .unwrap()
                    .lines()
                    .filter(|s| !s.contains("command "))
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            };
            assert_eq!(body(&child), body(&direct));
        }
        let per_chain: Vec<_> = events.iter().filter(|e| e["chain"] == index).collect();
        assert_eq!(per_chain.first().unwrap()["event"], "started");
        assert_eq!(per_chain.last().unwrap()["event"], "completed");
    }
    let comparison: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("comparison.json")).unwrap()).unwrap();
    assert_eq!(comparison["retained_per_chain"], 37);
    assert!(comparison["statistics"].as_array().unwrap().len() >= 8);
    assert!(!comparison["clades"].as_array().unwrap().is_empty());
}

#[test]
fn parallel_stopping_records_the_boundary_and_preserves_replay_prefixes() {
    let directory = scratch("parallel-stop");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let output = directory.join("group");
    let result = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--seed",
        "42",
        "--steps",
        "500000",
        "--sample-every",
        "100",
        "--gaps",
        "missing",
        "--chains",
        "2",
        "--stop-when-converged",
        "--check-every",
        "120000",
        "--min-ess",
        "1",
        "--max-rhat",
        "1000",
    ]);
    assert!(result.status.success(), "{}", error_text(&result));
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("run.json")).unwrap()).unwrap();
    assert_eq!(record["termination_reason"], "diagnostic_thresholds");
    let step = record["completed_steps"].as_u64().unwrap();
    assert!(step < 500000 && step % 120000 == 0);
    let checks = record["checks"].as_array().unwrap();
    assert!(checks.len() >= 2);
    assert_eq!(checks.last().unwrap()["consecutive"], 2);
    for index in 1..=2 {
        let child = output.join(format!("chain-{index:02}"));
        let result = rove(&[
            "diagnose",
            child.join("trace.log").to_str().unwrap(),
            "--burnin-percent",
            "10",
        ]);
        assert!(result.status.success(), "{}", error_text(&result));
        let direct = directory.join(format!("direct-{index}"));
        let result = rove(&[
            "sample",
            matrix.to_str().unwrap(),
            "--output",
            direct.to_str().unwrap(),
            "--seed",
            &(41 + index).to_string(),
            "--steps",
            &step.to_string(),
            "--sample-every",
            "100",
            "--gaps",
            "missing",
        ]);
        assert!(result.status.success(), "{}", error_text(&result));
        let samples = |folder: &Path| {
            fs::read_to_string(folder.join("trace.log"))
                .unwrap()
                .lines()
                .filter(|l| !l.starts_with('#'))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert_eq!(samples(&child), samples(&direct));
    }
}

#[test]
fn invalid_parallel_options_and_worker_failures_cannot_complete() {
    let directory = scratch("parallel-errors");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    for (index, flags) in [
        vec!["--chains", "0"],
        vec!["--chains", "17"],
        vec!["--chains", "2", "--burnin-percent", "100"],
        vec!["--chains", "2", "--max-rhat", "NaN"],
        vec!["--chains", "2", "--check-every", "15"],
        vec!["--stop-when-converged"],
    ]
    .iter()
    .enumerate()
    {
        let output = directory.join(format!("invalid-{index}"));
        let mut args = vec![
            "sample",
            matrix.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--seed",
            "1",
            "--steps",
            "100",
            "--sample-every",
            "10",
            "--gaps",
            "missing",
        ];
        args.extend(flags);
        assert!(!rove(&args).status.success());
        assert!(!output.exists());
    }
    let output = directory.join("failed-workers");
    let result = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--seed",
        "1",
        "--steps",
        "100",
        "--sample-every",
        "10",
        "--chains",
        "4",
        "--events-json",
    ]);
    assert!(!result.status.success()); // Gaps require an explicit policy.
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("run.json")).unwrap()).unwrap();
    assert_eq!(record["status"], "failed");
    assert_eq!(record["errors"].as_array().unwrap().len(), 4);
}

#[test]
fn ordered_import_and_run_metadata_preserve_the_requested_models() {
    let directory = scratch("ordered-cli");
    let matrix = directory.join("matrix.nex");
    fs::write(
        &matrix,
        include_str!("../fixtures/validation/ordered/matrix.nex"),
    )
    .unwrap();
    let inspected = rove(&["inspect", matrix.to_str().unwrap()]);
    assert!(inspected.status.success(), "{}", error_text(&inspected));
    let report: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
    let partitions = report["partitions"].as_array().unwrap();
    assert_eq!(partitions.len(), 4);
    assert_eq!(partitions[2]["ordering"], "ordered");
    assert_eq!(partitions[3]["states"], 4);
    assert!(
        report["informative_error"]
            .as_str()
            .unwrap()
            .contains("ordered")
    );
    let output = directory.join("run");
    let result = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--steps",
        "100",
        "--sample-every",
        "10",
        "--seed",
        "43",
        "--model",
        "reference",
    ]);
    assert!(result.status.success(), "{}", error_text(&result));
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("run.json")).unwrap()).unwrap();
    assert_eq!(metadata["model"]["partitions"], report["partitions"]);
    assert!(
        metadata["model"]["substitution"]
            .as_str()
            .unwrap()
            .contains("Ordered/unordered")
    );
    let rejected = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--output",
        directory.join("invalid").to_str().unwrap(),
        "--steps",
        "10",
        "--sample-every",
        "10",
        "--seed",
        "43",
        "--conditioning",
        "informative",
    ]);
    assert!(error(&rejected).contains("ordered"));
    assert!(!directory.join("invalid").exists());
}

#[test]
fn dating_preserves_ages_in_outputs_summaries_and_prediction() {
    let directory = scratch("dating-cli");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let dating = serde_json::json!({
        "time_unit":"Ma", "birth_rate":0.8, "death_rate":0.3,
        "fossil_sampling_rate":0.2, "present_sampling":0.7, "origin":4.0,
        "clock_rate":0.1, "clock_prior_mean":0.1,
        "ages":{"A":0,"B":[0.4,1.2],"C":0,"D":0.7,"E e":0}
    });
    let config = directory.join("dating.json");
    fs::write(&config, dating.to_string()).unwrap();
    for model in ["strict", "reference"] {
        let output = directory.join(model);
        let run = rove(&[
            "sample",
            matrix.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--dating",
            config.to_str().unwrap(),
            "--seed",
            "2",
            "--steps",
            "2000",
            "--sample-every",
            "10",
            "--gaps",
            "missing",
            "--conditioning",
            "none",
            "--model",
            model,
        ]);
        assert!(run.status.success(), "{}", error_text(&run));
        let record: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("run.json")).unwrap()).unwrap();
        assert_eq!(record["model"]["dating"], dating);
        assert_eq!(record["model"]["clock_multiplier"], "trace.clock_rate");
        let trace = fs::read_to_string(output.join("trace.log")).unwrap();
        assert!(trace.contains("\tfossilized_birth_death\tclock_rate\tdating_parameter_prior\t"));
        assert!(trace.contains("\ttip_age_5\n"));
        let trees = output.join("trees.nexus");
        let text = fs::read_to_string(&trees).unwrap();
        for line in text.lines().filter(|line| line.starts_with("tree ")) {
            assert!(line.contains("[&height="), "{line}");
            assert!(!line.contains("][&"), "adjacent metadata blocks: {line}");
            assert_eq!(line.contains(",rate="), model == "reference", "{line}");
        }
        let mut reader = rove::summary::nexus::Reader::new(std::io::BufReader::new(
            fs::File::open(&trees).unwrap(),
        ))
        .unwrap();
        let mut n = 0;
        while let Some(tree) = reader.next_tree().unwrap() {
            assert_eq!(tree.nodes[tree.tips[3]].height, 0.7);
            assert!((0.4..=1.2).contains(&tree.nodes[tree.tips[1]].height));
            n += 1;
        }
        assert_eq!(n, 201);
        for heights in ["keep", "ca"] {
            let summary = output.join(heights);
            let result = rove(&[
                "summarize",
                trees.to_str().unwrap(),
                "--output",
                summary.to_str().unwrap(),
                "--burnin-percent",
                "10",
                "--heights",
                heights,
            ]);
            assert!(result.status.success(), "{}", error_text(&result));
            let result = rove(&["inspect-tree", summary.to_str().unwrap()]);
            assert!(result.status.success(), "{}", error_text(&result));
        }
        let predicted = output.join("predict");
        let result = rove(&[
            "predict",
            output.to_str().unwrap(),
            "--matrix",
            matrix.to_str().unwrap(),
            "--output",
            predicted.to_str().unwrap(),
            "--burnin-percent",
            "10",
            "--seed",
            "17",
            "--replicates",
            "4",
        ]);
        assert!(result.status.success(), "{}", error_text(&result));
        let result = rove(&[
            "diagnose",
            output.join("trace.log").to_str().unwrap(),
            "--burnin-percent",
            "10",
        ]);
        assert!(result.status.success(), "{}", error_text(&result));
    }
    for (name, change) in [
        ("unknown", serde_json::json!({"unsupported":1})),
        ("missing-taxon", serde_json::json!({"ages":{"A":0}})),
        ("age-unit", serde_json::json!({"time_unit":""})),
        ("negative-rate", serde_json::json!({"clock_rate":-0.1})),
        ("invalid-origin", serde_json::json!({"origin":0.5})),
    ] {
        let mut invalid = dating.clone();
        for (k, v) in change.as_object().unwrap() {
            invalid[k] = v.clone();
        }
        fs::write(&config, invalid.to_string()).unwrap();
        let result = rove(&[
            "sample",
            matrix.to_str().unwrap(),
            "--output",
            directory.join(name).to_str().unwrap(),
            "--dating",
            config.to_str().unwrap(),
            "--seed",
            "1",
            "--steps",
            "10",
            "--sample-every",
            "1",
            "--gaps",
            "missing",
        ]);
        assert!(!result.status.success());
        assert!(!directory.join(name).exists());
    }
}

#[test]
fn dated_chain_comparison_distinguishes_fixed_and_sampled_parameters() {
    let directory = scratch("dating-comparison");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let config = serde_json::json!({
        "time_unit":"Ma", "birth_rate":0.8, "death_rate":0.3,
        "fossil_sampling_rate":0.2, "present_sampling":0.7, "origin":4.0,
        "clock_rate":0.1,
        "ages":{"A":0,"B":[0.4,1.2],"C":0,"D":[0.7,0.7],"E e":0}
    });
    let output = directory.join("chains");
    let result = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--dating-json",
        &config.to_string(),
        "--seed",
        "2",
        "--steps",
        "1000",
        "--sample-every",
        "10",
        "--gaps",
        "missing",
        "--conditioning",
        "none",
        "--chains",
        "2",
    ]);
    assert!(result.status.success(), "{}", error_text(&result));
    let comparison: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("comparison.json")).unwrap()).unwrap();
    for column in comparison["statistics"].as_array().unwrap() {
        let name = column["name"].as_str().unwrap();
        let fixed = matches!(
            name,
            "clock_rate"
                | "dating_parameter_prior"
                | "tip_age_1"
                | "tip_age_3"
                | "tip_age_4"
                | "tip_age_5"
        );
        assert_eq!(column["fixed_by_model"] == true, fixed, "{name}");
    }
    let compared = rove(&[
        "compare",
        output.join("chain-01").to_str().unwrap(),
        output.join("chain-02").to_str().unwrap(),
    ]);
    assert!(compared.status.success(), "{}", error_text(&compared));
    let external: serde_json::Value = serde_json::from_slice(&compared.stdout).unwrap();
    assert_eq!(comparison["statistics"], external["statistics"]);
}

#[test]
fn estimated_processes_preserve_live_values_and_fixed_diagnostics() {
    let directory = scratch("fbd-estimation");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let config = serde_json::json!({
        "time_unit":"Ma", "birth_rate":0.8, "death_rate":0.3,
        "fossil_sampling_rate":0.2, "present_sampling":0.7, "origin":4.0,
        "clock_rate":0.1, "process_model":"joint_sampling_v1",
        "process_priors":{"birth_rate_mean":1.1,"death_rate_mean":0.6,
            "fossil_sampling_rate_mean":0.3,"origin_excess_mean":2.0},
        "ages":{"A":0,"B":[0.4,1.2],"C":0,"D":0.7,"E e":0}
    });
    for model in ["strict", "reference"] {
        let output = directory.join(model);
        let result = rove(&[
            "sample",
            matrix.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--dating-json",
            &config.to_string(),
            "--seed",
            "41",
            "--steps",
            "10000",
            "--sample-every",
            "10",
            "--gaps",
            "missing",
            "--conditioning",
            "none",
            "--model",
            model,
            "--chains",
            "2",
        ]);
        assert!(result.status.success(), "{}", error_text(&result));
        let record: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("run.json")).unwrap()).unwrap();
        assert_eq!(record["model"]["dating"], config);
        assert_eq!(
            record["model"]["dating_density_convention"],
            "joint_sampling_v1"
        );
        let comparison: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("comparison.json")).unwrap()).unwrap();
        for col in comparison["statistics"].as_array().unwrap() {
            let name = col["name"].as_str().unwrap();
            if name.starts_with("fbd_") || name == "dating_parameter_prior" {
                assert_eq!(
                    col["fixed_by_model"] == true,
                    name == "fbd_present_sampling",
                    "{name}"
                );
            }
        }
        let chain = output.join("chain-01");
        let trace = fs::read_to_string(chain.join("trace.log")).unwrap();
        let mut rows = trace.lines().filter(|l| !l.starts_with('#'));
        let names: Vec<_> = rows.next().unwrap().split('\t').collect();
        let states: Vec<Vec<f64>> = rows
            .map(|r| r.split('\t').map(|v| v.parse().unwrap()).collect())
            .collect();
        for name in [
            "fbd_birth_rate",
            "fbd_death_rate",
            "fbd_fossil_sampling_rate",
            "fbd_origin",
        ] {
            let i = names.iter().position(|n| *n == name).unwrap();
            assert!(
                states.iter().any(|v| v[i] != states[0][i]),
                "{name} did not move"
            );
        }
        let predict = rove(&[
            "predict",
            chain.to_str().unwrap(),
            "--matrix",
            matrix.to_str().unwrap(),
            "--output",
            output.join("predict").to_str().unwrap(),
            "--seed",
            "99",
            "--replicates",
            "8",
            "--burnin-percent",
            "10",
        ]);
        assert!(predict.status.success(), "{}", error_text(&predict));
    }
    for change in [
        serde_json::json!({"process_model":"unknown"}),
        serde_json::json!({"process_priors":{}}),
        serde_json::json!({"process_priors":{"unknown":1}}),
        serde_json::json!({"process_priors":{"birth_rate_mean":0}}),
        serde_json::json!({"process_priors":{"present_sampling_beta":[0,1]}}),
        serde_json::json!({"death_rate":0}),
    ] {
        let mut bad = config.clone();
        for (k, v) in change.as_object().unwrap() {
            bad[k] = v.clone();
        }
        let out = directory.join("invalid");
        let run = rove(&[
            "sample",
            matrix.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--dating-json",
            &bad.to_string(),
            "--seed",
            "41",
            "--steps",
            "10",
            "--sample-every",
            "1",
            "--gaps",
            "missing",
            "--conditioning",
            "none",
        ]);
        assert!(!run.status.success());
        assert!(!out.exists());
    }
    let mut no_mode = config.clone();
    no_mode.as_object_mut().unwrap().remove("process_model");
    let run = rove(&[
        "sample",
        matrix.to_str().unwrap(),
        "--output",
        directory.join("no-mode").to_str().unwrap(),
        "--dating-json",
        &no_mode.to_string(),
        "--seed",
        "1",
        "--steps",
        "10",
        "--sample-every",
        "1",
        "--gaps",
        "missing",
        "--conditioning",
        "none",
    ]);
    assert!(error(&run).contains("joint_sampling_v1"));
}

#[test]
fn marginal_likelihood_runs_record_every_power_and_both_estimates() {
    let directory = scratch("marginal-cli");
    let matrix = directory.join("matrix.nex");
    fs::write(&matrix, MATRIX).unwrap();
    let output = directory.join("run");
    let run = rove(&[
        "marginal",
        matrix.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--seed",
        "5",
        "--steps",
        "4000",
        "--sample-every",
        "20",
        "--powers",
        "6",
        "--alpha",
        "0.3",
        "--burnin-percent",
        "25",
        "--gaps",
        "missing",
        "--conditioning",
        "none",
        "--monophyly",
        "A,B",
    ]);
    assert!(run.status.success(), "{}", error_text(&run));
    let message = String::from_utf8_lossy(&run.stderr);
    assert!(message.contains("stepping stone") && message.contains("path sampling"));
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("run.json")).unwrap()).unwrap();
    assert_eq!(record["status"], "completed");
    assert_eq!(record["kind"], "marginal_likelihood_run");
    assert_eq!(record["completed_steps"], 24_000);
    assert_eq!(record["settings"]["powers"], 6);
    assert_eq!(record["settings"]["alpha"], 0.3);
    assert_eq!(record["settings"]["burnin_percent"], 25);
    assert_eq!(record["model"]["conditioning"], "none (Mk)");
    assert_eq!(
        record["model"]["monophyly"][0],
        serde_json::json!(["A", "B"])
    );
    let schedule = record["settings"]["schedule"].as_array().unwrap();
    assert_eq!(schedule.len(), 6);
    assert_eq!(schedule[0], 0.0);
    assert_eq!(schedule[5], 1.0);
    let results: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("marginal.json")).unwrap()).unwrap();
    assert_eq!(results["kind"], "marginal_likelihood");
    for method in ["stepping_stone", "path_sampling"] {
        let estimate = &results[method];
        assert!(
            estimate["log_marginal_likelihood"]
                .as_f64()
                .unwrap()
                .is_finite()
        );
        assert!(estimate["standard_error"].as_f64().unwrap() > 0.0);
    }
    let powers = results["powers"].as_array().unwrap();
    assert_eq!(powers.len(), 6);
    assert_eq!(results["stones"].as_array().unwrap().len(), 5);
    for (index, (power, expected)) in powers.iter().zip(schedule).enumerate() {
        assert_eq!(power["index"], index);
        assert_eq!(power["power"], *expected);
        assert_eq!(power["samples"], 151);
        let trace = fs::read_to_string(output.join(format!("power-{index:02}/trace.log"))).unwrap();
        assert!(trace.contains(&format!("likelihood power {}", expected.as_f64().unwrap())));
        assert_eq!(trace.lines().filter(|l| !l.starts_with('#')).count(), 202);
    }
    let last = &powers[5];
    assert!(
        last["mean_log_likelihood"].as_f64().unwrap()
            > powers[0]["mean_log_likelihood"].as_f64().unwrap()
    );
    assert_eq!(results["input_sha256"], record["input"]["sha256"]);
    let rejected = directory.join("x");
    let rejected = rejected.to_str().unwrap();
    assert!(
        error(&rove(&[
            "marginal",
            matrix.to_str().unwrap(),
            "--output",
            rejected,
            "--seed",
            "1",
            "--steps",
            "100",
            "--sample-every",
            "10"
        ]))
        .contains("--powers is required")
    );
    for (extra, text) in [
        (vec!["--powers", "1"], "--powers must be 2 to 256"),
        (
            vec!["--powers", "4", "--alpha", "0"],
            "--alpha must be finite and positive",
        ),
        (
            vec!["--powers", "4", "--chains", "2"],
            "not available with rove marginal",
        ),
        (
            vec!["--powers", "4", "--sample-from-prior"],
            "not available with rove marginal",
        ),
        (
            vec!["--powers", "4", "--checkpoint-every", "10"],
            "not available with rove marginal",
        ),
    ] {
        let mut args = vec![
            "marginal",
            matrix.to_str().unwrap(),
            "--output",
            rejected,
            "--seed",
            "1",
            "--steps",
            "100",
            "--sample-every",
            "10",
            "--gaps",
            "missing",
        ];
        args.extend(extra);
        assert!(error(&rove(&args)).contains(text), "{text}");
        assert!(!Path::new(rejected).exists());
    }
    assert!(
        error(&rove(&[
            "sample",
            matrix.to_str().unwrap(),
            "--output",
            rejected,
            "--seed",
            "1",
            "--steps",
            "100",
            "--sample-every",
            "10",
            "--powers",
            "4"
        ]))
        .contains("belong to rove marginal")
    );
}
