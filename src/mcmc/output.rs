//! Trace and tree files in the layouts BEAST 2 writes, so Tracer, LogAnalyser
//! and TreeAnnotator can read them.

use std::io::{self, Write};

use super::Sampler;
use crate::rates::RateVariation;

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
    gamma: bool,
    clock: bool,
}

impl<W: Write> TraceWriter<W> {
    pub fn new(out: W, comments: &[String]) -> io::Result<Self> {
        Self::with_rates(out, comments, &RateVariation::default())
    }

    pub fn with_rates(mut out: W, comments: &[String], rates: &RateVariation) -> io::Result<Self> {
        for comment in comments {
            writeln!(out, "# {comment}")?;
        }
        write!(out, "{}", TRACE_COLUMNS.join("\t"))?;
        if rates.gamma.is_some() {
            write!(out, "\tgamma_shape\tgamma_prior")?;
        }
        if rates.clock.is_some() {
            write!(
                out,
                "\tclock_sigma\tclock_rate_prior\tclock_sigma_prior\tmean_branch_rate"
            )?;
        }
        writeln!(out)?;
        Ok(Self {
            out,
            gamma: rates.gamma.is_some(),
            clock: rates.clock.is_some(),
        })
    }

    pub fn write(&mut self, sample: u64, sampler: &Sampler) -> io::Result<()> {
        let posterior = sampler.posterior();
        let birth_death = sampler.birth_death();
        let tree = sampler.tree();
        if self.gamma != sampler.rates().gamma.is_some()
            || self.clock != sampler.rates().clock.is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "trace columns do not match the sampler's rate model",
            ));
        }
        write!(
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
        )?;
        if let Some(gamma) = &sampler.rates().gamma {
            write!(self.out, "\t{}\t{}", gamma.shape(), posterior.gamma_prior)?;
        }
        if let Some(clock) = &sampler.rates().clock {
            let mean = clock.rates().iter().sum::<f64>() / clock.rates().len() as f64;
            write!(
                self.out,
                "\t{}\t{}\t{}\t{}",
                clock.sigma(),
                posterior.clock_rate_prior,
                posterior.clock_sigma_prior,
                mean
            )?;
        }
        writeln!(self.out)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
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
        let tree = sampler.tree();
        let label = |tip: usize| (tip + 1).to_string();
        let newick = match &sampler.rates().clock {
            Some(clock) => {
                tree.to_newick_with_rates(label, |node| clock.branch_rate(node, tree.root()))
            }
            None => tree.to_newick(label),
        };
        writeln!(self.out, "tree STATE_{sample} = {newick}")
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
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
