//! Trace and tree files in the layouts BEAST 2 writes, so Tracer, LogAnalyser
//! and TreeAnnotator can read them.

use std::io::{self, Write};

use super::Sampler;

pub const TRACE_COLUMNS: [&str; 9] = [
    "Sample",
    "posterior",
    "likelihood",
    "prior",
    "birth_death",
    "birth_difference_rate",
    "relative_death_rate",
    "tree_height",
    "tree_length",
];

/// Tab-separated parameter trace, after `#` comment lines.
pub struct TraceWriter<W: Write> {
    out: W,
}

impl<W: Write> TraceWriter<W> {
    pub fn new(mut out: W, comments: &[String]) -> io::Result<Self> {
        for comment in comments {
            writeln!(out, "# {comment}")?;
        }
        writeln!(out, "{}", TRACE_COLUMNS.join("\t"))?;
        Ok(Self { out })
    }

    pub fn write(&mut self, sample: u64, sampler: &Sampler) -> io::Result<()> {
        let posterior = sampler.posterior();
        let birth_death = sampler.birth_death();
        let tree = sampler.tree();
        writeln!(
            self.out,
            "{sample}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            posterior.total(),
            posterior.likelihood,
            posterior.prior(),
            posterior.tree_prior,
            birth_death.birth_difference,
            birth_death.relative_death,
            tree.height(tree.root()),
            tree.tree_length()
        )
    }

    pub fn finish(mut self) -> io::Result<W> {
        self.out.flush()?;
        Ok(self.out)
    }
}

/// A NEXUS trees block whose tips are numbered through a translate table.
pub struct TreeWriter<W: Write> {
    out: W,
}

impl<W: Write> TreeWriter<W> {
    pub fn new(mut out: W, taxa: &[String], comments: &[String]) -> io::Result<Self> {
        writeln!(out, "#NEXUS")?;
        for comment in comments {
            writeln!(out, "[{}]", comment.replace(['[', ']'], ""))?;
        }
        writeln!(
            out,
            "\nBegin taxa;\n\tDimensions ntax={};\n\t\tTaxlabels",
            taxa.len()
        )?;
        for taxon in taxa {
            writeln!(out, "\t\t\t{}", label(taxon))?;
        }
        writeln!(out, "\t\t\t;\nEnd;\nBegin trees;\n\tTranslate")?;
        for (index, taxon) in taxa.iter().enumerate() {
            let separator = if index + 1 == taxa.len() { "" } else { "," };
            writeln!(out, "\t\t{} {}{separator}", index + 1, label(taxon))?;
        }
        writeln!(out, ";")?;
        Ok(Self { out })
    }

    pub fn write(&mut self, sample: u64, sampler: &Sampler) -> io::Result<()> {
        let newick = sampler.tree().to_newick(|tip| (tip + 1).to_string());
        writeln!(self.out, "tree STATE_{sample} = {newick}")
    }

    pub fn finish(mut self) -> io::Result<W> {
        writeln!(self.out, "End;")?;
        self.out.flush()?;
        Ok(self.out)
    }
}

/// Whether `name` can be written as a NEXUS word that BEAST's NexusParser
/// reads back: letters, digits, `_`, `.` and `-`, or spaces written as `_`
/// when the name has no underscore of its own.
pub fn is_plain_label(name: &str) -> bool {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | ' ');
    !name.is_empty() && name.chars().all(allowed) && !(name.contains(' ') && name.contains('_'))
}

/// A NEXUS word: spaces become underscores, as the reader expects, and any
/// other name is quoted.
fn label(name: &str) -> String {
    if is_plain_label(name) {
        name.replace(' ', "_")
    } else {
        format!("'{}'", name.replace('\'', "''"))
    }
}

#[cfg(test)]
mod tests {
    use super::label;

    #[test]
    fn labels_are_quoted_only_when_needed() {
        assert_eq!(label("T1"), "T1");
        assert_eq!(label("Homo_sapiens"), "Homo_sapiens");
        assert_eq!(label("Homo sapiens"), "Homo_sapiens");
        assert_eq!(label("O'Brien"), "'O''Brien'");
        assert_eq!(label("Homo_sapiens x"), "'Homo_sapiens x'");
        assert_eq!(label("A, b"), "'A, b'");
    }
}
