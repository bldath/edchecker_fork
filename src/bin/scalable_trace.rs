use std::collections::HashMap;
use std::io::Write;

use itertools::Itertools;
use lib::model::EdgeTp;
use lib::output::make_file;
use rand::prelude::*;

use clap::{command, Parser};
use lib::model::ReadResult;
use lib::model::Event;


fn generate_trace(num_handlers: usize, num_messages: usize, num_events: usize, remote_edges: usize) -> ReadResult {
    let mut events: HashMap<String, HashMap<String, Vec<Event>>> = HashMap::new();
    let mut edges = Vec::new();

    assert!(num_handlers > 0, "Number of handlers must be greater than 0");
    assert!(num_messages >= num_handlers, "At least one message per handler");
    assert!(num_events >= num_messages + 2 * remote_edges, "At least one event per message (Get), and two for every remote edge");

    for i in 0..num_handlers {
        let handler_id = format!("h_{}", i);
        let mut handler_msgs = HashMap::new();
        
        // Evenly distribute messages among handlers
        let n_msgs = num_messages / num_handlers + if i < num_messages % num_handlers { 1 } else { 0 };
        println!("Handler {}: {} messages", handler_id, n_msgs);
        for j in 0..n_msgs {
            let message_id = format!("h_{}_m_{}", i, j);
            let mut message_events = Vec::new();
            let n_events = num_events / n_msgs + if j < num_events % n_msgs { 1 } else { 0 };
            message_events.push(Event::Get(message_id.clone()));
            for _ in 1..n_events {
                let event = Event::NOOP;
                message_events.push(event);
            }

            handler_msgs.insert(message_id, message_events);
        }

        events.insert(handler_id, handler_msgs);
    }

    let mut rng = rand::rng();
    // Add remote edges
    for i in 0..remote_edges {

        // Randomly select two messages from different handlers
        let mut hdls = events.keys().cloned().collect::<Vec<_>>();
        hdls.shuffle(&mut rng);
        let [h1, h2, ..] = hdls.as_slice() else { panic!("Not enough handlers") };
        let h1 = h1.clone();
        let h2 = h2.clone();

        let m1 = events[&h1].keys().cloned().choose(&mut rng).unwrap();
        let m2 = events[&h2].keys().cloned().choose(&mut rng).unwrap();

        // Ensure they are different handlers
        let e1 = events[&h1][&m1]
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, e)| *e == Event::NOOP).collect_vec();

        let e1 = e1.choose(&mut rng).unwrap();

        let e2 = events[&h2][&m2]
            .iter()
            .cloned()
            .enumerate()
            .skip(1)
            .filter(|(_, e)| *e == Event::NOOP).collect_vec();

        let e2 = e2.choose(&mut rng).unwrap();
        let var = format!("x_{}", i);
        // Create the remote edge
        // For now just make it RF
        let m1evs = events.entry(h1.clone()).or_default().entry(m1.clone()).or_default();
        m1evs[e1.0] = Event::Write(var.clone(), "0".into());

        let m2evs = events.entry(h2.clone()).or_default().entry(m2.clone()).or_default();
        m2evs[e2.0] = Event::Read(var, "0".into());

        edges.push((EdgeTp::RF, (h1.clone(), m1.clone(), e1.0), (h2.clone(), m2.clone(), e2.0)));  
    }

    let msgs = events.iter().flat_map(|(hdl, msgs)| {
        msgs.keys().map(|k| (hdl.clone(), k.clone()))
    }).collect_vec();
    
    for (hdl, msg) in msgs.iter() {
        let poster : String = events.keys().cloned().choose(&mut rng).unwrap();
        let poster_msg : String = events[&poster].keys().filter(|x| *x != msg).cloned().choose(&mut rng).unwrap();
        
        let e1 = events[&poster][&poster_msg]
            .iter()
            .cloned()
            .enumerate()
            .skip(1)
            .filter(|(_, e)| *e == Event::NOOP).collect_vec();

        let poster_evs = events.entry(poster.clone()).or_default().entry(poster_msg.clone()).or_default();
        let e1 = e1.choose(&mut rng).unwrap();
        poster_evs[e1.0] = Event::Post(hdl.clone(), msg.clone());

        edges.push((EdgeTp::PB, (poster.clone(), poster_msg.clone(), e1.0), (hdl.clone(), msg.clone(), 0)));
    }

    //println!("Generated edges:\n{:?}", edges);
    
    ReadResult {
        events,
        edges,
        has_rf: true,
        has_fr: true,
        has_pb: true,
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
    let res: ReadResult = generate_trace(cli.num_handlers, cli.num_messages, cli.num_events, cli.remote_edges);

    let str = serde_json::to_string(&res).unwrap();
    let mut file = make_file(cli.output).expect("Unable to create file");
    file.write_all(str.as_bytes())
        .expect("Unable to write data");
}