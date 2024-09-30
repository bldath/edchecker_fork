#![allow(unused)]

pub mod model;
pub mod parser;
pub mod output;
pub mod preprocess;
pub mod algorithms;
pub mod msg_algorithms;

use std::io;
use std::fs;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use model::get_mgraph;
use model::mk_graph;
use msg_algorithms::extend_valid_multiset;
use msg_algorithms::extend_valid_queue;
use msg_algorithms::get_total_mo;
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
    write_dot(&g, cli.file.clone())?;

    println!("Cyclic: {}", is_cyclic_directed(&g));

    let g2 = get_mgraph(&g);

    write_dot(&g2, cli.file.replace(".trace", ".msgs.trace"))?;

    let tot = get_total_mo(&g2);

    let mut vld = 0;
    let mut vld_q = 0;
    let mut cnt = 0;
    for tord in tot {
        cnt += 1;

        let valid = extend_valid_multiset(&g, &tord);
        if valid { vld += 1; }

        let valid = extend_valid_queue(&g, &tord);
        if valid { vld_q += 1; }
    }
    println!("Graph has {:?}/{:?} valid multiset extensions", vld, cnt);
    println!("Graph has {:?}/{:?} valid queue extensions", vld_q, cnt);


    Ok(())
}
