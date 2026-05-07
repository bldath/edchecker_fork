extern crate lib;

use lib::cli::*;

use itertools::Itertools;
use lib::instance;

use clap::Parser;
use lib::instance::Instance;
use lib::model::mk_graph;
use lib::model::ReadResult;
use lib::output::*;
use lib::parser::read_file;

use std::io;
use z3::Solver;

use z3::{Config, Context};

use std::time::Instant;

use io::*;

fn print_result(res: z3::SatResult, _instance: &Instance, _solver: &Solver, _rr: &ReadResult) {
    match res {
        z3::SatResult::Unsat => println!("Result: false"),
        z3::SatResult::Unknown => println!("Result: unknown"),
        z3::SatResult::Sat => {
            println!("Result: true");
            println!("Recovery of graph unimplemented");
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
    // println!("{} cases.", n);

    Ok(())
}
