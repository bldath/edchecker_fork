use std::collections::HashMap;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use lib::model::{EGraph, ExecutionGraph};


use regex::Regex;

fn read_file(file: String) -> Result<String, std::io::Error> {
    use std::fs::File;
    use std::io::Read;

    let mut f = File::open(file)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

fn parse_str(s : String) -> Result<ExecutionGraph, std::io::Error> {

    let rw_regex : Regex= Regex::new(r"^rwId:(\d+) (\w+) tid:(\d+) obj:(\wx\w+).*$").unwrap();
    let post_regex = Regex::new(r"(\d+) POST src:(\d+) msg:(\d+)").unwrap();
    let call_regex = Regex::new(r"(\d+) CALL tid:(\d+)	 msg:(\d+)").unwrap();

    let g = EGraph::new();
    let hm = HashMap::new();

    let current = HashMap::<u32, u32>::new();
    for line in s.lines() {

    }



    Ok((g, hm))
}


#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct ConvertCli {
    pub file: String,
    #[command(flatten)]
    pub verbosity: Verbosity,
}

fn main() -> Result<(), std::io::Error> {
    let cli = ConvertCli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .init();

    let q = read_file(cli.file.clone())?;



    Ok(())
}
