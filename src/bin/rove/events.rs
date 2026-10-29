use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rove::mcmc::Sampler;

use super::Json;

pub(super) fn emit(fields: Vec<(&'static str, Json)>) -> io::Result<()> {
    let mut text = String::new();
    Json::Object(fields).write_compact(&mut text);
    let mut out = io::stdout().lock();
    writeln!(out, "{text}")?;
    out.flush()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn state(
    event: &str,
    step: u64,
    total_steps: u64,
    seconds: f64,
    input: &str,
    sampler: &Sampler,
    chain: Option<usize>,
    termination: Option<&str>,
) -> io::Result<()> {
    let posterior = sampler.posterior();
    let mut fields = vec![
        ("schema_version", Json::Integer(1)),
        ("event", Json::text(event)),
        ("step", Json::Integer(step)),
        ("total_steps", Json::Integer(total_steps)),
        ("elapsed_seconds", Json::Number(seconds)),
        ("input_sha256", Json::text(input)),
        ("posterior", Json::Number(posterior.total())),
        ("likelihood", Json::Number(posterior.likelihood)),
        ("prior", Json::Number(posterior.prior())),
    ];
    if let Some(chain) = chain {
        fields.push(("chain", Json::Integer(chain as u64)));
    }
    if let Some(reason) = termination {
        fields.push(("termination_reason", Json::text(reason)));
    }
    emit(fields)
}

#[derive(Clone)]
pub(super) struct Cancellation(Arc<AtomicBool>);

impl Cancellation {
    pub(super) fn watch_stdin(enabled: bool) -> Self {
        let requested = Arc::new(AtomicBool::new(false));
        if !enabled {
            return Self(requested);
        }
        let flag = requested.clone();
        // EOF also arrives when the GUI exits unexpectedly, so its child cannot
        // continue sampling after losing its owner. No signal handler is needed.
        std::thread::spawn(move || {
            let mut input = io::stdin().lock();
            let mut buffer = [0; 64];
            loop {
                match input.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            flag.store(true, Ordering::Relaxed);
        });
        Self(requested)
    }

    pub(super) fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub(super) fn requested(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
