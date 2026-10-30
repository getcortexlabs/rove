use proptest::prelude::*;
use rove::data::{Cell, CompressionError, GapTreatment, Matrix};
use rove::{LikelihoodCore, Scaling, model::Mk, nexus};

const REFERENCE: &str = include_str!("../fixtures/reference/20x50/matrix.nex");
const CONTROL: &str = include_str!("../fixtures/synthetic/control_20x50.nex");

fn document(ntax: usize, nchar: usize, format: &str, matrix: &str) -> String {
    format!(
        "#NEXUS\nBEGIN DATA;\nDIMENSIONS NTAX={ntax} NCHAR={nchar};\nFORMAT {format};\nMATRIX\n{matrix}\n; END;"
    )
}

fn compress_all(matrix: &Matrix) -> rove::data::Patterns {
    let sites = 0..matrix.character_count();
    matrix
        .compress(std::slice::from_ref(&sites), GapTreatment::Reject)
        .unwrap()
        .remove(0)
}

#[test]
fn both_fixtures_round_trip_their_observed_columns() {
    let reference = nexus::parse(REFERENCE).unwrap();
    let control = nexus::parse(CONTROL).unwrap();
    assert_ne!(reference.rows(), control.rows());
    for matrix in [reference, control] {
        assert_eq!(
            matrix.taxa(),
            &(1..=20).map(|i| format!("T{i}")).collect::<Vec<_>>()
        );
        assert_eq!(matrix.symbols(), &['0', '1']);
        assert_eq!(matrix.character_count(), 50);
        let patterns = compress_all(&matrix);
        assert_eq!(patterns.weights().len(), 23);
        assert_eq!(patterns.weights().iter().sum::<usize>(), 50);
        for (weight, count) in [(1, 5), (2, 9), (3, 9)] {
            assert_eq!(
                patterns.weights().iter().filter(|&&w| w == weight).count(),
                count
            );
        }
        for (taxon, row) in matrix.rows().iter().enumerate() {
            for (site, cell) in row.iter().enumerate() {
                let Cell::State(state) = cell else {
                    panic!("fixture has a non-state cell");
                };
                let pattern = patterns.site_to_pattern()[site];
                let partials = &patterns.tip_partials()[taxon][pattern * 2..pattern * 2 + 2];
                assert_eq!(partials[*state], 1.0);
                assert_eq!(partials[1 - state], 0.0);
            }
        }
    }
}

#[test]
fn comments_quotes_wrapped_rows_and_line_endings() {
    let text = "\u{feff}#nExUs [outer [nested] comment]\r\n\
        begin data; dimensions ntax=2 nchar=4; format symbols=\"01\";\r\n\
        matrix 'O''Brien; [specimen]' 0[inside]1\r\n\
        01\r\n\
        Big_[comment]Beetle 1 0 0 1; endblock;\r\n\
        begin trees; tree true = ('O''Brien; [specimen]',Big_Beetle); end;";
    let data = nexus::parse(text).unwrap();
    assert_eq!(data.taxa(), &["O'Brien; [specimen]", "Big Beetle"]);
    assert_eq!(
        data.rows()[0],
        vec![
            Cell::State(0),
            Cell::State(1),
            Cell::State(0),
            Cell::State(1)
        ]
    );
}

#[test]
fn character_block_keeps_declared_taxon_and_symbol_order() {
    let text = "#NEXUS
        BEGIN TAXA; DIMENSIONS NTAX=2; TAXLABELS 'Big beetle' Small; END;
        BEGIN CHARACTERS; DIMENSIONS NCHAR=3;
        FORMAT DATATYPE=STANDARD SYMBOLS=\"2 1 0\";
        CHARSTATELABELS 1 size / large small, 2 'body shape' / round oval long;
        MATRIX small 010 Big_beetle 110; END;";
    let data = nexus::parse(text).unwrap();
    assert_eq!(data.taxa(), &["Big beetle", "Small"]);
    assert_eq!(data.symbols(), &['2', '1', '0']);
    assert_eq!(
        data.rows()[0],
        [Cell::State(1), Cell::State(1), Cell::State(2)]
    );
    assert_eq!(
        data.character_labels(),
        &[Some("size".into()), Some("body shape".into()), None]
    );
    assert_eq!(data.state_labels()[1], ["round", "oval", "long"]);
    let patterns = compress_all(&data);
    assert_eq!(patterns.symbols().len(), 3); // State 2 is declared but unobserved.
    assert!(patterns.tip_partials()[0].chunks(3).all(|p| p[0] == 0.0));
}

#[test]
fn separate_character_and_state_labels() {
    let text = "#NEXUS BEGIN CHARACTERS; DIMENSIONS NEWTAXA NTAX=2 NCHAR=2;
        TAXLABELS A B; CHARLABELS size shape;
        STATELABELS 1 small large, 2 round long;
        MATRIX B 10 A 01; END;";
    let data = nexus::parse(text).unwrap();
    assert_eq!(data.taxa(), &["A", "B"]);
    assert_eq!(
        data.character_labels(),
        &[Some("size".into()), Some("shape".into())]
    );
    assert_eq!(
        data.state_labels(),
        &[vec!["small", "large"], vec!["round", "long"]]
    );
}

#[test]
fn labeled_interleaved_pages_match_sequential_data() {
    let format = "SYMBOLS=\"012\" MISSING=? GAP=-";
    let expected = nexus::parse(&document(2, 6, format, "A 0{01}(12)?-2\nB 102210")).unwrap();
    for interleave in ["INTERLEAVE", "INTERLEAVE=YES"] {
        let actual = nexus::parse(&document(
            2,
            6,
            &format!("{format} {interleave}"),
            "A 0{10}\nB 10\n\nA (12)?-\nB 221\nA 2\nB 0",
        ))
        .unwrap();
        assert_eq!(actual, expected);
    }
    let no = nexus::parse(&document(
        2,
        6,
        &format!("{format} INTERLEAVE=NO"),
        "A 0{01}(12)?-2 B 102210",
    ))
    .unwrap();
    assert_eq!(no, expected);
}

#[test]
fn uncertainty_is_a_likelihood_mask_and_gaps_need_a_choice() {
    let data = nexus::parse(&document(
        2,
        5,
        "SYMBOLS=\"012\" GAP=-",
        "A 0{01}?-1\nB 00000",
    ))
    .unwrap();
    let sites = 0..5;
    assert!(matches!(
        data.compress(std::slice::from_ref(&sites), GapTreatment::Reject),
        Err(CompressionError::Gap { site: 3, .. })
    ));
    let patterns = data
        .compress(std::slice::from_ref(&sites), GapTreatment::Missing)
        .unwrap()
        .remove(0);
    assert_eq!(
        patterns.tip_partials()[0],
        [
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0,
        ]
    );
    assert_eq!(data.rows()[0][2], Cell::Missing);
    assert_eq!(data.rows()[0][3], Cell::Gap);
}

#[test]
fn polymorphism_is_retained_but_not_passed_to_the_mk_kernel() {
    let data = nexus::parse(&document(2, 2, "SYMBOLS=\"012\"", "A (10){01}\nB 00")).unwrap();
    assert_eq!(
        data.rows()[0],
        [Cell::Polymorphic(vec![0, 1]), Cell::Uncertain(vec![0, 1])]
    );
    let sites = 0..2;
    assert_eq!(
        data.compress(std::slice::from_ref(&sites), GapTreatment::Missing),
        Err(CompressionError::Polymorphism {
            taxon: "A".into(),
            site: 0
        })
    );
}

#[test]
fn compression_preserves_partition_boundaries_and_constant_sites() {
    let data = nexus::parse(&document(2, 6, "SYMBOLS=\"012\"", "A 00?00?\nB 00?00?")).unwrap();
    let all = compress_all(&data);
    assert_eq!(all.weights(), &[4, 2]);
    assert_eq!(all.site_to_pattern(), &[0, 0, 1, 0, 0, 1]);
    let split = data.compress(&[0..3, 3..6], GapTreatment::Reject).unwrap();
    for (i, patterns) in split.iter().enumerate() {
        assert_eq!(patterns.sites(), &(i * 3..i * 3 + 3).collect::<Vec<_>>());
        assert_eq!(patterns.weights(), &[2, 1]);
        assert_eq!(patterns.site_to_pattern(), &[0, 0, 1]);
        assert_eq!(patterns.symbols(), &['0', '1', '2']);
    }
    let invalid = vec![
        vec![],
        vec![0..0, 0..6],
        vec![0..2, 3..6],
        vec![0..4, 3..6],
        vec![0..7],
        vec![1..6],
        vec![0..5],
    ];
    for partitions in invalid {
        assert_eq!(
            data.compress(&partitions, GapTreatment::Reject),
            Err(CompressionError::InvalidPartitions)
        );
    }
}

#[test]
fn alphabet_case_missing_and_gap_declarations_are_respected() {
    let default_case =
        nexus::parse(&document(1, 3, "SYMBOLS=\"Ab\" MISSING=X", "Taxon abx")).unwrap();
    assert_eq!(
        default_case.rows()[0],
        [Cell::State(0), Cell::State(1), Cell::Missing]
    );
    let case = nexus::parse(&document(
        1,
        4,
        "RESPECTCASE SYMBOLS=\"Aa+-\"",
        "Taxon Aa+-",
    ))
    .unwrap();
    assert_eq!(
        case.rows()[0],
        [
            Cell::State(0),
            Cell::State(1),
            Cell::State(2),
            Cell::State(3)
        ]
    );
    assert!(nexus::parse(&document(1, 1, "SYMBOLS=\"01\"", "A -")).is_err());
}

#[test]
fn malformed_data_produces_located_errors() {
    let cases = [
        (document(2, 2, "", "A 01\na 10"), "duplicate taxon"),
        (document(2, 2, "", "A_B 01\n'A B' 10"), "duplicate taxon"),
        (document(2, 2, "", "A 01\nB 0"), "incomplete row"),
        (document(2, 2, "", "A 010\nB 01"), "exceeds NCHAR"),
        (document(2, 2, "", "A 01\nB 02"), "undeclared state"),
        (document(2, 2, "", "A 01"), "expected 2 taxa"),
        (document(1, 2, "", "A 01 B 10"), "extra data"),
        (document(1, 1, "", "A {}"), "empty state set"),
        (document(1, 1, "", "A (01}"), "well-formed state set"),
        (document(1, 1, "", "A {0?}"), "only declared states"),
        (document(1, 1, "", "A {0~1}"), "undeclared state"),
        (document(1, 1, "", "A {0"), "unterminated state set"),
        (document(1, 1, "SYMBOLS=\"0\"", "A 0"), "at least two"),
        (document(1, 1, "SYMBOLS=\"Aa\"", "A A"), "must be distinct"),
        (document(1, 1, "SYMBOLS=\"0?\"", "A 0"), "must be distinct"),
        (document(1, 1, "GAP=?", "A 0"), "must be distinct"),
        (document(0, 1, "", ""), "positive integer"),
        (document(1, 0, "", "A"), "positive integer"),
        ("#NEXUS BEGIN DATA; [no end".into(), "unterminated comment"),
        (
            "#NEXUS BEGIN DATA; MATRIX 'no end".into(),
            "unterminated quoted",
        ),
        ("#NEXUS BEGIN DATA".into(), "unterminated command"),
        ("#NEXUS BEGIN DATA;".into(), "unterminated block"),
    ];
    for (text, message) in cases {
        let error = nexus::parse(&text).expect_err(&text);
        assert!(
            error.message.contains(message),
            "{text}: {error}, expected {message}"
        );
        assert!(error.line > 0 && error.column > 0);
    }
    let error = nexus::parse(&document(1, 1, "", "A 2")).unwrap_err();
    assert_eq!((error.line, error.column), (6, 3));
}

#[test]
fn unsupported_semantics_are_rejected_instead_of_ignored() {
    for option in [
        "DATATYPE=DNA",
        "TOKENS",
        "TRANSPOSE",
        "NOLABELS",
        "MATCHCHAR=.",
        "EQUATE=\"x={01}\"",
        "STATESFORMAT=FREQUENCY",
        "ITEMS=STATES",
    ] {
        assert!(
            nexus::parse(&document(1, 1, option, "A 0")).is_err(),
            "{option}"
        );
    }
    for block in [
        "SETS; CHARSET first = 1-2;",
        "ASSUMPTIONS; OPTIONS GAPMODE=NEWSTATE;",
        "PAUP; exclude 1;",
        "CODONS;",
    ] {
        let text = format!("{} BEGIN {block} END;", document(1, 2, "", "A 01"));
        let error = nexus::parse(&text).unwrap_err();
        assert!(error.message.contains("unsupported block"), "{error}");
    }
    let duplicate = format!(
        "{} BEGIN DATA; DIMENSIONS NTAX=1 NCHAR=1; MATRIX A 0; END;",
        document(1, 1, "", "A 0")
    );
    assert!(
        nexus::parse(&duplicate)
            .unwrap_err()
            .message
            .contains("only one DATA")
    );
}

#[test]
fn malformed_interleaved_pages_do_not_shift_cells_between_taxa() {
    for text in [
        "A 0\nB 1\nB 0\nA 1",
        "A 0\nB 10\nA 1\nB 0",
        "A 0\nB 1\nA 1",
        "A 0\nB 1\n1\n0",
        "A 0\nB 1",
        "A 010\nB 101",
        "A {0\nB 1}\nA 1\nB 0",
    ] {
        assert!(
            nexus::parse(&document(2, 2, "INTERLEAVE", text)).is_err(),
            "{text}"
        );
    }
}

#[test]
fn inconsistent_declarations_are_rejected() {
    let prefix = "#NEXUS BEGIN DATA; DIMENSIONS NTAX=2 NCHAR=2;";
    for command in [
        "TAXLABELS A;",
        "TAXLABELS A C;",
        "TAXLABELS A a;",
        "CHARLABELS size;",
        "CHARSTATELABELS 3 size;",
        "CHARSTATELABELS 1 size / small medium large;",
        "CHARSTATELABELS 1 size, 1 shape;",
        "STATELABELS 1 small large, 1 short long;",
        "STATELABELS 1 small large,;",
        "DIMENSIONS NTAX=2 NCHAR=2;",
    ] {
        let text = format!("{prefix} {command} MATRIX A 01 B 10; END;");
        assert!(nexus::parse(&text).is_err(), "{text}");
    }
    for interleave in ["", "INTERLEAVE"] {
        let text = format!(
            "#NEXUS BEGIN DATA; DIMENSIONS NCHAR=2; TAXLABELS; FORMAT {interleave}; MATRIX; END;"
        );
        assert!(nexus::parse(&text).is_err());
    }
    // A declaration must not trigger allocation before its rows are validated.
    let text = document(1, usize::MAX, "", "A 01");
    assert!(nexus::parse(&text).is_err());
    let text = document(usize::MAX, 1, "INTERLEAVE", "A 0");
    assert!(nexus::parse(&text).is_err());
}

fn likelihood(tips: &[Vec<f64>], weights: &[f64], states: usize) -> f64 {
    let model = Mk::uniform(states);
    let mut core = LikelihoodCore::new(states, weights.len(), 2, 5, Scaling::Always);
    for (node, partials) in tips.iter().enumerate() {
        core.set_tip_partials(node, partials);
    }
    let mut matrix = vec![0.0; states * states];
    for (node, length) in [(0, 0.1), (1, 0.3), (2, 0.2), (3, 0.15)] {
        for (category, rate) in [0.5, 1.5].iter().enumerate() {
            model.transition_matrix(length * rate, &mut matrix);
            core.set_node_matrix(node, category, &matrix);
        }
    }
    core.calculate_partials(0, 1, 3);
    core.calculate_partials(3, 2, 4);
    core.log_likelihood(4, &[0.5, 0.5], model.frequencies(), weights)
}

proptest! {
    #[test]
    fn compression_preserves_likelihood(cells in prop::collection::vec(prop::collection::vec(0u8..5, 3), 1..60)) {
        let mut text = vec![String::new(); 3];
        let mut tips = vec![Vec::new(); 3];
        for column in &cells {
            for (taxon, &cell) in column.iter().enumerate() {
                let (symbol, values) = match cell {
                    0 => ("0", [1.0, 0.0, 0.0]),
                    1 => ("1", [0.0, 1.0, 0.0]),
                    2 => ("2", [0.0, 0.0, 1.0]),
                    3 => ("{02}", [1.0, 0.0, 1.0]),
                    _ => ("?", [1.0, 1.0, 1.0]),
                };
                // Repeat every column to exercise non-unit weights.
                for _ in 0..2 { text[taxon].push_str(symbol); tips[taxon].extend(values); }
            }
        }
        let rows = format!("A {}\nB {}\nC {}", text[0], text[1], text[2]);
        let data = nexus::parse(&document(3, cells.len() * 2, "SYMBOLS=\"012\"", &rows)).unwrap();
        let compressed = compress_all(&data);
        let weights: Vec<_> = compressed.weights().iter().map(|&w| w as f64).collect();
        let actual = likelihood(compressed.tip_partials(), &weights, 3);
        let expected = likelihood(&tips, &vec![1.0; cells.len() * 2], 3);
        prop_assert!((actual - expected).abs() < 1e-10 * expected.abs().max(1.0));
    }

    #[test]
    fn arbitrary_input_does_not_panic(text in ".{0,2000}") {
        let _ = nexus::parse(&text);
    }

    #[test]
    fn malformed_matrix_content_does_not_panic(
        body in "[A-C01?{}()\\[\\];=,_' \\n\\r-]{0,200}",
        ntax in 1usize..5,
        nchar in 1usize..15,
        interleave in any::<bool>(),
    ) {
        let format = if interleave { "INTERLEAVE" } else { "" };
        if let Ok(data) = nexus::parse(&document(ntax, nchar, format, &body)) {
            let sites = 0..data.character_count();
            let _ = data.compress(std::slice::from_ref(&sites), GapTreatment::Reject);
        }
    }
}
