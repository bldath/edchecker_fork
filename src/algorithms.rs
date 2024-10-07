use std::{collections::HashSet, ptr::addr_eq};

use itertools::{iproduct, Itertools};
use petgraph::{adj::EdgeIndex, algo::has_path_connecting, graph::{EdgeReference, NodeIndex}, visit::{Dfs, EdgeFiltered, EdgeRef, IntoEdgesDirected, IntoNeighbors, NodeRef}, Direction::Outgoing};

use crate::{model::{Argument, EGraph, EPair, EdgeTp, Event}, preprocess::get_pairs};
use crate::model::EdgeTp::*;
// pub fn po_rf_graph(g : &EGraph) -> EGraph {
//     EdgeFiltered::from_fn(&g, |e| {
//         vec![PO, RF].contains(e.weight())
//     }).into()
// }

pub fn add_edges<I>(g : &mut EGraph, it : I) where I : IntoIterator<Item = (EdgeTp, NodeIndex, NodeIndex)>{
    for (et, from, to) in it {
        g.add_edge(from, to, et);
    }
}


pub fn po_rf_path(g : &EGraph, a : NodeIndex, b: NodeIndex) -> bool {
    let fg = EdgeFiltered::from_fn(&g, |e_ref| {
        vec![PO, RF].contains(e_ref.weight())
    });

    has_path_connecting(&fg, a.into(), b.into(), None)
}


fn missing_co(g : &EGraph) -> Vec<(EdgeTp, NodeIndex, NodeIndex)> {
    get_pairs(g, | x, y | {
        x < y &&
        match (&g[x], &g[y]) {
            (EPair(_, _, Event::Write(var1, val1)), EPair(_, _, Event::Write(var2, val2))) => {
                var1 == var2 && val1 != val2 && (!g.contains_edge(x, y) && !g.contains_edge(y,x))
            },
            _ => false
        }
    }).iter().map(|(x, y)| (CO, *x, *y)).collect_vec()
}

pub fn get_missing_totality(g : &EGraph) -> Vec<(EdgeTp, NodeIndex, NodeIndex)> {
    missing_co(g)
}

pub fn try_extend(g : &mut EGraph, e : (EdgeTp, NodeIndex, NodeIndex)) {
    let (et, x, y) = e;

    if has_path_connecting(&*g, x, y, None) {
        g.add_edge(x, y, et);
    } else if has_path_connecting(&*g, y, x, None) {
        g.add_edge(y, x, et);
    } else { println!("Cannot infer direction of edge: {:?}", e) }
}


pub fn last_evt(g : &EGraph, mid: &Argument) -> Option<NodeIndex> {
    let mut mevs = g.node_indices().filter(|x| &g[*x].1 == mid);

    let q = mevs.find(|m| { g.edges_directed(*m, Outgoing).filter(|x| { *x.weight() == PO }).collect_vec().len() == 0 });

    q
}

fn proj_graph(g: &EGraph, mevs: Vec<NodeIndex>, vec: Vec<EdgeTp>) -> EGraph {
    let q = g.filter_map(|x, et| { if mevs.contains(&x) { Some(et.clone()) } else { None } },
                 |x, et| { if vec.contains(et) { Some(et.clone()) } else { None } });
    q
}

// pub fn add_eo2(g : &mut EGraph, mid1 : &Argument, mid2 : &Argument) {

//     if let Some(e1) = last_evt(g, mid1) {
//         if let Some(e2) = g.node_indices().find(|x| {
//             match &g[*x] {
//                 EPair(hdl1, m1, Event::Get) => {
//                     m1 == mid2
//                 },
//                 _ => false
//             }
//         }) {
//             g.add_edge(e1, e2, EO);
//         }
//     }
// }

// pub fn add_eo(g : &mut EGraph, mid1 : &Argument, mid2 : &Argument) {
//     let message_1 = g.node_indices().filter(|x| &g[*x].1 == mid1).map(|x| x.clone()).collect_vec();
//     let message_2 = g.node_indices().filter(|x| &g[*x].1 == mid2).map(|x| x.clone()).collect_vec();
//     for (q1, q2) in iproduct!(message_1, message_2) {
//         g.add_edge(q1, q2, EO);
//     }
// }


// pub fn deduce_eo( g : &mut EGraph ) {
//     let q = get_pairs(g, |x, y| {
//         &g[x].0 == &g[y].0 && // Same handler
//         &g[x].1 != &g[y].1 && // Different message
//         has_path_connecting(&*g, x, y, None) // There is a path!
//     });

//     let q2 = Itertools::unique_by(q.iter(), | a | {
//         let (a1, a2) = a;
//         (g[*a1].1.clone(), g[*a2].1.clone())
//     }).collect_vec();

//     for (x, y) in q2 {
//         let x1 = &g[*x].1.clone();
//         let y1 = &g[*y].1.clone();
//         println!("Deduced EO: ({:?}, {:?})", x1, y1);
//         add_eo2(g, x1, y1);
//     }
// }
