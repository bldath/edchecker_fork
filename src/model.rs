use std::{fmt::Debug, path};

use itertools::iproduct;
use petgraph::{algo::has_path_connecting, csr::IndexType, data::{Build, FromElements}, graph::{DiGraph, NodeIndex}, visit::{NodeRef, Visitable}, EdgeType};

use crate::{msg_algorithms::transitive_closure, preprocess::get_pairs};


pub type Argument = String;

// #[derive(Clone, Copy, Debug)]
// pub enum OpType {
//     Write,
//     Read,
//     Post,
//     Get,
// }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Write(Argument, Argument),
    Read(Argument, Argument),
    Post(Argument, Argument),
    Get,
}


// impl std::fmt::Debug for Event {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {

//         }
//     }
// }


#[derive(Clone, Debug)]
pub struct Message {
    pub id: Argument,
    pub evs : Vec<Event>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct EPair(pub Argument, pub Argument, pub Event);

impl Debug for EPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}): {:?}", &self.0, &self.1, &self.2)
    }
}

#[macro_export]
macro_rules! epW {
    ($var:ident, $val:ident) => {
        EPair(_, _, Event::Write($var, $val))
    };
}

#[macro_export]
macro_rules! epR {
    ($var:ident, $val:ident) => {
        EPair(_, _, Event::Read($var, $val))
    };
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeTp {
    RF,
    CO,
    PO,
    EO,
    PB,
    MO,
}

impl EdgeType for EdgeTp {
    fn is_directed() -> bool {
        true
    }
}

pub type EGraph = DiGraph<EPair, EdgeTp>;


#[derive(Clone, PartialEq, Eq)]
pub struct MGraphE(pub bool,pub NodeIndex,pub Argument);

impl Debug for MGraphE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 {
            write!(f, "Get({:?})", self.2)
        } else {
            write!(f, "Post({:?})", self.2)
        }
    }
}

pub type MGraph = DiGraph<MGraphE, ()>;

#[derive(Clone, Debug)]
pub struct Handler {
    pub id: Argument,
    pub messages: Vec<Message>
}

#[derive(Clone, Debug)]
pub struct EGraphData {
    handlers : Vec<Handler>,
}

pub struct ReadResult(pub Vec<Handler>, pub Vec<(EdgeTp, Event, Event)>);

pub fn mk_graph(rr: &ReadResult) -> EGraph {
    let ReadResult(hdl, edges) = rr;
    let mut d = EGraph::new();
    hdl.iter().for_each(| h | {
        let mut last : Option<NodeIndex<u32>> = None;
        h.messages.iter().for_each(| msg | {
            let mut last : Option<NodeIndex<u32>> = None;
            msg.evs.iter().for_each(| ev | {
                let n = d.add_node(EPair(h.id.clone(), msg.id.clone(), ev.clone()));
                if let Some(l) = last {
                    d.add_edge(l, n, EdgeTp::PO);
                }
                last = Some(n);
            });
        });
    });

    edges.iter().for_each(| e | {
        let (et, from, to) = e;
        if let Some(f) =  d.node_indices().find(|x| &d[*x].2 == from) {
            if let Some(t) = d.node_indices().find(|x| &d[*x].2 == to) {
                d.add_edge(f, t, et.clone());
            } else { println!("Could not find event {:?} in graph:\n{:?}", to, d); }
        } else { println!("Could not find event {:?} in graph:\n{:?}", from, d); }
    });
    d
}

pub fn get_mgraph(g : &EGraph) -> MGraph {
    let mut m = MGraph::new();
    for n in g.node_indices() {
        match &g[n] {
            EPair(hdl1, mid, Event::Get) => {
                m.add_node(MGraphE(true, n, mid.clone()));
            },
            EPair(hdl1, mid, Event::Post(th, mid2)) => {
                m.add_node(MGraphE(false, n, mid2.clone()));
            }
            _ => ()
        }
    }

    for (p1, p2) in iproduct!(m.node_indices(), m.node_indices()) {
        if p1 != p2 {
            let MGraphE(b1, n1, s1) = &m[p1];
            let MGraphE(b2, n2, s2) = &m[p2];
            if has_path_connecting(g, *n1, *n2, None) {
                m.add_edge(p1, p2, ());
            }
        }
    }
    transitive_closure(&mut m);
    m
}
