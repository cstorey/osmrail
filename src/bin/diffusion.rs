use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    io::{Read, Seek},
    path::PathBuf,
};

use anyhow::{Context, Result};
use im::Vector;
use osmpbfreader::{Node, NodeId, OsmId, OsmObj, OsmPbfReader, Relation, Way};
use petgraph::{
    graph::{NodeIndex, UnGraph},
    visit::EdgeRef,
};
use smartstring::alias::String;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Args {
    src: PathBuf,
}

#[derive(Clone, Debug, Default)]
struct Map {
    graph: UnGraph<NodeId, OsmId>,
    vertex_by_osm_id: BTreeMap<NodeId, NodeIndex>,
    osm_id_by_vertex: BTreeMap<NodeIndex, NodeId>,
    objs: BTreeMap<OsmId, OsmObj>,
    node_id_by_crs: BTreeMap<String, NodeId>,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::from_args();

    let r = std::fs::File::open(&args.src)?;

    let mut pbf = OsmPbfReader::new(r);

    let map = Map::from_reader(&mut pbf)?;

    let _crses = vec!["HYS", "WWI", "EDN", "ELE", "LSY", "CFB"]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    let crses = map.node_id_by_crs.keys().cloned().collect::<HashSet<_>>();

    println!("---");

    let mut stack = VecDeque::<(String, Vector<OsmId>, NodeIndex)>::new();
    // We might not need to keep the path here. Given that we only care the
    // paths for vertexes on the "fringe", we _could_ maybe get away with only
    // storing it in the fringe.
    let mut seen = HashMap::<NodeIndex, (String, Vector<OsmId>)>::new();
    let mut boundaries = HashMap::<(String, String), Vector<OsmId>>::new();

    for crs in crses.iter().cloned() {
        let node_id = map.node_id_by_crs.get(&crs).cloned().expect(&crs);
        let idx = map.vertex_by_osm_id.get(&node_id).cloned().expect("vertex");
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
        for succ_ref in map.graph.edges(idx) {
            assert_eq!(succ_ref.source(), idx);
            let succ = succ_ref.target();

            let succ_osm_id = map.osm_id_by_vertex.get(&succ).cloned().expect("osm_id");
            match seen.get(&succ) {
                Some((succ_crs, succ_path)) if crs != *succ_crs => {
                    println!("Boundary:\t{}[{:?}]--{}[{:?}]", crs, idx, succ_crs, succ);
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

                    path.push_back(*succ_ref.weight());
                    path.push_back(succ_osm_id.into());

                    seen.insert(succ, (crs.clone(), path.clone()));
                    stack.push_back((crs.clone(), path, succ));
                }
            }
        }
    }
    println!("---");

    for ((a, b), path) in boundaries {
        // println!("{}--{}", a, b);
        // for osm_id in path.iter() {
        //     println!("\t{:?}: {:?}", osm_id, map.objs.get(osm_id));
        // }
        println!("{}--{}; {:?}", a, b, path);
    }

    Ok(())
}

impl Map {
    fn from_reader<R: Read + Seek>(pbf: &mut OsmPbfReader<R>) -> Result<Self> {
        let mut map = Map::default();

        for it in pbf.iter() {
            let it = it.context("Read item")?;
            match it {
                OsmObj::Node(n) => map.add_node(n),
                OsmObj::Way(w) => map.add_way(w),
                OsmObj::Relation(r) => map.add_rel(r),
            }
        }

        Ok(map)
    }

    fn obj_by_idx(&self, idx: NodeIndex) -> Option<OsmObj> {
        self.id_by_idx(idx)
            .and_then(|osm_id| self.objs.get(&osm_id.into()).cloned())
    }

    fn id_by_idx(&self, idx: NodeIndex) -> Option<NodeId> {
        self.osm_id_by_vertex.get(&idx).cloned()
    }

    // We mostly just want railway=station here.
    fn add_node(&mut self, node: Node) {
        if !(node.tags.contains_key("railway") || node.tags.contains_key("public_transport")) {
            return;
        }

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

    // "We mostly just care about routes here."
    fn add_way(&mut self, w: Way) {
        if !(w.tags.contains_key("railway") || w.tags.contains_key("public_transport")) {
            return;
        }

        println!("{:?}: {:?}; {:?}", OsmId::from(w.id), w.tags, w.nodes);

        for (a, b) in w.nodes.iter().cloned().zip(w.nodes.iter().skip(1).cloned()) {
            let a_idx = self.index(a);
            let b_idx = self.index(b);
            println!("\t{:?}[{:?}] -- {:?}[{:?}]", a, a_idx, b, b_idx);
            self.graph.add_edge(a_idx, b_idx, w.id.into());
        }
        self.objs.insert(w.id.into(), w.into());
    }

    fn add_rel(&mut self, r: Relation) {
        if r.tags.get("public_transport").map(|s| &**s) != Some("stop_area") {
            return;
        }

        println!("{:?}: {:?}; {:?}", OsmId::from(r.id), r.tags, r.refs);

        let nodes = r.refs.iter().flat_map(|r| r.member.node());
        for a in nodes.clone() {
            let a_idx = self.index(a);
            for b in nodes.clone().filter(|&b| a < b) {
                let b_idx = self.index(b);
                println!("\t{:?}[{:?}] -- {:?}[{:?}]", a, a_idx, b, b_idx);
                self.graph.add_edge(a_idx, b_idx, r.id.into());
            }
        }
        self.objs.insert(r.id.into(), r.into());
    }

    fn index(&mut self, node_id: NodeId) -> NodeIndex {
        let Self {
            graph,
            vertex_by_osm_id,
            osm_id_by_vertex,
            ..
        } = self;
        *vertex_by_osm_id.entry(node_id).or_insert_with(|| {
            let idx = graph.add_node(node_id);
            osm_id_by_vertex.insert(idx, node_id);
            idx
        })
    }
}
