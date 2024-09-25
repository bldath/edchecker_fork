use std::fmt::Debug;

use petgraph::{csr::IndexType, data::{Build, FromElements}, graph::{DiGraph, NodeIndex}};

pub type Argument = String;

// #[derive(Clone, Copy, Debug)]
// pub enum OpType {
//     Write,
//     Read,
//     Post,
//     Get,
// }

#[derive(Clone, Debug)]
pub enum Event {
    Write(Argument, Argument),
    Read(Argument, Argument),
    Post(Argument, Argument),
    Get(Argument),
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

#[derive(Clone)]
pub struct EPair(pub Argument, pub Argument, pub Event);

impl Debug for EPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}): {:?}", &self.0, &self.1, &self.2)
    }
}


#[derive(Clone, Copy, Debug)]
pub enum EdgeTp {
    PO,
    CO,
    RF,
    MO,
}

pub type IdxType = u32;

pub type EGraph = DiGraph<EPair, EdgeTp, IdxType>;

#[derive(Clone, Debug)]
pub struct Handler {
    pub id: Argument,
    pub messages: Vec<Message>
}

#[derive(Clone, Debug)]
pub struct EGraphData {
    handlers : Vec<Handler>,
}

pub fn mk_graph(handlers : &Vec<Handler>) -> EGraph {
    let mut d = EGraph::new();
    handlers.iter().for_each(| h | {
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
    return d
}
