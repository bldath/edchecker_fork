use std::collections::HashSet;

use itertools::Itertools;
use petgraph::algo::{has_path_connecting, DfsSpace};
use petgraph::data::Build;
use petgraph::graph::Edge;
use petgraph::graph::NodeIndex;
use petgraph::visit::{
    Data, Dfs, GraphBase, GraphRef, IntoEdges, IntoEdgesDirected, IntoNeighbors, VisitMap,
    Visitable, Walker,
};
use petgraph::{Graph, IntoWeightedEdge};

use crate::algorithms::add_edges;
use crate::cli::*;
use crate::do_edges::get_post;
use crate::model::{get_mgraph, Argument, EGraphData, EPair, Event};
use crate::msg_algorithms::{flip_iter, flip_iterator};
use crate::preprocess::{pair_fmap, quad_fmap};
use crate::{model::EGraph, preprocess::triple_fmap};

use crate::model::EdgeTp::{self, *};

pub fn mo_cases<'a>(
    g: &'a EGraph,
    missing: &'a [(NodeIndex, NodeIndex)],
) -> impl Iterator<Item = EGraph> + 'a {
    flip_iter(missing).map(|q| {
        let mut gp = g.clone();
        for (q1, q2) in q {
            gp.add_edge(q1, q2, MO);
        }
        gp
    })
}

// pub fn make_guesses<'a>(
//     g: &'a EGraph,
//     data: &'a EGraphData,
//     heur: Heuristic,
//     adt: ADT
// ) {
//     let mis = missing_eo(g, data);
//     if let Some(((hdl, m1, m2), b)) = mis.split_first() {
//         let mut g1 = g.clone();
//         insert_eo(&mut g1.clone(), data, hdl.clone(), m1.clone(), m2.clone());
//         add_heuristics(&mut g1, data, heur, adt);

//         let g2 = g.clone();
//         insert_eo(&mut g2.clone(), data, hdl.clone(), m2.clone(), m1.clone());
//         add_heuristics(&mut g2, data, heur, adt);

//     }
// }

pub fn eo_cases<'a>(
    g: &'a EGraph,
    data: &'a EGraphData,
    missing: &'a [(Argument, Argument, Argument)],
) -> impl Iterator<Item = (Vec<(Argument, Argument, Argument)>, EGraph)> + 'a {
    flip_iterator(missing).map(move |q| {
        let mut gp = g.clone();
        for (hdl, q1, q2) in &q {
            insert_eo(&mut gp, data, hdl.clone(), q1.clone(), q2.clone());
        }
        (q.clone(), gp)
    })
}

pub fn missing_eo(g: &EGraph, data: &EGraphData) -> Vec<(Argument, Argument, Argument)> {
    let mut q: HashSet<(Argument, Argument, Argument)> = data
        .iter()
        .flat_map(|(hdl, msgs)| {
            msgs.iter()
                .tuple_combinations()
                .filter_map(|((m1, m1e), (m2, m2e))| {
                    if m1 != m2
                        && !g.contains_edge(*m1e.last().unwrap(), *m2e.first().unwrap())
                        && !g.contains_edge(*m2e.last().unwrap(), *m1e.first().unwrap())
                    {
                        return Some((hdl.clone(), m1.clone(), m2.clone()));
                    }
                    None
                })
        })
        .collect();

    q.into_iter().collect_vec()
}

pub fn missing_mo(g: &EGraph, data: &EGraphData) -> Vec<(bool, NodeIndex, NodeIndex)> {
    data.iter()
        .flat_map(|(hdl, msgs)| {
            msgs.iter()
                .tuple_combinations()
                .filter_map(|((m1, e1), (m2, e2))| {
                    let m1get = *e1.first().unwrap();
                    let m1done = *e1.last().unwrap();

                    let m2get = *e2.first().unwrap();
                    let m2done = *e2.last().unwrap();

                    if let (Some(m1post), Some(m2post)) = (get_post(g, m1get), get_post(g, m2get)) {
                        if has_path_connecting(g, m1post, m2post, None) {
                            Some((true, m1post, m2post))
                        } else if has_path_connecting(g, m2post, m1post, None) {
                            Some((true, m2post, m1post))
                        } else {
                            Some((false, m1post, m2post))
                        }
                    } else {
                        None
                    }
                })
        })
        .collect_vec()
}

fn insert_eo(g: &mut EGraph, data: &EGraphData, hdl: Argument, m1: Argument, m2: Argument) {
    let m1 = data[&hdl][&m1].last().unwrap();
    let m2 = data[&hdl][&m2].first().unwrap();

    g.add_edge(*m1, *m2, EO);

    // let q = pair_fmap(&g, |x, y| match (&g[x], &g[y]) {
    //     (EPair(hdl1, mid1, e1), EPair(hdl2, mid2, e2)) => {
    //         if hdl1 == hdl2 && *mid1 == m1 && *mid2 == m2 {
    //             Some((EO, x, y))
    //         } else {
    //             None
    //         }
    //     }
    //     _ => None,
    // });

    // add_edges(g, q);
}

fn get_fr(g: &mut EGraph) -> Vec<(EdgeTp, NodeIndex, NodeIndex)> {
    triple_fmap(g, |x, y, z| {
        if let Some(e1) = g.edges_connecting(y, x).find(|e| *e.weight() == RF) {
            if let Some(e2) = g.edges_connecting(y, z).find(|e| *e.weight() == CO) {
                return Some((FR, x, z));
            }
        }
        None
    })
}

pub fn proj_edges<V, E>(g: &Graph<V, E>, et: E) -> Graph<V, E>
where
    E: Clone + Eq,
    V: Clone,
{
    g.filter_map(
        |x, n| Some(n.clone()),
        |e, w| if *w == et { Some(w.clone()) } else { None },
    )
}

pub fn has_edge_weight_path_connecting<V, E>(
    g: &Graph<V, E>,
    et: E,
    src: NodeIndex,
    dst: NodeIndex,
) -> bool
where
    V: Clone,
    E: Clone + Eq,
{
    has_path_connecting(&proj_edges(g, et), src, dst, None)
}

pub fn get_eod(g: &EGraph) -> Vec<(EdgeTp, NodeIndex, NodeIndex)> {
    let g_po = proj_edges(g, PO);
    quad_fmap(g, |x, y, z, w| {
        if let Some(yz) = g.edges_connecting(y, z).find(|e| *e.weight() == EO) {
            if has_path_connecting(&g_po, y, x, None) && has_path_connecting(&g_po, z, w, None) {
                return Some((EOD, x, w));
            }
        }
        None
    })
}

pub fn remove_eo(g: EGraph) -> EGraph {
    g.filter_map(
        |x, n| Some(n.clone()),
        |e, w| if *w == EO { None } else { Some(*w) },
    )
}
