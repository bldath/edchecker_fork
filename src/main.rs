#![allow(unused)]

pub mod algorithms;
pub mod eo_edges;
pub mod heuristics;
pub mod model;
pub mod msg_algorithms;
pub mod output;
pub mod parser;
pub mod preprocess;

pub mod do_edges;

use algorithms::add_edges;
use clap::ValueEnum;
use do_edges::*;
use eo_edges::eo_cases;
use eo_edges::get_eod;
use eo_edges::missing_eo;
use eo_edges::missing_mo;
use eo_edges::mo_cases;
use eo_edges::remove_eo;
use heuristics::*;
use itertools::Itertools;
use model::EGraph;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use model::get_mgraph;
use model::mk_graph;
use model::EGraphData;
use msg_algorithms::extend_valid_multiset;
use msg_algorithms::extend_valid_queue;
use output::*;
use parser::parse_str;
use parser::read_file;
use petgraph::algo::is_cyclic_directed;
use petgraph::algo::kosaraju_scc;
use petgraph::dot::Dot;
use preprocess::preprocess;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::time::Instant;

use io::*;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ADT {
    Multiset,
    Queue,
    Stack,
    Register,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Cli {
    /// ADT to check consistency for
    #[arg(value_enum)]
    adt: ADT,

    #[arg(value_enum)]
    heuristics: Heuristic,

    /// Input file
    file: String,

    /// Print output graphs to dotfiles with name <FILE>.dot and <FILE>_ok.dot if check succeeds.
    #[arg(short, long)]
    draw: bool,

    /// Verbosity for more debugging output. The more -v's the more verbose. -vvvvvvvvvvvvvvvvvvvvvvvvvvvv
    #[command(flatten)]
    verbosity: Verbosity,
}

fn run_check(g: EGraph, data : &EGraphData, heur: Heuristic, adt: ADT) -> Option<EGraph> {
    let missing_eo = missing_eo(&g, data);
    let missing_mo = missing_mo(&g);
    let mut saved = false;
    let mut i = 0;
    let numcases = i128::pow(2, missing_eo.len() as u32);
    //println!("Missing EO: {:?}", missing_eo);
    for (q, mut g) in eo_cases(&g, data, &missing_eo) {
        i += 1;
        match adt {
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
                        //write_dot(&eg, "multiset".into(), "cycle".into()).unwrap();
                        saved = true;
                    }
                }
            }
            _ => {
                for gp in mo_cases(&g, &missing_mo) {

                    if let Some(q) = match adt {
                        ADT::Queue => Some(queue_do(gp)),
                        ADT::Stack => Some(stack_do(gp)),
                        ADT::Register => Some(reg_do(gp)),
                        _ => None,
                    } {
                        if !is_cyclic_directed(&q) {
                            return Some(q);
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
    let q = read_file(cli.file.clone());
    let (mut g, data) = mk_graph(&q);
    let parsed = Instant::now();

    if cli.draw {
        let eg = (g.clone(), data.clone());
        write_dot(&eg, cli.file.clone(), "input".into())?;
    }

    println!("Parsing: {:?}µs", (parsed - start).as_micros());
    preprocess(&mut g, &data, cli.heuristics, cli.adt);
    let preprocessed = Instant::now();
    println!("Preprocessing: {:?}µs", (preprocessed - parsed).as_micros());

    if cli.draw {
        let eg = (g.clone(), data.clone());
        write_dot(&eg, cli.file.clone().into(), "pp".into())?;
    }

    let res = run_check(g, &data, cli.heuristics, cli.adt);
    let done = Instant::now();
    println!("Check: {:?}µs", (done - preprocessed).as_micros());
    println!("Total: {:?}µs", (done - start).as_micros());

    println!("Result: {:?}", res.is_some());

    if let Some(q) = res {
        if cli.draw {
            let eg = (q.clone(), data.clone());
            write_dot(&eg, cli.file.clone(), "ok".into())?;
        }
    }

    println!("Handlers: {:?}", q.0.len());
    let num_mess: usize =
        q.0.iter()
            .map(|x| x.messages.len())
            .collect_vec()
            .iter()
            .sum();
    println!("Messages: {:?}", num_mess);

    // println!("{} cases.", n);

    //println!("Result: {:?}", res.is_some());
    // println!("Multiset: {:?}", ms_ok);
    // println!("Queue: {:?}", q_ok);
    // println!("Stack: {:?}", s_ok);
    // println!("Reg: {:?}", r_ok);

    Ok(())
}
