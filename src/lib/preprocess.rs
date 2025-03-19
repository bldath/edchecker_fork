

use crate::model::EdgeTp::*;

use crate::cli::ADT;
use crate::heuristics::{add_heuristics, Heuristic};
use petgraph::{
    algo, csr::IndexType, graph::NodeIndex, visit::IntoNodeIdentifiers, EdgeType, Graph,
};

use itertools::{iproduct, Itertools};

use crate::{
    algorithms::{get_missing_totality, po_rf_path, try_extend},
    epR, epW,
    model::*,
};

pub fn preprocess(g: &mut EGraph, data: &EGraphData, heur: Heuristic, adt : ADT) {
    add_pb(g);
    add_rf(g);
    add_fr(g);
    //add_co(g); //Is done manually
    add_heuristics(g, &data, heur, adt);
    //deduce_eo(g);
}

pub fn get_pairs<V, E>(
    g: &Graph<V, E>,
    rel: impl Fn(NodeIndex, NodeIndex) -> bool,
) -> Vec<(NodeIndex, NodeIndex)> {
    let product = iproduct!(g.node_indices(), g.node_indices());
    product.filter(|(x, y)| rel(*x, *y)).collect_vec()
}

pub fn get_triples<V, E>(
    g: &Graph<V, E>,
    rel: impl Fn(NodeIndex, NodeIndex, NodeIndex) -> bool,
) -> Vec<(NodeIndex, NodeIndex, NodeIndex)> {
    let product = iproduct!(g.node_indices(), g.node_indices(), g.node_indices());
    product.filter(|(x, y, z)| rel(*x, *y, *z)).collect_vec()
}

pub fn get_quadruples<V, E>(
    g: &Graph<V, E>,
    rel: impl Fn(NodeIndex, NodeIndex, NodeIndex, NodeIndex) -> bool,
) -> Vec<(NodeIndex, NodeIndex, NodeIndex, NodeIndex)> {
    let product = iproduct!(
        g.node_indices(),
        g.node_indices(),
        g.node_indices(),
        g.node_indices()
    );
    product
        .filter(|(x, y, z, w)| rel(*x, *y, *z, *w))
        .collect_vec()
}

pub fn pair_fmap<V, E, Q>(
    g: &Graph<V, E>,
    f: impl Fn(NodeIndex, NodeIndex) -> Option<Q>,
) -> Vec<Q> {
    iproduct!(g.node_indices(), g.node_indices())
        .filter_map(|(x, y)| f(x, y))
        .collect_vec()
}

pub fn triple_fmap<V, E, Q>(
    g: &Graph<V, E>,
    f: impl Fn(NodeIndex, NodeIndex, NodeIndex) -> Option<Q>,
) -> Vec<Q> {
    iproduct!(g.node_indices(), g.node_indices(), g.node_indices())
        .filter_map(|(x, y, z)| f(x, y, z))
        .collect_vec()
}

pub fn quad_fmap<V, E, Q>(
    g: &Graph<V, E>,
    f: impl Fn(NodeIndex, NodeIndex, NodeIndex, NodeIndex) -> Option<Q>,
) -> Vec<Q> {
    iproduct!(
        g.node_indices(),
        g.node_indices(),
        g.node_indices(),
        g.node_indices()
    )
    .filter_map(|(x, y, z, w)| f(x, y, z, w))
    .collect_vec()
}

fn add_pb(g: &mut EGraph) {
    let new_edges: Vec<(NodeIndex, NodeIndex)> = get_pairs(&g, |x, y| -> bool {
        match (&g[x], &g[y]) {
            (EPair(hdl, _, Event::Post(to, sent)), EPair(hdl2, gotten, Event::Get(mid))) => {
                sent == gotten
            }
            _ => false,
        }
    });

    for e in new_edges {
        g.add_edge(e.0, e.1, EdgeTp::PB);
    }
}

fn add_fr(g: &mut EGraph) {
    let new_edges = triple_fmap(&g, |x, y, z| {
        if let Some(e1) = g.edges_connecting(y, x).find(|e| *e.weight() == RF) {
            if let Some(e2) = g.edges_connecting(y, z).find(|e| *e.weight() == CO) {
                return Some((FR, x, z));
            }
        }
        None
    });
    for e in new_edges {
        g.add_edge(e.1, e.2, e.0);
    }
}

fn add_rf(g: &mut EGraph) {
    let new_edges = get_pairs(&g, |x, y| match (&g[x], &g[y]) {
        (EPair(_, _, Event::Write(var, val)), EPair(_, _, Event::Read(var2, val2))) => {
            var == var2 && val == val2
        }
        _ => false,
    });

    for e in new_edges {
        g.add_edge(e.0, e.1, EdgeTp::RF);
    }
}

fn add_co(g: &mut EGraph) {
    return ();
    let new_edges = get_quadruples(&g, |x, y, z, w| match (&g[x], &g[y], &g[z], &g[w]) {
        (epW!(var, val), epW!(var2, val2), epR!(var3, val3), epR!(var4, val4)) => {
            var == var2
                && var2 == var3
                && var3 == var4
                && val == val3
                && val2 == val4
                && val != val2
                && po_rf_path(&g, z, w)
        }
        _ => false,
    });

    for (w1, w2, _, _) in new_edges {
        g.add_edge(w1, w2, EdgeTp::CO);
    }
}
