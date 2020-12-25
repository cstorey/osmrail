use std::{
    fs::File,
    io::{Read, Seek},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use osmpbfreader::{Node, OsmId, OsmObj, OsmPbfReader, Relation, Way};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Args {
    src: PathBuf,
    dst_dir: PathBuf,
}

#[derive(Debug)]
struct ExtractTransform {
    nodes: csv::Writer<File>,
    ways: csv::Writer<File>,
    way_nodes: csv::Writer<File>,
    rels: csv::Writer<File>,
    rel_members: csv::Writer<File>,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::from_args();

    let r = std::fs::File::open(&args.src).context("open src")?;

    let mut pbf = OsmPbfReader::new(r);

    let mut writer = ExtractTransform::new(&args.dst_dir).context("create extractor")?;

    writer.extract(&mut pbf).context("run extract")?;

    writer.finish()?;

    Ok(())
}

impl ExtractTransform {
    fn new(dir: &Path) -> Result<Self> {
        let mut nodes = csv::Writer::from_path(dir.join("nodes.csv"))?;
        nodes.write_record(&["node_id", "lat", "lon", "tags"])?;

        let mut ways = csv::Writer::from_path(dir.join("ways.csv"))?;
        ways.write_record(&["way_id", "tags"])?;

        let mut way_nodes = csv::Writer::from_path(dir.join("way-nodes.csv"))?;
        way_nodes.write_record(&["way_id", "ordinal", "node_id"])?;

        let mut rels = csv::Writer::from_path(dir.join("relations.csv"))?;
        rels.write_record(&["rel_id", "tags"])?;

        let mut rel_members = csv::Writer::from_path(dir.join("relation-members.csv"))?;
        rel_members.write_record(&["rel_id", "ordinal", "role", "node_id", "way_id", "rel_id"])?;

        let me = Self {
            nodes,
            ways,
            way_nodes,
            rels,
            rel_members,
        };
        Ok(me)
    }

    fn extract<R: Read + Seek>(&mut self, pbf: &mut OsmPbfReader<R>) -> Result<()> {
        for it in pbf.iter() {
            let it = it.context("Read item")?;
            match it {
                OsmObj::Node(n) => self.add_node(n)?,
                OsmObj::Way(w) => self.add_way(w)?,
                OsmObj::Relation(r) => self.add_rel(r)?,
            }
        }

        Ok(())
    }

    fn finish(self) -> Result<()> {
        let Self {
            mut nodes,
            mut ways,
            mut way_nodes,
            mut rels,
            mut rel_members,
        } = self;
        nodes.flush()?;
        ways.flush()?;
        way_nodes.flush()?;
        rels.flush()?;
        rel_members.flush()?;

        Ok(())
    }

    // nodes.write_record(&["node_id", "lat", "lon", "tags"])?;

    fn add_node(&mut self, node: Node) -> Result<()> {
        self.nodes.write_record(&[
            &format!("{}", node.id.0) as &dyn AsRef<[u8]>,
            &format!("{}", node.lat()),
            &format!("{}", node.lon()),
            &serde_json::to_vec(&node.tags)?,
        ] as &[&dyn AsRef<[u8]>])?;
        Ok(())
    }

    // ways.write_record(&["way_id", "tags"])?;
    // ways.write_record(&["way_id", "ordinal", "node_id"])?;
    fn add_way(&mut self, way: Way) -> Result<()> {
        self.ways.write_record(&[
            &format!("{}", way.id.0) as &dyn AsRef<[u8]>,
            &serde_json::to_vec(&way.tags)?,
        ])?;

        for (i, node) in way.nodes.iter().enumerate() {
            self.way_nodes.write_record(&[
                &format!("{}", way.id.0),
                &format!("{}", i),
                &format!("{}", node.0),
            ])?;
        }

        Ok(())
    }

    // rels.write_record(&["rel_id", "tags"])?;
    // rel_members.write_record(&["rel_id", "ordinal", "role", "node_id", "way_id", "rel_id"])?;
    fn add_rel(&mut self, rel: Relation) -> Result<()> {
        self.rels
            .write_record(&[&format!("{}", rel.id.0), &serde_json::to_string(&rel.tags)?])?;

        for (i, member) in rel.refs.iter().enumerate() {
            match member.member {
                OsmId::Node(node_id) => {
                    self.rel_members.write_record(&[
                        &format!("{}", rel.id.0) as &dyn AsRef<[u8]>,
                        &format!("{}", i),
                        &member.role,
                        &format!("{}", node_id.0),
                        &"",
                        &"",
                    ])?;
                }
                OsmId::Way(way_id) => {
                    self.rel_members.write_record(&[
                        &format!("{}", rel.id.0) as &dyn AsRef<[u8]>,
                        &format!("{}", i),
                        &member.role,
                        &"",
                        &format!("{}", way_id.0),
                        &"",
                    ])?;
                }
                OsmId::Relation(rel_id) => {
                    self.rel_members.write_record(&[
                        &format!("{}", rel.id.0) as &dyn AsRef<[u8]>,
                        &format!("{}", i),
                        &member.role,
                        &"",
                        &"",
                        &format!("{}", rel_id.0),
                    ])?;
                }
            }
        }

        Ok(())
    }
}
