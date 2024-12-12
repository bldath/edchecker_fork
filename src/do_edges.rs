use std::collections::{BTreeSet, HashSet};

use itertools::Itertools;
use petgraph::graph::NodeIndex;

use crate::model::EdgeTp::*;
use crate::{
    algorithms::add_edges,
    model::{EGraph, EdgeTp, MGraph},
    preprocess::{get_quadruples, get_triples, quad_fmap, triple_fmap},
};

pub fn multiset_do(g: EGraph) -> EGraph {
    g
}

pub fn queue_do(mut g: EGraph) -> EGraph {
    let q = quad_fmap(&g, |x, y, z, w| {
        if let Some(pb1) = g.edges_connecting(y, x).find(|e| *e.weight() == PB) {
            if let Some(mo) = g.edges_connecting(y, z).find(|e| *e.weight() == MO) {
                if let Some(pb) = g.edges_connecting(z, w).find(|e| *e.weight() == PB) {
                    return Some((DO, x, w));
                }
            }
        }
        None
    });
    add_edges(&mut g, q);
    g
}

pub fn stack_do(mut g: EGraph) -> EGraph {
    let q1 = triple_fmap(&g, |x, y, z| {
        if let Some(e1) = g.edges_connecting(y, x).find(|e| *e.weight() == PB) {
            if let Some(e2) = g.edges_connecting(y, z).find(|e| *e.weight() == MO) {
                return Some((DO, x, z));
            }
        }
        None
    });

    let q2 = triple_fmap(&g, |x, y, z| {
        if let Some(e1) = g.edges_connecting(x, y).find(|e| *e.weight() == EO) {
            if let Some(e2) = g.edges_connecting(z, y).find(|e| *e.weight() == PB) {
                return Some((DO, x, z));
            }
        }
        None
    });

    let h1 = BTreeSet::from_iter(q1.into_iter());
    let h2 = BTreeSet::from_iter(q2.into_iter());

    let v = h1.intersection(&h2).map(|x| *x).collect_vec();
    add_edges(&mut g, v);
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
