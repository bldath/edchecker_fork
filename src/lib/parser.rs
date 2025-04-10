use itertools::Itertools;
use log::{debug, info};
use regex::Regex;
use std::{
    collections::HashMap,
    fs,
    iter::{self, Chain},
    str::FromStr,
};

use model::Idx;

use crate::model::{self, EGraph, EPair, EdgeTp, EdgeTp::*, Event, Handler, Message, ReadResult};

pub fn read_file(filename: String) -> ReadResult {
    if let Ok(q) = fs::read_to_string(&filename) {
        if filename.split('.').last().unwrap() == "json" {
            return serde_json::from_str(&q).unwrap();
        }
        parse_str(&q)
    } else {
        (HashMap::new(), vec![])
    }
}

pub fn parse_event(s: &String) -> Option<Event> {
    let ev_regex = Regex::new(r"(\w+)\(\s*(\w*)\s*,\s*([\w\.]*)\s*\)").unwrap();
    if let Some(c) = ev_regex.captures(s) {
        let op: String = c.get(1).unwrap().as_str().to_lowercase();
        let a1: String = c.get(2).unwrap().as_str().into();
        let a2: String = c.get(3).unwrap().as_str().into();
        return match op.as_str() {
            "write" => Some(Event::Write(a1, a2)),
            "read" => Some(Event::Read(a1, a2)),
            "post" => Some(Event::Post(a1, a2)),
            _ => None,
        };
    }
    debug!("Unmatched: {}", s);
    None
}

pub fn list_to_message(l: &[String]) -> Option<Message> {
    let get_msg = &l[0];
    let events = l.iter().skip(1);

    let get_regex = Regex::new(r"[gG]et\(([\w\.]*)\)").unwrap();

    let mid = if let Some(cap) = get_regex.captures(get_msg) {
        cap.get(1).unwrap().as_str()
    } else {
        panic!("What {}", get_msg);
    };

    let sevs = events.filter_map(parse_event);

    let head = vec![Event::Get(mid.into())];
    let evs: Vec<Event> = head.into_iter().chain(sevs).collect();

    let m = Message {
        id: mid.into(),
        evs,
    };
    Some(m)
}

pub fn parse_edgetp(s: &str) -> Option<EdgeTp> {
    match &s[0..2] {
        "RF" => Some(RF),
        "CO" => Some(CO),
        "PO" => Some(PO),
        "EO" => Some(EO),
        "PB" => Some(PB),
        "MO" => Some(MO),
        _ => None,
    }
}

pub fn parse_edges(s: &str) -> Vec<(EdgeTp, Event, Event)> {
    let et = parse_edgetp(&s[1..3]).unwrap();

    let q = String::from_str(&s[4..]).unwrap();

    let chains = q
        .split(';')
        .map(|x| x.split("->").map(|x| x.to_string()).collect_vec())
        .collect_vec();
    // TODO make CO total by (in addition to x_i CO x_i+1 add x_m CO x_i for all m < i)
    //info!("Edges: {:?}", chains);

    chains
        .iter()
        .flat_map(|c| {
            c.iter()
                .filter_map(parse_event)
                .combinations(2)
                .map(|ls| (et, ls[0].clone(), ls[1].clone()))
        })
        .collect_vec()
}

fn idx_of(
    event: Event,
    readers: &HashMap<(String, String), Vec<Idx>>,
    writers: &HashMap<(String, String), Idx>,
) -> Idx {
    match event {
        Event::Write(var, val) => writers.get(&(var, val)).expect("Writer not found").clone(),
        Event::Read(var, val) => {
            let read_vec = readers
                .get(&(var.clone(), val.clone()))
                .expect("Readers not found");
            if read_vec.len() > 1 {
                panic!("Multiple Read({}, {})", &var, &val);
            }
            read_vec.first().expect("Reader list is empty").clone()
        }
        _ => panic!("Unsupported event type for idx_of"),
    }
}

pub fn parse_str(s: &str) -> ReadResult {
    let mut handlers = HashMap::<String, HashMap<String, Vec<Event>>>::new();
    let mut active_handler: String = "NONE".into();
    let strs: Vec<String> = s.split('$').map(|x| x.replace('\n', " ")).collect();
    let handler_strs: Vec<String> = strs[0].split('@').skip(1).map(|x| x.into()).collect();

    let msg_regex = Regex::new(r"\{.*\}").unwrap();

    let mut writers = HashMap::new();
    let mut readers = HashMap::new();
    let mut pb = Vec::new();

    let q = handler_strs
        .iter()
        .map(|h| {
            let v: Vec<String> = h
                .split('{')
                .map(|x| x.replace('}', "").trim().into())
                .collect();
            let hid = v[0].clone();
            let m: Vec<Vec<String>> = v
                .iter()
                .skip(1)
                .map(|ms| ms.split("->").map(|x| x.trim().replace(' ', "")).collect())
                .collect();

            // Now we have the events as text! Time to map them to Messages!
            let msgs: HashMap<String, Vec<Event>> = m
                .iter()
                .filter_map(|s| list_to_message(s))
                .map(|msg| (msg.id, msg.evs))
                .collect();

            for (mid, evs) in msgs.iter() {
                for (i, e) in evs.iter().enumerate() {
                    let idx: Idx = (h.clone(), mid.clone(), i);
                    match e {
                        Event::Write(var, val) => {
                            writers.insert((var.clone(), val.clone()), idx);
                        }
                        Event::Read(var, val) => {
                            readers
                                .entry((var.clone(), val.clone()))
                                .or_insert(vec![])
                                .push(idx);
                        }
                        Event::Post(hdl, msg) => {
                            pb.push((PB, idx, (hdl.clone(), msg.clone(), 0_usize)));
                        }
                        Event::Get(_) => {}
                    }
                }
            }

            (hid, msgs)
        })
        .collect();

    let edges = strs
        .iter()
        .skip(1)
        .flat_map(|x| parse_edges(x))
        .collect_vec();
    let indices = edges.iter().map(|(et, from, to)| {
        let from = from.clone();
        let to = to.clone();
        let et = *et;

        let from = idx_of(from, &readers, &writers);
        let to = idx_of(to, &readers, &writers);

        (et, from, to)
    });

    let edges = indices.chain(pb);

    // Get EO from the order handlers appear in the trace
    // let eit = edges.chain(q.iter().flat_map(| hdl | {
    //     hdl.messages.iter().tuples().map(| (Message { id, evs }, Message { id: i2, evs: e2 }) | {
    //         (EO, evs[0].clone(), e2[0].clone())
    //     })
    // })).collect_vec();
    (q, edges.collect_vec())
}
