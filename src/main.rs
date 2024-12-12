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
use msg_algorithms::extend_valid_multiset;
use msg_algorithms::extend_valid_queue;
use output::*;
use parser::parse_str;
use parser::read_file;
use petgraph::algo::is_cyclic_directed;
use petgraph::dot::Dot;
use preprocess::preprocess;
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

fn run_check(g: EGraph, adt: ADT) -> Option<EGraph> {
    let missing_eo = missing_eo(&g);
    let missing_mo = missing_mo(&g);
    for g in eo_cases(&g, &missing_eo) {
        match adt {
            ADT::Multiset => {
                let g_multiset = multiset_do(g);
                if !is_cyclic_directed(&g_multiset) {
                    return Some(g_multiset);
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
    println!("Parsing: {:?}µs", (parsed - start).as_micros());
    preprocess(&mut g, &data, cli.heuristics, cli.adt);
    let preprocessed = Instant::now();
    println!("Preprocessing: {:?}µs", (preprocessed - parsed).as_micros());

    if cli.draw {
        println!("Printing dot! {:?}.dot", cli.file);
        write_dot(&g, cli.file.clone())?;
    }

    let res = run_check(g, cli.adt);
    let done = Instant::now();
    println!("Check: {:?}µs", (done - preprocessed).as_micros());
    println!("{:?}: {:?}", cli.adt, res.is_some());
    if let Some(q) = res {
        if cli.draw {
            println!("Printing dot {:?}_ok.dot", cli.file);
            write_dot(&q, (cli.file.clone() + "_ok").into());
        }
    }
    // println!("Total: {:?}µs", (done - start).as_micros());
    // println!("Handlers: {:?}", q.0.len());
    let num_mess: usize =
        q.0.iter()
            .map(|x| x.messages.len())
            .collect_vec()
            .iter()
            .sum();
    // println!("Messages: {:?}", num_mess);

    // println!("{} cases.", n);

    // println!("Multiset: {:?}", ms_ok);
    // println!("Queue: {:?}", q_ok);
    // println!("Stack: {:?}", s_ok);
    // println!("Reg: {:?}", r_ok);

    Ok(())
}
