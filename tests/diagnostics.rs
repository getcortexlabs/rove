use rove::diagnostics::{Trace, statistics};
use rove::summary::Burnin;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rove-diagnostics-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn near(a: f64, b: f64, relative: f64) {
    assert!((a - b).abs() <= relative * b.abs().max(1.0), "{a} != {b}");
}

#[test]
fn synthetic_statistics_match_pinned_beast_ess_and_python_quantiles() {
    let trace = Trace::read(
        include_bytes!("../fixtures/diagnostics/synthetic.tsv"),
        &|| false,
    )
    .unwrap();
    let expected: Value =
        serde_json::from_str(include_str!("../fixtures/diagnostics/expected.json")).unwrap();
    let skip = Burnin::Percent(25).discard(trace.states.len()).unwrap();
    assert_eq!(skip, 512);
    for column in &trace.columns {
        let actual = statistics(&column.values[skip..], &|| false).unwrap();
        let reference = &expected["statistics"][&column.name];
        near(actual.mean, reference["mean"].as_f64().unwrap(), 1e-10);
        near(actual.sd, reference["sd"].as_f64().unwrap(), 1e-10);
        match actual.ess {
            Some(ess) => near(ess, reference["ess"].as_f64().unwrap(), 1e-8),
            None => {
                assert!(reference["ess"].is_null());
                assert_eq!(actual.ess_reason, Some("constant"));
            }
        }
        for i in 0..2 {
            near(
                actual.interval.unwrap()[i],
                reference["interval"][i].as_f64().unwrap(),
                1e-10,
            );
        }
    }
    let ar = &trace
        .columns
        .iter()
        .find(|c| c.name == "ar_positive")
        .unwrap()
        .values[skip..];
    let theoretical = ar.len() as f64 * (1.0 - 0.9) / (1.0 + 0.9);
    near(
        statistics(ar, &|| false).unwrap().ess.unwrap(),
        theoretical,
        0.2,
    );
}

#[test]
fn analytic_moments_short_constant_and_extreme_traces_are_explicit() {
    let values: Vec<_> = (0..=100).map(f64::from).collect();
    let stats = statistics(&values, &|| false).unwrap();
    near(stats.mean, 50.0, 1e-12);
    near(stats.sd, 850.0_f64.sqrt(), 1e-12);
    assert_eq!(stats.interval, Some([2.5, 97.5]));
    for values in [&[7.0][..], &[7.0, 7.0, 7.0, 7.0]] {
        let stats = statistics(values, &|| false).unwrap();
        assert_eq!(stats.ess, None);
        assert_eq!(stats.ess_reason, Some("constant"));
    }
    let short = statistics(&[1.0, 2.0, 4.0], &|| false).unwrap();
    assert_eq!(short.ess_reason, Some("too_short"));
    assert!(short.interval.is_some());
    for values in [&[][..], &[f64::NAN], &[f64::INFINITY]] {
        assert!(statistics(values, &|| false).is_err());
    }
    let huge = statistics(&[-f64::MAX, f64::MAX, -f64::MAX, f64::MAX], &|| false).unwrap();
    assert!(huge.mean.is_finite() && huge.sd.is_finite());
    assert!(huge.interval.unwrap().iter().all(|x| x.is_finite()));
    let shifted: Vec<_> = values.iter().map(|x| x * 1e-3 + 1e9).collect();
    near(
        statistics(&shifted, &|| false).unwrap().ess.unwrap(),
        stats.ess.unwrap(),
        1e-5,
    );
    assert!(statistics(&values, &|| true).is_err());
    let trend: Vec<_> = (0..10_000).map(f64::from).collect();
    let capped = statistics(&trend, &|| false).unwrap();
    assert!(capped.lag_limit_reached && capped.last_lag == 1998);
}

#[test]
fn parser_rejects_truncation_gaps_nonfinite_and_duplicate_columns() {
    for input in [
        "Sample x\n0 1",
        "Sample x\n",
        "Sample x x\n0 1 2\n",
        "Sample x\n0 NaN\n",
        "Sample x\n0 inf\n",
        "Sample x\n0 1\n0 2\n",
        "Sample x\n10 1\n0 2\n",
        "Sample x\n0 1\n10 2\n30 3\n",
        "Sample x y\n0 1\n",
        "Sample x\n0 1 2\n",
    ] {
        assert!(
            Trace::read(input.as_bytes(), &|| false).is_err(),
            "accepted {input}"
        );
    }
    let trace = Trace::read(b"# run\r\nSample x\r\n0 1\r\n100 2\r\n", &|| false).unwrap();
    assert_eq!(trace.states, [0, 100]);
    assert_eq!(trace.sample_every, Some(100));
}

#[test]
fn plot_keeps_extrema_and_burnin_boundary_without_changing_statistics() {
    let mut data = String::from("Sample x\n");
    for i in 0..10_001 {
        data.push_str(&format!(
            "{} {}\n",
            i * 100,
            if i == 5001 { 1000 } else { i % 7 }
        ));
    }
    let trace = Trace::read(data.as_bytes(), &|| false).unwrap();
    let discarded = Burnin::Percent(25).discard(trace.states.len()).unwrap();
    let points = trace.plot(0, discarded);
    assert!(points.len() <= 1028 && points.len() < trace.states.len());
    assert!(
        points
            .iter()
            .any(|p| p.state == 500100 && p.value == 1000.0)
    );
    assert!(points.iter().any(|p| p.state == 249900 && !p.retained));
    assert!(points.iter().any(|p| p.state == 250000 && p.retained));
    let full = statistics(&trace.columns[0].values[discarded..], &|| false).unwrap();
    let reduced = points
        .iter()
        .filter(|p| p.retained)
        .map(|p| p.value)
        .collect::<Vec<_>>();
    let plotted = statistics(&reduced, &|| false).unwrap();
    assert!((full.mean - plotted.mean).abs() > 0.01);
}

#[test]
fn cli_records_burnin_and_detects_incomplete_runs_without_rewriting_sources() {
    let folder = Temp::new();
    let input = folder.0.join("trace.log");
    let data =
        b"Sample posterior likelihood prior\n0 -3 -4 1\n100 -2 -3 1\n200 -1 -2 1\n300 -2 -3 1\n";
    fs::write(&input, data).unwrap();
    let output = folder.0.join("diagnostics");
    let call = || {
        Command::new(env!("CARGO_BIN_EXE_rove"))
            .args([
                "diagnose",
                input.to_str().unwrap(),
                "--burnin-percent",
                "25",
                "--output",
                output.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let result = call();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(output.join("diagnostics.json")).unwrap()).unwrap();
    assert_eq!(report["first_retained_state"], 100);
    assert_eq!(report["retained_samples"], 3);
    assert_eq!(report["source"]["completion"], "unverified");
    assert_eq!(report["convergence_assessed"], false);
    assert!(report["rhat"].is_null());
    assert_eq!(fs::read(&input).unwrap(), data);
    assert!(!call().status.success());
    fs::remove_dir_all(&output).unwrap();
    let record = folder.0.join("run.json");
    fs::write(&record, r#"{"status":"completed","completed_steps":400,"settings":{"steps":400,"sample_every":100}}"#).unwrap();
    assert!(!call().status.success());
    assert!(!output.exists());
    fs::write(&record, r#"{"status":"cancelled","completed_steps":350,"settings":{"steps":1000,"sample_every":100}}"#).unwrap();
    assert!(call().status.success());
    let report: Value =
        serde_json::from_slice(&fs::read(output.join("diagnostics.json")).unwrap()).unwrap();
    assert_eq!(report["source"]["completion"], "cancelled");
    assert_eq!(fs::read(&input).unwrap(), data);
}

#[test]
fn rank_normalized_rhat_matches_the_python_oracle() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/diagnostics/rhat.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let chains: Vec<Vec<f64>> = serde_json::from_value(case["chains"].clone()).unwrap();
        let slices: Vec<_> = chains.iter().map(Vec::as_slice).collect();
        let actual = rove::diagnostics::rhat(&slices).unwrap();
        match &case["rhat"] {
            serde_json::Value::Null => assert!(actual.is_none()),
            serde_json::Value::String(_) => assert!(actual.unwrap().is_infinite()),
            value => assert!(
                (actual.unwrap() - value.as_f64().unwrap()).abs() < 1e-8,
                "{}: {actual:?} != {value}",
                case["name"]
            ),
        }
    }
    let a = [-1.0, 0.25, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0];
    let b = [0.25, 1.0, 0.5, -1.0, 1.0, 1.0, 1.0, 1.0];
    let huge_a: Vec<_> = a.iter().map(|x| x * f64::MAX).collect();
    let huge_b: Vec<_> = b.iter().map(|x| x * f64::MAX).collect();
    near(
        rove::diagnostics::rhat(&[&huge_a, &huge_b])
            .unwrap()
            .unwrap(),
        rove::diagnostics::rhat(&[&a, &b]).unwrap().unwrap(),
        1e-8,
    );
    assert!(rove::diagnostics::rhat(&[&[1., 2., 3., 4.]]).is_err());
    assert!(rove::diagnostics::rhat(&[&[1., 2., 3., 4.], &[1., 2., 3.]]).is_err());
    assert!(rove::diagnostics::rhat(&[&[1., 2., 3., 4.], &[1., 2., 3., f64::NAN]]).is_err());
}
