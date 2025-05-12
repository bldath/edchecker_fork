extern crate lib;

use lib::cli::*;

use itertools::Itertools;
use lib::instance;

use clap::Parser;
use lib::instance::Instance;
use lib::model::mk_graph;
use lib::model::EdgeTp;
use lib::model::ReadResult;
use lib::output::*;
use lib::parser::read_file;

use petgraph::adj::UnweightedList;
use petgraph::algo::toposort;
use petgraph::algo::tred::dag_to_toposorted_adjacency_list;
use petgraph::algo::tred::dag_transitive_reduction_closure;
use petgraph::graph::NodeIndex;
use std::io;
use z3::Solver;

use z3::{Config, Context};

use std::time::Instant;

use io::*;

fn print_result(res: z3::SatResult, instance: &Instance, solver: &Solver, q: &ReadResult) {
    match res {
        z3::SatResult::Unsat => println!("Result: false"),
        z3::SatResult::Unknown => println!("Result: unknown"),
        z3::SatResult::Sat => {
            let model = solver.get_model().unwrap();

            let (mut graph, maps) = mk_graph(q);

            let indices = &instance.indices;

            for (i, j) in indices.iter().tuple_combinations() {
                for ((av, a), (bv, b)) in [(i, j), (j, i)] {
                    let ag = maps[&a.0][&a.1][a.2];
                    let bg = maps[&b.0][&b.1][b.2];

                    if graph.contains_edge(ag, bg) {
                        // Ignore existing edges
                        continue;
                    }

                    let ab_eval = model
                        .eval(&instance.order.apply(&[av, bv]).as_bool().unwrap(), true)
                        .unwrap();
                    let ab = ab_eval.as_bool().unwrap();

                    if ab {
                        graph.add_edge(ag, bg, EdgeTp::ANY);
                    }
                }
            }

            let _ = graph.edge_count();
            //println!("Cyclic: {:?}", is_cyclic_directed(&graph));
            let toposort = toposort(&graph, None).unwrap();
            let (res, revmap): (UnweightedList<NodeIndex>, Vec<NodeIndex>) =
                dag_to_toposorted_adjacency_list(&graph, &toposort);

            let (trans_red, _trans_closure) = dag_transitive_reduction_closure(&res);

            println!("Edges in TransRed: {}", trans_red.edge_count());

            for ei in trans_red.edge_indices() {
                let (a, b) = trans_red.edge_endpoints(ei).unwrap();

                let _ag = revmap[a.index()];
                let _bg = revmap[b.index()];

                // if !graph.contains_edge(ag, bg) {
                //     println!("MISSING EDGE: {:?} -> {:?}", graph[ag], graph[bg]);
                // } else {
                //     println!("EXISTING EDGE: {:?} -> {:?}", graph[ag], graph[bg]);
                // }
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

            // for e in trans_red.edge_indices() {
            //     let (src, dst) = trans_red.edge_endpoints(e).unwrap();
            //     let (src, dst) = (revmap[src.index()], revmap[dst.index()]);
            //     //gp.add_edge(src, dst, EdgeTp::EOD);
            // }
            write_dot(&(graph, maps.clone()), "Z3".into(), "ok".into()).unwrap();
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .init();

    let start = Instant::now();
    let mut q: ReadResult = read_file(cli.file.clone());
    q.build();

    let parsed = Instant::now();

    println!("Handlers: {:?}", q.events.len());
    let num_mess: usize = q
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

    if num_ev == 0 {
        println!("Empty trace");
        return Ok(());
    }

    println!("Parsing: {:?}µs", (parsed - start).as_micros());

    let ctx = Context::new(&Config::default());

    let q_render = if cli.draw { Some(q.clone()) } else { None };

    if let Some(ref q) = q_render {
        let (g, data) = mk_graph(q);
        if cli.draw {
            let eg = (g.clone(), data.clone());
            write_dot(&eg, cli.file.clone(), "input".into())?;
        }
    }

    let instance = instance::construct_instance(&ctx, q);
    let solver = Solver::new(&ctx);
    instance.assert(&solver);
    instance.add_do(&solver, cli.adt);

    let preprocessed = Instant::now();
    println!("Preprocessing: {:?}µs", (preprocessed - parsed).as_micros());

    let res = solver.check();
    let done = Instant::now();

    // Save the model to a smt file
    // let f = fs::File::create("z3_model.smt")?;
    // let mut writer = io::BufWriter::new(f);
    // write!(writer, "{:?}", solver)?;

    println!("Check: {:?}µs", (done - preprocessed).as_micros());
    println!("Total: {:?}µs", (done - start).as_micros());

    println!("Result: {:?}", res == z3::SatResult::Sat);

    if let Some(q) = q_render {
        print_result(res, &instance, &solver, &q);
    }

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
