use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rove::data::GapTreatment;
use rove::diagnostics::Trace;
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::predictive::{CharacterStatistics, Design, Statistics, TailCount, to_nexus};
use rove::random::Random;
use rove::rates::{GammaRates, RateVariation, RelaxedClock};
use rove::summary::{Burnin, nexus::Reader};
use rove::tree::Tree;
use serde_json::{Value, json};

use super::{events::Cancellation, partitions::Partitioning, sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Output {
    path: PathBuf,
    committed: bool,
}
impl Drop for Output {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(format!("{} exceeds its input size limit", path.display()).into());
    }
    Ok(bytes)
}

fn copy(source: &Path, target: &Path, limit: u64, cancel: &Cancellation) -> Result<String> {
    let mut input = File::open(source)?;
    let mut output = BufWriter::new(File::create(target)?);
    let mut buffer = [0; 128 * 1024];
    let mut digest = sha256::Digest::new();
    let mut total = 0u64;
    loop {
        if cancel.requested() {
            return Err("posterior prediction cancelled".into());
        }
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > limit {
            return Err(format!("{} exceeds its input size limit", source.display()).into());
        }
        digest.update(&buffer[..n]);
        output.write_all(&buffer[..n])?;
    }
    output.flush()?;
    Ok(digest.finish())
}

fn column(trace: &Trace, name: &str, row: usize) -> Result<f64> {
    trace
        .columns
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.values[row])
        .ok_or_else(|| format!("missing trace column {name}").into())
}

fn tree_state(
    source: &rove::summary::nexus::Tree,
    reference: bool,
    trace: &Trace,
    row: usize,
) -> Result<(Tree, RateVariation)> {
    let tips = source.tips.len();
    let n = source.nodes.len();
    let mut map = vec![0; n];
    let mut internal = tips;
    for (i, node) in source.nodes.iter().enumerate() {
        map[i] = if let Some(tip) = node.tip {
            tip
        } else {
            let i = internal;
            internal += 1;
            i
        };
    }
    let mut children = vec![None; n];
    let mut heights = vec![0.0; n];
    let mut branch_rates = vec![1.0; n - 1];
    for (i, node) in source.nodes.iter().enumerate() {
        let at = map[i];
        heights[at] = node.height;
        children[at] = node.children.map(|[a, b]| (map[a], map[b]));
        if i != source.root {
            if reference != node.rate.is_some() {
                return Err("tree branch rates disagree with the recorded clock".into());
            }
            // Postorder numbering puts the root last, so branch indices are direct.
            branch_rates[at] = node.rate.unwrap_or(1.0);
        }
    }
    let tree = Tree::new(children, heights)?;
    let rates = if reference {
        RateVariation {
            gamma: Some(GammaRates::new(column(trace, "gamma_shape", row)?)?),
            clock: Some(RelaxedClock::new(
                column(trace, "clock_sigma", row)?,
                branch_rates,
            )?),
        }
    } else {
        RateVariation::default()
    };
    Ok((tree, rates))
}

fn stats(s: &Statistics) -> Value {
    let t = s.total();
    json!({"parsimony_length":t.steps,"minimum_steps":t.minimum,"maximum_steps":t.maximum,
        "consistency_index":t.consistency(),"retention_index":t.retention(),
        "informative_characters":s.informative(),"pattern_surprise":t.surprise})
}
fn tails(t: &TailCount, replicates: usize) -> Value {
    json!({"valid_pairs":t.pairs,"undefined_pairs":replicates-t.pairs,
        "lower_count":t.lower,"upper_count":t.upper,"ties":t.ties,
        "p_lower":t.probability(t.lower),"p_upper":t.probability(t.upper),
        "lower_simulation_interval_95":t.interval(t.lower),"upper_simulation_interval_95":t.interval(t.upper)})
}
fn update(t: &mut [TailCount], a: CharacterStatistics, b: CharacterStatistics) {
    t[0].add(Some(a.steps as f64), Some(b.steps as f64));
    t[1].add(a.consistency(), b.consistency());
    t[2].add(a.retention(), b.retention());
    t[3].add(Some(a.surprise), Some(b.surprise));
}
fn metric_names() -> [&'static str; 4] {
    [
        "parsimony_length",
        "consistency_index",
        "retention_index",
        "pattern_surprise",
    ]
}

pub(super) fn run(args: &[String]) -> Result<()> {
    let mut source = None;
    let mut matrix = None;
    let mut output = None;
    let mut seed = None;
    let mut burnin = None;
    let mut replicates = 100usize;
    let mut max_attempts = 100_000usize;
    let mut owner = false;
    let mut seen = HashSet::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg.starts_with('-') && !seen.insert(arg.as_str()) {
            return Err(format!("duplicate option {arg}").into());
        }
        if arg == "--cancel-on-stdin-close" {
            owner = true;
            continue;
        }
        if !arg.starts_with('-') {
            if source.replace(PathBuf::from(arg)).is_some() {
                return Err("only one source run directory is accepted".into());
            }
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {arg}"))?;
        match arg.as_str() {
            "--matrix" => matrix = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--burnin-percent" => burnin = Some(Burnin::Percent(value.parse()?)),
            "--replicates" => replicates = value.parse()?,
            "--max-attempts" => max_attempts = value.parse()?,
            _ => return Err(format!("unknown prediction option {arg}").into()),
        }
    }
    let source = source.ok_or("a completed Rove run directory is required")?;
    let matrix = matrix.ok_or("--matrix is required; provide the exact fitted NEXUS input")?;
    let output = output.ok_or("--output is required")?;
    let seed = seed.ok_or("--seed is required")?;
    let burnin = burnin.ok_or("--burnin-percent is required")?;
    if !(1..=10_000).contains(&replicates) || !(1..=10_000_000).contains(&max_attempts) {
        return Err("replicates must be 1..10000 and max-attempts 1..10000000".into());
    }
    let cancel = Cancellation::watch_stdin(owner);
    let began = Instant::now();
    let record_bytes = bounded(&source.join("run.json"), 8 * 1024 * 1024)?;
    let record: Value = serde_json::from_slice(&record_bytes)?;
    if record["status"] != "completed" || record["model"]["sample_from_prior"] != false {
        return Err("posterior prediction requires a completed posterior run, not a partial or prior-only run".into());
    }
    let steps = record["completed_steps"]
        .as_u64()
        .ok_or("missing completed step count")?;
    let requested = record["settings"]["steps"]
        .as_u64()
        .ok_or("missing requested step count")?;
    if steps == 0
        || steps > requested
        || (steps != requested && record["termination_reason"] != "diagnostic_thresholds")
    {
        return Err("run completion disagrees with the recorded settings".into());
    }
    if source.file_name().is_some_and(|s| s == "results") {
        if let Some(parent) = source.parent() {
            let session = parent.join("session.json");
            if session.exists() {
                let s: Value = serde_json::from_slice(&bounded(&session, 8 * 1024 * 1024)?)?;
                if s["status"] != "completed" || s["exitCode"] != 0 {
                    return Err("source app session has no successful finish".into());
                }
            }
        }
    }
    let input = bounded(&matrix, 64 * 1024 * 1024)?;
    if record["input"]["sha256"] != sha256::hex(&input) {
        return Err("matrix fingerprint differs from the fitted input".into());
    }
    let data = rove::nexus::parse(std::str::from_utf8(&input)?)?;
    let gaps = match record["input"]["gaps"].as_str() {
        Some("reject") => GapTreatment::Reject,
        Some("missing") => GapTreatment::Missing,
        _ => return Err("unknown gap treatment".into()),
    };
    let partitioning = Partitioning::parse(
        record["model"]["partitioning"]
            .as_str()
            .ok_or("missing partitioning")?,
    )?;
    let partitions = partitioning.compress(&data, gaps)?;
    let mut described = String::new();
    super::partitions::describe(&partitions).write(&mut described, 0);
    if record["model"]["partitions"] != serde_json::from_str::<Value>(&described)? {
        return Err("fitted partitions disagree with the input".into());
    }
    let conditioning = match record["model"]["conditioning"].as_str() {
        Some("none (Mk)") => Conditioning::None,
        Some("variable characters (Mkv)") => Conditioning::Variable,
        Some("parsimony-informative characters (Mkparsinf)") => Conditioning::Informative,
        _ => return Err("unknown ascertainment conditioning".into()),
    };
    let reference = match record["model"]["preset"].as_str() {
        Some("strict") => false,
        Some("reference") => true,
        _ => return Err("unsupported model preset".into()),
    };
    if record["model"]["clock_multiplier"] != 1.0 || record["model"]["mutation_rate"] != 1.0 {
        return Err("unsupported rate multiplier".into());
    }
    let design = Design::new(&data, &partitions, gaps, conditioning)?;
    if data.character_count().saturating_mul(replicates) > 2_000_000
        || data
            .character_count()
            .saturating_mul(data.taxa().len())
            .saturating_mul(replicates)
            > 200_000_000
    {
        return Err("requested replicates exceed the output budget; reduce --replicates".into());
    }
    fs::create_dir(&output)?;
    let mut cleanup = Output {
        path: output.clone(),
        committed: false,
    };
    fs::create_dir(output.join("source"))?;
    fs::create_dir(output.join("matrices"))?;
    fs::write(output.join("source/run.json"), &record_bytes)?;
    fs::write(output.join("source/matrix.nex"), &input)?;
    let trace_hash = copy(
        &source.join("trace.log"),
        &output.join("source/trace.log"),
        128 * 1024 * 1024,
        &cancel,
    )?;
    let trees_hash = copy(
        &source.join("trees.nexus"),
        &output.join("source/trees.nexus"),
        1024 * 1024 * 1024,
        &cancel,
    )?;
    let trace = Trace::read(
        &bounded(&output.join("source/trace.log"), 128 * 1024 * 1024)?,
        &|| cancel.requested(),
    )?;
    let interval = record["settings"]["sample_every"]
        .as_u64()
        .ok_or("missing sample interval")?;
    if interval == 0
        || steps % interval != 0
        || trace.states.first() != Some(&0)
        || trace.states.last() != Some(&steps)
        || trace.sample_every != Some(interval)
        || trace.states.len() as u64 != steps / interval + 1
    {
        return Err("trace samples disagree with the completed run".into());
    }
    let discarded = burnin.discard(trace.states.len())?;
    let mut random = Random::new(seed);
    let mut selected: BTreeMap<usize, Vec<(usize, u64)>> = BTreeMap::new();
    for replica in 0..replicates {
        let row = discarded + random.below(trace.states.len() - discarded);
        selected
            .entry(row)
            .or_default()
            .push((replica + 1, random.next_u64()));
    }
    let mut counts = vec![TailCount::default(); 4];
    let mut informative = TailCount::default();
    let mut character_counts = vec![vec![TailCount::default(); 4]; data.character_count()];
    let mut details = BufWriter::new(File::create(output.join("replicates.jsonl"))?);
    let mut characters = BufWriter::new(File::create(output.join("characters.tsv"))?);
    writeln!(
        characters,
        "replicate\tstate\tcharacter\tobserved_steps\tsimulated_steps\tobserved_minimum\tsimulated_minimum\tobserved_maximum\tsimulated_maximum\tobserved_surprise\tsimulated_surprise"
    )?;
    let mut reader = Reader::new(BufReader::new(File::open(
        output.join("source/trees.nexus"),
    )?))?;
    let mut row = 0;
    let mut done = 0;
    let mut max_likelihood_delta = 0.0f64;
    while let Some(raw) = reader.next_tree()? {
        if cancel.requested() {
            return Err("posterior prediction cancelled".into());
        }
        if reader.taxa != data.taxa() || raw.state != trace.states.get(row).copied() {
            return Err("tree taxa or sample states disagree with the matrix/trace".into());
        }
        if let Some(draws) = selected.get(&row) {
            let (tree, rates) = tree_state(&raw, reference, &trace, row)?;
            // Reconstruct the fitted likelihood before trusting the pairing of
            // this tree, branch rates and gamma shape with its trace row.
            let settings = Settings {
                conditioning,
                ..if reference {
                    Settings::reference()
                } else {
                    Settings::default()
                }
            };
            let check =
                Sampler::with_rates(&partitions, &settings, tree.clone(), rates.clone(), 0)?;
            let expected = column(&trace, "likelihood", row)?;
            let delta = (expected - check.posterior().likelihood).abs();
            if delta > 1e-7 + 1e-9 * expected.abs() {
                return Err(format!("reconstructed likelihood disagrees with trace at state {} (difference {delta})", trace.states[row]).into());
            }
            max_likelihood_delta = max_likelihood_delta.max(delta);
            let observed = design.statistics(&data, &tree, &rates, &|| cancel.requested())?;
            for &(replica, simulation_seed) in draws {
                let simulated =
                    design.simulate(&tree, &rates, simulation_seed, max_attempts, &|| {
                        cancel.requested()
                    })?;
                let comparison =
                    design.statistics(&simulated, &tree, &rates, &|| cancel.requested())?;
                update(&mut counts, observed.total(), comparison.total());
                informative.add(
                    Some(observed.informative() as f64),
                    Some(comparison.informative() as f64),
                );
                for (site, (a, b)) in observed
                    .characters
                    .iter()
                    .zip(&comparison.characters)
                    .enumerate()
                {
                    update(&mut character_counts[site], *a, *b);
                    writeln!(
                        characters,
                        "{replica}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        trace.states[row],
                        site + 1,
                        a.steps,
                        b.steps,
                        a.minimum,
                        b.minimum,
                        a.maximum,
                        b.maximum,
                        a.surprise,
                        b.surprise
                    )?;
                }
                let filename = format!("matrices/replicate-{replica:05}.nex");
                let text = to_nexus(&simulated);
                fs::write(output.join(&filename), &text)?;
                writeln!(
                    details,
                    "{}",
                    json!({"replicate":replica,"source_sample_index":row,"state":trace.states[row],"seed":simulation_seed,
                    "matrix":filename,"matrix_sha256":sha256::hex(text.as_bytes()),"gamma_shape":rates.gamma.as_ref().map(|g|g.shape()),
                    "observed":stats(&observed),"simulated":stats(&comparison),"reconstructed_likelihood_difference":delta})
                )?;
                done += 1;
                eprintln!("posterior prediction: {done}/{replicates}");
            }
        }
        row += 1;
    }
    if row != trace.states.len() || done != replicates {
        return Err("incomplete tree file or missing selected posterior draws".into());
    }
    details.flush()?;
    characters.flush()?;
    let mut summaries = serde_json::Map::new();
    for (name, count) in metric_names().into_iter().zip(&counts) {
        summaries.insert(name.into(), tails(count, replicates));
    }
    summaries.insert(
        "informative_characters".into(),
        tails(&informative, replicates),
    );
    let character_summaries: Vec<_> = character_counts
        .iter()
        .enumerate()
        .map(|(site, counts)| {
            let metrics: serde_json::Map<_, _> = metric_names()
                .into_iter()
                .zip(counts)
                .map(|(name, count)| (name.into(), tails(count, replicates)))
                .collect();
            json!({"character":site+1,"label":data.character_labels()[site],"metrics":metrics})
        })
        .collect();
    let report = json!({
        "schema_version":1,"method":"posterior-predictive-paired-v1","status":"completed",
        "rove_version":env!("CARGO_PKG_VERSION"),"executable_sha256":sha256::file(&std::env::current_exe()?)?,
        "command":std::env::args().collect::<Vec<_>>(),"created_unix_seconds":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        "source":{"directory":source,"run_sha256":sha256::hex(&record_bytes),"matrix_sha256":sha256::hex(&input),"trace_sha256":trace_hash,"trees_sha256":trees_hash,
            "run_record":record,"total_samples":row,"discarded_samples":discarded,"retained_samples":row-discarded,
            "first_retained_state":trace.states[discarded],"max_reconstructed_likelihood_difference":max_likelihood_delta},
        "seed":seed,"replicates":replicates,"max_attempts_per_character":max_attempts,
        "sampling":"uniform posterior rows with replacement after burn-in; independent per-replicate xoshiro256++ seeds",
        "burnin":"floor(total samples * percent / 100), including STATE_0; applied before selecting draws",
        "frozen_character_sets":design.character_sets().iter().map(|s|json!({"sites":s.sites.iter().map(|i|i+1).collect::<Vec<_>>(),"symbols":s.symbols.iter().collect::<String>()})).collect::<Vec<_>>(),
        "ascertainment":"reject complete latent characters before masking missing/gap cells; redraw gamma category on each attempt; never require all model states to appear",
        "statistics":"Sankoff parsimony on each sampled tree, not a search for a new most-parsimonious tree; ordered cost |i-j|, unordered cost 1; CI=sum(min)/sum(steps); RI=(sum(max)-sum(steps))/(sum(max)-sum(min)); zero denominators undefined",
        "character_fit":"negative unconditioned log pattern likelihood at the paired posterior draw, with the original missing mask; a discrepancy, not a conditioned predictive density",
        "tails":"p_upper=mean[T(replica,theta)>=T(observed,theta)]; p_lower uses <=; inclusive ties are reported; no automatic adequacy decision",
        "uncertainty":"Wilson 95% intervals quantify simulation error conditional on the finite empirical posterior; they do not include MCMC error. Undefined CI/RI pairs are excluded and counted. Per-character checks are exploratory, not multiplicity-adjusted.",
        "convergence_assessed":false,"global":summaries,"characters":character_summaries,
        "replicate_records":"replicates.jsonl","character_records":"characters.tsv","wall_seconds":began.elapsed().as_secs_f64(),
    });
    fs::write(
        output.join("predictive.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    cleanup.committed = true;
    println!(
        "{}",
        json!({"status":"completed","replicates":replicates,"report":output.join("predictive.json"),"global":report["global"]})
    );
    Ok(())
}
