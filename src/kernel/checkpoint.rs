use std::io::{self, Read, Write};

use super::LikelihoodCore;
use crate::checkpoint::{read_f64, write_f64};

impl LikelihoodCore {
    pub(crate) fn save(&self, out: &mut impl Write) -> io::Result<()> {
        // Only live buffers affect the next proposal. Scratch arrays are
        // overwritten before use; store() replaces the rollback indices.
        for node in 0..self.node_count {
            for slice in [
                &self.partials[self.partials_buffer(node)],
                &self.log_scalers[self.partials_buffer(node)],
                &self.transitions[self.transitions_buffer(node)],
            ] {
                for &value in slice {
                    write_f64(out, value)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn load(&mut self, input: &mut impl Read) -> io::Result<()> {
        for node in 0..self.node_count {
            for buffers in [
                &mut self.partials,
                &mut self.log_scalers,
                &mut self.transitions,
            ] {
                for value in &mut buffers[node * 2] {
                    *value = read_f64(input)?;
                }
                let (before, after) = buffers.split_at_mut(node * 2 + 1);
                after[0].copy_from_slice(&before[node * 2]);
            }
        }
        self.current_partials.fill(0);
        self.current_transitions.fill(0);
        self.store();
        Ok(())
    }
}
