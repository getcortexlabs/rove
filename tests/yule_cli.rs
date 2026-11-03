use std::fs;
use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_rove");

#[test]
fn incompatible_yule_options_fail_before_creating_output() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("yule-invalid");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let matrix = root.join("matrix.nex");
    fs::write(
        &matrix,
        "#NEXUS BEGIN DATA; DIMENSIONS NTAX=3 NCHAR=1; MATRIX a 0 b 1 c 0; END;",
    )
    .unwrap();
    for options in [
        vec!["--tree-prior", "unknown"],
        vec!["--birth-rate-upper", "4"],
        vec!["--tree-prior", "yule", "--birth-rate-upper", "0"],
        vec!["--tree-prior", "yule", "--birth-rate-upper", "nan"],
        vec!["--tree-prior", "yule", "--birth-rate-upper", "1e-999"],
        vec!["--tree-prior", "yule", "--dating-json", "{}"],
        vec!["--tree-prior", "yule", "--weight", "relative_death_scale=1"],
        vec![
            "--tree-prior",
            "yule",
            "--tuning",
            "birth_difference_scale=0.5",
        ],
        vec!["--tuning", "birth_rate_scale=0.5"],
    ] {
        let output = root.join("rejected");
        let result = Command::new(BIN)
            .arg("sample")
            .arg(&matrix)
            .args([
                "--steps",
                "20",
                "--sample-every",
                "10",
                "--seed",
                "42",
                "--output",
            ])
            .arg(&output)
            .args(&options)
            .output()
            .unwrap();
        assert!(!result.status.success(), "{options:?}");
        assert!(!output.exists(), "{options:?}");
    }
}
