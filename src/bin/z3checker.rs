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
use petgraph::algo::is_cyclic_directed;
use petgraph::algo::kosaraju_scc;
use petgraph::dot::Dot;
use lib::preprocess::preprocess;
use std::collections::HashSet;
use std::fs;
use std::io;
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
    println!("Preprocessing...");
    instance::construct_instance(&q);


    let done = Instant::now();
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
