use super::{Error, Result};
use std::collections::{BTreeSet, HashSet};

pub const MAX_CELLS: usize = 8_000_000;
pub const MAX_ROWS: usize = 1_000_000;
pub const MAX_COLUMNS: usize = 128;
const PLOT_BUCKETS: usize = 256;

#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub values: Vec<f64>,
}
#[derive(Debug)]
pub struct Trace {
    pub states: Vec<u64>,
    pub columns: Vec<Column>,
    pub sample_every: Option<u64>,
}
#[derive(Debug)]
pub struct PlotPoint {
    pub state: u64,
    pub value: f64,
    pub retained: bool,
}

impl Trace {
    pub fn read(bytes: &[u8], cancel: &dyn Fn() -> bool) -> Result<Self> {
        if !bytes.ends_with(b"\n") {
            return Err(Error("incomplete final trace line; wait for the writer to finish or inspect a complete file".into()));
        }
        let text = std::str::from_utf8(bytes).map_err(|_| Error("trace must be UTF-8".into()))?;
        let mut lines = text
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'));
        let (_, header) = lines
            .next()
            .ok_or_else(|| Error("missing trace header".into()))?;
        let labels: Vec<_> = header.split_whitespace().collect();
        if labels.len() < 2
            || labels.len() > MAX_COLUMNS + 1
            || labels[0] != "Sample"
            || labels.iter().any(|s| s.len() > 128)
            || labels.iter().copied().collect::<HashSet<_>>().len() != labels.len()
        {
            return Err(Error(
                "expected Sample and 1 to 128 distinct numeric trace columns".into(),
            ));
        }
        let mut trace = Self {
            states: Vec::new(),
            columns: labels[1..]
                .iter()
                .map(|s| Column {
                    name: (*s).into(),
                    values: Vec::new(),
                })
                .collect(),
            sample_every: None,
        };
        for (line, row) in lines {
            if row.len() > 65536 {
                return Err(Error(format!("trace line {} exceeds 64 KiB", line + 1)));
            }
            if trace.states.len() % 1024 == 0 && cancel() {
                return Err(Error("trace diagnostics cancelled".into()));
            }
            if trace.states.len() >= MAX_ROWS
                || (trace.states.len() + 1) * trace.columns.len() > MAX_CELLS
            {
                return Err(Error(
                    "trace exceeds one million rows or eight million numeric cells".into(),
                ));
            }
            let fields: Vec<_> = row.split_whitespace().collect();
            if fields.len() != labels.len() {
                return Err(Error(format!(
                    "trace line {} has missing or extra columns",
                    line + 1
                )));
            }
            let state: u64 = fields[0]
                .parse()
                .map_err(|_| Error(format!("invalid sample state on line {}", line + 1)))?;
            if let Some(&previous) = trace.states.last() {
                if state <= previous {
                    return Err(Error("sample states must increase strictly".into()));
                }
                let interval = state - previous;
                if trace.sample_every.is_some_and(|n| n != interval) {
                    return Err(Error("ESS requires regularly spaced sample states; the trace has gaps or mixed intervals".into()));
                }
                trace.sample_every = Some(interval);
            }
            for (column, field) in trace.columns.iter_mut().zip(&fields[1..]) {
                let value: f64 = field.parse().map_err(|_| {
                    Error(format!(
                        "invalid {} value on line {}",
                        column.name,
                        line + 1
                    ))
                })?;
                if !value.is_finite() {
                    return Err(Error(format!(
                        "non-finite {} value on line {}",
                        column.name,
                        line + 1
                    )));
                }
                column.values.push(value);
            }
            trace.states.push(state);
        }
        if trace.states.is_empty() {
            return Err(Error("trace contains no samples".into()));
        }
        Ok(trace)
    }

    pub fn plot(&self, column: usize, discarded: usize) -> Vec<PlotPoint> {
        let values = &self.columns[column].values;
        let n = values.len();
        let width = n.div_ceil(PLOT_BUCKETS);
        let mut indices = BTreeSet::from([0, n - 1, discarded]);
        if discarded > 0 {
            indices.insert(discarded - 1);
        }
        for start in (0..n).step_by(width) {
            let end = (start + width).min(n);
            let mut min = start;
            let mut max = start;
            for i in start..end {
                if values[i] < values[min] {
                    min = i;
                }
                if values[i] > values[max] {
                    max = i;
                }
            }
            indices.extend([start, min, max, end - 1]);
        }
        indices
            .into_iter()
            .map(|i| PlotPoint {
                state: self.states[i],
                value: values[i],
                retained: i >= discarded,
            })
            .collect()
    }
}
