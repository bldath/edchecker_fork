use petgraph::graph::NodeIndex;
use petgraph::algo::{has_path_connecting, DfsSpace};
use petgraph::graph::Edge;
use petgraph::visit::{Data, Dfs, GraphBase, GraphRef, IntoEdges, IntoEdgesDirected, IntoNeighbors, VisitMap, Visitable, Walker};
use petgraph::{Graph, IntoWeightedEdge};

use crate::preprocess::quad_fmap;
use crate::{model::EGraph, preprocess::triple_fmap};

use crate::model::EdgeTp::{self, *};


fn get_fr(g : &mut EGraph) -> Vec<(EdgeTp, NodeIndex, NodeIndex)>{
    triple_fmap(&g, |x, y, z| {
        if let Some(e1) = g.edges_connecting(y, x).find(|e| *e.weight() == RF) {
            if let Some(e2) = g.edges_connecting(y, z).find(|e| *e.weight() == CO) {
                return Some((FR, x.clone(), z.clone()))
            }
        }
        None
    })
}

pub fn proj_edges<V, E>(g : &Graph<V, E>, et: E) -> Graph<V, E>
where
    E: Clone + Eq,
    V: Clone,
{
    g.filter_map(|x, n| Some(n.clone()), |e, w| if *w == et { Some(w.clone()) } else { None })
}

pub fn has_edge_weight_path_connecting<V, E>(g: &Graph<V, E>, et: E, src: NodeIndex, dst: NodeIndex) -> bool
where
    V : Clone,
    E : Clone + Eq,
{
    has_path_connecting(&proj_edges(&g, et), src.into(), dst.into(), None)
}

pub fn get_eod(g : &EGraph) -> Vec<(EdgeTp, NodeIndex, NodeIndex)> {
    let g_po = proj_edges(&g, PO);
    quad_fmap(&g, | x, y, z, w | {
        if let Some(yz) = g.edges_connecting(y, z).find(|e| *e.weight() == EO) {
            if has_path_connecting(&g_po, y, x, None) {
                if has_path_connecting(&g_po, z, w, None) {
                    return Some((EOD, x, w))
                }
            }
        }
        None
    })
}

pub fn remove_eo(g : EGraph) -> EGraph {
    g.filter_map(|x, n| Some(n.clone()), |e, w| if *w == EO { None } else { Some(w.clone()) })
}
