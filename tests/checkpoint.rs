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
fn command(root: &Path, name: &str, chains: usize) -> Command {
    let mut c = Command::new(BIN);
    c.arg("sample")
        .arg(root.join("matrix.nex"))
        .arg("--output")
        .arg(root.join(name))
        .args([
            "--seed",
            "42",
            "--steps",
            "6000",
            "--sample-every",
            "10",
            "--model",
            "reference",
            "--print-every",
            "10",
            "--checkpoint-every",
            "100",
            "--chains",
            &chains.to_string(),
        ]);
    c
}
fn content(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|s| !s.starts_with("# command") && !s.starts_with("[command"))
        .collect::<Vec<_>>()
        .join("\n")
}
fn interrupt(mut cmd: Command, step: u64, kill: bool) {
    let mut child = cmd
        .args(["--events-json", "--cancel-on-stdin-close"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut paused = false;
    for line in BufReader::new(stdout).lines() {
        let event: Value = serde_json::from_str(&line.unwrap()).unwrap();
        if !paused && event["step"].as_u64().is_some_and(|n| n >= step) {
            if kill {
                child.kill().unwrap();
                break;
            }
            child.stdin.as_mut().unwrap().write_all(b"pause\n").unwrap();
            paused = true;
        }
    }
    let status = child.wait().unwrap();
    if !kill {
        assert_eq!(status.code(), Some(130));
    }
}
#[test]
fn pause_crash_and_resume_preserve_all_samples_and_moves() {
    for (kind, matrix) in [
        ("binary", MATRIX),
        (
            "multistate",
            include_str!("../fixtures/validation/multistate/matrix.nex"),
        ),
    ] {
        for chains in [1, 4] {
            let root = scratch(&format!("checkpoint-{kind}-{chains}"));
            fs::write(root.join("matrix.nex"), matrix).unwrap();
            assert!(
                command(&root, "full", chains)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success()
            );
            for (i, kill) in [false, true].into_iter().enumerate() {
                let name = format!("restart-{i}");
                interrupt(command(&root, &name, chains), 230, kill);
                let output = root.join(&name);
                let manifest: Value =
                    serde_json::from_slice(&fs::read(output.join("checkpoint.json")).unwrap())
                        .unwrap();
                for c in 0..chains {
                    let dir = if chains == 1 {
                        output.clone()
                    } else {
                        output.join(format!("chain-{:02}", c + 1))
                    };
                    for file in ["trace.log", "trees.nexus"] {
                        // Simulate a crash after writing past the committed offsets.
                        fs::OpenOptions::new()
                            .append(true)
                            .open(dir.join(file))
                            .unwrap()
                            .write_all(b"uncommitted tail\n")
                            .unwrap();
                    }
                    assert!(manifest["chains"][c]["step"].as_u64().unwrap() < 6000);
                }
                let resumed = Command::new(BIN)
                    .args(["sample", "--resume"])
                    .arg(&output)
                    .output()
                    .unwrap();
                assert!(
                    resumed.status.success(),
                    "{}",
                    String::from_utf8_lossy(&resumed.stderr)
                );
                for c in 0..chains {
                    let member = if chains == 1 {
                        String::new()
                    } else {
                        format!("chain-{:02}", c + 1)
                    };
                    let full = root.join("full").join(&member);
                    let actual = output.join(member);
                    for file in ["trace.log", "trees.nexus"] {
                        assert_eq!(content(&full.join(file)), content(&actual.join(file)));
                    }
                    let read = |p: &Path| {
                        serde_json::from_slice::<Value>(&fs::read(p.join("run.json")).unwrap())
                            .unwrap()
                    };
                    assert_eq!(read(&full)["moves"], read(&actual)["moves"]);
                    assert_eq!(read(&full)["final_state"], read(&actual)["final_state"]);
                }
            }
        }
    }
}

#[test]
fn incompatible_or_damaged_restarts_leave_outputs_untouched() {
    let root = scratch("checkpoint-invalid");
    interrupt(command(&root, "run", 1), 230, false);
    let dir = root.join("run");
    let trace = fs::read(dir.join("trace.log")).unwrap();
    let check = |needle: &str, extra: &[&str]| {
        let result = Command::new(BIN)
            .args(["sample", "--resume"])
            .arg(&dir)
            .args(extra)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(needle),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(trace, fs::read(dir.join("trace.log")).unwrap());
    };
    check("cannot change", &["--seed", "43"]);
    fs::write(root.join("matrix.nex"), format!("{MATRIX}\n")).unwrap();
    check("matrix changed", &[]);
    fs::write(root.join("matrix.nex"), MATRIX).unwrap();
    let path = dir.join("checkpoint.json");
    let original = fs::read(&path).unwrap();
    let mut value: Value = serde_json::from_slice(&original).unwrap();
    value["executable_sha256"] = Value::String("0".repeat(64));
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    check("different executable", &[]);
    fs::write(&path, &original).unwrap();
    let state = dir.join(value["chains"][0]["file"].as_str().unwrap());
    fs::write(state, b"truncated").unwrap();
    check("state fingerprint changed", &[]);
}

#[test]
#[cfg(unix)]
fn active_runs_are_locked_and_a_failed_manifest_write_keeps_the_previous_save() {
    let root = scratch("checkpoint-atomic");
    let mut cmd = command(&root, "run", 1);
    let mut child = cmd
        .args(["--events-json", "--cancel-on-stdin-close"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut blocked = false;
    let dir = root.join("run");
    let mut saved = Vec::new();
    for line in BufReader::new(child.stdout.take().unwrap()).lines() {
        let event: Value = serde_json::from_str(&line.unwrap()).unwrap();
        if !blocked && event["step"].as_u64().is_some_and(|s| s >= 100) {
            // Freeze the owned test child while fault injection modifies its
            // temporary path. No timing assumptions about the next save.
            assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGSTOP) }, 0);
            let mut status = 0;
            // Wait for the stop notification before touching checkpoint paths.
            assert_eq!(
                unsafe { libc::waitpid(child.id() as i32, &mut status, libc::WUNTRACED) },
                child.id() as i32
            );
            assert!(libc::WIFSTOPPED(status));
            let refusal = Command::new(BIN)
                .args(["sample", "--resume"])
                .arg(&dir)
                .output()
                .unwrap();
            assert!(!refusal.status.success());
            assert!(String::from_utf8_lossy(&refusal.stderr).contains("already open"));
            // The temporary manifest cannot be created, as on a failed write.
            // Creating a directory is deterministic even when tests run as root.
            let _ = fs::remove_file(dir.join("checkpoint.pending"));
            fs::create_dir(dir.join("checkpoint.pending")).unwrap();
            saved = fs::read(dir.join("checkpoint.json")).unwrap();
            blocked = true;
            assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGCONT) }, 0);
        }
    }
    assert!(blocked);
    assert_eq!(child.wait().unwrap().code(), Some(2));
    assert_eq!(fs::read(dir.join("checkpoint.json")).unwrap(), saved);
    fs::remove_dir(dir.join("checkpoint.pending")).unwrap();
    let resumed = Command::new(BIN)
        .args(["sample", "--resume"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
}

#[test]
fn restarting_keeps_the_diagnostic_stopping_history() {
    let root = scratch("checkpoint-stopping");
    let command = |name: &str| {
        let mut c = Command::new(BIN);
        c.arg("sample")
            .arg(root.join("matrix.nex"))
            .arg("--output")
            .arg(root.join(name))
            .args([
                "--seed",
                "42",
                "--steps",
                "50000",
                "--sample-every",
                "10",
                "--chains",
                "2",
                "--print-every",
                "100",
                "--checkpoint-every",
                "500",
                "--stop-when-converged",
                "--min-ess",
                "1",
                "--max-rhat",
                "1000",
                "--check-every",
                "12000",
            ]);
        c
    };
    assert!(
        command("full")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    interrupt(command("paused"), 12500, false);
    let resumed = Command::new(BIN)
        .args(["sample", "--resume"])
        .arg(root.join("paused"))
        .output()
        .unwrap();
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    let read = |name: &str| {
        serde_json::from_slice::<Value>(&fs::read(root.join(name).join("run.json")).unwrap())
            .unwrap()
    };
    assert_eq!(
        read("full")["completed_steps"],
        read("paused")["completed_steps"]
    );
    assert_eq!(read("full")["checks"], read("paused")["checks"]);
    for chain in ["chain-01", "chain-02"] {
        for file in ["trace.log", "trees.nexus"] {
            assert_eq!(
                content(&root.join("full").join(chain).join(file)),
                content(&root.join("paused").join(chain).join(file))
            );
        }
    }
}
