use std::error::Error;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

use rove::summary::nexus::Reader;
use serde_json::{Value, json};

use super::{sha256, summarize};

fn count(record: &Value, field: &str) -> Result<u64, Box<dyn Error>> {
    record[field]
        .as_u64()
        .ok_or_else(|| format!("invalid summary {field}").into())
}

pub fn inspect(directory: &Path) -> Result<Value, Box<dyn Error>> {
    let record = summarize::record(&directory.join("summary.json"))?;
    if record["schema_version"] != 1
        || record["status"] != "completed"
        || record["method"] != "MCC"
        || record["convergence_assessed"] != false
        || record["annotated_tree"] != "mcc.tree"
        || record["raw_trees"] != "samples.trees"
        || !matches!(record["height_policy"].as_str(), Some("ca" | "keep"))
    {
        return Err("unsupported or incomplete posterior summary record".into());
    }
    let total = count(&record, "total_samples")?;
    let discarded = count(&record, "discarded_samples")?;
    let retained = count(&record, "retained_samples")?;
    if retained == 0 || discarded >= total || retained != total - discarded {
        return Err("inconsistent summary sample counts".into());
    }
    let burnin = &record["burnin"];
    let amount = count(burnin, "value")?;
    let expected = match burnin["unit"].as_str() {
        Some("percent") if amount < 100 => (total as u128 * amount as u128 / 100) as u64,
        Some("samples") => amount,
        _ => return Err("unsupported burn-in unit or value".into()),
    };
    let selected = count(&record, "selected_state")?;
    if discarded != expected
        || selected < count(&record, "first_retained_state")?
        || selected > count(&record, "last_state")?
    {
        return Err("summary states disagree with retained samples".into());
    }
    let source = &record["source"];
    if source["path"].as_str().is_none()
        || !source["sha256"]
            .as_str()
            .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        || record["rove_version"].as_str().is_none()
        || !matches!(
            source["completion"].as_str(),
            Some("recorded_completed" | "unverified")
        )
    {
        return Err("invalid summary source identity".into());
    }
    let path = directory.join("mcc.tree");
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err("summary tree must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("summary tree exceeds 16 MiB".into());
    }
    let hash = sha256::hex(&bytes);
    if record["annotated_tree_sha256"] != hash {
        return Err(
            "annotated tree fingerprint differs from its summary record; regenerate the summary"
                .into(),
        );
    }
    let mut reader = Reader::annotated(Cursor::new(bytes))?;
    let tree = reader.next_tree()?.ok_or("missing summary tree")?;
    if reader.next_tree()?.is_some() {
        return Err("expected a single summary tree".into());
    }
    let taxa: Vec<String> = serde_json::from_value(record["taxa"].clone())?;
    if taxa != reader.taxa {
        return Err("summary taxa disagree with the tree translation".into());
    }
    let constraints: Vec<Vec<String>> = serde_json::from_value(source["constraints"].clone())?;
    let masks = tree.clades();
    let mut imposed = vec![false; tree.nodes.len()];
    for group in &constraints {
        if group.len() < 2 {
            return Err("invalid recorded clade constraint".into());
        }
        let mut mask = vec![0; taxa.len().div_ceil(64)];
        for name in group {
            let index = taxa
                .iter()
                .position(|t| t == name)
                .ok_or("unknown constrained taxon")?;
            let bit = 1u64 << (index % 64);
            if mask[index / 64] & bit != 0 {
                return Err("duplicate constrained taxon".into());
            }
            mask[index / 64] |= bit;
        }
        let index = masks
            .iter()
            .position(|m| *m == mask)
            .ok_or("constrained clade is absent from summary")?;
        imposed[index] = true;
    }
    let mut nodes = Vec::with_capacity(tree.nodes.len());
    for (id, (node, annotation)) in tree.nodes.iter().zip(&tree.annotations).enumerate() {
        if annotation
            .constrained
            .is_some_and(|value| value != imposed[id])
        {
            return Err("clade constraint annotation disagrees with the recorded setup".into());
        }
        if record["height_policy"] == "ca"
            && annotation
                .ancestor_mean
                .is_some_and(|h| (h - node.height).abs() > 1e-9 * node.height.abs().max(1e-12))
        {
            return Err("common ancestor mean disagrees with displayed heights".into());
        }
        nodes.push(json!({
            "id": id, "parent": node.parent, "children": node.children,
            "taxon": node.tip.map(|i| &taxa[i]), "height": node.height, "length": node.length,
            "posterior": annotation.posterior, "constrained": imposed[id], "rate": node.rate,
            "height_mean": annotation.height_mean, "height_range": annotation.height_range,
            "ancestor_mean": annotation.ancestor_mean, "ancestor_range": annotation.ancestor_range,
            "rate_range": annotation.rate_range,
        }));
    }
    Ok(json!({
        "schema_version": 1, "kind": "posterior_summary", "root": tree.root,
        "nodes": nodes, "taxa": taxa, "constraints": constraints, "tree_sha256": hash,
        "summary": {
            "method": "MCC", "height_policy": record["height_policy"], "burnin": burnin,
            "total_samples": total, "discarded_samples": discarded, "retained_samples": retained,
            "selected_state": selected, "source_path": source["path"],
            "source_sha256": source["sha256"], "source_completion": source["completion"],
            "matrix_sha256": source["run_record"]["input"]["sha256"],
            "rove_version": record["rove_version"],
        },
    }))
}

pub fn run(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [directory] = arguments else {
        return Err("usage: rove inspect-tree <summary-directory>".into());
    };
    println!(
        "{}",
        serde_json::to_string(&inspect(Path::new(directory))?)?
    );
    Ok(())
}
