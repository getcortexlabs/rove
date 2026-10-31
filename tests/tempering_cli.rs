use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_rove");
const MATRIX: &str =
    "#NEXUS BEGIN DATA; DIMENSIONS NTAX=5 NCHAR=4; MATRIX a 0111 b 0011 c 0001 d 1000 e 1010; END;";

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("matrix.nex"), MATRIX).unwrap();
    dir
}

fn command(root: &Path, name: &str, powers: Option<&str>) -> Command {
    seeded_command(root, name, powers, "42")
}

fn seeded_command(root: &Path, name: &str, powers: Option<&str>, seed: &str) -> Command {
    let mut c = Command::new(BIN);
    c.arg(if powers.is_some() {
        "coupled"
    } else {
        "sample"
    })
    .arg(root.join("matrix.nex"))
    .arg("--output")
    .arg(root.join(name))
    .args([
        "--seed",
        seed,
        "--steps",
        "6000",
        "--sample-every",
        "10",
        "--model",
        "reference",
        "--checkpoint-every",
        "100",
    ]);
    if let Some(powers) = powers {
        c.args(["--likelihood-powers", powers, "--swap-every", "13"]);
    }
    c
}

fn record(dir: &Path) -> Value {
    serde_json::from_slice(&fs::read(dir.join("run.json")).unwrap()).unwrap()
}

fn content(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|s| !s.starts_with('#') && !s.starts_with('['))
        .collect::<Vec<_>>()
        .join("\n")
}

fn complete(mut command: Command) {
    let out = command.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn pause(mut command: Command, kill: bool) {
    let mut child = command
        .args(["--events-json", "--cancel-on-stdin-close"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut sent = false;
    for line in BufReader::new(child.stdout.take().unwrap()).lines() {
        let event: Value = serde_json::from_str(&line.unwrap()).unwrap();
        if !sent && event["step"].as_u64().is_some_and(|n| n >= 230) {
            if kill {
                child.kill().unwrap();
                break;
            }
            child.stdin.as_mut().unwrap().write_all(b"pause\n").unwrap();
            sent = true;
        }
    }
    let status = child.wait().unwrap();
    if !kill {
        assert_eq!(status.code(), Some(130));
    }
}

#[test]
fn power_one_preserves_samples_and_works_with_diagnostics() {
    let root = scratch("tempering-single");
    complete(command(&root, "ordinary", None));
    complete(command(&root, "ensemble", Some("1")));
    for file in ["trace.log", "trees.nexus"] {
        assert_eq!(
            content(&root.join("ordinary").join(file)),
            content(&root.join("ensemble").join(file))
        );
    }
    assert_eq!(
        record(&root.join("ordinary"))["moves"],
        record(&root.join("ensemble"))["moves"]
    );
    let mut diagnose = Command::new(BIN);
    diagnose
        .arg("diagnose")
        .arg(root.join("ensemble/trace.log"));
    diagnose.args(["--burnin-percent", "10"]);
    complete(diagnose);
}

#[test]
fn coupled_pause_and_crash_resume_preserve_cold_samples_and_every_slot() {
    let root = scratch("tempering-resume");
    let dating = root.join("dating.json");
    let configured = |name: &str| {
        fs::write(
            &dating,
            serde_json::json!({"time_unit":"Ma","birth_rate":0.8,
            "death_rate":0.3,"fossil_sampling_rate":0.2,"present_sampling":0.7,
            "origin":4.0,"clock_rate":0.1,"clock_prior_mean":0.1,
            "ages":{"a":0,"b":[0.4,1.2],"c":0,"d":0.7,"e":0}})
            .to_string(),
        )
        .unwrap();
        let mut cmd = command(&root, name, Some("1,0.9,0.7,0.5"));
        cmd.arg("--dating").arg(&dating);
        cmd
    };
    complete(configured("full"));
    for (name, kill) in [("paused", false), ("crashed", true)] {
        pause(configured(name), kill);
        fs::remove_file(&dating).unwrap();
        let dir = root.join(name);
        for file in ["trace.log", "trees.nexus"] {
            fs::OpenOptions::new()
                .append(true)
                .open(dir.join(file))
                .unwrap()
                .write_all(b"uncommitted tail\n")
                .unwrap();
        }
        let mut resume = Command::new(BIN);
        resume.args(["coupled", "--resume"]).arg(&dir);
        complete(resume);
        for file in ["trace.log", "trees.nexus"] {
            assert_eq!(
                content(&root.join("full").join(file)),
                content(&dir.join(file))
            );
        }
        assert_eq!(
            record(&root.join("full"))["coupling"],
            record(&dir)["coupling"]
        );
        assert_eq!(record(&root.join("full"))["moves"], record(&dir)["moves"]);
        assert_eq!(record(&dir)["status"], "completed");
        // Recovery may leave one orphan from a crash between state and manifest,
        // but ordinary checkpointing keeps just the current state.
        if !kill {
            assert_eq!(
                fs::read_dir(&dir)
                    .unwrap()
                    .filter_map(Result::ok)
                    .filter(|e| e.path().extension().is_some_and(|s| s == "bin"))
                    .count(),
                1
            );
        }
    }
}

#[test]
fn damaged_tree_prefix_is_rejected_before_truncating_trace() {
    let root = scratch("tempering-damaged");
    pause(command(&root, "run", Some("1,0.9,0.7,0.5")), false);
    let dir = root.join("run");
    fs::OpenOptions::new()
        .append(true)
        .open(dir.join("trace.log"))
        .unwrap()
        .write_all(b"uncommitted trace\n")
        .unwrap();
    let trace = fs::read(dir.join("trace.log")).unwrap();
    fs::write(dir.join("trees.nexus"), b"damaged").unwrap();
    let out = Command::new(BIN)
        .args(["coupled", "--resume"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(trace, fs::read(dir.join("trace.log")).unwrap());
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(dir.join("coupling.json")).unwrap()).unwrap();
    manifest["step"] = serde_json::json!(6000);
    fs::write(dir.join("coupling.json"), manifest.to_string()).unwrap();
    let out = Command::new(BIN)
        .args(["coupled", "--resume"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("checkpoint changed"));
    assert_eq!(trace, fs::read(dir.join("trace.log")).unwrap());
}

#[test]
fn invalid_ladder_does_not_create_output_and_completed_run_is_not_overwritten() {
    let root = scratch("tempering-options");
    for (i, powers) in ["0.9,0.5", "1,1", "1,0", "1,NaN"].iter().enumerate() {
        let name = format!("bad-{i}");
        assert!(
            !command(&root, &name, Some(powers))
                .output()
                .unwrap()
                .status
                .success()
        );
        assert!(!root.join(name).exists());
    }
    complete(command(&root, "full", Some("1,0.9,0.7,0.5")));
    let trace = fs::read(root.join("full/trace.log")).unwrap();
    assert!(
        !command(&root, "full", Some("1,0.9,0.7,0.5"))
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !Command::new(BIN)
            .args(["coupled", "--resume"])
            .arg(root.join("full"))
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(trace, fs::read(root.join("full/trace.log")).unwrap());
}

#[test]
fn independent_ensembles_can_be_compared_and_owner_exit_stops_workers() {
    let root = scratch("tempering-compare");
    for (name, seed) in [("first", "42"), ("second", "52")] {
        complete(seeded_command(&root, name, Some("1,0.9,0.7,0.5"), seed));
    }
    let result = Command::new(BIN)
        .arg("compare")
        .arg(root.join("first"))
        .arg(root.join("second"))
        .args(["--burnin-percent", "10"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = command(&root, "cancelled", Some("1,0.9,0.7,0.5"))
        .arg("--cancel-on-stdin-close")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(130));
    let record = record(&root.join("cancelled"));
    assert_eq!(record["status"], "cancelled");
    assert!(record["completed_steps"].as_u64().unwrap() < 6000);
    assert!(
        fs::read_to_string(root.join("cancelled/trees.nexus"))
            .unwrap()
            .trim_end()
            .ends_with("End;")
    );
}
