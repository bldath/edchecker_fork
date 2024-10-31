use std::{fs, iter::{self, Chain}, str::FromStr};
use itertools::Itertools;
use regex::Regex;

use crate::model::{self, EGraph, EPair, EdgeTp, EdgeTp::*, Event, Handler, Message, ReadResult};


pub fn read_file(filename : String) -> ReadResult {
    if let Ok(q) = fs::read_to_string(filename) {
        parse_str(&q)
    } else { ReadResult(vec![], vec![]) }
}


pub fn parse_event(s : &String) -> Event {
    let ev_regex = Regex::new(r"(\w+)\((.*),(.*)\)").unwrap();
    let c = ev_regex.captures(s).unwrap();
    let op : &str = c.get(1).unwrap().as_str();
    let a1 : String = c.get(2).unwrap().as_str().into();
    let a2 : String = c.get(3).unwrap().as_str().into();
    match op {
        "write" => Some(Event::Write(a1, a2)),
        "read" => Some(Event::Read(a1, a2)),
        "post" => Some(Event::Post(a1, a2)),
        _ => None,
    }.unwrap()
}

pub fn list_to_message(l : &Vec<String>) -> Option<Message> {
    let get_msg = &l[0];
    let events = l.iter().skip(1);

    let get_regex = Regex::new(r"get\((.*)\)").unwrap();

    let mid = get_regex.captures(get_msg).unwrap().get(1)?.as_str();

    let sevs = events.map(| e | parse_event(e));

    let head = vec![Event::Get(mid.into())];
    let evs :Vec<Event> = head.into_iter().chain(sevs).collect();

    let m = Message {
        id: mid.into(),
        evs,
    };
    Some(m)
}

pub fn parse_edgetp(s : &str) -> Option<EdgeTp> {
    match &s[0..2] {
        "RF" => Some(RF),
        "CO" => Some(CO),
        "PO" => Some(PO),
        "EO" => Some(EO),
        "PB" => Some(PB),
        "MO" => Some(MO),
        _ => None
    }
}

pub fn parse_edges(s : &String) -> Vec<(EdgeTp, Event, Event)> {

    let et = parse_edgetp(&s[1..3]).unwrap();

    let q = String::from_str(&s[3..]).unwrap();

    let chains  = q.split(';').map(|x| x.split("->").map(|x| x.to_string()).collect_vec()).collect_vec();

    chains.iter().flat_map(| c | {
        c.iter().map(|x| parse_event(x)).tuple_windows().map(| (e1, e2) | {
            (et, e1, e2)
        })
    }).collect_vec()
}

pub fn parse_str(s : &String) -> ReadResult {
    let mut handlers : Vec<Handler> = vec![];
    let mut active_handler : String = "NONE".into();
    let mut msgs : Vec<Message> = vec![];
    let strs : Vec<String> = s.split('$').map(|x| x.replace('\n', " ").into()).collect();
    let handler_strs : Vec<String> = strs[0].split('@').skip(1).map(|x| x.into()).collect();

    let msg_regex = Regex::new(r"\{.*\}").unwrap();

    let q : Vec<Handler> = handler_strs.iter().map(|h| {
        let v : Vec<String> = h.split('{').map(|x| x.replace('}', "").trim().into()).collect();
        let hid = v[0].clone();
        let m : Vec<Vec<String>> = v.iter().skip(1).map(| ms | {
           ms.split("->").map(|x| x.trim().replace(' ', "").into()).collect()
        }).collect();

        // Now we have the events as text! Time to map them to Messages!
        let msgs : Vec<Message> = m.iter().filter_map(|x| list_to_message(x)).collect();
        Handler {
            id: hid,
            messages: msgs,
        }
    }).collect();

    let edges = strs.iter().skip(1).flat_map(|seq| {
        parse_edges(seq)
    });

    // Get EO from the order handlers appear in the trace
    // let eit = edges.chain(q.iter().flat_map(| hdl | {
    //     hdl.messages.iter().tuples().map(| (Message { id, evs }, Message { id: i2, evs: e2 }) | {
    //         (EO, evs[0].clone(), e2[0].clone())
    //     })
    // })).collect_vec();
    ReadResult(q, edges.collect_vec())
}
