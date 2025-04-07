use itertools::Itertools;
use petgraph::{
    dot::{Config, Dot},
    visit::{EdgeIndexable, EdgeRef, NodeIndexable, NodeRef},
    Graph,
};

use crate::model::*;
use core::fmt;
use std::{
    fs::File,
    io::{self, Error, Write},
};

fn make_dot<A>(graph: &ExecutionGraph, f: &mut A) -> Result<(), Error>
where
    A: Write,
{
    let (g, map) = graph;

    writeln!(f, "digraph {{")?;
    writeln!(f, "rankdir = TB;")?;
    for (handler, messages) in map {
        writeln!(f, "subgraph cluster_{} {{", mk_dot_safe(handler))?;
        writeln!(f, "rankdir = TB;")?;
        let mut h = vec![];
        for (m, evs) in messages {
            writeln!(f, "subgraph cluster_{} {{", mk_dot_safe(m))?;
            writeln!(f, "rankdir = TB;")?;
            for &ev in evs {
                writeln!(
                    f,
                    "{} [label=\"{:?}\"];",
                    NodeIndexable::to_index(g, ev),
                    g[ev]
                )?;
                h.push(ev);
            }
            writeln!(f, "color = \"blue\";")?;
            writeln!(f, "label = \"Message {}\";", mk_dot_safe(m))?;
            writeln!(f, "}}")?;
        }
        writeln!(f, "label = \"Handler {}\";", mk_dot_safe(handler))?;

        writeln!(f, "}}")?;
    }

    //writeln!(f, "edge [constraint=false]")?;
    for edge in g.edge_references() {
        writeln!(
            f,
            "{:?} -> {:?} [label=\"{:?}\", color=\"{}\"];",
            NodeIndexable::to_index(g, edge.source()),
            NodeIndexable::to_index(g, edge.target()),
            g[edge.id()],
            match g.edge_weight(edge.id()).unwrap() {
                EdgeTp::RF => "red",
                EdgeTp::CO => "blue",
                EdgeTp::PO => "green",
                EdgeTp::EO => "magenta",
                EdgeTp::PB => "brown",
                EdgeTp::MO => "brown",
                EdgeTp::DO => "orange",
                EdgeTp::FR => "purple",
                EdgeTp::EOD => "cyan",
                EdgeTp::ANY => "black",
            }
        )?;
    }
    writeln!(f, "}}")?;
    Ok(())
}

pub fn write_dot(graph: &ExecutionGraph, filename: String, suffix: String) -> Result<(), Error> {
    if let Some(basename) = filename.split(".").next() {
        //let gr = Dot::new(&graph);
        //println!("Making dot! {}{}.dot", basename, suffix);
        let mut f = File::create(basename.to_string() + &suffix.to_string() + ".dot")?;
        make_dot(graph, &mut f)
    } else {
        Err(Error::new(
            io::ErrorKind::InvalidInput,
            "Could not modify filename",
        ))
    }
}

pub fn write_graph(graph: &ExecutionGraph, filename: String) -> Result<(), Error> {
    let parent_dir = filename
        .split('/')
        .take_while(|q| !q.contains('.'))
        .join("/");
    std::fs::create_dir_all(parent_dir)?;
    let mut f = File::create(filename)?;

    let (eg, hm) = graph;
    for (handler, msgs) in hm {
        writeln!(f, "@{}", handler)?;
        for (mid, evs) in msgs {
            let evsp = evs
                .iter()
                .take(evs.len() - 1)
                .map(|x| format!("{}", eg[*x].2))
                .collect_vec();
            writeln!(f, "{{ {} }}", evsp.join(" -> "))?;
        }
    }

    for e in eg.edge_indices() {
        let (src, dst) = eg.edge_endpoints(e).unwrap();
        if eg[e] == EdgeTp::CO {
            writeln!(f, "$(CO)")?;
            writeln!(f, "{} -> {}", eg[src].2, eg[dst].2)?;
        }
    }

    Ok(())
}
