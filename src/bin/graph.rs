use std::{
    collections::BTreeMap,
    io::{Read, Seek},
    path::PathBuf,
};

use anyhow::{Context, Result};
use osmpbfreader::{
    Node, NodeId, OsmId, OsmObj, OsmPbfReader, Relation, RelationId, Tags, Way, WayId,
};
use petgraph::{
    algo::astar,
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
    graph: UnGraph<OsmId, Option<String>>,
    vertex_by_osm_id: BTreeMap<OsmId, NodeIndex>,
    osm_id_by_vertex: BTreeMap<NodeIndex, OsmId>,
    nodes: BTreeMap<NodeId, Node>,
    ways: BTreeMap<WayId, Way>,
    rels: BTreeMap<RelationId, Relation>,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::from_args();

    let r = std::fs::File::open(&args.src)?;

    let mut pbf = OsmPbfReader::new(r);

    let map = Map::from_reader(&mut pbf)?;

    // Sundridge Park
    // let station_sdp = NodeId(7860900545);
    // let station_grp = NodeId(5872906104);
    // let area_sdp = RelationId(11563276);
    // let se_mainline = RelationId(4860731);
    // let bmn_shuttle = RelationId(168686);

    let _hys = NodeId(7159246417);
    let _wwi = NodeId(5883033870);
    let _edn = NodeId(5883033871);
    let _ele = NodeId(5883033872);

    let _cfb = NodeId(5883033866);
    let rt = [_hys, _wwi, _edn, _ele];

    for (src, dst) in rt.iter().cloned().zip(rt.iter().cloned().skip(1)) {
        let src_idx = *map.vertex_by_osm_id.get(&src.into()).expect("hys");
        let dst_idx = *map.vertex_by_osm_id.get(&dst.into()).expect("cfb");

        println!();

        // let edge_cost_node_dist = |edge_ref: EdgeReference<_>| {
        //     let aid = map
        //         .obj_by_idx(edge_ref.source())
        //         .and_then(|obj| obj.node().cloned());
        //     let bid = map
        //         .obj_by_idx(edge_ref.target())
        //         .and_then(|obj| obj.node().cloned());
        //     let cost = if let Some((a, b)) = aid.zip(bid) {
        //         // Technically, we should be calculating these on a spheroid… but this will do for now, I guess.
        //         let sq_dist = (a.lat() - b.lat()).powf(2.0) + (a.lon() - b.lon()).powf(2.0);
        //         sq_dist.sqrt()
        //     } else {
        //         0.0
        //     };
        //     println!(
        //         "Δ: {:?}→{:?}: {:?}",
        //         map.id_by_idx(edge_ref.source()),
        //         map.id_by_idx(edge_ref.target()),
        //         cost
        //     );
        //     cost
        // };
        fn score_node_id(obj_id: OsmId) -> f64 {
            match obj_id {
                OsmId::Relation(_) => 1.0,
                OsmId::Way(_) => 2.0,
                OsmId::Node(_) => 4.0,
            }
        }
        let resp = astar(
            &map.graph,
            src_idx,
            |idx| idx == dst_idx,
            |e| {
                let aid = map.id_by_idx(e.source()).expect("a");
                let bid = map.id_by_idx(e.target()).expect("b");
                score_node_id(aid) + score_node_id(bid)
            },
            |_| 0.0,
        );

        // let resp = dijkstra(
        //     &map.graph,
        //     src_idx,
        //      Some(dst_idx),
        //     |_edge_ref| 1);

        println!("Result: {:?} → {:?}: {:?}", src, dst, resp);
        if let Some((_dist, path)) = resp {
            for idx in path {
                // Nb. We filter out any and all objects that don't have a
                // railway or public_transport tag, so we might end up without
                // a node to show here.
                println!(
                    "{:?}\t{:?}\t{:?}",
                    idx,
                    map.id_by_idx(idx),
                    map.obj_by_idx(idx)
                );
            }
        }

        // let mut visit = Dfs::new(&map.graph, src_idx);
        // println!("Start: {:?}", map.obj_by_idx(src_idx));
        // while let Some(node_idx) = visit.next(&map.graph) {
        //     if let Some(osm_id) = map.id_by_idx(node_idx) {
        //         println!("Found: {:?} / {:?}", osm_id, map.obj_by_idx(node_idx));
        //     }
        // }
    }

    Ok(())
}

impl Map {
    fn from_reader<R: Read + Seek>(pbf: &mut OsmPbfReader<R>) -> Result<Self> {
        let mut map = Map::default();

        fn is_relevant(tags: &Tags) -> bool {
            tags.contains_key("railway")
                || tags.contains_key("public_transport")
                || tags.get("route") == Some(&"train".into())
        }

        // for block in pbf.primitive_blocks() {
        //     let block = block.context("Read block")?;
        //     // let _ : PrimitiveBlock = block;
        //     println!("{:#?}", block);
        //     println!("---");
        // }
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
        self.id_by_idx(idx).and_then(|osm_id| match osm_id {
            OsmId::Node(node_id) => self.nodes.get(&node_id).cloned().map(OsmObj::from),
            OsmId::Way(way_id) => self.ways.get(&way_id).cloned().map(OsmObj::from),
            OsmId::Relation(rel_id) => self.rels.get(&rel_id).cloned().map(OsmObj::from),
        })
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
        self.nodes.insert(node.id, node);
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
        self.ways.insert(w.id, w);
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
        self.rels.insert(r.id, r);
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
