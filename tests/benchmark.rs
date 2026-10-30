#[path = "../benches/support/workload.rs"]
#[allow(dead_code)] // The executable also uses this module for timing and metadata.
mod workload;

use rove::Backend;
use workload::{PATHS, Path, backends};

#[test]
fn benchmark_paths_match_the_reference_and_restore_rejected_changes() {
    for case in workload::suite() {
        let [matrices, mk] = PATHS.map(|path| case.validate(path, Backend::Portable));
        for backend in backends().into_iter().filter(|&b| b != Backend::Portable) {
            for (path, expected) in PATHS.into_iter().zip([matrices, mk]) {
                let actual = case.validate(path, backend);
                assert_eq!(
                    actual.to_bits(),
                    expected.to_bits(),
                    "{} {path:?} {backend:?}",
                    case.name
                );
            }
        }
        assert!(matrices.is_finite(), "{}", case.name);
        assert!(
            (matrices - mk).abs() < 1e-9,
            "{}: {matrices} != {mk}",
            case.name
        );
        assert_eq!(
            case.weights.iter().sum::<f64>(),
            case.data.character_count() as f64
        );
        if case.name.starts_with("reference_20x50") {
            assert_eq!(case.weights.len(), 23);
        }
        if case.name == "reference_20x50_mkv" {
            assert_eq!(case.core_patterns(), 25);
            assert_eq!(
                case.engine(Path::Mk, Backend::NATIVE).core.layout().stride,
                32
            );
        } else {
            assert_eq!(case.core_patterns(), case.weights.len());
        }
        if case.name == "repeated_columns" {
            assert_eq!(case.weights.len(), 25);
        }
        if case.name == "small_unpadded" {
            assert_eq!(case.weights.len(), 16);
            assert_eq!(
                case.engine(Path::Matrices, Backend::NATIVE)
                    .core
                    .layout()
                    .stride,
                16
            );
        }
        if case.name == "small_padded" {
            assert_eq!(case.weights.len(), 17);
            assert_eq!(
                case.engine(Path::Mk, Backend::NATIVE).core.layout().stride,
                24
            );
        }
        if case.name == "missing_20_percent" {
            assert!(case.missing_cells() > 0);
        } else {
            assert_eq!(case.missing_cells(), 0);
        }
        if case.shape == "ladder" {
            assert_eq!(case.tree.depth(case.tip_branch), 89);
        }
    }
}
#[test]
fn distributed_benchmark_stops_when_its_owner_closes_stdin() {
    use std::io::{BufRead, BufReader, Read};
    use std::process::{Command, Stdio};

    let mut child = Command::new(env!("CARGO_BIN_EXE_rove"))
        .args(["benchmark", "--cancel-on-stdin-close"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    let first: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(first["event"], "started");
    assert_eq!(first["protocol"]["id"], "rove-mac-v1");
    drop(child.stdin.take());
    let mut remaining = String::new();
    output.read_to_string(&mut remaining).unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(130));
    assert!(remaining.lines().any(|line| {
        serde_json::from_str::<serde_json::Value>(line).unwrap()["event"] == "cancelled"
    }));
    assert!(!remaining.contains("completed"));
}
