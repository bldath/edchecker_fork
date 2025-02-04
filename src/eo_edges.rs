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
use crate::model::{get_mgraph, Argument, EGraphData, EPair, Event};
use crate::msg_algorithms::{flip_iter, flip_iterator};
use crate::preprocess::{pair_fmap, quad_fmap};
use crate::{model::EGraph, preprocess::triple_fmap};

use crate::model::EdgeTp::{self, *};

pub fn mo_cases<'a>(
    g: &'a EGraph,
    missing: &'a Vec<(NodeIndex, NodeIndex)>,
) -> impl Iterator<Item = EGraph> + 'a {
    flip_iter(&missing).map(|q| {
        let mut gp = g.clone();
        for (q1, q2) in q {
            gp.add_edge(q1, q2, MO);
        }
        gp
    })
}

pub fn eo_cases<'a>(
    g: &'a EGraph,
    data: &'a EGraphData,
    missing: &'a Vec<(Argument, Argument, Argument)>,
) -> impl Iterator<Item = EGraph> + 'a {
    flip_iterator(&missing).map(move |q| {
        let mut gp = g.clone();
        for (hdl, q1, q2) in q {
            insert_eo(&mut gp, data, hdl, q1, q2);
        }
        gp
    })
}

pub fn insert_forced_eo(g: &mut EGraph) {
    let q = pair_fmap(&g, |x, y| match (&g[x], &g[y]) {
        (EPair(hdl1, mid1, Event::Get(mid1p)), EPair(hdl2, mid2, Event::Done(mid2p))) => {
            if hdl1 == hdl2 && mid1 != mid2 && has_path_connecting(&g.clone(), x, y, None) {
                Some((EO, x, y))
            } else {
                None
            }
        }
        _ => None,
    })
    .into_iter()
    .unique()
    .collect_vec();

    // let ne = pair_fmap(&g, | x, y | {
    //     match (&g[x], &g[y]) {
    //         (EPair(hdl1, mid1, e1), EPair(hdl2, mid2, e2)) => {
    //             if q.contains(&(EO, mid1.clone(), mid2.clone())) {
    //                 Some((EO, x, y))
    //             } else { None }
    //         }
    //     }
    // });
    println!("Adding EO: {:?}", q);
    add_edges(g, q);
}

pub fn insert_forced_mo(g: &mut EGraph) {
    let q = pair_fmap(&g, |x, y| match (&g[x], &g[y]) {
        (EPair(hdl1, mid1, Event::Post(rcv1, rm1)), EPair(hdl2, mid2, Event::Post(rcv2, rm2))) => {
            if rcv1 == rcv2 && x != y && has_path_connecting(&g.clone(), x, y, None) {
                Some((MO, x.clone(), y.clone()))
            } else {
                None
            }
        }
        _ => None,
    })
    .into_iter()
    .unique()
    .collect_vec();

    println!("Adding MO: {:?}", q);

    add_edges(g, q);
}

pub fn missing_eo(g: &EGraph, data : &EGraphData) -> Vec<(Argument, Argument, Argument)> {
    data.iter().flat_map(| (hdl, msgs) | {
        msgs.iter().tuple_combinations().filter_map(| ((m1, m1e), (m2, m2e)) | {
            if m1 != m2 {
                if !g.contains_edge(*m1e.last().unwrap(), *m2e.first().unwrap()) && !g.contains_edge(*m2e.last().unwrap(), *m1e.first().unwrap()) {
                    return Some((hdl.clone(), m1.clone(), m2.clone()))
                }
            }
            None
        })
    }).collect_vec()
    // pair_fmap(&g, |x, y| match (&g[x], &g[y]) {
    //     (EPair(hdl1, mid1, e1), EPair(hdl2, mid2, e2)) => {
    //         if hdl1 == hdl2
    //             && mid1 < mid2
    //             && !has_path_connecting(&g.clone(), x, y, None)
    //             && !has_path_connecting(&g.clone(), y, x, None)
    //         {
    //             Some((mid1.clone(), mid2.clone()))
    //         } else {
    //             None
    //         }
    //     }
    //     _ => None,
    // })
    // .into_iter()
    // .unique()
    // .collect_vec()
}

pub fn missing_mo(g: &EGraph) -> Vec<(NodeIndex, NodeIndex)> {
    pair_fmap(&g, |x, y| match (&g[x], &g[y]) {
        (EPair(hdl1, mid1, Event::Post(r1, rm1)), EPair(hdl2, mid2, Event::Post(r2, rm2))) => {
            if r1 == r2 && rm1 < rm2 && !g.contains_edge(x, y) && !g.contains_edge(y, x) {
                Some((x, y))
            } else {
                None
            }
        }
        _ => None,
    })
    .into_iter()
    .unique()
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
    triple_fmap(&g, |x, y, z| {
        if let Some(e1) = g.edges_connecting(y, x).find(|e| *e.weight() == RF) {
            if let Some(e2) = g.edges_connecting(y, z).find(|e| *e.weight() == CO) {
                return Some((FR, x.clone(), z.clone()));
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
    has_path_connecting(&proj_edges(&g, et), src.into(), dst.into(), None)
}

pub fn get_eod(g: &EGraph) -> Vec<(EdgeTp, NodeIndex, NodeIndex)> {
    let g_po = proj_edges(&g, PO);
    quad_fmap(&g, |x, y, z, w| {
        if let Some(yz) = g.edges_connecting(y, z).find(|e| *e.weight() == EO) {
            if has_path_connecting(&g_po, y, x, None) {
                if has_path_connecting(&g_po, z, w, None) {
                    return Some((EOD, x, w));
                }
            }
        }
        None
    })
}

pub fn remove_eo(g: EGraph) -> EGraph {
    g.filter_map(
        |x, n| Some(n.clone()),
        |e, w| if *w == EO { None } else { Some(w.clone()) },
    )
}
