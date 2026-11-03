//! Self-contained morphology/DNA sources with explicit taxon matching.
use crate::data::{Cell, DataType, Matrix};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub data_type: DataType,
    pub start: usize,
    pub characters: usize,
    pub symbols: Vec<char>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub name: String,
    pub sha256: String,
    pub data_type: DataType,
    pub missing_taxa: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub missing_taxa_policy: String,
    pub sources: Vec<Source>,
}

impl Metadata {
    pub fn json(&self) -> Value {
        json!({"missing_taxa_policy": self.missing_taxa_policy,
            "taxon_matching": "exact normalized names; source order does not determine correspondence",
            "sources": self.sources.iter().map(|s| json!({"name":s.name,"sha256":s.sha256,
                "data_type":s.data_type.name(),"missing_taxa":s.missing_taxa})).collect::<Vec<_>>()})
    }
}

fn fields(value: &Value, keys: &[&str]) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or("mixed manifest must contain objects")?;
    if object.len() != keys.len() || keys.iter().any(|k| !object.contains_key(*k)) {
        return Err(format!(
            "mixed manifest requires exactly these fields: {}",
            keys.join(", ")
        ));
    }
    Ok(())
}

/// Every source is embedded verbatim. No filesystem paths are followed on import.
pub fn manifest(
    morphology: (&str, &str),
    dna: (&str, &str),
    missing: &str,
) -> Result<String, String> {
    let source = |(name, text): (&str, &str), data_type: &str| {
        json!({
        "name": name, "data_type":data_type, "sha256":crate::fingerprint::hex(text.as_bytes()), "content":text})
    };
    let value = json!({"format":"rove-mixed", "version":1,"missing_taxa":missing,
        "sources":[source(morphology,"standard"),source(dna,"dna")]});
    let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())? + "\n";
    parse(&text)?;
    Ok(text)
}

pub fn parse(input: &str) -> Result<Matrix, String> {
    let value: Value =
        serde_json::from_str(input).map_err(|e| format!("invalid mixed manifest: {e}"))?;
    fields(&value, &["format", "version", "missing_taxa", "sources"])?;
    if value["format"] != "rove-mixed" || value["version"] != 1 {
        return Err("unsupported mixed manifest format/version".into());
    }
    let policy = value["missing_taxa"]
        .as_str()
        .ok_or("missing taxon policy")?;
    if !matches!(policy, "reject" | "missing") {
        return Err("missing_taxa must be reject or missing".into());
    }
    let sources = value["sources"]
        .as_array()
        .filter(|v| v.len() == 2)
        .ok_or("mixed input needs exactly one morphology source followed by one DNA source")?;
    let mut matrices = Vec::new();
    let mut metadata = Metadata {
        missing_taxa_policy: policy.into(),
        sources: Vec::new(),
    };
    for (source, expected) in sources.iter().zip([DataType::Standard, DataType::Dna]) {
        fields(source, &["name", "data_type", "sha256", "content"])?;
        let text = source["content"].as_str().ok_or("missing source text")?;
        // Reject nested manifests before dispatching to the input parser.
        if text
            .trim_start_matches('\u{feff}')
            .trim_start()
            .starts_with('{')
        {
            return Err("nested mixed manifests are unsupported".into());
        }
        let hash = crate::fingerprint::hex(text.as_bytes());
        if source["sha256"] != hash {
            return Err("embedded source fingerprint does not match its bytes".into());
        }
        let name = source["name"]
            .as_str()
            .filter(|s| !s.trim().is_empty() && !s.contains(['/', '\\', '\n', '\r']))
            .ok_or("source name must be a nonempty filename")?;
        let matrix = crate::alignment::parse(text).map_err(|e| format!("{name}: {e}"))?;
        if matrix.data_type() != expected || source["data_type"] != expected.name() {
            return Err(format!("{name}: expected {} data", expected.name()));
        }
        metadata.sources.push(Source {
            name: name.into(),
            sha256: hash,
            data_type: expected,
            missing_taxa: Vec::new(),
        });
        matrices.push(matrix);
    }
    let mut taxa = matrices[0].taxa.clone();
    let shared = matrices[1].taxa.iter().filter(|t| taxa.contains(t)).count();
    if shared < 2 {
        return Err("mixed sources must share at least two exactly matching taxon names".into());
    }
    for taxon in &matrices[1].taxa {
        if !taxa.contains(taxon) {
            if taxa.iter().any(|t| t.eq_ignore_ascii_case(taxon)) {
                return Err(format!(
                    "taxon spelling differs between sources: {taxon:?}; match normalized names exactly"
                ));
            }
            taxa.push(taxon.clone());
        }
    }
    for (matrix, source) in matrices.iter_mut().zip(&mut metadata.sources) {
        source.missing_taxa = taxa
            .iter()
            .filter(|t| !matrix.taxa.contains(t))
            .cloned()
            .collect();
        if policy == "reject" && !source.missing_taxa.is_empty() {
            return Err(format!(
                "{} lacks taxa: {}; choose missing explicitly to retain them",
                source.name,
                source.missing_taxa.join(", ")
            ));
        }
        matrix.rows = taxa
            .iter()
            .map(|t| {
                matrix.taxa.iter().position(|name| name == t).map_or_else(
                    || vec![Cell::Missing; matrix.character_count()],
                    |i| matrix.rows[i].clone(),
                )
            })
            .collect();
        matrix.taxa = taxa.clone();
    }
    let mut result = Matrix {
        data_type: DataType::Mixed,
        blocks: Vec::new(),
        mixed: Some(metadata),
        rows: vec![Vec::new(); taxa.len()],
        taxa,
        symbols: Vec::new(),
        character_labels: Vec::new(),
        state_labels: Vec::new(),
        ordering: Vec::new(),
    };
    for matrix in matrices {
        let start = result.character_count();
        let mut map = Vec::new();
        for symbol in &matrix.symbols {
            let index = result
                .symbols
                .iter()
                .position(|s| s == symbol)
                .unwrap_or_else(|| {
                    result.symbols.push(*symbol);
                    result.symbols.len() - 1
                });
            map.push(index);
        }
        let recode = |cell: Cell| match cell {
            Cell::State(s) => Cell::State(map[s]),
            Cell::Uncertain(v) => Cell::Uncertain(v.into_iter().map(|s| map[s]).collect()),
            Cell::Polymorphic(v) => Cell::Polymorphic(v.into_iter().map(|s| map[s]).collect()),
            other => other,
        };
        result.blocks.push(Block {
            data_type: matrix.data_type,
            start,
            characters: matrix.character_count(),
            symbols: matrix.symbols,
        });
        result.character_labels.extend(matrix.character_labels);
        result.state_labels.extend(matrix.state_labels);
        result.ordering.extend(matrix.ordering);
        for (out, row) in result.rows.iter_mut().zip(matrix.rows) {
            out.extend(row.into_iter().map(recode));
        }
    }
    Ok(result)
}
