use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    path,
};

use itertools::iproduct;
use petgraph::{
    algo::has_path_connecting,
    csr::IndexType,
    data::{Build, FromElements},
    graph::{DiGraph, NodeIndex},
    visit::{NodeRef, Visitable},
    EdgeType,
};
use serde::{Deserialize, Serialize};

use crate::{msg_algorithms::transitive_closure, preprocess::get_pairs};

pub type Argument = String;

// #[derive(Clone, Copy, Debug)]
// pub enum OpType {
//     Write,
//     Read,
//     Post,
//     Get,
// }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    Write(Argument, Argument),
    Read(Argument, Argument),
    Post(Argument, Argument),
    Get(Argument),
    NOOP
}

impl Event {
    pub fn variable(&self) -> Option<Argument> {
        match self {
            Event::Write(v, _) => Some(v.clone()),
            Event::Read(v, _) => Some(v.clone()),
            _ => None,
        }
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Write(a, b) => write!(f, "Write({}, {})", mk_dot_safe(a), mk_dot_safe(b)),
            Event::Read(a, b) => write!(f, "Read({}, {})", mk_dot_safe(a), mk_dot_safe(b)),
            Event::Post(a, b) => write!(f, "Post({}, {})", mk_dot_safe(a), mk_dot_safe(b)),
            Event::Get(a) => write!(f, "Get({})", mk_dot_safe(a)),
            Event::NOOP => write!(f, "NOOP"),
        }
    }
}

// impl std::fmt::Debug for Event {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {

//         }
//     }
// }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Argument,
    pub evs: Vec<Event>,
}

pub fn mk_dot_safe(arg: &str) -> String {
    arg.replace(" ", "_")
        .replace("(", "")
        .replace(")", "")
        .replace(".", "_")
        .replace("-", "_")
}

#[derive(Clone, PartialEq, Eq)]
pub struct EPair(pub Argument, pub Argument, pub Event);

impl Debug for EPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}): {}",
            mk_dot_safe(&self.0),
            mk_dot_safe(&self.1),
            &self.2
        )
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

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum EdgeTp {
    RF,
    CO,
    PO,
    EO,
    PB,
    MO,
    DO,
    FR,
    EOD,
    #[default]
    ANY,
}

impl EdgeType for EdgeTp {
    fn is_directed() -> bool {
        true
    }
}

pub type EGraph = DiGraph<EPair, EdgeTp>;

#[derive(Clone, PartialEq, Eq)]
pub struct MGraphE(pub bool, pub NodeIndex, pub Argument);

impl Debug for MGraphE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 {
            write!(f, "Get({})", self.2)
        } else {
            write!(f, "Post({})", self.2)
        }
    }
}

pub type MGraph = DiGraph<MGraphE, ()>;

#[derive(Clone, Debug)]
pub struct Handler {
    pub id: Argument,
    pub messages: Vec<Message>,
}

//pub struct ReadResult(pub Vec<Handler>, pub Vec<(EdgeTp, Event, Event)>);

pub type Idx = (String, String, usize);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadResult {
    pub events: HashMap<String, HashMap<String, Vec<Event>>>,
    pub edges: Vec<(EdgeTp, Idx, Idx)>,
    pub has_rf: bool,
    pub has_fr: bool,
    pub has_pb: bool,
}

impl ReadResult {
    pub fn new(
        events: HashMap<String, HashMap<String, Vec<Event>>>,
        edges: Vec<(EdgeTp, Idx, Idx)>,
    ) -> Self {
        Self {
            events,
            edges,
            has_rf: false,
            has_fr: false,
            has_pb: false,
        }
    }

    pub fn with_rf(mut self) -> Self {
        self.has_rf = true;
        self
    }
    pub fn with_fr(mut self) -> Self {
        self.has_fr = true;
        self
    }
    pub fn with_pb(mut self) -> Self {
        self.has_pb = true;
        self
    }

    pub fn build(&mut self) {
        if !self.has_rf {
            self.compute_rf();
        }

        if !self.has_fr {
            self.compute_fr();
        }

        if !self.has_pb {
            self.compute_pb();
        }

        // PO is implicit in the hashmap.
        // EO/MO are to be guessed
    }

    pub fn compute_rf(&mut self) {
        let mut writers: HashMap<(String, String), Idx> = HashMap::new();
        let mut readers: Vec<(String, String, Idx)> = vec![];
        assert!(!self.has_rf, "RF already computed");
        self.has_rf = true;

        for (hdl, msgs) in self.events.iter() {
            for (mid, evs) in msgs.iter() {
                for (i, ev) in evs.iter().enumerate() {
                    match ev {
                        Event::Write(v, val) => {
                            writers.insert((v.clone(), val.clone()), (hdl.clone(), mid.clone(), i));
                        }
                        Event::Read(v, val) => {
                            readers.push((v.clone(), val.clone(), (hdl.clone(), mid.clone(), i)));
                        }
                        _ => (),
                    }
                }
            }
        }
        for (v, val, ri) in readers.into_iter() {
            if let Some(wi) = writers.get(&(v.clone(), val.clone())) {
                self.edges.push((EdgeTp::RF, wi.clone(), ri));
            }
        }
    }

    pub fn compute_fr(&mut self) {
        let co: Vec<_> = self
            .edges
            .iter()
            .filter(|(et, _, _)| *et == EdgeTp::CO)
            .cloned()
            .collect();
        let rf: Vec<(_, Idx, Idx)> = self
            .edges
            .iter()
            .filter(|(et, _, _)| *et == EdgeTp::RF)
            .cloned()
            .collect();

        assert!(!self.has_fr, "FR already computed");
        self.has_fr = true;

        for (_, a, b) in co.iter() {
            for (_, c, d) in rf.iter() {
                if c != a {
                    continue;
                }
                //We have d --[rf^-1 . co]-> b
                self.edges.push((EdgeTp::FR, d.clone(), b.clone()));
            }
        }
    }

    pub fn compute_pb(&mut self) {
        assert!(!self.has_pb, "PB already computed");
        self.has_pb = true;

        for (hdl, msgs) in &self.events {
            for (mid, evs) in msgs {
                for (i, ev) in evs.iter().enumerate() {
                    if let Event::Post(phdl, pmsg) = ev {
                        //let idx = (hdl.clone(), mid.clone(), i);
                        let idx = (hdl.clone(), mid.clone(), i);
                        self.edges
                            .push((EdgeTp::PB, idx.clone(), (phdl.clone(), pmsg.clone(), 0)));
                    }
                }
            }
        }
    }
}

pub type HandlerData = HashMap<Argument, Vec<NodeIndex>>;
pub type EGraphData = HashMap<Argument, HandlerData>;

pub type ExecutionGraph = (EGraph, EGraphData);

pub fn mk_graph(rr: &ReadResult) -> ExecutionGraph {
    let mut d = EGraph::new();
    let mut hd: HashMap<String, HashMap<String, Vec<NodeIndex>>> = HashMap::new();

    rr.events.iter().for_each(|(hid, msgs)| {
        //let mut hid = Argument::new();
        let mut map = HashMap::<Argument, Vec<NodeIndex>>::new();
        msgs.iter().for_each(|(mid, evs)| {
            let mut mdata = Vec::<NodeIndex>::new();
            let mut last: Option<NodeIndex<u32>> = None;
            evs.iter().for_each(|ev| {
                let n = d.add_node(EPair(hid.clone(), mid.clone(), ev.clone()));
                mdata.push(n);
                if let Some(l) = last {
                    d.add_edge(l, n, EdgeTp::PO);
                }
                last = Some(n);
            });

            map.insert(mid.clone(), mdata);
        });

        hd.insert(hid.clone(), map);
    });

    rr.edges.iter().for_each(|e| {
        let (et, from, to) = e;
        let from = hd[&from.0][&from.1][from.2];
        let to = hd[&to.0][&to.1][to.2];
        d.add_edge(from, to, *et);
    });
    (d, hd)
}

pub fn get_mgraph(g: &EGraph) -> MGraph {
    let mut m = MGraph::new();
    for n in g.node_indices() {
        match &g[n] {
            EPair(hdl1, mid, Event::Get(mid2)) => {
                assert!(mid == mid2);
                m.add_node(MGraphE(true, n, mid.clone()));
            }
            EPair(hdl1, mid, Event::Post(th, mid2)) => {
                m.add_node(MGraphE(false, n, mid2.clone()));
            }
            _ => (),
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
    transitive_closure(&mut m, ());
    m
}
