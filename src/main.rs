#![allow(unused)]

pub mod model;
pub mod parser;
pub mod output;
pub mod preprocess;
pub mod algorithms;
pub mod msg_algorithms;
pub mod eo_edges;


pub mod do_edges;

use algorithms::add_edges;
use do_edges::*;
use eo_edges::eo_cases;
use eo_edges::get_eod;
use eo_edges::missing_eo;
use eo_edges::missing_mo;
use eo_edges::mo_cases;
use eo_edges::remove_eo;

use std::io;
use std::fs;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use model::get_mgraph;
use model::mk_graph;
use msg_algorithms::extend_valid_multiset;
use msg_algorithms::extend_valid_queue;
use parser::parse_str;
use parser::read_file;
use output::*;
use petgraph::algo::is_cyclic_directed;
use petgraph::dot::Dot;
use preprocess::preprocess;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Cli {
    file: String,
    #[arg(short, long)]
    draw: bool,
    #[command(flatten)]
    verbosity: Verbosity,
}


fn main() -> Result<(), std::io::Error> {
    let cli = Cli::parse();
    let q = read_file(cli.file.clone());
    let mut g = mk_graph(&q);
    preprocess(&mut g);
    if cli.draw {
        println!("Printing dot!");
        write_dot(&g, cli.file.clone())?;
    }

    let missing_eo = missing_eo(&g);
    let missing_mo = missing_mo(&g);

    println!("Missing EO: {:?}", missing_eo);
    println!("Missing MO: {:?}", missing_mo);

    let mut ms_ok = false;
    let mut q_ok = false;
    let mut s_ok = false;
    let mut r_ok = false;

    let mut n = 0;
    for g in eo_cases(&g, &missing_eo) {
        let g_multiset = multiset_do(g.clone());
        for g in mo_cases(&g, &missing_mo) {
            n += 1;
            let eod_edges = get_eod(&g);
            let mut g = remove_eo(g);
            add_edges(&mut g, eod_edges);

            let g_queue = queue_do(g.clone());
            let g_stack = stack_do(g.clone());
            let g_reg = reg_do(g.clone());

            ms_ok |= !is_cyclic_directed(&g_multiset);
            q_ok |= !is_cyclic_directed(&g_queue);
            s_ok |= !is_cyclic_directed(&g_stack);
            r_ok |= !is_cyclic_directed(&g_reg);
        }
    }

    println!("{} cases.", n);

    println!("Multiset: {:?}", ms_ok);
    println!("Queue: {:?}", q_ok);
    println!("Stack: {:?}", s_ok);
    println!("Reg: {:?}", r_ok);

    Ok(())
}
