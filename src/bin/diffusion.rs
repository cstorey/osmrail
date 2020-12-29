use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    io::{Read, Seek},
    path::PathBuf,
};

use anyhow::{Context, Result};
use im::Vector;
use osmpbfreader::{Node, NodeId, OsmId, OsmObj, OsmPbfReader, Relation, Tags, Way};
use petgraph::graph::{NodeIndex, UnGraph};
use smartstring::alias::String;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Args {
    src: PathBuf,
}

#[derive(Clone, Debug, Default)]
struct Map {
    graph: UnGraph<OsmId, Option<String>>,
    vertex_by_osm_id: BTreeMap<OsmId, NodeIndex>,
    osm_id_by_vertex: BTreeMap<NodeIndex, OsmId>,
    objs: BTreeMap<OsmId, OsmObj>,
    node_id_by_crs: BTreeMap<String, NodeId>,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::from_args();

    let r = std::fs::File::open(&args.src)?;

    let mut pbf = OsmPbfReader::new(r);

    let map = Map::from_reader(&mut pbf)?;

    let crses = vec!["HYS", "WWI", "EDN", "ELE", "LSY", "CFB"]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();

    println!("---");

    let mut stack = VecDeque::<(String, Vector<OsmId>, NodeIndex)>::new();
    // We might not need to keep the path here. Given that we only care the
    // paths for vertexes on the "fringe", we _could_ maybe get away with only
    // storing it in the fringe.
    let mut seen = HashMap::<NodeIndex, (String, Vector<OsmId>)>::new();
    let mut boundaries = HashMap::<(String, String), Vector<OsmId>>::new();

    for crs in crses.iter().cloned() {
        let node_id = map.node_id_by_crs.get(&crs).cloned().expect(&crs);
        let idx = map
            .vertex_by_osm_id
            .get(&node_id.into())
            .cloned()
            .expect("vertex");
        let path = Vector::unit(node_id.into());
        seen.insert(idx, (crs.clone(), path.clone()));
        stack.push_back((crs, path, idx));
    }

    while let Some((crs, path, idx)) = stack.pop_front() {
        println!(
            "Visit:\t{:?}: {:?}; {:?}, {:?}; {:?}",
            crs,
            idx,
            path,
            map.id_by_idx(idx),
            map.obj_by_idx(idx)
        );
        for succ in map.graph.neighbors(idx) {
            let succ_osm_id = map.osm_id_by_vertex.get(&succ).cloned().expect("osm_id");
            match seen.get(&succ) {
                Some((succ_crs, succ_path)) if crs != *succ_crs => {
                    println!("Boundary:\t{}[{:?}]—{}[{:?}]", crs, idx, succ_crs, succ);
                    let key;
                    let full_path;
                    if &crs <= succ_crs {
                        key = (crs.clone(), succ_crs.clone());
                        full_path = path
                            .iter()
                            .cloned()
                            .chain(succ_path.iter().rev().cloned())
                            .collect();
                    } else {
                        key = (succ_crs.clone(), crs.clone());
                        full_path = succ_path
                            .iter()
                            .cloned()
                            .chain(path.iter().rev().cloned())
                            .collect();
                    };
                    boundaries.entry(key).or_insert(full_path);
                }
                // Seen, but uninteresting.
                Some(_succ_crs) => {
                    // println!("Seen:\t{}[{:?}]", _succ_crs, succ);
                }
                None => {
                    println!("New:\t{}[{:?}]", crs, succ);
                    let mut path = path.clone();
                    path.push_back(succ_osm_id);

                    seen.insert(succ, (crs.clone(), path.clone()));
                    stack.push_back((crs.clone(), path, succ));
                }
            }
        }
    }
    println!("---");

    for ((a, b), path) in boundaries {
        println!("{}—{}", a, b);
        for osm_id in path.iter() {
            println!("\t{:?}: {:?}", osm_id, map.objs.get(osm_id));
        }
    }

    Ok(())
}

impl Map {
    fn from_reader<R: Read + Seek>(pbf: &mut OsmPbfReader<R>) -> Result<Self> {
        let mut map = Map::default();

        fn is_relevant(tags: &Tags) -> bool {
            tags.contains_key("railway") || tags.contains_key("public_transport")
        }

        for it in pbf.iter() {
            let it = it.context("Read item")?;
            if is_relevant(it.tags()) {
                match it {
                    OsmObj::Node(n) => map.add_node(n),
                    OsmObj::Way(w) => map.add_way(w),
                    OsmObj::Relation(r) => map.add_rel(r),
                }
            }
        }

        Ok(map)
    }

    fn obj_by_idx(&self, idx: NodeIndex) -> Option<OsmObj> {
        self.id_by_idx(idx)
            .and_then(|osm_id| self.objs.get(&osm_id).cloned())
    }

    fn id_by_idx(&self, idx: NodeIndex) -> Option<OsmId> {
        self.osm_id_by_vertex.get(&idx).cloned()
    }

    fn add_node(&mut self, node: Node) {
        let vid = self.index(node.id);
        println!(
            "{:?}[{:?}]: {:?}, {},{}",
            node.id,
            vid,
            node.tags,
            node.lat(),
            node.lon()
        );
        if let Some(crs) = node.tags.get("ref:crs") {
            self.node_id_by_crs.insert(crs.clone(), node.id);
        }
        self.objs.insert(node.id.into(), node.into());
    }

    fn add_way(&mut self, w: Way) {
        let way_idx = self.index(w.id);
        println!(
            "{:?}[{:?}]: {:?}; {:?}",
            OsmId::from(w.id),
            way_idx,
            w.tags,
            w.nodes
        );

        for node_id in w.nodes.iter().cloned() {
            println!("{:?} -- {:?}", OsmId::from(w.id), OsmId::from(node_id));
            let n_idx = self.index(node_id);
            self.graph.add_edge(way_idx, n_idx, None);
        }
        self.objs.insert(w.id.into(), w.into());
    }

    fn add_rel(&mut self, r: Relation) {
        let rel_idx = self.index(r.id);
        println!(
            "{:?}[{:?}]: {:?}; {:?}",
            OsmId::from(r.id),
            rel_idx,
            r.tags,
            r.refs
        );

        for it in r.refs.iter() {
            println!("{:?} -- {:?}[{}]", OsmId::from(r.id), it.member, it.role);
            let member_idx = self.index(it.member);
            self.graph
                .add_edge(rel_idx, member_idx, Some(it.role.clone()));
        }
        self.objs.insert(r.id.into(), r.into());
    }

    fn index(&mut self, osm_id: impl Into<OsmId>) -> NodeIndex {
        let osm_id = osm_id.into();
        let Self {
            graph,
            vertex_by_osm_id,
            osm_id_by_vertex,
            ..
        } = self;
        *vertex_by_osm_id.entry(osm_id).or_insert_with(|| {
            let idx = graph.add_node(osm_id);
            osm_id_by_vertex.insert(idx, osm_id);
            idx
        })
    }
}
