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

pub(super) fn state(
    event: &str,
    step: u64,
    total_steps: u64,
    seconds: f64,
    input: &str,
    sampler: &Sampler,
) -> io::Result<()> {
    let posterior = sampler.posterior();
    emit(vec![
        ("schema_version", Json::Integer(1)),
        ("event", Json::text(event)),
        ("step", Json::Integer(step)),
        ("total_steps", Json::Integer(total_steps)),
        ("elapsed_seconds", Json::Number(seconds)),
        ("input_sha256", Json::text(input)),
        ("posterior", Json::Number(posterior.total())),
        ("likelihood", Json::Number(posterior.likelihood)),
        ("prior", Json::Number(posterior.prior())),
    ])
}

pub(super) struct Cancellation(Option<Arc<AtomicBool>>);

impl Cancellation {
    pub(super) fn watch_stdin(enabled: bool) -> Self {
        if !enabled {
            return Self(None);
        }
        let requested = Arc::new(AtomicBool::new(false));
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
        Self(Some(requested))
    }

    pub(super) fn requested(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }
}
