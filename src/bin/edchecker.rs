extern crate lib;

use lib::cli::*;

use itertools::Itertools;
use lib::do_edges::*;
use lib::eo_edges::eo_cases;
use lib::eo_edges::missing_eo;
use lib::eo_edges::missing_mo;
use lib::eo_edges::mo_cases;
use lib::model::EGraph;

use clap::Parser;
use lib::model::mk_graph;
use lib::model::EGraphData;
use lib::model::EdgeTp;
use lib::output::*;
use lib::parser::read_file;
use lib::preprocess::preprocess;
use petgraph::algo::is_cyclic_directed;
use std::io;
use std::time::Instant;

use io::*;

/*
NOTES:

pub struct Handler {
    pub id: Argument,
    pub messages: Vec<Message>,
}

pub struct Message {
    pub id: Argument,
    pub evs: Vec<Event>,
}

We see that a handler has a handler ID (Argument is type String) and a list of messages, and
a message has a message ID and a list of events contained in the message (i.e. po from first to last)
We can borrow a struct and extract its fields by dot notation and name (i.e. &msg.id for msg ID, &hdl.msgs[1].id for 2nd msg in hdl).
(Must add .clone() if we need an owned value (i.e. not just for reading it).)

EGraph is type DiGraph<EPair, EdgeTp>, and
Struct EPair(pub Argument, pub Argument, pub Event), where
pub enum Event {
    Write(Argument, Argument),
    Read(Argument, Argument),
    Post(Argument, Argument),
    Get(Argument),
    NOOP,
} and
type Argument is String, and
this is how to create an EPair, so the name is confusing: EPair(hid.clone(), mid.clone(), ev.clone()))
and it's type struct EPair(pub Argument, pub Argument, pub Event), so
extracting the middle field will give the message ID.
Since EPair is a tuple struct where fields don't have names, extraction must be done by position &e.1 (which doesn't work when fields are named).

type HandlerData = HashMap<Argument, Vec<NodeIndex>>

type EGraphData = HashMap<Argument, HandlerData>

type ExecutionGraph = (EGraph, EGraphData)

*/
fn run_check(mut g: EGraph, data: &EGraphData, cli: &Cli) -> Option<EGraph> {
    let missing_eo = missing_eo(&g, data); //Beware outout type. Not filtered by connecting path in graph
    let missing_tmp = missing_mo(&g, data); //Beware output type. Filtered by connecting path in graph
    let mut missing_mo = vec![]; //Beware input type.

    for (b, x, y) in &missing_tmp {
        if *b && cli.adt != ADT::Multiset { //If mo edge exists and mailbox is not Multiset
            g.add_edge(*x, *y, EdgeTp::MO); //Insert missing mo edge in graph
        } else {
            missing_mo.push((*x, *y)); //Else, save the pair of node indexes in vector
        }
    }

    let mut saved = false;
    let _numcases = i128::pow(2, missing_eo.len() as u32);
    //println!("Missing MO: {:?}", missing_mo.iter().map(|(x, y)| (g[*x].clone(), g[*y].clone())).collect_vec());
    // println!("Missing EO: {:?}", missing_eo.len());
    for (_q, g) in eo_cases(&g, data, &missing_eo) {
        match cli.adt {
            ADT::Multiset => {
                let g_multiset = multiset_do(g.clone());
                if !is_cyclic_directed(&g_multiset) {
                    return Some(g_multiset); //return acyclic graph if there is some
                } else if !saved {
                    //let _ig = g.clone();
                    //let q = kosaraju_scc(&g).iter().filter(|x| x.len() > 1).map(|x| x.iter().map(|y| ig[*y].clone()).collect_vec()).collect_vec();
                    //println!("Cycles: {:?}", q);
                    let eg = (g.clone(), data.clone());
                    if cli.draw {
                        write_dot(&eg, "multiset".into(), "cycle".into()).unwrap();
                    }
                    saved = true; //TODO: CHECK THIS BOOL
                }
            }
            _ => {
                for gp in mo_cases(&g, &missing_mo) {
                    if let Some(q) = match cli.adt {
                        ADT::Queue => Some(queue_do(gp, data)),
                        ADT::Stack => Some(stack_do(gp, data)),
                        ADT::Register => Some(reg_do(gp)),
                        ADT::PriorityQueue => Some(priority_queue_do(gp, data)), //ADDED
                        _ => None,
                    } {
                        if !is_cyclic_directed(&q) {
                            return Some(q);
                        } else if !saved {
                            // let ig = g.clone();
                            //let q = kosaraju_scc(&g).iter().filter(|x| x.len() > 1).map(|x| x.iter().map(|y| ig[*y].clone()).collect_vec()).collect_vec();
                            //println!("Cycles: {:?}", q);
                            let eg = (q.clone(), data.clone());
                            if cli.draw {
                                write_dot(&eg, "adt".into(), "cycle".into()).unwrap();
                            }
                            saved = true;
                        }
                    }
                }
            }
        }
    }
    None //Return None if there is no acyclic graph
}

fn main() -> Result<()> {
    let cli = Cli::parse(); //Parse arguments and put them in struct

    env_logger::Builder::new() //Error logging
        .filter_level(cli.verbosity.log_level_filter())
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .init();

    let start = Instant::now(); //Start timer
    let mut q = read_file(cli.file.clone()); //Parse input file and put data in struct
    q.build(); //Compute rf, fr and pb and add to struct

    let (mut g, data) = mk_graph(&q); //Make execution graph from struct
    let parsed = Instant::now(); //Time

    if cli.draw { //if success
        let eg = (g.clone(), data.clone());
        write_dot(&eg, cli.file.clone(), "input".into())?; //Write parsed input graph in dot format
    }

    println!("Handlers: {:?}", q.events.len()); //Print statistics
    let num_mess: usize = q //Calculate statistics
        .events
        .iter()
        .map(|x| x.1.len())
        .collect_vec()
        .iter()
        .sum();
    println!("Messages: {:?}", num_mess);

    let num_ev: usize = q
        .events
        .iter()
        .map(|x| x.1.iter().map(|y| y.1.len()).sum::<usize>())
        .sum();

    println!("Events: {:?}", num_ev);
    println!("Parsing: {:?}µs", (parsed - start).as_micros());
    preprocess(&mut g, &data, cli.heuristics, cli.adt); //Preprocess graph by adding fr, rf and pb edges to it
    let preprocessed = Instant::now(); //Time
    println!("Preprocessing: {:?}µs", (preprocessed - parsed).as_micros()); //Print preprocessing time

    if cli.draw { //if success
        let eg = (g.clone(), data.clone());
        write_dot(&eg, cli.file.clone(), "pp".into())?; //Write preprocessed graph in dot format
    }

    let res = run_check(g, &data, &cli); //Result is EGraph or None
    let done = Instant::now(); //Time
    println!("Check: {:?}µs", (done - preprocessed).as_micros());
    println!("Total: {:?}µs", (done - start).as_micros());

    println!("Result: {:?}", res.is_some());

    if let Some(q) = res { //if result is not None
        if cli.draw { //if success
            let eg = (q.clone(), data.clone());
            write_dot(&eg, cli.file.clone(), "ok".into())?; //Write result graph in dot format
        }
    }

    // println!("{} cases.", n);

    //println!("Result: {:?}", res.is_some());
    // println!("Multiset: {:?}", ms_ok);
    // println!("Queue: {:?}", q_ok);
    // println!("Stack: {:?}", s_ok);
    // println!("Reg: {:?}", r_ok);

    Ok(()) //return value
}
