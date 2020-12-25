use std::{
    collections::BTreeMap,
    io::{Read, Seek},
    path::PathBuf,
};

use anyhow::Result;
use osmpbfreader::{OsmId, OsmObj, OsmPbfReader, RelationId};
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

    dump_a_thing(&mut pbf)
}

fn print(depth: usize, id: OsmId, data: &BTreeMap<OsmId, OsmObj>) {
    if let Some(val) = data.get(&id) {
        print!("{:width$}", "", width = depth * 2);
        match id {
            OsmId::Node(id) => print!("N{:<14}\t", id.0),
            OsmId::Way(id) => print!("W{:<14}\t", id.0),
            OsmId::Relation(id) => print!("R{:<14}\t", id.0),
        };
        for (i, (k, v)) in val.tags().iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{}={}", k, v);
        }
        print!(";");

        match val {
            OsmObj::Relation(rel) => {
                // println!("{:width$}{:?}\t{:?}", "", id, val, width = depth * 2);
                println!();

                for child in rel.refs.iter() {
                    print(depth + 1, child.member, data)
                }
            }
            OsmObj::Way(way) => {
                // println!("{:width$}{:?}\t{:?}", "", id, val, width = depth * 2);

                println!();
                for child in way.nodes.iter() {
                    print(depth + 1, child.clone().into(), data)
                }
            }
            OsmObj::Node(node) => {
                println!("\t{:06},{:06}", node.lat(), node.lon());
            }
        }
    }
}

fn dump_a_thing<R: Read + Seek>(pbf: &mut OsmPbfReader<R>) -> Result<()> {
    // Sundridge Park
    // let station_sdp = NodeId(7860900545);
    // let station_grp = NodeId(5872906104);
    // let area_sdp = RelationId(11563276);
    // let se_mainline = RelationId(4860731);
    // let bmn_shuttle = RelationId(168686);
    // Public transport route
    // let hayes_to_chx = RelationId(8648176);
    // let hayes_to_cst = RelationId(8648633);
    let hayes_line = RelationId(408573);
    let x = pbf.get_objs_and_deps(|obj| {
        // obj.relation()
        //     .map(|r| r.refs.iter().any(|r| r.member == station_grp.into()))
        //     .unwrap_or_default()
        // obj.tags().contains_key("public_transport")
        obj.id() == hayes_line.into()
    })?;

    // for it in pbf.iter() {
    //     let it = it.context("Read item")?;
    //     if it.tags().contains("route", "train") {
    //         println!("{:?}", it);
    //     }
    // }

    print(0, hayes_line.into(), &x);
    // for (_id, obj) in x {
    //     // println!("{:?}: {:?}", obj.id(), obj.tags())
    //     println!("{:?}", obj)
    // }
    Ok(())
}
