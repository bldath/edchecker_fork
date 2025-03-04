use std::{collections::HashMap, rc::Rc, sync::Arc};

use itertools::Itertools;
use petgraph::{ graph::{DiGraph, EdgeIndex, Frozen, Neighbors, NodeIndex, NodeIndices}, graphmap::NeighborsDirected, visit::{GraphBase, GraphRef, IntoNeighborsDirected, IntoNodeIdentifiers, Visitable}, Direction, Graph};

use crate::model::{self, EGraph, EPair, EdgeTp};

#[derive(Clone)]
pub struct ExtEGraph {
    graph: Arc<EGraph>,
    extra_edges: Vec<(NodeIndex, NodeIndex)>,
}

impl ExtEGraph {
    pub fn new(g : EGraph) -> ExtEGraph {
        ExtEGraph {
            graph: Arc::new(g),
            extra_edges: vec![],
        }
    }
}

impl GraphBase for ExtEGraph {
    #[doc = r" edge identifier"]
    type EdgeId = EdgeIndex;

    #[doc = r" node identifier"]
    type NodeId = NodeIndex;
}

impl Visitable for ExtEGraph {
    #[doc = r" The associated map type"]
    type Map = <Graph<model::EPair, model::EdgeTp> as Visitable>::Map;

    #[doc = r" Create a new visitor map"]
    fn visit_map(self: &Self) -> Self::Map {
        self.graph.visit_map()
    }

    #[doc = r" Reset the visitor map (and resize to new size of graph if needed)"]
    fn reset_map(self: &Self,map: &mut Self::Map) {
        self.graph.reset_map(map);
    }
}

impl IntoNodeIdentifiers for &ExtEGraph {
    type NodeIdentifiers = NodeIndices;

    fn node_identifiers(self) -> Self::NodeIdentifiers {
        self.graph.node_indices()
    }
}


impl<'a> IntoNeighborsDirected for &ExtEGraph {
    type NeighborsDirected = Neighbors<'a, EdgeTp, NodeIndex>;

    fn neighbors_directed(self,n:Self::NodeId,d:Direction) -> Self::NeighborsDirected {
        let q = self.graph.neighbors_directed(n, d);
        let qp = self.extra_edges
            .iter()
            .filter_map(|(u,v)| if (*u == n) { Some(*v) } else { None })
            .chain(q)
            .unique()
            .collect_vec();
    }
}
