use std::collections::{HashMap, HashSet};
use std::io::Write;

use clap::{Parser};
use itertools::Itertools;
use lib::model::EdgeTp;
use lib::model::Event;
use lib::model::ReadResult;
use lib::model::MidStruct;
use lib::output::make_file;
use petgraph::algo::has_path_connecting;
use petgraph::graph::{DiGraph, NodeIndex};
use rand::prelude::*;

use indextree::{Arena, Node, NodeId};

#[test]
fn test_arena() {
    let arena = &mut Arena::new();
    let root = arena.new_node(0);
    let child = arena.new_node(1);
    root.append(child, arena);
    assert_eq!(*arena[root].get(), 0);
    assert_eq!(*arena[child].get(), 1);
}

fn random_pb<R: Rng>(rr: &mut ReadResult, rng: &mut R) -> Arena<(String, MidStruct)> {
    let ReadResult { events, edges, .. } = rr;

    let msgs = events
        .iter()
        .flat_map(|(k, v)| v.iter().map(|(m, _)| (k.clone(), m.clone())))
        .collect_vec();

    let mut post_tree = Arena::new();

    let mut ord = HashSet::new();

    post_tree.new_node(msgs[0].clone());

    for (hdl, msg) in msgs.iter().skip(1) {
        let post_node = post_tree.iter().choose(rng).unwrap();
        let poster = post_node.get().clone();

        let pid = post_tree.get_node_id(post_node).unwrap();

        let e1 = events[&poster.0][&poster.1] 
            .iter()
            .cloned()
            .enumerate()
            .skip(1)
            .filter(|(_, e)| *e == Event::NOOP)
            .collect_vec();

        let e1 = e1.choose(rng).unwrap();

        let evs = events
            .entry(poster.0.clone())
            .or_default()
            .entry(poster.1.clone()) 
            .or_default();
        evs[e1.0] = Event::Post(hdl.clone(), msg.id.clone(), msg.priority.clone());

        ord.insert(((hdl, msg), poster.clone()));
        edges.push((
            EdgeTp::PB,
            (poster.0.clone(), poster.1.clone(), e1.0),
            (hdl.clone(), msg.clone(), 0),
        ));

        let nn = post_tree.new_node((hdl.clone(), msg.clone()));
        pid.append(nn, &mut post_tree);
    }

    post_tree
}

fn unreachable_iter<T>(a: &Arena<T>, node: &Node<T>) -> Box<dyn Iterator<Item = NodeId>> {
    let parent = node.parent();
    let mut siblings = Vec::new();
    let mut tmp: &Node<T> = node;
    while let Some(s) = tmp.next_sibling() {
        siblings.push(s);
        tmp = a.get(s).unwrap();
    }
    if let Some(p) = parent {
        let it = unreachable_iter(a, a.get(p).unwrap());
        Box::new(it.chain(siblings))
    } else {
        Box::new(siblings.into_iter())
    }
}

fn generate_trace(
    num_handlers: usize,
    num_messages: usize,
    num_events: usize,
    remote_edges: usize,
) -> ReadResult {
    let mut events: HashMap<String, HashMap<MidStruct, Vec<Event>>> = HashMap::new();
    let edges = Vec::new();

    assert!(
        num_handlers > 0,
        "Number of handlers must be greater than 0"
    );
    assert!(
        num_messages >= num_handlers,
        "At least one message per handler"
    );
    assert!(
        num_events >= num_messages + 2 * remote_edges,
        "At least one event per message (Get), and two for every remote edge"
    );

    for i in 0..num_handlers {
        let handler_id = format!("h_{}", i);
        let mut handler_msgs = HashMap::new();

        // Evenly distribute messages among handlers
        let n_msgs = num_messages / num_handlers
            + if i < num_messages % num_handlers {
                1
            } else {
                0
            };
        println!("Handler {}: {} messages", handler_id, n_msgs);
        for j in 0..n_msgs {
            let message_id = format!("h_{}_m_{}", i, j);
            let mut message_events = Vec::new();
            let n_events = num_events / n_msgs + if j < num_events % n_msgs { 1 } else { 0 };
            message_events.push(Event::Get(message_id.clone(), None)); //
            for _ in 1..n_events {
                let event = Event::NOOP;
                message_events.push(event);
            }

            handler_msgs.insert(MidStruct {id: message_id, priority: None}, message_events); //
        }

        events.insert(handler_id, handler_msgs);
    }

    let mut rng = rand::rng();

    let mut rr = ReadResult {
        events: events.clone(),
        edges: edges.clone(),
        has_rf: true,
        has_fr: true,
        has_pb: true,
    };

    let mut graph = DiGraph::new();

    let tree = random_pb(&mut rr, &mut rng);

    let mut id_map = HashMap::new();

    tree.iter().for_each(|n| {
        let id = tree.get_node_id(n).unwrap();
        let curr = graph.add_node(id);
        id_map.insert(id, curr);
        if let Some(p) = n.parent() {
            graph.add_edge(id_map[&p], curr, ());
        }
    });

    //Pick two random nodes that are not related
    for i in 0..remote_edges {
        let (n1, n2) = unrelated_nodes(&tree, &mut rng, &id_map, &graph);
        // Less granular when creating random connections
        graph.add_edge(id_map[&n1], id_map[&n2], ());

        let (h1, m1) = tree.get(n1).unwrap().get();
        let (h2, m2) = tree.get(n2).unwrap().get();

        let e1 = events[h1][m1]
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, e)| *e == Event::NOOP)
            .collect_vec();

        let e1 = e1.choose(&mut rng).unwrap();

        let e2 = events[h2][m2]
            .iter()
            .cloned()
            .enumerate()
            .skip(1)
            .filter(|(_, e)| *e == Event::NOOP)
            .collect_vec();

        let e2 = e2.choose(&mut rng).unwrap();
        let var = format!("x_{}", i);
        // Create the remote edge
        // For now just make it RF
        let m1evs = rr
            .events
            .entry(h1.clone())
            .or_default()
            .entry(m1.clone())
            .or_default();
        m1evs[e1.0] = Event::Write(var.clone(), "0".into());

        let m2evs = rr
            .events
            .entry(h2.clone())
            .or_default()
            .entry(m2.clone())
            .or_default();
        m2evs[e2.0] = Event::Read(var, "0".into());

        rr.edges.push((
            EdgeTp::RF,
            (h1.clone(), m1.clone(), e1.0),
            (h2.clone(), m2.clone(), e2.0),
        ));
    }

    //println!("Generated edges:\n{:?}", edges);

    rr
}

fn unrelated_nodes<T>(
    tree: &Arena<T>,
    rng: &mut ThreadRng,
    idmap: &HashMap<NodeId, NodeIndex>,
    graph: &DiGraph<NodeId, ()>,
) -> (NodeId, NodeId) {
    loop {
        if let Some(n1) = tree.iter().choose(rng) {
            if let Some(n2) = unreachable_iter(tree, n1).choose(rng) {
                if has_path_connecting(
                    graph,
                    idmap[&tree.get_node_id(n1).unwrap()],
                    idmap[&n2],
                    None,
                ) {
                    continue;
                } else {
                    return (tree.get_node_id(n1).unwrap(), n2);
                }
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ScalableCLI {
    num_handlers: usize,
    num_messages: usize,
    num_events: usize,
    remote_edges: usize,
    output: String,
}

pub fn main() {
    // Parse CLI arguments
    let cli = ScalableCLI::parse();

    // Generate a trace with the specified number of handlers, messages, and events
    let res: ReadResult = generate_trace(
        cli.num_handlers,
        cli.num_messages,
        cli.num_events,
        cli.remote_edges,
    );

    let str = serde_json::to_string(&res).unwrap();
    let mut file = make_file(cli.output).expect("Unable to create file");
    file.write_all(str.as_bytes())
        .expect("Unable to write data");
}
