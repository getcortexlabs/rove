//! `rove marginal`: power posteriors over a schedule of likelihood exponents,
//! then stepping-stone and path-sampling estimates of the marginal likelihood.

use std::error::Error;
use std::fs;
use std::io::BufWriter;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rove::marginal::{Analysis, Estimate, PowerSample, analyse, schedule};
use rove::mcmc::Sampler;
use rove::mcmc::output::TraceWriter;

use super::{
    Configured, Json, Options, TUNING_SCHEDULE, checkpoint, configure, events, input_json,
    model_json, moves_json, optional, progress, quoted,
};

const STEPPING_STONE: &str = "Xie, Lewis, Fan, Kuo and Chen (2011): sum over adjacent powers of log E[L^(upper - lower)] under the lower power posterior, with the largest log-likelihood factored out; standard error from the delta method with the ESS of each stone's weights, treating stones as independent";
const PATH_SAMPLING: &str = "Lartillot and Philippe (2006): trapezoidal rule over the mean log-likelihood at each power; standard error from the ESS of each mean; the discretization error of the trapezoid is not estimated";
const SCHEDULE: &str =
    "quantiles of Beta(alpha, 1) at i / (powers - 1), as in BEAST 2's PathSampler";
const ORDER: &str = "descending powers; each power continues from the previous power's final state and keeps its operator tuning";

pub(super) fn run(
    arguments: &[String],
    mut options: Options,
    cancellation: &events::Cancellation,
) -> Result<bool, Box<dyn Error>> {
    let powers = schedule(options.powers, options.alpha)?;
    let Configured {
        input,
        data,
        patterns,
        settings,
    } = configure(&mut options)?;
    let taxa = data.taxa();
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
    let executable = options.executable_sha256.clone();
    let comments = vec![
        format!("rove {}", env!("CARGO_PKG_VERSION")),
        format!("command {command_json}"),
        format!("input sha256 {input}"),
        format!(
            "executable sha256 {}",
            executable.as_deref().unwrap_or("unknown")
        ),
        "posterior, likelihood and prior are untempered; acceptance used likelihood^power"
            .to_string(),
    ];
    let started = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let clock = Instant::now();
    let print_every = options.print_every.unwrap_or((options.steps / 20).max(1));
    progress("power\tstep\tposterior\tlikelihood\tprior\tseconds");
    let mut samples = Vec::with_capacity(powers.len());
    let mut completed_steps = 0;
    let mut cancelled = false;
    for (index, &power) in powers.iter().enumerate().rev() {
        sampler.set_likelihood_power(power)?;
        let directory = options.output.join(format!("power-{index:02}"));
        fs::create_dir(&directory)?;
        let mut comments = comments.clone();
        comments.push(format!(
            "likelihood power {power}; index {index} of {}",
            powers.len()
        ));
        let file = fs::File::create(directory.join("trace.log"))?;
        let mut trace = TraceWriter::with_sampler(BufWriter::new(file), &comments, &sampler)?;
        trace.write(0, &sampler)?;
        let mut log_likelihoods = vec![sampler.posterior().likelihood];
        let mut step = 0;
        while step < options.steps {
            if step % 1024 == 0 && cancellation.requested() {
                cancelled = true;
                break;
            }
            step += 1;
            sampler.step();
            if step % options.sample_every == 0 {
                trace.write(step, &sampler)?;
                log_likelihoods.push(sampler.posterior().likelihood);
            }
            if step % print_every == 0 {
                let posterior = sampler.posterior();
                progress(format_args!(
                    "{power:.6}\t{step}\t{:.4}\t{:.4}\t{:.4}\t{:.1}",
                    posterior.total(),
                    posterior.likelihood,
                    posterior.prior(),
                    clock.elapsed().as_secs_f64()
                ));
            }
        }
        trace.finish()?;
        completed_steps += step;
        if cancelled {
            break;
        }
        let discard = log_likelihoods.len() * options.burnin / 100;
        samples.push(PowerSample {
            power,
            log_likelihoods: log_likelihoods.split_off(discard),
        });
    }
    samples.reverse();
    let analysis = if cancelled {
        None
    } else {
        Some(analyse(&samples)?)
    };
    let seconds = clock.elapsed().as_secs_f64();
    let status = if cancelled { "cancelled" } else { "completed" };
    let schedule_json = || {
        Json::Object(vec![
            ("seed", Json::Integer(options.seed)),
            ("steps_per_power", Json::Integer(options.steps)),
            ("sample_every", Json::Integer(options.sample_every)),
            ("burnin_percent", Json::Integer(options.burnin as u64)),
            ("powers", Json::Integer(options.powers as u64)),
            ("alpha", Json::Number(options.alpha)),
            (
                "schedule",
                Json::List(powers.iter().copied().map(Json::Number).collect()),
            ),
            ("schedule_method", Json::text(SCHEDULE)),
            ("order", Json::text(ORDER)),
        ])
    };
    let record = Json::Object(vec![
        ("status", Json::text(status)),
        ("kind", Json::text("marginal_likelihood_run")),
        ("completed_steps", Json::Integer(completed_steps)),
        ("rove_version", Json::text(env!("CARGO_PKG_VERSION"))),
        (
            "executable_sha256",
            executable.clone().map_or(Json::Null, Json::Text),
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
        ("input", input_json(&options, &input, &data, &patterns)),
        ("model", model_json(&options, &settings, &patterns, taxa)),
        ("settings", {
            let Json::Object(mut fields) = schedule_json() else {
                unreachable!()
            };
            fields.extend([
                ("tuning", Json::Bool(settings.tuning)),
                ("tuning_schedule", Json::text(TUNING_SCHEDULE)),
                ("tuning_delay", Json::Integer(settings.tuning_delay)),
                ("tuning_target_acceptance", Json::Number(0.234)),
                ("backend", Json::text(settings.backend.name())),
            ]);
            Json::Object(fields)
        }),
        ("moves", moves_json(&sampler)),
        ("started_unix_seconds", Json::Integer(started)),
        ("wall_seconds", Json::Number(seconds)),
    ]);
    write(&options.output.join("run.json"), &record)?;
    let Some(analysis) = analysis else {
        progress(format_args!(
            "cancelled after {completed_steps} steps in {seconds:.1} s; wrote {}",
            options.output.display()
        ));
        return Ok(false);
    };
    let results = Json::Object(vec![
        ("kind", Json::text("marginal_likelihood")),
        ("stepping_stone", estimate_json(&analysis.stepping_stone)),
        ("path_sampling", estimate_json(&analysis.path_sampling)),
        ("powers", powers_json(&analysis)),
        ("stones", stones_json(&analysis)),
        (
            "methods",
            Json::Object(vec![
                ("stepping_stone", Json::text(STEPPING_STONE)),
                ("path_sampling", Json::text(PATH_SAMPLING)),
                ("schedule", Json::text(SCHEDULE)),
                (
                    "burnin",
                    Json::text(
                        "the first burnin_percent of each power's samples by count, including the sample before the first step",
                    ),
                ),
            ]),
        ),
        ("settings", schedule_json()),
        ("input_sha256", Json::text(&input)),
        (
            "executable_sha256",
            executable.map_or(Json::Null, Json::Text),
        ),
    ]);
    write(&options.output.join("marginal.json"), &results)?;
    let describe = |estimate: &Estimate| {
        format!(
            "{:.4} (standard error {})",
            estimate.log_marginal_likelihood,
            estimate
                .standard_error
                .map_or("unavailable".to_string(), |e| format!("{e:.4}"))
        )
    };
    progress(format_args!(
        "completed: log marginal likelihood by stepping stone {}; by path sampling {}; {} powers in {seconds:.1} s; wrote {}",
        describe(&analysis.stepping_stone),
        describe(&analysis.path_sampling),
        powers.len(),
        options.output.display()
    ));
    Ok(true)
}

fn write(path: &std::path::Path, value: &Json) -> std::io::Result<()> {
    let mut text = String::new();
    value.write(&mut text, 0);
    text.push('\n');
    checkpoint::atomic(path, text.as_bytes())
}

fn estimate_json(estimate: &Estimate) -> Json {
    Json::Object(vec![
        (
            "log_marginal_likelihood",
            Json::Number(estimate.log_marginal_likelihood),
        ),
        ("standard_error", optional(estimate.standard_error)),
    ])
}

fn powers_json(analysis: &Analysis) -> Json {
    Json::List(
        analysis
            .powers
            .iter()
            .enumerate()
            .map(|(index, summary)| {
                Json::Object(vec![
                    ("index", Json::Integer(index as u64)),
                    ("power", Json::Number(summary.power)),
                    ("samples", Json::Integer(summary.samples as u64)),
                    ("mean_log_likelihood", Json::Number(summary.mean)),
                    ("sd_log_likelihood", Json::Number(summary.sd)),
                    ("ess", optional(summary.ess)),
                    ("lag_limit_reached", Json::Bool(summary.lag_limit_reached)),
                ])
            })
            .collect(),
    )
}

fn stones_json(analysis: &Analysis) -> Json {
    Json::List(
        analysis
            .stones
            .iter()
            .map(|stone| {
                Json::Object(vec![
                    ("lower", Json::Number(stone.lower)),
                    ("upper", Json::Number(stone.upper)),
                    ("log_ratio", Json::Number(stone.log_ratio)),
                    ("variance", optional(stone.variance)),
                    ("ess", optional(stone.ess)),
                ])
            })
            .collect(),
    )
}
