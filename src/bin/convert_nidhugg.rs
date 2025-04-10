#![allow(unused)]
use std::{
    collections::{btree_map::OccupiedEntry, hash_map::Entry, HashMap, HashSet},
    default,
    hash::Hash,
};

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use itertools::Itertools;
use lib::{
    model::{mk_graph, EGraph, EGraphData, EPair, EdgeTp, Event, ExecutionGraph, ReadResult},
    output::write_graph,
};

use glob::glob;

use petgraph::graph::NodeIndex;
use regex::Regex;

fn read_file(file: String) -> Result<String, std::io::Error> {
    use std::fs::File;
    use std::io::Read;

    let mut f = File::open(file)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

fn add_event(
    hm: &mut HashMap<String, HashMap<String, Vec<Event>>>,
    tid: String,
    mid: String,
    e: Event,
) {
    let mevs: &mut Vec<Event> = hm
        .entry(tid.clone())
        .or_default()
        .entry(mid.clone())
        .or_insert(vec![Event::Get(mid.clone())]);
    //let last = mevs.last().cloned();
    if let Event::Get(m) = e {
    } else {
        mevs.push(e);
    }

    // let node_idx = eg.add_node(EPair(tid, mid, e));
    // mevs.push(node_idx);

    // if let Some(lst) = last {
    //     eg.add_edge(lst, node_idx, EdgeTp::PO);
    // }
}

pub fn split_input(s: String) -> Vec<String> {
    let split_string = "=== EventTraceBuilder reset ===";
    s.split(split_string)
        .skip(1)
        .map(|x| x.to_string())
        .collect_vec()
}

fn get_var(s: &str, var_ids: &mut HashMap<String, String>, var_ctr: &mut u32) -> String {
    match var_ids.get(s) {
        Some(v) => v.clone(),
        None => {
            let id = format!("x{}", var_ctr);
            *var_ctr += 1;
            var_ids.insert(s.to_string(), id.clone());
            id
        }
    }
}

fn wt_val(s: &String, var_ctrs: &mut HashMap<String, u32>) -> u32 {
    //println!("Write on val: {}", s);
    match var_ctrs.entry(s.to_string()) {
        Entry::Occupied(mut entry) => {
            let id = *entry.get() + 1;
            entry.insert(id);
            id
        }
        Entry::Vacant(entry) => {
            let id = 1;
            entry.insert(id);
            id
        }
    }
}

fn rd_val(s: &String, var_ctrs: &HashMap<String, u32>) -> u32 {
    var_ctrs.get(s).cloned().unwrap_or(0)
}

pub fn parse_str(s: String) -> Result<ReadResult, std::io::Error> {
    let ev_regex = Regex::new(
        r"^\s*\(<(?P<tid>.*?)>,(?P<eid>\d+-?\d*)\)\s*(?P<hdl>-?\d+):\s*(?P<evt>.*)\s*SLP",
    )
    .unwrap();

    let post_re = Regex::new(r"Post\(<(?P<mid>.*?)>\)\s*").unwrap();
    let store_re = Regex::new(r"Store\((?P<var>.*),(?P<val>.*)\)\s*").unwrap();
    let load_re = Regex::new(r"Load\((?P<var>.*)\)").unwrap();

    let mut var_ctr = 0;
    let mut var_ctrs = HashMap::<String, u32>::new();

    let mut var_ids = HashMap::<String, String>::new();
    let mut hdl_of_msg: HashMap<String, String> = HashMap::new();

    let mut evs: HashMap<String, HashMap<String, Vec<Event>>> = HashMap::new();
    let mut co_var = HashMap::<String, Vec<Event>>::new();

    for line in s.lines() {
        if let Some(m) = ev_regex.captures(line) {
            let evt = m.name("evt").unwrap().as_str();
            let hdl = m.name("hdl").unwrap().as_str();
            let tid = m.name("tid").unwrap().as_str();

            let pre = post_re.captures(evt);
            let sre = store_re.captures(evt);
            let lre = load_re.captures(evt);

            hdl_of_msg.insert(tid.to_string(), hdl.to_string());
            evs.entry(hdl.to_string())
                .or_default()
                .entry(tid.to_string())
                .or_default();

            if let Some(pre) = pre {
                evs.entry(hdl.to_string())
                    .or_default()
                    .entry(tid.to_string())
                    .or_default()
                    .push(Event::Post(
                        hdl.to_string(),
                        pre.name("mid").unwrap().as_str().to_string(),
                    ));
            } else if let Some(sre) = sre {
                let var = sre.name("var").unwrap().as_str();
                let val = sre.name("val").unwrap().as_str();

                let var_id = get_var(var, &mut var_ids, &mut var_ctr);
                let val_id = wt_val(&var_id.to_string(), &mut var_ctrs);
                //println!("Var: {} -> {}, Val: {} -> {}", var, var_id, val, val_id);
                let evt = Event::Write(var_id.clone(), val_id.to_string());

                evs.entry(hdl.to_string())
                    .or_default()
                    .entry(tid.to_string())
                    .or_default()
                    .push(evt.clone());

                co_var.entry(var_id).or_default().push(evt);
            } else if let Some(lre) = lre {
                let var = lre.name("var").unwrap().as_str();

                let var_id = get_var(var, &mut var_ids, &mut var_ctr);
                //println!("Var: {}, Ctrs: {:?}", var_id, var_ctrs);
                let val_id = rd_val(&var_id, &var_ctrs);

                if val_id == 0 {
                    // This value was not written to
                    continue;
                }

                evs.entry(hdl.to_string())
                    .or_default()
                    .entry(tid.to_string())
                    .or_default()
                    .push(Event::Read(var_id, val_id.to_string()));
            }
        }
    }

    let edges = co_var
        .iter()
        .flat_map(|(k, v)| {
            v.iter()
                .tuple_windows()
                .map(|(a, b)| {
                    let a = a.clone();
                    let b = b.clone();
                    (EdgeTp::CO, a, b)
                })
                .collect_vec()
        })
        .collect_vec();

    // Preprocess trace
    // Remove empty messages
    let mut new_evs: HashMap<String, HashMap<String, Vec<Event>>> = evs
        .iter()
        .filter_map(|(hdl, msgs)| {
            let new_msgs: HashMap<String, Vec<Event>> = msgs
                .iter()
                .filter(|(mid, evs)| !evs.is_empty())
                .map(|(x, y)| {
                    let mut new_evs: Vec<Event> = y.clone();
                    new_evs.insert(0, Event::Get(x.clone()));
                    (x.clone(), new_evs)
                })
                .collect();
            if !new_msgs.is_empty() {
                Some((hdl.clone(), new_msgs))
            } else {
                None
            }
        })
        .collect();

    // Fix post events
    for (hdl, msgs) in new_evs.iter_mut() {
        for (mid, evs) in msgs.iter_mut() {
            for ev in evs.iter_mut() {
                if let Event::Post(ph, pm) = ev {
                    //println!("Post of {}", pm);
                    if let Some(hdl) = hdl_of_msg.get(pm) {
                        *ph = hdl.clone();
                    }
                }
            }
        }
    }

    Ok((new_evs, edges))
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct ConvertCli {
    pub input_dir: String,
    pub output_dir: String,
    pub count: usize,

    #[command(flatten)]
    pub verbosity: Verbosity,
}

fn main() -> Result<(), std::io::Error> {
    let cli = ConvertCli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .init();

    let inputs =
        glob(format!("{}/*.log", cli.input_dir).as_str()).expect("Failed to read input directory");
    for e in inputs.flatten() {
        println!("Experiment: {}", e.display());

        let path = e.as_path().to_str().unwrap();
        let file = read_file(path.to_string())?;

        let q = split_input(file)
            .into_iter()
            .take(cli.count)
            .map(|x| parse_str(x.to_string()));
        for (i, graph) in q.enumerate() {
            match graph {
                Ok((eg, co_edges)) => {
                    let filename = path.split('/').last().unwrap();
                    let out_dir = filename.split('.').next().unwrap();

                    let file = format!("{}/{}/trace{}.trace", cli.output_dir, out_dir, i);

                    println!("File: {}", file);
                    let eg = mk_graph(&(eg, co_edges));
                    write_graph(&eg, file)?;
                }
                Err(e) => {
                    println!("Error parsing graph {}: {}", i, e);
                }
            }
        }
    }

    Ok(())
}
