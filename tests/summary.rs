use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use rove::summary::{Burnin, Heights, summarize};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rove-summary-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn input(&self, text: &str) -> PathBuf {
        let p = self.0.join("input.trees");
        fs::write(&p, text).unwrap();
        p
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn sample() -> String {
    include_str!("../fixtures/summary/four-taxa.trees").into()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
}

#[test]
fn mcc_support_burnin_ties_and_common_ancestor_heights() {
    let temp = Temp::new();
    let input = temp.input(&sample());
    let summary = summarize(
        &input,
        Burnin::Percent(25),
        Heights::CommonAncestor,
        &[],
        &|| false,
    )
    .unwrap();
    assert_eq!(
        (summary.total, summary.discarded, summary.retained()),
        (4, 1, 3)
    );
    assert_eq!(
        (
            summary.first_state,
            summary.first_retained_state,
            summary.last_state
        ),
        (0, 100, 300)
    );
    assert_eq!(summary.target.state, Some(100));
    near(summary.log_credibility, 2.0 * (2.0f64 / 3.0).ln());
    let masks = summary.target.clades();
    let ab = masks.iter().position(|m| m == &[3]).unwrap();
    near(summary.nodes[ab].posterior, 2.0 / 3.0);
    near(summary.nodes[ab].height.mean, 1.5);
    near(summary.nodes[ab].ancestor.mean, 8.0 / 3.0);
    near(summary.nodes[summary.target.root].ancestor.mean, 4.0);
    let mut output = Vec::new();
    summary.write_tree(&mut output).unwrap();
    let annotated = String::from_utf8(output).unwrap();
    assert!(annotated.contains("posterior=0.6666666666666666"));
    assert!(annotated.contains("[&R]"));
    assert!(!annotated.contains("HPD"));
    let keep = summarize(&input, Burnin::Samples(1), Heights::Keep, &[], &|| false).unwrap();
    assert_eq!(keep.target.state, Some(100));
    near(keep.target.nodes[ab].height, 1.0);
    let untrimmed = summarize(&input, Burnin::Percent(10), Heights::Keep, &[], &|| false).unwrap();
    assert_eq!(untrimmed.discarded, 0);
    assert_eq!(untrimmed.target.state, Some(0));
    assert!(summarize(&input, Burnin::Samples(4), Heights::Keep, &[], &|| false).is_err());
    assert!(summarize(&input, Burnin::Percent(100), Heights::Keep, &[], &|| false).is_err());
}

#[test]
fn rate_means_are_conditional_and_constraints_are_not_invented() {
    let temp = Temp::new();
    let text = sample()
        .lines()
        .map(|line| {
            if !line.starts_with("tree ") {
                return line.to_owned();
            }
            let rate = if line.contains("STATE_200") {
                3
            } else if line.contains("STATE_300") {
                5
            } else {
                1
            };
            line.replace(':', &format!("[&rate={rate}]:"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let input = temp.input(&text);
    let s = summarize(&input, Burnin::Samples(1), Heights::Keep, &[], &|| false).unwrap();
    let ab = s.target.clades().iter().position(|m| m == &[3]).unwrap();
    near(s.nodes[ab].rate.mean, 2.0);
    assert_eq!(s.nodes[ab].rate.count, 2);
    near(s.nodes[s.target.tips[0]].rate.mean, 3.0);
    assert!(s.nodes.iter().all(|n| !n.constrained));
    assert!(
        summarize(
            &input,
            Burnin::Samples(1),
            Heights::Keep,
            &[vec!["A".into(), "B".into()]],
            &|| false
        )
        .is_err()
    );
    let all = vec![s.taxa.clone()];
    let marked = summarize(&input, Burnin::Samples(1), Heights::Keep, &all, &|| false).unwrap();
    assert!(marked.nodes[marked.target.root].constrained);
    assert!(
        summarize(
            &input,
            Burnin::Samples(1),
            Heights::Keep,
            &[vec!["X".into(), "B".into()]],
            &|| false
        )
        .is_err()
    );
}

#[test]
fn dated_reader_accepts_single_and_adjacent_metadata_blocks() {
    let single = "#NEXUS\nBegin trees;Translate 1 A,2 B,3 C;tree STATE_0 = [&R] \
((1[&height=0,rate=0.5]:2,2[&height=1,rate=1.5]:1)[&height=2,rate=0.8]:1,3[&height=0.5,rate=2]:2.5)[&height=3];End;";
    let adjacent = single
        .replace("[&height=0,rate=0.5]", "[&height=0][&rate=0.5]")
        .replace("[&height=1,rate=1.5]", "[&height=1][&rate=1.5]")
        .replace("[&height=2,rate=0.8]", "[&height=2][&rate=0.8]")
        .replace("[&height=0.5,rate=2]", "[&height=0.5][&rate=2]");
    assert_ne!(single, adjacent);
    let read = |text: &str| {
        let mut reader = rove::summary::nexus::Reader::new(text.as_bytes()).unwrap();
        let tree = reader.next_tree().unwrap().unwrap();
        assert!(reader.next_tree().unwrap().is_none());
        tree
    };
    let (a, b) = (read(single), read(&adjacent));
    let values = |tree: &rove::summary::nexus::Tree| {
        tree.nodes
            .iter()
            .map(|n| (n.height, n.length, n.rate))
            .collect::<Vec<_>>()
    };
    assert_eq!(values(&a), values(&b));
    assert_eq!(a.nodes[a.tips[1]].height, 1.0);
    assert_eq!(a.nodes[a.tips[1]].rate, Some(1.5));
    assert_eq!(a.nodes[a.tips[2]].height, 0.5);
    assert_eq!(a.nodes[a.root].height, 3.0);
    assert_eq!(a.nodes[a.root].rate, None);
}

#[test]
fn malformed_incomplete_mixed_and_nonclock_trees_are_rejected() {
    for text in [
        sample().replace("End;", ""),
        sample().replace("4 D", "4 C"),
        sample().replace(
            "Begin trees;",
            "Begin taxa; Dimensions ntax=5; End; Begin trees;",
        ),
        format!("{}Begin taxa; Taxlabels X Y; End;", sample()),
        sample().replace("End;", "End garbage;"),
        sample().replace("1:1,3:1", "1:1,5:1"),
        sample().replace("1:1,3:1", "1:1,1:1"),
        sample().replace("1:1,3:1", "1:2,3:1"),
        sample().replace("STATE_200", "STATE_100"),
        sample().replace("1:1,3:1", "1:-1,3:1"),
        sample().replace("1:1,3:1", "1:NaN,3:1"),
        sample().replace("tree STATE_0 =", "tree STATE_0 = [&U]"),
        sample().replace("1:1,3:1", "1[&rate=1]:1,3:1"),
        sample().replace("((1:1,3:1):5,(2:1,4:1):5)", "(1:6,2:6,3:6,4:6)"),
        sample().replace("((1:1,3:1):5,(2:1,4:1):5)", "((1:1,3:1):5,2:6)"),
        "#NEXUS\nBegin trees;End;".into(),
    ] {
        let temp = Temp::new();
        let path = temp.input(&text);
        assert!(
            summarize(&path, Burnin::Percent(0), Heights::Keep, &[], &|| false).is_err(),
            "accepted: {text}"
        );
    }
    let temp = Temp::new();
    let path = temp.input(&sample());
    assert!(summarize(&path, Burnin::Percent(0), Heights::Keep, &[], &|| true).is_err());
}

#[test]
fn beast_tree_files_keep_their_zero_length_root_stem_and_lose_the_root_rate() {
    // BEAST 2 writes a rate on every branch, including a zero-length root stem.
    let text = "#NEXUS\nBegin trees;Translate 1 A, 2 B, 3 C;\n\
        tree STATE_0 = ((1[&rate=0.9]:1,2[&rate=1.1]:1)[&rate=1.0]:1,3[&rate=0.8]:2)[&rate=1.0]:0.0;\n\
        tree STATE_100 = ((1[&rate=1.2]:1,3[&rate=0.7]:1)[&rate=0.9]:1,2[&rate=1.0]:2)[&rate=1.0]:0.0;\n\
        End;";
    let temp = Temp::new();
    let summary = summarize(
        &temp.input(text),
        Burnin::Percent(0),
        Heights::Keep,
        &[],
        &|| false,
    )
    .unwrap();
    assert_eq!((summary.total, summary.retained()), (2, 2));
    let root = summary.target.root;
    assert!(summary.target.nodes[root].rate.is_none());
    assert_eq!(summary.nodes[root].rate.count, 0);
    for stem in [":0.5;", ":1e-9;"] {
        let temp = Temp::new();
        let path = temp.input(&text.replace(":0.0;", stem));
        assert!(summarize(&path, Burnin::Percent(0), Heights::Keep, &[], &|| false).is_err());
    }
}

#[test]
fn quoted_translation_and_nested_comments_preserve_taxa() {
    let temp = Temp::new();
    let input = temp.input(
        &sample()
            .replace("1 A", "1 'O''Brien; [tip]'")
            .replace("2 B", "2 'B taxon'")
            .replace("Begin trees;", "[outer [inner]] Begin trees;")
            .replace("End;", "End; [trailing comment]"),
    );
    let s = summarize(&input, Burnin::Samples(0), Heights::Keep, &[], &|| false).unwrap();
    assert_eq!(s.taxa[0], "O'Brien; [tip]");
    assert_eq!(s.taxa[1], "B taxon");
    let mut bytes = Vec::new();
    s.write_tree(&mut bytes).unwrap();
    assert!(
        String::from_utf8(bytes)
            .unwrap()
            .contains("'O''Brien; [tip]'")
    );
}

#[test]
fn cli_preserves_raw_samples_and_never_overwrites_or_summarizes_partial_runs() {
    let temp = Temp::new();
    let input = temp.input(&sample());
    let output = temp.0.join("summary");
    let call = |out: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_rove"))
            .args([
                "summarize",
                input.to_str().unwrap(),
                "--output",
                out.to_str().unwrap(),
                "--burnin-percent",
                "25",
                "--heights",
                "ca",
            ])
            .output()
            .unwrap()
    };
    let result = call(&output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("summary.json")).unwrap()).unwrap();
    assert_eq!(record["retained_samples"], 3);
    assert_eq!(record["selected_state"], 100);
    assert_eq!(record["source"]["completion"], "unverified");
    assert_eq!(record["convergence_assessed"], false);
    assert_eq!(
        fs::read(output.join("samples.trees")).unwrap(),
        fs::read(&input).unwrap()
    );
    assert!(!call(&output).status.success());
    assert!(output.join("summary.json").exists());
    fs::write(temp.0.join("run.json"), r#"{"status":"cancelled"}"#).unwrap();
    let partial = temp.0.join("partial");
    assert!(!call(&partial).status.success());
    assert!(!partial.exists());
    fs::remove_file(temp.0.join("run.json")).unwrap();
    fs::write(&input, sample().replace("End;", "")).unwrap();
    assert!(!call(&partial).status.success());
    assert!(!partial.exists());
}

#[test]
fn translated_clades_span_multiple_mask_words() {
    fn balanced(start: usize, end: usize) -> String {
        if end - start == 1 {
            return format!("{}:1", start + 1);
        }
        let middle = (start + end) / 2;
        format!("({},{}):1", balanced(start, middle), balanced(middle, end))
    }
    let temp = Temp::new();
    let taxa: Vec<_> = (1..=128).map(|i| format!("T{i}")).collect();
    let translation = taxa
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{} {t}", i + 1))
        .collect::<Vec<_>>()
        .join(",");
    let tree = balanced(0, 128);
    let tree = tree.strip_suffix(":1").unwrap();
    let source = format!(
        "#NEXUS\nBegin taxa; Taxlabels {}; End; Begin trees; Translate {translation}; tree STATE_0 = {tree}; tree STATE_100 = {tree}; End;",
        taxa.iter().rev().cloned().collect::<Vec<_>>().join(" ")
    );
    let input = temp.input(&source);
    let summary = summarize(
        &input,
        Burnin::Samples(0),
        Heights::CommonAncestor,
        &[taxa[64..].to_vec()],
        &|| false,
    )
    .unwrap();
    assert_eq!(summary.target.state, Some(0));
    assert_eq!(summary.sampling_interval, Some(100));
    assert_eq!(summary.target.tips.len(), 128);
    let marked: Vec<_> = summary.nodes.iter().filter(|n| n.constrained).collect();
    assert_eq!(marked.len(), 1);
    near(marked[0].ancestor.mean, 6.0);
    near(summary.nodes[summary.target.root].ancestor.mean, 7.0);
}

#[test]
fn source_record_checks_every_state_and_owner_exit_cancels_work() {
    use std::process::Stdio;
    use std::time::{Duration, Instant};
    let temp = Temp::new();
    let input = temp.input(&sample().replace("STATE_200", "STATE_201"));
    fs::write(temp.0.join("run.json"), r#"{"status":"completed","completed_steps":300,"settings":{"steps":300,"sample_every":100},"model":{"monophyly":[]}}"#).unwrap();
    let output = temp.0.join("out");
    let args = [
        "summarize",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--burnin-percent",
        "25",
        "--heights",
        "ca",
    ];
    let failed = Command::new(env!("CARGO_BIN_EXE_rove"))
        .args(args)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("disagree"));
    assert!(!output.exists());
    fs::remove_file(temp.0.join("run.json")).unwrap();
    let mut big = String::from("#NEXUS\nBegin trees;Translate 1 A,2 B,3 C,4 D;\n");
    for i in 0..100_000 {
        big.push_str(&format!("tree STATE_{i} = ((1:1,2:1):2,(3:1,4:1):2);\n"));
    }
    big.push_str("End;\n");
    fs::write(&input, big).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_rove"))
        .args(args)
        .arg("--cancel-on-stdin-close")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !output.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(child.stdin.take());
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(!status.success());
            assert!(!output.exists());
            break;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("summary ignored owner EOF");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn annotated_reader_preserves_statistics_and_rejects_raw_or_invalid_metadata() {
    use rove::summary::nexus::Reader;
    use std::io::Cursor;
    let temp = Temp::new();
    let summary = summarize(
        &temp.input(&sample()),
        Burnin::Percent(25),
        Heights::CommonAncestor,
        &[],
        &|| false,
    )
    .unwrap();
    let mut bytes = Vec::new();
    summary.write_tree(&mut bytes).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let mut reader = Reader::annotated(Cursor::new(&text)).unwrap();
    let tree = reader.next_tree().unwrap().unwrap();
    assert_eq!(tree.state, None);
    let ab = tree.clades().iter().position(|m| m == &[3]).unwrap();
    near(tree.annotations[ab].posterior.unwrap(), 2.0 / 3.0);
    near(tree.nodes[ab].height, 8.0 / 3.0);
    near(tree.annotations[ab].height_mean.unwrap(), 1.5);
    assert_eq!(tree.annotations[ab].height_range, Some([1.0, 2.0]));
    assert_eq!(tree.annotations[ab].ancestor_range, Some([1.0, 5.0]));
    assert!(tree.nodes[ab].rate.is_none());
    assert!(reader.next_tree().unwrap().is_none());
    for bad in [
        sample(),
        text.replace("MCC", "STATE_100"),
        text.replace("[&R]", ""),
        text.replace("posterior=0.6666666666666666", "posterior=1.1"),
        text.replace("posterior=0.6666666666666666", "posterior=NaN"),
        text.replace("height_range={1,2}", "height_range={2,1}"),
        text.replace("height_mean=1.5", "height_mean=1.5,height_mean=2"),
        text.replace("End;", ""),
    ] {
        assert_ne!(bad, text, "mutation did not match fixture");
        let mut reader = Reader::annotated(Cursor::new(&bad)).unwrap();
        assert!(
            reader.next_tree().and_then(|_| reader.next_tree()).is_err(),
            "accepted: {bad}"
        );
    }
    let minimal = "#NEXUS\nBegin trees;Translate 1 A,2 B;tree MCC = [&R] (1:1,2:1);End;";
    let mut reader = Reader::annotated(Cursor::new(minimal)).unwrap();
    let tree = reader.next_tree().unwrap().unwrap();
    assert!(
        tree.annotations
            .iter()
            .all(|a| a.posterior.is_none() && a.ancestor_mean.is_none())
    );
    let duplicate = minimal.replace("End;", "tree MCC = [&R] (1:1,2:1);End;");
    let mut reader = Reader::annotated(Cursor::new(duplicate)).unwrap();
    assert!(reader.next_tree().unwrap().is_some());
    assert!(reader.next_tree().is_err());
}

#[test]
fn inspector_checks_provenance_and_never_changes_scientific_files() {
    let temp = Temp::new();
    let input = temp.input(&sample());
    let output = temp.0.join("summary");
    let summarize = Command::new(env!("CARGO_BIN_EXE_rove"))
        .args([
            "summarize",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--burnin-percent",
            "25",
            "--heights",
            "ca",
        ])
        .output()
        .unwrap();
    assert!(
        summarize.status.success(),
        "{}",
        String::from_utf8_lossy(&summarize.stderr)
    );
    let files = ["mcc.tree", "samples.trees", "summary.json"];
    let original: Vec<_> = files
        .iter()
        .map(|f| fs::read(output.join(f)).unwrap())
        .collect();
    let inspect = || {
        Command::new(env!("CARGO_BIN_EXE_rove"))
            .args(["inspect-tree", output.to_str().unwrap()])
            .output()
            .unwrap()
    };
    let result = inspect();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let tree: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(tree["kind"], "posterior_summary");
    assert_eq!(tree["summary"]["retained_samples"], 3);
    assert_eq!(tree["summary"]["selected_state"], 100);
    let root = tree["root"].as_u64().unwrap() as usize;
    near(tree["nodes"][root]["height"].as_f64().unwrap(), 4.0);
    assert!(tree["nodes"][0]["posterior"].is_null());
    for (name, bytes) in files.iter().zip(&original) {
        assert_eq!(fs::read(output.join(name)).unwrap(), *bytes);
    }
    fs::write(output.join("mcc.tree"), b"changed").unwrap();
    assert!(String::from_utf8_lossy(&inspect().stderr).contains("fingerprint"));
    fs::write(output.join("mcc.tree"), &original[0]).unwrap();
    let mut record: serde_json::Value = serde_json::from_slice(&original[2]).unwrap();
    record["retained_samples"] = 999.into();
    fs::write(output.join("summary.json"), record.to_string()).unwrap();
    assert!(!inspect().status.success());
    record["retained_samples"] = 3.into();
    record["source"]["constraints"] = serde_json::json!([["A", "C"]]);
    fs::write(output.join("summary.json"), record.to_string()).unwrap();
    assert!(String::from_utf8_lossy(&inspect().stderr).contains("absent"));
}
