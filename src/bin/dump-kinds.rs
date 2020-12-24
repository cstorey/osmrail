use std::{
    collections::BTreeMap,
    io::{Read, Seek},
    path::PathBuf,
};

use anyhow::{Context, Result};
use osmpbfreader::{OsmObj, OsmPbfReader};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Args {
    src: PathBuf,
}
fn main() -> Result<()> {
    env_logger::init();
    let args = Args::from_args();

    let r = std::fs::File::open(&args.src)?;

    let mut pbf = OsmPbfReader::new(r);

    railways(&mut pbf)?;
    Ok(())
}

fn railways<R: Read + Seek>(pbf: &mut OsmPbfReader<R>) -> Result<()> {
    let mut nodes = BTreeMap::<_, u64>::new();
    let mut ways = BTreeMap::<_, u64>::new();
    let mut rels = BTreeMap::<_, u64>::new();

    for it in pbf.iter() {
        let it = it.context("Read item")?;
        match it {
            OsmObj::Node(n) if n.tags.contains_key("railway") => {
                if let Some(val) = n.tags.get("railway") {
                    *nodes.entry(val.clone()).or_default() += 1u64
                }
            }
            OsmObj::Way(w) if w.tags.contains_key("railway") => {
                if let Some(val) = w.tags.get("railway") {
                    *ways.entry(val.clone()).or_default() += 1u64
                }
            }
            OsmObj::Relation(r) if r.tags.contains_key("railway") => {
                if let Some(val) = r.tags.get("railway") {
                    *rels.entry(val.clone()).or_default() += 1u64
                }
            }
            _ => (),
        }
    }
    println!("Nodes: {:#?}", nodes);
    println!("Ways: {:#?}", ways);
    println!("Relations: {:#?}", rels);

    Ok(())
}
