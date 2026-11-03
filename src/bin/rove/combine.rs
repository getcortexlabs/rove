use std::{collections::HashMap, error::Error, fs, io::Write, path::Path};

const USAGE: &str = "usage: rove combine --morphology <matrix.nex> --dna <alignment> --output <new.mixed.json> [--missing-taxa reject|missing]";

pub(super) fn run(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let mut options = HashMap::new();
    let mut args = arguments.iter();
    while let Some(key) = args.next() {
        if !matches!(
            key.as_str(),
            "--morphology" | "--dna" | "--output" | "--missing-taxa"
        ) {
            return Err(USAGE.into());
        }
        let value = args.next().ok_or(USAGE)?;
        if options.insert(key.as_str(), value.as_str()).is_some() {
            return Err("duplicate combine option".into());
        }
    }
    let morphology = Path::new(options.get("--morphology").ok_or(USAGE)?);
    let dna = Path::new(options.get("--dna").ok_or(USAGE)?);
    let output = Path::new(options.get("--output").ok_or(USAGE)?);
    let morph_text = fs::read_to_string(morphology)?;
    let dna_text = fs::read_to_string(dna)?;
    let filename = |path: &Path| {
        path.file_name()
            .and_then(|s| s.to_str())
            .map(String::from)
            .ok_or("source needs a UTF-8 filename")
    };
    let text = rove::mixed::manifest(
        (&filename(morphology)?, &morph_text),
        (&filename(dna)?, &dna_text),
        options.get("--missing-taxa").copied().unwrap_or("reject"),
    )?;
    // Validate completely before creating a new output; never overwrite a source.
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(())
}
