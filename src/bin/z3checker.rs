#![allow(unused)]

extern crate lib;

use lib::cli::*;

use lib::algorithms::add_edges;
use clap::ValueEnum;
use lib::do_edges::*;
use lib::eo_edges::eo_cases;
use lib::eo_edges::get_eod;
use lib::eo_edges::missing_eo;
use lib::eo_edges::missing_mo;
use lib::eo_edges::mo_cases;
use lib::eo_edges::remove_eo;
use lib::heuristics::*;
use itertools::Itertools;
use lib::instance;
use lib::model::EGraph;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use lib::model::get_mgraph;
use lib::model::mk_graph;
use lib::model::EGraphData;
use lib::model::EdgeTp;
use lib::model::ReadResult;
use lib::msg_algorithms::extend_valid_multiset;
use lib::msg_algorithms::extend_valid_queue;
use lib::output::*;
use lib::parser::parse_str;
use lib::parser::read_file;
use lib::preprocess;
use petgraph::adj::List;
use petgraph::adj::UnweightedList;
use petgraph::algo::is_cyclic_directed;
use petgraph::algo::kosaraju_scc;
use petgraph::algo::toposort;
use petgraph::algo::tred::dag_to_toposorted_adjacency_list;
use petgraph::algo::tred::dag_transitive_reduction_closure;
use petgraph::dot::Dot;
use lib::preprocess::preprocess;
use petgraph::graph::EdgeIndex;
use petgraph::graph::Node;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::visit::GetAdjacencyMatrix;
use petgraph::visit::IntoEdgeReferences;
use petgraph::visit::IntoEdges;
use petgraph::visit::NodeIndexable;
use z3::Solver;
use std::collections::HashSet;
use std::fs;
use std::io;

use z3::{Context, Config};

use std::time::Instant;

use io::*;

fn run_check(mut g: EGraph, data : &EGraphData, cli : &Cli) -> Option<EGraph> {
    let missing_eo = missing_eo(&g, data);
    let missing_tmp = missing_mo(&g, data);
    let mut missing_mo = vec![];

    for (b, x, y) in &missing_tmp {
        if *b && cli.adt != ADT::Multiset {
            g.add_edge(*x, *y, EdgeTp::MO);
        } else {
            missing_mo.push((*x, *y));
        }
    }

    let mut saved = false;
    let mut i = 0;
    let numcases = i128::pow(2, missing_eo.len() as u32);
    //println!("Missing MO: {:?}", missing_mo.iter().map(|(x, y)| (g[*x].clone(), g[*y].clone())).collect_vec());
    // println!("Missing EO: {:?}", missing_eo.len());
    for (q, mut g) in eo_cases(&g, data, &missing_eo) {
        i += 1;
        match cli.adt {
            ADT::Multiset => {
                let g_multiset = multiset_do(g.clone());
                if !is_cyclic_directed(&g_multiset) {
                    return Some(g_multiset);
                } else {
                    if !saved {
                        let ig = g.clone();
                        //let q = kosaraju_scc(&g).iter().filter(|x| x.len() > 1).map(|x| x.iter().map(|y| ig[*y].clone()).collect_vec()).collect_vec();
                        //println!("Cycles: {:?}", q);
                        let eg = (g.clone(), data.clone());
                        if(cli.draw) {
                            write_dot(&eg, "multiset".into(), "cycle".into()).unwrap();
                        }
                        saved = true;
                    }
                }
            }
            _ => {
                for gp in mo_cases(&g, &missing_mo) {

                    if let Some(q) = match cli.adt {
                        ADT::Queue => Some(queue_do(gp, data)),
                        ADT::Stack => Some(stack_do(gp, data)),
                        ADT::Register => Some(reg_do(gp)),
                        _ => None,
                    } {
                        if !is_cyclic_directed(&q) {
                            return Some(q);
                        } else {
                            if !saved {
                                let ig = g.clone();
                                //let q = kosaraju_scc(&g).iter().filter(|x| x.len() > 1).map(|x| x.iter().map(|y| ig[*y].clone()).collect_vec()).collect_vec();
                                //println!("Cycles: {:?}", q);
                                let eg = (q.clone(), data.clone());
                                if(cli.draw) {
                                    write_dot(&eg, "adt".into(), "cycle".into()).unwrap();
                                }
                                saved = true;
                            }
                        }
                    }
                }
            }
        }
    }
    None
}


fn main() -> Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .init();

    let start = Instant::now();
    let q : ReadResult = read_file(cli.file.clone());

    let parsed = Instant::now();
    
    println!("Handlers: {:?}", q.0.len());
    let num_mess: usize =
        q.0.iter()
            .map(|x| x.1.len())
            .collect_vec()
            .iter()
            .sum();
    println!("Messages: {:?}", num_mess);

    let num_ev : usize =
        q.0.iter()
            .map(|x|
                 x.1.iter()
                    .map(|y| y.1.len()).sum::<usize>()).sum();

    println!("Events: {:?}", num_ev);
    println!("Parsing: {:?}µs", (parsed - start).as_micros());

    let ctx = Context::new(&Config::default());

    let instance = instance::construct_instance(&ctx, &q);

    let solver = Solver::new(&ctx);
    instance.assert(&solver);
    instance.add_do(&solver, cli.adt);
    
    let preprocessed = Instant::now();
    println!("Preprocessing: {:?}µs", (preprocessed - parsed).as_micros());

    match solver.check() {
        z3::SatResult::Unsat => println!("Result: false"),
        z3::SatResult::Unknown => println!("Result: unknown"),
        z3::SatResult::Sat => {
            println!("Result: true");
            if cli.draw {
                let model = solver.get_model().unwrap();
                let func = model.get_func_interp(&instance.order).unwrap();
                //println!("{:?}", func);
                let (mut graph, maps) = mk_graph(&q);

                let indices = &instance.indices;

                for (i, j) in indices.iter().tuple_combinations() {
                    for ((av, a), (bv, b)) in vec![ (i, j), (j, i)] {
                        let ag = maps[&a.0][&a.1][a.2];
                        let bg = maps[&b.0][&b.1][b.2];

                        if graph.contains_edge(ag, bg) {
                            // Ignore existing edges
                            continue;
                        }
                        
                        let ab_eval = model.eval(&instance.order.apply(&[av, bv]).as_bool().unwrap(), true).unwrap();
                        let ab = ab_eval.as_bool().unwrap();

                        if ab {
                            graph.add_edge(ag, bg, EdgeTp::ANY);
                        }
                    }
                }
                

                let _ = graph.edge_count();
                ///println!("Cyclic: {:?}", is_cyclic_directed(&graph));

            
                let toposort = toposort(&graph, None).unwrap();
                let (res, revmap) : (UnweightedList<NodeIndex>, Vec<NodeIndex>) = dag_to_toposorted_adjacency_list(&graph, &toposort);

                let (trans_red, trans_closure) = dag_transitive_reduction_closure(&res);

                println!("Edges in TransRed: {}", trans_red.edge_count());

                for ei in trans_red.edge_indices() {
                    let (a, b) = trans_red.edge_endpoints(ei).unwrap();

                    let ag = revmap[a.index()];
                    let bg = revmap[b.index()];

                    if ! graph.contains_edge(ag, bg) {
                        println!("MISSING EDGE: {:?} -> {:?}", graph[ag], graph[bg]);
                    } else {
                        println!("EXISTING EDGE: {:?} -> {:?}", graph[ag], graph[bg]);
                    }
                }
                /* 
                graph.retain_edges(| g, idx | {
                    let (from, to) = g.edge_endpoints(idx).unwrap();
                    // From/To are indices in the graph
                    let from_idx = revmap.iter().find_position(|x: &&NodeIndex| **x == from).map(|(x,y)| x).unwrap();
                    let to_idx  = revmap.iter().find_position(|x| **x == to).map(|(x, y)| x).unwrap();
                    // From/To are indices in the transitive reduction
                    let a_idx = trans_red.from_index(from_idx);
                    let b_idx = trans_red.from_index(to_idx);

                    trans_red.contains_edge(a_idx, b_idx) //|| g.edge_weight(idx) != Some(&EdgeTp::ANY)
                });
                */
                // Why does this call make the retain happen?
                let nc = graph.node_count();
                println!("Nodes in graph: {}", nc);

                for e in trans_red.edge_indices() {
                    let (src, dst) = trans_red.edge_endpoints(e).unwrap();
                    let (src, dst) = (revmap[src.index()], revmap[dst.index()]);
                    //gp.add_edge(src, dst, EdgeTp::EOD);
                }
                write_dot(&(graph, maps.clone()), "Z3".into(), "ok".into()).unwrap();
            }
        },
    }

    let done = Instant::now();
    let f = fs::File::create("z3_model.smt")?;

    let mut writer = io::BufWriter::new(f);
    write!(writer, "{:?}", solver)?;

    println!("Check: {:?}µs", (done - preprocessed).as_micros());
    println!("Total: {:?}µs", (done - start).as_micros());

    //println!("Result: {:?}", res.is_some());

    // if let Some(q) = res {
    //     if cli.draw {
    //         let eg = (q.clone(), data.clone());
    //         write_dot(&eg, cli.file.clone(), "ok".into())?;
    //     }
    // }



    // println!("{} cases.", n);

    //println!("Result: {:?}", res.is_some());
    // println!("Multiset: {:?}", ms_ok);
    // println!("Queue: {:?}", q_ok);
    // println!("Stack: {:?}", s_ok);
    // println!("Reg: {:?}", r_ok);

    Ok(())
}
