//! Lossless, bounded encoding used by sampler checkpoints.

use std::io::{self, Read, Write};

pub(crate) fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid sampler checkpoint")
}

pub(crate) fn write_u64(out: &mut impl Write, value: u64) -> io::Result<()> {
    out.write_all(&value.to_le_bytes())
}

pub(crate) fn read_u64(input: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0; 8];
    input.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

pub(crate) fn write_f64(out: &mut impl Write, value: f64) -> io::Result<()> {
    write_u64(out, value.to_bits())
}

pub(crate) fn read_f64(input: &mut impl Read) -> io::Result<f64> {
    Ok(f64::from_bits(read_u64(input)?))
}
