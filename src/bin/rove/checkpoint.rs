//! Atomic restart manifests and append-only, fingerprinted sampling outputs.

use serde_json::{Value, json};
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use super::{Options, parse_options, sha256};

pub(super) struct Lock {
    _file: File,
}
impl Lock {
    pub fn acquire(directory: &Path) -> Result<Self, Box<dyn Error>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join(".run.lock"))?;
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            // The descriptor is owned by `file` and remains open for this guard's
            // lifetime. The kernel releases flock even after a process crash.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err("this run is already open in another process".into());
            }
        }
        #[cfg(not(unix))]
        return Err("checkpoint locking is not supported on this platform".into());
        Ok(Self { _file: file })
    }
}

pub(super) fn atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = path.with_extension("pending");
    let mut file = File::create(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temp, path)?;
    File::open(path.parent().unwrap())?.sync_all()
}

pub(super) fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    atomic(path, &serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

pub(super) struct Output {
    buffered: BufWriter<TrackedFile>,
}

struct TrackedFile {
    file: File,
    digest: sha256::Digest,
    bytes: u64,
}

impl Write for TrackedFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let n = self.file.write(bytes)?;
        self.digest.update(&bytes[..n]);
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Output {
    pub fn create(path: &Path) -> io::Result<Self> {
        Ok(Self {
            buffered: BufWriter::new(TrackedFile {
                file: File::create(path)?,
                digest: sha256::Digest::new(),
                bytes: 0,
            }),
        })
    }
    pub fn resume(path: &Path, record: &Value) -> Result<Self, Box<dyn Error>> {
        let bytes = number(record, "bytes")?;
        let digest = verify_prefix(path, record)?;
        let mut file = OpenOptions::new().write(true).open(path)?;
        file.set_len(bytes)?;
        file.seek(SeekFrom::Start(bytes))?;
        Ok(Self {
            buffered: BufWriter::new(TrackedFile {
                file,
                digest,
                bytes,
            }),
        })
    }
    pub fn record(&mut self) -> Result<Value, Box<dyn Error>> {
        self.buffered.flush()?;
        let out = self.buffered.get_ref();
        out.file.sync_all()?;
        Ok(json!({"bytes": out.bytes, "sha256": out.digest.clone().finish()}))
    }
}
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.buffered.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.buffered.flush()
    }
}

fn verify_prefix(path: &Path, record: &Value) -> Result<sha256::Digest, Box<dyn Error>> {
    let length = number(record, "bytes")?;
    let file = File::open(path)?;
    if file.metadata()?.len() < length {
        return Err(format!("{} is shorter than its checkpoint", path.display()).into());
    }
    let mut input = file.take(length);
    let mut digest = sha256::Digest::new();
    let mut buffer = [0; 65536];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    if digest.clone().finish() != string(record, "sha256")? {
        return Err(format!("{} changed before the checkpoint", path.display()).into());
    }
    Ok(digest)
}

pub(super) fn number(record: &Value, key: &str) -> Result<u64, Box<dyn Error>> {
    record[key]
        .as_u64()
        .ok_or_else(|| format!("checkpoint has no valid {key}").into())
}
fn string<'a>(record: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    record[key]
        .as_str()
        .ok_or_else(|| format!("checkpoint has no valid {key}").into())
}

pub(super) fn save_chain(
    directory: &Path,
    step: u64,
    state: &[u8],
    trace: &mut Output,
    trees: &mut Output,
    started: u64,
    elapsed: f64,
) -> Result<Value, Box<dyn Error>> {
    let trace = trace.record()?;
    let trees = trees.record()?;
    let name = format!("checkpoint-{step}.bin");
    atomic(&directory.join(&name), state)?;
    Ok(
        json!({"step": step, "file": name, "sha256": sha256::hex(state),
        "trace": trace, "trees": trees, "started_unix_seconds": started, "elapsed_seconds": elapsed}),
    )
}

pub(super) fn commit(
    options: &Options,
    arguments: &[String],
    chains: Vec<Value>,
    checks: &[Value],
) -> Result<(), Box<dyn Error>> {
    let path = options.output.join("checkpoint.json");
    let previous: Option<Value> = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    let mut command = arguments.to_vec();
    // Paths must still resolve after the app or CLI is launched elsewhere.
    let mut i = 1;
    while i < command.len() {
        if command[i] == "--output" {
            command[i + 1] = fs::canonicalize(&options.output)?.display().to_string();
            i += 2;
        } else if command[i] == options.matrix.to_string_lossy() {
            command[i] = fs::canonicalize(&options.matrix)?.display().to_string();
            i += 1;
        } else {
            i += 1;
        }
    }
    let settings_sha256 = sha256::hex(&serde_json::to_vec(&command)?);
    let mut value = json!({"schema_version": 1, "arguments": command, "settings_sha256": settings_sha256,
        "input_sha256": options.expected_input, "executable_sha256": options.executable_sha256,
        "chains": chains, "checks": checks});
    value["manifest_sha256"] = json!(sha256::hex(&serde_json::to_vec(&value)?));
    write_json(&path, &value)?;
    // Delete only files referenced by the replaced manifest, after the new one
    // and its directory entry have reached disk. Uncommitted files are harmless.
    if let Some(previous) = previous {
        if let Some(old) = previous["chains"].as_array() {
            for (i, record) in old.iter().enumerate() {
                if record["file"] != value["chains"][i]["file"] {
                    if let Some(name) = record["file"].as_str().filter(|s| valid_name(s)) {
                        let _ = fs::remove_file(
                            chain_dir(&options.output, options.chains, i).join(name),
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

fn valid_name(name: &str) -> bool {
    name.starts_with("checkpoint-") && name.ends_with(".bin") && !name.contains(['/', '\\'])
}
fn chain_dir(root: &Path, count: usize, i: usize) -> PathBuf {
    if count == 1 {
        root.to_path_buf()
    } else {
        root.join(format!("chain-{:02}", i + 1))
    }
}

#[derive(Clone)]
pub(super) struct Resume {
    pub manifest: Value,
    pub states: Vec<Vec<u8>>,
}

pub(super) fn load(arguments: &[String]) -> Result<(Vec<String>, Options, Lock), Box<dyn Error>> {
    if arguments.get(1).map(String::as_str) != Some("--resume") {
        return Err("use rove sample --resume <run-directory>".into());
    }
    let directory = PathBuf::from(arguments.get(2).ok_or("--resume needs a run directory")?);
    let guard = Lock::acquire(&directory)?;
    let path = directory.join("checkpoint.json");
    if fs::metadata(&path)?.len() > 4 * 1024 * 1024 {
        return Err("checkpoint manifest is too large".into());
    }
    let manifest: Value = serde_json::from_slice(&fs::read(path)?)?;
    if manifest["schema_version"] != 1 {
        return Err("unsupported checkpoint version".into());
    }
    if sha256::file(&std::env::current_exe()?)? != string(&manifest, "executable_sha256")? {
        return Err(
            "checkpoint was written by a different executable; use the original Rove build".into(),
        );
    }
    let command: Vec<String> = serde_json::from_value(manifest["arguments"].clone())?;
    if sha256::hex(&serde_json::to_vec(&command)?) != string(&manifest, "settings_sha256")? {
        return Err("checkpoint settings fingerprint changed".into());
    }
    let mut content = manifest.clone();
    content
        .as_object_mut()
        .ok_or("invalid checkpoint manifest")?
        .remove("manifest_sha256");
    if sha256::hex(&serde_json::to_vec(&content)?) != string(&manifest, "manifest_sha256")? {
        return Err("checkpoint manifest fingerprint changed".into());
    }
    let mut options = parse_options(&command)?;
    if fs::canonicalize(&options.output)? != fs::canonicalize(&directory)? {
        return Err("resume from the original run directory".into());
    }
    if sha256::file(&options.matrix)? != string(&manifest, "input_sha256")? {
        return Err("the matrix changed since the checkpoint".into());
    }
    if let Ok(bytes) = fs::read(directory.join("run.json")) {
        if serde_json::from_slice::<Value>(&bytes)
            .ok()
            .is_some_and(|v| v["status"] == "completed")
        {
            return Err("this run is already completed".into());
        }
    }
    options.events_json = false;
    options.cancel_on_stdin_close = false;
    let mut extra = arguments[3..].iter();
    while let Some(flag) = extra.next() {
        match flag.as_str() {
            "--events-json" => options.events_json = true,
            "--cancel-on-stdin-close" => options.cancel_on_stdin_close = true,
            "--expect-input-sha256" => {
                if extra.next().map(String::as_str) != manifest["input_sha256"].as_str() {
                    return Err("the selected input differs from the checkpoint".into());
                }
            }
            _ => {
                return Err(format!(
                    "cannot change {flag} when resuming; the saved settings must be kept"
                )
                .into());
            }
        }
    }
    let chains = manifest["chains"]
        .as_array()
        .ok_or("checkpoint has no chains")?;
    if chains.len() != options.chains {
        return Err("checkpoint chain count differs".into());
    }
    let mut states = Vec::new();
    let mut step = None;
    for (i, record) in chains.iter().enumerate() {
        let current = number(record, "step")?;
        if current > options.steps
            || current % options.sample_every != 0
            || step.is_some_and(|s| s != current)
        {
            return Err("checkpoint chains have inconsistent steps".into());
        }
        step = Some(current);
        let dir = chain_dir(&directory, chains.len(), i);
        let name = string(record, "file")?;
        if !valid_name(name) {
            return Err("invalid checkpoint state file name".into());
        }
        let path = dir.join(name);
        if fs::metadata(&path)?.len() > 1024 * 1024 * 1024 {
            return Err("checkpoint state is too large".into());
        }
        let state = fs::read(path)?;
        if sha256::hex(&state) != string(record, "sha256")? {
            return Err("checkpoint state fingerprint changed".into());
        }
        verify_prefix(&dir.join("trace.log"), &record["trace"])?;
        verify_prefix(&dir.join("trees.nexus"), &record["trees"])?;
        states.push(state);
    }
    options.resume = Some(std::sync::Arc::new(Resume { manifest, states }));
    Ok((command, options, guard))
}
