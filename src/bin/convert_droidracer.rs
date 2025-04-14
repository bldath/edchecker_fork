#![allow(unused)]
use std::{
    collections::{btree_map::OccupiedEntry, hash_map::Entry, HashMap, HashSet},
    default,
    fs::File,
    io::Write,
};

use glob::glob;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use itertools::Itertools;
use lib::{
    model::{mk_graph, EGraph, EGraphData, EPair, EdgeTp, Event, ExecutionGraph, Idx, ReadResult},
    output::{make_file, write_graph},
};

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

fn parse_str(s: String) -> Result<ReadResult, std::io::Error> {
    let rw_regex: Regex = Regex::new(r"^rwId:(\d+) (\w+) tid:(\d+) obj:(\wx\w+).*$").unwrap();
    let post_regex = Regex::new(r"(\d+) POST src:(\d+) msg:(\d+)").unwrap();
    let call_regex = Regex::new(r"(\d+) CALL tid:(\d+)	 msg:(\d+)").unwrap();
    // let mut eg : ExecutionGraph = (EGraph::new(), HashMap::new());

    let mut eg = HashMap::new();
    let mut active = HashMap::<String, String>::new();

    let mut last_write = HashMap::<String, String>::new();
    let mut co_edges = HashMap::<String, Vec<String>>::new();
    let mut variable_occurrence = HashMap::<String, HashSet<String>>::new();

    for line in s.lines() {
        if let Some(m) = rw_regex.captures(line) {
            let (q, [id, op, tid, obj]) = m.extract();
            let mid = active.entry(tid.to_string()).or_insert("NO_MSG".into());

            let ev = if (op == "WRITE") {
                co_edges
                    .entry(obj.to_string())
                    .or_default()
                    .push(id.to_string());

                last_write.insert(obj.to_string(), id.to_string());
                variable_occurrence
                    .entry(obj.to_string())
                    .or_default()
                    .insert(mid.clone());
                Some(Event::Write(obj.to_string(), id.to_string()))
            } else if let Entry::Occupied(lw) = last_write.entry(obj.to_string()) {
                variable_occurrence
                    .entry(obj.to_string())
                    .or_default()
                    .insert(mid.clone());
                Some(Event::Read(obj.to_string(), lw.get().to_string()))
            } else {
                None
            };
            if let Some(e) = ev {
                add_event(&mut eg, tid.to_string(), mid.clone(), e);
            }
        } else if let Some(m) = post_regex.captures(line) {
            let (q, [id, tid, mid]) = m.extract();
            let src = active.entry(tid.to_string()).or_insert("NO_MSG".into());
            add_event(
                &mut eg,
                tid.to_string(),
                src.clone(),
                Event::Post("UNKNOWN".into(), mid.to_string()),
            );
        } else if let Some(m) = call_regex.captures(line) {
            let (q, [id, tid, mid]) = m.extract();
            active.insert(tid.into(), mid.into());
            add_event(
                &mut eg,
                tid.to_string(),
                mid.to_string(),
                Event::Get(mid.into()),
            );
        }
    }

    // Variables that only occur in one message
    let thread_local: HashSet<String> = variable_occurrence
        .iter()
        .filter_map(|(k, v)| if v.len() < 2 { Some(k.clone()) } else { None })
        .collect();

    println!("Thread local: {:?}", thread_local);

    let mut to_rmv: HashMap<(String, String), Vec<usize>> = HashMap::new();

    for (hdl, msgs) in eg.iter() {
        for (mid, evs) in msgs.iter() {
            for (idx, ev) in evs.iter().enumerate() {
                if let Some(v) = ev.variable() {
                    if thread_local.contains(&v) {
                        to_rmv
                            .entry((hdl.clone(), mid.clone()))
                            .or_default()
                            .push(idx);
                    }
                }
            }
        }
    }

    let count = to_rmv.iter().fold(0, |acc, (k, vec)| acc + vec.len());

    println!("Removing {} thread-local operations", count);

    // Remove them in *reverse* order, so the indices remain stable.
    for ((hdl, mid), vec) in to_rmv.iter() {
        for idx in vec.iter().rev() {
            eg.entry(hdl.clone())
                .or_default()
                .entry(mid.clone())
                .or_default()
                .remove(*idx);
        }
    }

    let mut changed = true;

    while changed {
        changed = false;


        let posts_without_target = eg
            .iter()
            .flat_map(|(hdl, msgs)| {
                msgs.iter().filter_map(|(mid, evs)| {
                    let post_idx = evs.iter()
                        .enumerate()
                        .filter_map(|(i, ev)| {
                            if let Event::Post(ph, pm) = ev {
                                if !eg.contains_key(ph) || !eg.get(ph).unwrap().contains_key(pm) {
                                    return Some(i);
                                }
                            }
                            None
                        })
                        .collect_vec();
                    if post_idx.len() > 0 {
                        Some((hdl.clone(), mid.clone(), post_idx))
                    } else {
                        None
                    }
                })
            }).collect_vec();

        for (hdl, mid, indices) in posts_without_target.iter() {
            // indices is in ascending order, we go in reverse
            for i in indices.iter().rev() {
                eg.entry(hdl.clone())
                    .or_default()
                    .entry(mid.clone())
                    .or_default()
                    .remove(*i);
            }
            changed = true;
        }

        let mut rm: Vec<(String, String)> = Vec::new();
        for (hdl, msgs) in eg.iter() {
            for (mid, evs) in msgs {
                if evs.len() == 1 {
                    rm.push((hdl.clone(), mid.clone()));
                    changed = true;
                }
            }
        }

        for (hdl, mid) in rm.iter() {
            eg.entry(hdl.clone()).or_default().remove(mid);
        }
    }

    let writers = eg
        .iter()
        .flat_map(|(hdl, msgs)| {
            msgs.iter().flat_map(|(mid, evs)| {
                evs.iter()
                    .enumerate()
                    .filter_map(|(i, ev)| {
                        if let Event::Write(var, val) = ev {
                            Some(((var.clone(), val.clone()), (hdl.clone(), mid.clone(), i)))
                        } else {
                            None
                        }
                    })
                    .collect_vec()
            })
        })
        .collect::<HashMap<_, _>>();

    let co = co_edges
        .iter()
        .filter(|(v, _)| !thread_local.contains(*v))
        .flat_map(|(k, v)| {
            v.iter().tuple_windows().map(|(a, b)| {
                (
                    EdgeTp::CO,
                    writers
                        .get(&(k.clone(), a.to_string()))
                        .unwrap_or_else(|| panic!("Cannot find {},{}", k.clone(), a.clone()))
                        .clone(),
                    writers
                        .get(&(k.clone(), b.to_string()))
                        .unwrap_or_else(|| panic!("Cannot find {},{}", k.clone(), b.clone()))
                        .clone(),
                )
            })
        })
        .collect_vec();

    Ok(ReadResult::new(eg, co))
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct ConvertCli {
    pub input_dir: String,
    pub output_dir: String,

    #[command(flatten)]
    pub verbosity: Verbosity,
}

fn main() -> Result<(), std::io::Error> {
    let cli = ConvertCli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .init();

    let mut ctrs = HashMap::<String, u8>::new();
    let inputs =
        glob(format!("{}/**/abc_log*", cli.input_dir).as_str()).expect("Failed to read input");

    for e in inputs.flatten() {
        let path = e.as_path().to_str().unwrap();

        let strs = path.split('/').collect_vec();
        let expt = strs[strs.len() - 2];
        let trace_num = ctrs.entry(expt.to_string()).or_default();
        *trace_num += 1;

        let contents = read_file(path.to_string())?;
        let Ok(mut eg) = parse_str(contents) else {
            continue;
        };
        eg.build();
        let out = format!("{}/{}/trace{}.json", cli.output_dir, expt, trace_num);
        let mut file = make_file(out).expect("Unable to create file");
        file.write_all(serde_json::to_string(&eg).unwrap().as_bytes())
            .expect("Unable to write data");
    }

    // let pathvec = cli.file.split('/').collect_vec();
    // let expt = pathvec[pathvec.len() - 2];
    // let file = format!("{}/{}/trace0.trace", cli.output_dir, expt);
    // write_graph(&eg, file);

    Ok(())
}
