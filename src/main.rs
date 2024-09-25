pub mod model;
pub mod parser;
pub mod output;
pub mod preprocess;

use std::io;
use std::fs;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use model::mk_graph;
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
    write_dot(&g, cli.file)?;

    println!("Cyclic: {}", is_cyclic_directed(&g));

    Ok(())
}
