use petgraph::{algo::has_path_connecting, graph::NodeIndex, visit::{Dfs, EdgeFiltered, EdgeRef}};

use crate::model::EGraph;
use crate::model::EdgeTp::*;

// pub fn po_rf_graph(g : &EGraph) -> EGraph {
//     EdgeFiltered::from_fn(&g, |e| {
//         vec![PO, RF].contains(e.weight())
//     }).into()
// }

pub fn po_rf_path(g : &EGraph, a : NodeIndex, b: NodeIndex) -> bool {
    let fg = EdgeFiltered::from_fn(&g, |e_ref| {
        vec![PO, RF].contains(e_ref.weight())
    });

    has_path_connecting(&fg, a.into(), b.into(), None)
}
