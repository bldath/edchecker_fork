use petgraph::graph::NodeIndex;

use crate::{algorithms::add_edges, model::{EGraph, EdgeTp, MGraph}, preprocess::{get_quadruples, get_triples, quad_fmap}};
use crate::model::EdgeTp::*;


pub fn multiset_do(g: EGraph) -> EGraph {
    g
}

pub fn queue_do(mut g: EGraph) -> EGraph {
    let q = quad_fmap(&g, | x, y, z, w |  {
        if let Some(pb1) = g.edges_connecting(y, x).find(| e | *e.weight() == PB) {
            if let Some(mo) = g.edges_connecting(y, z).find(| e | *e.weight() == MO) {
                if let Some(pb) = g.edges_connecting(z, w).find(| e | *e.weight() == PB) {
                    return Some((DO, x, w))
                }
            }
        }
        None
    });
    add_edges(&mut g, q);
    g
}
