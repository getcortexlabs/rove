#[path = "../benches/support/workload.rs"]
#[allow(dead_code)] // The executable also uses this module for timing and metadata.
mod workload;

use workload::{PATHS, Path};

#[test]
fn benchmark_paths_match_the_reference_and_restore_rejected_changes() {
    for case in workload::suite() {
        let [matrices, mk] = PATHS.map(|path| case.validate(path));
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
            assert_eq!(case.engine(Path::Mk).core.layout().stride, 32);
        } else {
            assert_eq!(case.core_patterns(), case.weights.len());
        }
        if case.name == "repeated_columns" {
            assert_eq!(case.weights.len(), 25);
        }
        if case.name == "small_unpadded" {
            assert_eq!(case.weights.len(), 16);
            assert_eq!(case.engine(Path::Matrices).core.layout().stride, 16);
        }
        if case.name == "small_padded" {
            assert_eq!(case.weights.len(), 17);
            assert_eq!(case.engine(Path::Mk).core.layout().stride, 24);
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
