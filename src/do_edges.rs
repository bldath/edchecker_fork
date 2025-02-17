use std::collections::{BTreeSet, HashSet};

use itertools::{iproduct, Itertools};
use petgraph::graph::NodeIndex;
use petgraph::visit::{EdgeRef, IntoNeighborsDirected, NodeRef};
use petgraph::Direction::Incoming;

use crate::model::{EGraphData, EdgeTp::*};
use crate::msg_algorithms::flip_iter;
use crate::{
    algorithms::add_edges,
    model::{EGraph, EdgeTp, MGraph},
    preprocess::{get_quadruples, get_triples, quad_fmap, triple_fmap},
};


pub fn get_post(g : &EGraph, idx : NodeIndex) -> Option<NodeIndex> {
    g.edges_directed(idx, Incoming).find(| x | *x.weight() == PB).map(| q | q.source())
}

pub fn multiset_do(g: EGraph) -> EGraph {
    g
}

pub fn queue_do(mut g: EGraph, data: &EGraphData) -> EGraph {

    let q = data.iter()
        .flat_map(| (h1, m1) | {
            m1
                .iter()
                .tuple_combinations()
                .flat_map(| (x, y) | vec![(x, y), (y, x)])
                .filter_map( |((m1, e1), (m2, e2))| {
                    let m1get = *e1.first().unwrap();
                    let m1done = *e1.last().unwrap();

                    let m2get = *e2.first().unwrap();
                    let m2done = *e2.last().unwrap();

                    if let (Some(m1post), Some(m2post)) = (get_post(&g, m1get), get_post(&g, m2get)) {
                        if let Some(mo_edge) = g.edges_connecting(m1post, m2post).find(| x | *x.weight() == MO) {
                            // Now, we have m1 --[pb^-1]-> post(m1) --[MO]-> post(m2) --[PB]-> m2
                            // So we should add m1 --[DO]-> m2, which goes from done to get
                            return Some((DO, m1done, m2get))
                        }
                    }
                    None
                })
        }).collect_vec();
    add_edges(&mut g, q);
    g
}

pub fn stack_do(mut g: EGraph, data: &EGraphData) -> EGraph {

    let q = data.iter()
        .flat_map(| (h1, m1) | {
            m1
                .iter()
                .tuple_combinations()
                .flat_map(| (x, y) | vec![(x, y), (y, x)])
                .filter_map( |((m1, e1), (m2, e2))| {
                    let m1get = *e1.first().unwrap();
                    let m1done = *e1.last().unwrap();

                    let m2get = *e2.first().unwrap();
                    let m2done = *e2.last().unwrap();

                    if let (Some(m1post), Some(m2post)) = (get_post(&g, m1get), get_post(&g, m2get)) {
                        if let Some(mo_edge) = g.edges_connecting(m1post, m2post).find(| x | *x.weight() == MO) {
                            // Now, we have m1 --[pb^-1]-> post(m1) --[MO]-> post(m2)
                            // Do we have m1 --[EO]-> m2? If so, we have
                            // m1 --[EO]-> m2 --[PB]-> post(m2)
                            if let Some(edge) = g.edges_connecting(m1done, m2get).find(| x | *x.weight() == EO) {
                                return Some((DO, m1get, m2post))
                            }
                        }
                    }
                    None
                })
        }).collect_vec();
    add_edges(&mut g, q);
    g
}

pub fn reg_do(mut g: EGraph) -> EGraph {
    let q1 = triple_fmap(&g, |x, y, z| {
        if let Some(e1) = g.edges_connecting(y, x).find(|e| *e.weight() == PB) {
            if let Some(e2) = g.edges_connecting(y, z).find(|e| *e.weight() == MO) {
                return Some((DO, x, z));
            }
        }
        None
    });
    add_edges(&mut g, q1);
    g
}
