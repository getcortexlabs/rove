use std::error::Error;
use std::fs;

use rove::data::GapTreatment;
use rove::nexus;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next().ok_or("usage: read_nexus <matrix.nex>")?;
    if arguments.next().is_some() {
        return Err("usage: read_nexus <matrix.nex>".into());
    }
    let data = nexus::parse(&fs::read_to_string(path)?)?;
    let sites = 0..data.character_count();
    let partitions = data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)?;
    let patterns = &partitions[0];
    println!(
        "{} taxa, {} characters, {} declared states",
        data.taxa().len(),
        data.character_count(),
        data.symbols().len()
    );
    println!(
        "{} observed patterns ({} total weight)",
        patterns.weights().len(),
        patterns.weights().iter().sum::<usize>()
    );
    Ok(())
}
