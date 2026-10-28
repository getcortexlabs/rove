#[path = "support/allocations.rs"]
mod allocations;
#[path = "support/timing.rs"]
mod timing;
#[path = "support/workload.rs"]
mod workload;

use std::error::Error;
use std::hint::black_box;
use std::time::Duration;

use rove::{Backend, data::GapTreatment, nexus};
use workload::{Engine, PATHS, Path, Workload, backends};

#[global_allocator]
static ALLOCATOR: allocations::CountingAllocator = allocations::CountingAllocator;

#[derive(Clone, Copy)]
enum Metric {
    Parse,
    Compress,
    CoreSetup,
    FullCached,
    FullBranches,
    TipUpdate,
    InternalUpdate,
    Accept,
    Reject,
    Store,
    StoreRestore,
}

impl Metric {
    fn name(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Compress => "compress",
            Self::CoreSetup => "core_setup",
            Self::FullCached => "full_cached",
            Self::FullBranches => "full_with_branches",
            Self::TipUpdate => "tip_update",
            Self::InternalUpdate => "internal_update",
            Self::Accept => "accept_proposal",
            Self::Reject => "reject_proposal",
            Self::Store => "store",
            Self::StoreRestore => "store_restore",
        }
    }

    fn dirty_nodes(self, case: &Workload) -> usize {
        match self {
            Self::FullCached | Self::FullBranches => case.data.taxa().len() - 1,
            Self::TipUpdate | Self::Accept | Self::Reject => case.tree.depth(case.tip_branch),
            Self::InternalUpdate => case.tree.depth(case.internal_branch),
            _ => 0,
        }
    }

    fn run(self, case: &Workload, path: Path, backend: Backend, engine: &mut Engine<'_>) -> f64 {
        match self {
            Self::Parse => {
                black_box(nexus::parse(black_box(&case.input)).unwrap()).character_count() as f64
            }
            Self::Compress => {
                let sites = 0..case.data.character_count();
                let patterns = black_box(
                    case.data
                        .compress(std::slice::from_ref(&sites), GapTreatment::Reject)
                        .unwrap(),
                );
                patterns[0].weights().len() as f64
            }
            Self::CoreSetup => black_box(case.engine(path, backend)).core.node_count() as f64,
            Self::FullCached => engine.full(),
            Self::FullBranches => engine.full_with_branches(),
            Self::TipUpdate => engine.partial(case.tip_branch),
            Self::InternalUpdate => engine.partial(case.internal_branch),
            Self::Accept => engine.proposal(case.tip_branch, false),
            Self::Reject => engine.proposal(case.tip_branch, true),
            Self::Store => {
                engine.core.store();
                0.0
            }
            Self::StoreRestore => {
                engine.core.store();
                engine.core.restore();
                0.0
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut settings = timing::Settings {
        samples: 15,
        sample_time: Duration::from_millis(20),
        warmup: Duration::from_millis(100),
    };
    let mut check = false;
    let mut reverse = false;
    let mut filter = None;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--bench" => {}
            "--check" => check = true,
            "--reverse" => reverse = true,
            "--case" => filter = Some(args.next().ok_or("--case requires a name")?),
            "--samples" => {
                settings.samples = args.next().ok_or("--samples requires a count")?.parse()?
            }
            "--sample-ms" => {
                settings.sample_time = Duration::from_millis(
                    args.next()
                        .ok_or("--sample-ms requires a duration")?
                        .parse()?,
                )
            }
            "--warmup-ms" => {
                settings.warmup = Duration::from_millis(
                    args.next()
                        .ok_or("--warmup-ms requires a duration")?
                        .parse()?,
                )
            }
            _ => return Err(format!("unknown argument {argument}").into()),
        }
    }
    if settings.samples < 3 || settings.sample_time.is_zero() {
        return Err("use at least three samples and a positive sample duration".into());
    }
    let cases: Vec<_> = workload::suite()
        .into_iter()
        .filter(|case| filter.as_ref().is_none_or(|name| case.name == name))
        .collect();
    if cases.is_empty() {
        return Err("no matching benchmark case".into());
    }
    let variants: Vec<(Path, Backend)> = PATHS
        .into_iter()
        .flat_map(|path| backends().into_iter().map(move |backend| (path, backend)))
        .collect();
    let baselines: Vec<Vec<f64>> = cases
        .iter()
        .map(|case| {
            variants
                .iter()
                .map(|&(path, backend)| case.validate(path, backend))
                .collect()
        })
        .collect();
    for (case, values) in cases.iter().zip(&baselines) {
        for path in PATHS {
            let mut bits = variants
                .iter()
                .zip(values)
                .filter(|((p, _), _)| *p == path)
                .map(|(_, value)| value.to_bits());
            let first = bits.next();
            if !bits.all(|value| Some(value) == first) {
                return Err(
                    format!("{}: the backends disagree on {}", case.name, path.name()).into(),
                );
            }
        }
    }
    if check {
        eprintln!(
            "Validated {} benchmark cases on both transition paths and {} backends, with their update/restore paths.",
            cases.len(),
            backends().len()
        );
        return Ok(());
    }
    println!(
        "case\ttransitions\tbackend\tmetric\ttaxa\tcharacters\tpatterns\tcore_patterns\tstride\tstates\tcategories\tshape\tmissing_cells\tdirty_nodes\tbaseline_log_likelihood\tallocation_calls\treallocation_calls\trequested_bytes\tdeallocation_calls\tsample\titerations\telapsed_ns"
    );
    for (case, baselines) in cases.iter().zip(baselines) {
        for (index, metric) in [
            Metric::Parse,
            Metric::Compress,
            Metric::CoreSetup,
            Metric::FullCached,
            Metric::FullBranches,
            Metric::TipUpdate,
            Metric::InternalUpdate,
            Metric::Accept,
            Metric::Reject,
            Metric::Store,
            Metric::StoreRestore,
        ]
        .into_iter()
        .enumerate()
        {
            // Every variant back to back, so they are measured close in time.
            // The order reverses between metrics, and with `--reverse`.
            let mut order: Vec<_> = variants
                .iter()
                .copied()
                .zip(baselines.iter().copied())
                .collect();
            if (index % 2 == 1) != reverse {
                order.reverse();
            }
            for ((path, backend), baseline) in order {
                let mut engine = case.engine(path, backend);
                engine.full();
                let stride = engine.core.layout().stride;
                let measured = timing::measure(&settings, || {
                    metric.run(black_box(case), path, backend, black_box(&mut engine))
                });
                for (sample, elapsed) in measured.elapsed.iter().enumerate() {
                    println!(
                        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.17}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        case.name,
                        path.name(),
                        backend.name(),
                        metric.name(),
                        case.data.taxa().len(),
                        case.data.character_count(),
                        case.weights.len(),
                        case.core_patterns(),
                        stride,
                        case.model.states(),
                        case.rates.len(),
                        case.shape,
                        case.missing_cells(),
                        metric.dirty_nodes(case),
                        baseline,
                        measured.counts.allocations,
                        measured.counts.reallocations,
                        measured.counts.bytes,
                        measured.counts.deallocations,
                        sample,
                        measured.iterations,
                        elapsed.as_nanos()
                    );
                }
            }
        }
        eprintln!("Measured {} ({} patterns).", case.name, case.weights.len());
    }
    Ok(())
}
