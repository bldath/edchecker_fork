use petgraph::{csr::IndexType, graph::NodeIndex, visit::IntoNodeIdentifiers, EdgeType, Graph};

use itertools::{iproduct, Itertools};

use crate::model::*;

pub fn preprocess(g : &mut EGraph) {
    add_mo(g);
    add_rf(g);
}


pub fn get_pairs(g : &EGraph, rel : impl Fn(&EPair, &EPair) -> bool) -> Vec<(NodeIndex, NodeIndex)> {
    let product = iproduct!(g.node_indices(), g.node_indices());
    product.filter_map(|(x,y)| { if rel(&g[x], &g[y]) { Some((x.clone(), y.clone())) } else { None }}).collect_vec()
}

fn add_mo(g : &mut EGraph) {
    let new_edges: Vec<(NodeIndex, NodeIndex)> = get_pairs(&g, |x, y| -> bool {
        match (x, y) {
            (EPair(hdl, _, Event::Post(to, sent)), EPair(hdl2, _, Event::Get(gotten))) => sent == gotten,
            _ => false
        }
    });

    for e in new_edges {
        g.add_edge(e.0, e.1, EdgeTp::MO);
    }
}

fn add_rf(g : &mut EGraph) {
    let new_edges = get_pairs(&g, |x, y| {
        match (x, y) {
            (EPair(_, _, Event::Write(var, val)), EPair(_, _, Event::Read(var2, val2))) => var == var2 && val == val2,
            _ => false
        }
    });

    for e in new_edges {
        g.add_edge(e.0, e.1, EdgeTp::RF);
    }
}
