mod diagnose;
mod events;
mod inspect;
mod inspect_tree;
mod sha256;
mod summarize;

use std::collections::HashSet;
use std::error::Error;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rove::data::GapTreatment;
use rove::mcmc::output::{TraceWriter, TreeWriter, is_plain_label};
use rove::mcmc::{Conditioning, Move, Sampler, Settings};
use rove::{Backend, nexus};

const USAGE: &str =
    "usage: rove sample <matrix.nex> --output <directory> --seed <n> --steps <n> --sample-every <n>
                   [--monophyly <taxon,taxon,...>]... [--gaps reject|missing]
                   [--model strict|reference] [--conditioning variable|none] [--sample-from-prior]
                   [--no-tuning] [--tuning <move>=<value>]... [--backend portable|neon]
                   [--print-every <n>] [--events-json] [--cancel-on-stdin-close]
                   [--expect-input-sha256 <hash>]
       rove inspect <matrix.nex> [--gaps reject|missing]
       rove inspect-tree <summary-directory>
       rove diagnose <trace.log> --burnin-percent <0..99> [--output <new-directory>]
       rove summarize <trees.nexus> --output <directory> --burnin-percent <0..99> --heights keep|ca";

const TUNING_SCHEDULE: &str = "BEAST's OperatorSchedule.calcDelta and optimize, with its square-root transform; a failed proposal counts as acceptance zero";

struct Options {
    matrix: PathBuf,
    output: PathBuf,
    seed: u64,
    steps: u64,
    sample_every: u64,
    print_every: Option<u64>,
    events_json: bool,
    cancel_on_stdin_close: bool,
    expected_input: Option<String>,
    monophyly: Vec<Vec<String>>,
    gaps: GapTreatment,
    conditioning: Conditioning,
    reference_model: bool,
    sample_from_prior: bool,
    tuning: bool,
    tuning_values: Vec<(Move, f64)>,
    backend: Backend,
}

fn parse_options(arguments: &[String]) -> Result<Options, String> {
    let mut arguments = arguments.iter();
    if arguments.next().map(String::as_str) != Some("sample") {
        return Err(USAGE.into());
    }
    let mut matrix = None;
    let (mut output, mut seed, mut steps, mut sample_every, mut print_every) =
        (None, None, None, None, None);
    let mut monophyly = Vec::new();
    let mut events_json = false;
    let mut cancel_on_stdin_close = false;
    let mut expected_input = None;
    let mut gaps = GapTreatment::Reject;
    let mut conditioning = Conditioning::Variable;
    let mut reference_model = false;
    let mut sample_from_prior = false;
    let mut tuning = true;
    let mut tuning_values = Vec::new();
    let mut backend = Backend::NATIVE;
    let mut seen = HashSet::new();
    while let Some(argument) = arguments.next() {
        let repeatable = matches!(argument.as_str(), "--monophyly" | "--tuning");
        if argument.starts_with("--") && !repeatable && !seen.insert(argument.as_str()) {
            return Err(format!("{argument} is given more than once"));
        }
        let mut value = |name: &str| {
            arguments
                .next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        let number = |name: &str, text: String| {
            text.parse::<u64>()
                .map_err(|_| format!("{name} needs a non-negative integer, not {text:?}"))
        };
        match argument.as_str() {
            "--events-json" => events_json = true,
            "--cancel-on-stdin-close" => cancel_on_stdin_close = true,
            "--expect-input-sha256" => {
                let hash = value("--expect-input-sha256")?;
                if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
                    return Err("--expect-input-sha256 needs 64 hexadecimal digits".into());
                }
                expected_input = Some(hash.to_ascii_lowercase());
            }
            "--output" => output = Some(PathBuf::from(value("--output")?)),
            "--seed" => seed = Some(number("--seed", value("--seed")?)?),
            "--steps" => steps = Some(number("--steps", value("--steps")?)?),
            "--sample-every" => {
                sample_every = Some(number("--sample-every", value("--sample-every")?)?)
            }
            "--print-every" => {
                print_every = Some(number("--print-every", value("--print-every")?)?)
            }
            "--monophyly" => monophyly.push(
                value("--monophyly")?
                    .split(',')
                    .map(|taxon| taxon.trim().to_string())
                    .collect(),
            ),
            "--gaps" => {
                gaps = match value("--gaps")?.as_str() {
                    "reject" => GapTreatment::Reject,
                    "missing" => GapTreatment::Missing,
                    other => {
                        return Err(format!("--gaps must be reject or missing, not {other:?}"));
                    }
                }
            }
            "--conditioning" => {
                conditioning = match value("--conditioning")?.as_str() {
                    "variable" => Conditioning::Variable,
                    "none" => Conditioning::None,
                    other => {
                        return Err(format!(
                            "--conditioning must be variable or none, not {other:?}"
                        ));
                    }
                }
            }
            "--model" => {
                reference_model = match value("--model")?.as_str() {
                    "strict" => false,
                    "reference" => true,
                    other => {
                        return Err(format!(
                            "--model must be strict or reference, not {other:?}"
                        ));
                    }
                };
            }
            "--backend" => {
                let name = value("--backend")?;
                backend = Backend::ALL
                    .into_iter()
                    .find(|backend| backend.name() == name)
                    .ok_or_else(|| format!("--backend must be portable or neon, not {name:?}"))?;
            }
            "--sample-from-prior" => sample_from_prior = true,
            "--no-tuning" => tuning = false,
            "--tuning" => {
                let text = value("--tuning")?;
                let parsed = text.split_once('=').and_then(|(name, number)| {
                    let kind = Move::ALL.into_iter().find(|m| m.name() == name)?;
                    Some((kind, number.parse::<f64>().ok()?))
                });
                tuning_values.push(parsed.ok_or_else(|| {
                    format!(
                        "--tuning needs <move>=<value>, such as subtree_slide=0.001, not {text:?}"
                    )
                })?);
            }
            flag if flag.starts_with("--") => {
                return Err(format!("unsupported option {flag}\n{USAGE}"));
            }
            path if matrix.is_none() => matrix = Some(PathBuf::from(path)),
            extra => return Err(format!("unexpected argument {extra:?}\n{USAGE}")),
        }
    }
    let required = |value: Option<u64>, name: &str| {
        value.ok_or_else(|| format!("{name} is required\n{USAGE}"))
    };
    let options = Options {
        matrix: matrix.ok_or_else(|| format!("a NEXUS matrix is required\n{USAGE}"))?,
        output: output.ok_or_else(|| format!("--output is required\n{USAGE}"))?,
        seed: required(seed, "--seed")?,
        steps: required(steps, "--steps")?,
        sample_every: required(sample_every, "--sample-every")?,
        print_every,
        events_json,
        cancel_on_stdin_close,
        expected_input,
        monophyly,
        gaps,
        conditioning,
        reference_model,
        sample_from_prior,
        tuning,
        tuning_values,
        backend,
    };
    if options.steps == 0 || options.sample_every == 0 || options.steps % options.sample_every != 0
    {
        return Err(
            "--steps and --sample-every must be positive, and --steps a multiple of --sample-every"
                .into(),
        );
    }
    if options.print_every == Some(0) {
        return Err("--print-every must be positive".into());
    }
    Ok(options)
}

/// Writes a line to standard error, ignoring a closed stream.
fn progress(line: impl Display) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

/// A JSON string literal.
fn quoted(text: &str) -> String {
    let mut out = String::from('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A minimal JSON value for the run metadata.
enum Json {
    Null,
    Bool(bool),
    Integer(u64),
    Number(f64),
    Text(String),
    List(Vec<Json>),
    Object(Vec<(&'static str, Json)>),
}

impl Json {
    fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    fn write_compact(&self, out: &mut String) {
        match self {
            Self::List(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write_compact(out);
                }
                out.push(']');
            }
            Self::Object(fields) => {
                out.push('{');
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&quoted(key));
                    out.push(':');
                    value.write_compact(out);
                }
                out.push('}');
            }
            _ => self.write(out, 0),
        }
    }

    fn write(&self, out: &mut String, depth: usize) {
        let indent = |out: &mut String, depth: usize| out.push_str(&"  ".repeat(depth));
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(value) => out.push_str(&value.to_string()),
            Self::Integer(value) => out.push_str(&value.to_string()),
            Self::Number(value) if value.is_finite() => out.push_str(&format!("{value:?}")),
            Self::Number(_) => out.push_str("null"),
            Self::Text(text) => out.push_str(&quoted(text)),
            Self::List(items) if items.is_empty() => out.push_str("[]"),
            Self::List(items) => {
                out.push_str("[\n");
                for (index, item) in items.iter().enumerate() {
                    indent(out, depth + 1);
                    item.write(out, depth + 1);
                    out.push_str(if index + 1 < items.len() { ",\n" } else { "\n" });
                }
                indent(out, depth);
                out.push(']');
            }
            Self::Object(fields) => {
                out.push_str("{\n");
                for (index, (key, value)) in fields.iter().enumerate() {
                    indent(out, depth + 1);
                    out.push_str(&quoted(key));
                    out.push_str(": ");
                    value.write(out, depth + 1);
                    out.push_str(if index + 1 < fields.len() {
                        ",\n"
                    } else {
                        "\n"
                    });
                }
                indent(out, depth);
                out.push('}');
            }
        }
    }
}

fn run(arguments: &[String]) -> Result<bool, Box<dyn Error>> {
    let options = parse_options(arguments)?;
    let cancellation = events::Cancellation::watch_stdin(options.cancel_on_stdin_close);
    let executable = std::env::current_exe()
        .and_then(fs::read)
        .map(|binary| sha256::hex(&binary))
        .ok();
    let bytes = fs::read(&options.matrix)
        .map_err(|e| format!("cannot read {}: {e}", options.matrix.display()))?;
    let input = sha256::hex(&bytes);
    if options
        .expected_input
        .as_ref()
        .is_some_and(|expected| *expected != input)
    {
        return Err("the matrix fingerprint changed; inspect and review the input again".into());
    }
    let data = nexus::parse(std::str::from_utf8(&bytes)?)?;
    let sites = 0..data.character_count();
    let patterns = data
        .compress(std::slice::from_ref(&sites), options.gaps)?
        .remove(0);
    let taxa = data.taxa();
    if let Some(name) = taxa.iter().find(|name| !is_plain_label(name)) {
        return Err(format!(
            "taxon label {name:?} cannot be written so that BEAST reads it back; use letters, digits, spaces, '_', '.' or '-', and not both spaces and '_'"
        )
        .into());
    }
    let mut monophyly = Vec::new();
    for clade in &options.monophyly {
        let mut tips = Vec::new();
        for name in clade {
            // As in the matrix, an underscore stands for a space.
            let wanted = name.replace('_', " ");
            let tip = taxa
                .iter()
                .position(|taxon| *taxon == wanted)
                .ok_or_else(|| format!("--monophyly names unknown taxon {name:?}"))?;
            tips.push(tip);
        }
        monophyly.push(tips);
    }
    let settings = Settings {
        conditioning: options.conditioning,
        sample_from_prior: options.sample_from_prior,
        monophyly,
        tuning: options.tuning,
        tuning_values: options.tuning_values.clone(),
        backend: options.backend,
        ..if options.reference_model {
            Settings::reference()
        } else {
            Settings::default()
        }
    };
    let mut sampler = Sampler::new(&patterns, &settings, options.seed)?;

    if let Some(parent) = options
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&options.output).map_err(|e| {
        format!(
            "cannot create {} (it must not exist yet): {e}",
            options.output.display()
        )
    })?;
    let command: Vec<String> = std::iter::once("rove".to_string())
        .chain(arguments.iter().cloned())
        .collect();
    let command_json = format!(
        "[{}]",
        command
            .iter()
            .map(|a| quoted(a))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let comments = vec![
        format!("rove {}", env!("CARGO_PKG_VERSION")),
        format!("command {command_json}"),
        format!("input sha256 {input}"),
        format!(
            "executable sha256 {}",
            executable.as_deref().unwrap_or("unknown")
        ),
    ];
    let mut trace = TraceWriter::with_rates(
        BufWriter::new(File::create(options.output.join("trace.log"))?),
        &comments,
        sampler.rates(),
    )?;
    let mut trees = TreeWriter::new(
        BufWriter::new(File::create(options.output.join("trees.nexus"))?),
        taxa,
        &comments,
    )?;

    let started = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let clock = Instant::now();
    let print_every = options.print_every.unwrap_or((options.steps / 20).max(1));
    trace.write(0, &sampler)?;
    trees.write(0, &sampler)?;
    if options.events_json {
        events::state("started", 0, options.steps, 0.0, &input, &sampler)?;
    } else {
        progress("step\tposterior\tlikelihood\tprior\tseconds");
    }
    let mut completed_steps = 0;
    for step in 1..=options.steps {
        if step % 1024 == 1 && cancellation.requested() {
            break;
        }
        sampler.step();
        completed_steps = step;
        if step % options.sample_every == 0 {
            trace.write(step, &sampler)?;
            trees.write(step, &sampler)?;
        }
        if step % print_every == 0 {
            if options.events_json {
                events::state(
                    "progress",
                    step,
                    options.steps,
                    clock.elapsed().as_secs_f64(),
                    &input,
                    &sampler,
                )?;
                continue;
            }
            let posterior = sampler.posterior();
            progress(format_args!(
                "{step}\t{:.4}\t{:.4}\t{:.4}\t{:.1}",
                posterior.total(),
                posterior.likelihood,
                posterior.prior(),
                clock.elapsed().as_secs_f64()
            ));
        }
    }
    trace.finish()?;
    trees.finish()?;
    let seconds = clock.elapsed().as_secs_f64();
    let completed = completed_steps == options.steps;
    let status = if completed { "completed" } else { "cancelled" };

    let names = |tips: &[usize]| Json::List(tips.iter().map(|&t| Json::text(&taxa[t])).collect());
    let posterior = sampler.posterior();
    let optional = |value: Option<f64>| value.map_or(Json::Null, Json::Number);
    let moves = sampler
        .operators()
        .iter()
        .map(|operator| {
            Json::Object(vec![
                ("name", Json::text(operator.kind.name())),
                ("weight", Json::Number(operator.weight)),
                ("initial_tuning", optional(operator.initial_tuning)),
                ("final_tuning", optional(operator.tuning)),
                ("accepted", Json::Integer(operator.accepted)),
                ("rejected", Json::Integer(operator.rejected)),
                ("failed", Json::Integer(operator.failed)),
            ])
        })
        .collect();
    let conditioning = match settings.conditioning {
        Conditioning::Variable => "variable characters (Mkv)",
        Conditioning::None => "none (Mk)",
    };
    let metadata = Json::Object(vec![
        ("status", Json::text(status)),
        ("completed_steps", Json::Integer(completed_steps)),
        ("rove_version", Json::text(env!("CARGO_PKG_VERSION"))),
        (
            "executable_sha256",
            executable.map_or(Json::Null, Json::Text),
        ),
        (
            "platform",
            Json::text(format!(
                "{}-{}",
                std::env::consts::ARCH,
                std::env::consts::OS
            )),
        ),
        (
            "command",
            Json::List(command.into_iter().map(Json::Text).collect()),
        ),
        (
            "input",
            Json::Object(vec![
                ("path", Json::text(options.matrix.display().to_string())),
                ("sha256", Json::text(&input)),
                ("taxa", Json::Integer(taxa.len() as u64)),
                ("characters", Json::Integer(data.character_count() as u64)),
                (
                    "observed_patterns",
                    Json::Integer(patterns.weights().len() as u64),
                ),
                (
                    "gaps",
                    Json::text(match options.gaps {
                        GapTreatment::Reject => "reject",
                        GapTreatment::Missing => "missing",
                    }),
                ),
            ]),
        ),
        (
            "model",
            Json::Object(vec![
                (
                    "substitution",
                    Json::text(if options.reference_model {
                        "Mk, two states, equal frequencies, four midpoint gamma categories normalized to mean one"
                    } else {
                        "Mk, two states, equal frequencies, one rate category"
                    }),
                ),
                (
                    "preset",
                    Json::text(if options.reference_model {
                        "reference"
                    } else {
                        "strict"
                    }),
                ),
                ("conditioning", Json::text(conditioning)),
                ("mutation_rate", Json::Number(1.0)),
                ("clock_multiplier", Json::Number(1.0)),
                (
                    "gamma_shape_prior",
                    if options.reference_model {
                        Json::text(
                            "Exponential(mean=1), parameter bounds [0.001, 1000], density without truncation normalization",
                        )
                    } else {
                        Json::Null
                    },
                ),
                (
                    "clock_sigma_prior",
                    if options.reference_model {
                        Json::text("Gamma(shape=5, scale=0.2)")
                    } else {
                        Json::Null
                    },
                ),
                (
                    "branch_rate_prior",
                    if options.reference_model {
                        Json::text(
                            "independent LogNormal(mu=-sigma^2/2, sigma), positive rates; BEAST clock initialization sets lower bound to zero",
                        )
                    } else {
                        Json::Null
                    },
                ),
                (
                    "rate_mapping",
                    if options.reference_model {
                        Json::text(
                            "node index; last node uses the root index when it is not the root; root has no branch rate",
                        )
                    } else {
                        Json::Null
                    },
                ),
                (
                    "clock",
                    Json::text(if options.reference_model {
                        "lognormal explicit branch rates; real-space distribution mean 1; no realized-mean normalization; fixed clock multiplier 1"
                    } else {
                        "strict, rate fixed at 1; heights in expected changes per character"
                    }),
                ),
                (
                    "tree_prior",
                    Json::text(
                        "birth-death (Gernhard 2008), unscaled, every lineage sampled, conditioned on the number of tips",
                    ),
                ),
                (
                    "birth_difference_rate_prior",
                    Json::text(format!("Uniform(0, {})", settings.birth_difference_upper)),
                ),
                ("relative_death_rate_prior", Json::text("Uniform(0, 1)")),
                ("sample_from_prior", Json::Bool(settings.sample_from_prior)),
                (
                    "monophyly",
                    Json::List(settings.monophyly.iter().map(|c| names(c)).collect()),
                ),
            ]),
        ),
        (
            "settings",
            Json::Object(vec![
                ("seed", Json::Integer(options.seed)),
                ("initial_gamma_shape", optional(settings.gamma_shape)),
                ("initial_clock_sigma", optional(settings.clock_sigma)),
                (
                    "initial_branch_rate",
                    if options.reference_model {
                        Json::Number(0.5)
                    } else {
                        Json::Null
                    },
                ),
                ("steps", Json::Integer(options.steps)),
                ("sample_every", Json::Integer(options.sample_every)),
                (
                    "initial_birth_difference_rate",
                    Json::Number(settings.initial.birth_difference),
                ),
                (
                    "initial_relative_death_rate",
                    Json::Number(settings.initial.relative_death),
                ),
                (
                    "initial_tree",
                    Json::text("random, with every monophyly constraint satisfied"),
                ),
                ("tuning", Json::Bool(settings.tuning)),
                ("tuning_schedule", Json::text(TUNING_SCHEDULE)),
                ("tuning_delay", Json::Integer(settings.tuning_delay)),
                ("tuning_target_acceptance", Json::Number(0.234)),
                ("backend", Json::text(settings.backend.name())),
            ]),
        ),
        ("moves", Json::List(moves)),
        (
            "final_state",
            Json::Object(vec![
                ("posterior", Json::Number(posterior.total())),
                ("likelihood", Json::Number(posterior.likelihood)),
                ("tree_prior", Json::Number(posterior.tree_prior)),
                ("parameter_prior", Json::Number(posterior.parameter_prior)),
                ("gamma_prior", Json::Number(posterior.gamma_prior)),
                ("clock_rate_prior", Json::Number(posterior.clock_rate_prior)),
                (
                    "clock_sigma_prior",
                    Json::Number(posterior.clock_sigma_prior),
                ),
                (
                    "gamma_shape",
                    optional(sampler.rates().gamma.as_ref().map(|g| g.shape())),
                ),
                (
                    "category_rates",
                    Json::List(
                        sampler
                            .rates()
                            .category_rates()
                            .iter()
                            .copied()
                            .map(Json::Number)
                            .collect(),
                    ),
                ),
                (
                    "category_weights",
                    Json::List(
                        sampler
                            .rates()
                            .category_weights()
                            .iter()
                            .copied()
                            .map(Json::Number)
                            .collect(),
                    ),
                ),
                (
                    "clock_sigma",
                    optional(sampler.rates().clock.as_ref().map(|c| c.sigma())),
                ),
                (
                    "branch_rate_parameters",
                    sampler.rates().clock.as_ref().map_or(Json::Null, |c| {
                        Json::List(c.rates().iter().copied().map(Json::Number).collect())
                    }),
                ),
                ("root_node", Json::Integer(sampler.tree().root() as u64)),
            ]),
        ),
        ("started_unix_seconds", Json::Integer(started)),
        ("wall_seconds", Json::Number(seconds)),
    ]);
    let mut text = String::new();
    metadata.write(&mut text, 0);
    text.push('\n');
    fs::write(options.output.join("run.json"), text)?;
    if options.events_json {
        events::state(
            status,
            completed_steps,
            options.steps,
            seconds,
            &input,
            &sampler,
        )?;
    }
    progress(format_args!(
        "{status}: {completed_steps} of {} steps in {seconds:.1} s; wrote {}",
        options.steps,
        options.output.display()
    ));
    Ok(completed)
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = if arguments.first().is_some_and(|a| a == "diagnose") {
        diagnose::run(&arguments[1..]).map(|_| true)
    } else if arguments.first().is_some_and(|a| a == "inspect-tree") {
        inspect_tree::run(&arguments[1..]).map(|_| true)
    } else if arguments.first().is_some_and(|a| a == "summarize") {
        summarize::run(&arguments[1..]).map(|_| true)
    } else if arguments.first().is_some_and(|a| a == "inspect") {
        inspect::run(&arguments[1..]).map(|_| true)
    } else {
        run(&arguments)
    };
    match result {
        Ok(true) => {}
        Ok(false) => std::process::exit(130),
        Err(error) => {
            if arguments.iter().any(|arg| arg == "--events-json") {
                let _ = events::emit(vec![
                    ("schema_version", Json::Integer(1)),
                    ("event", Json::text("failed")),
                    ("message", Json::text(error.to_string())),
                ]);
            }
            progress(format_args!("rove: {error}"));
            std::process::exit(2);
        }
    }
}
