use std::{fs, iter::Chain};
use regex::Regex;

use crate::model::{self, EGraph, EPair, Event, Handler, Message};

pub fn read_file(filename : String) -> Vec<Handler> {
    if let Ok(q) = fs::read_to_string(filename) {
        parse_str(&q)
    } else { vec![] }
}


pub fn list_to_message(l : &Vec<String>) -> Option<Message> {
    let get_msg = &l[0];
    let events = l.iter().skip(1);

    let ev_regex = Regex::new(r"(\w+)\((.*),(.*)\)").unwrap();
    let get_regex = Regex::new(r"get\((.*)\)").unwrap();

    let mid = get_regex.captures(get_msg).unwrap().get(1)?.as_str();

    println!("Parsing Message: {:?}", mid);

    let sevs = events.filter_map(| e | -> Option<Event> {
        let c = ev_regex.captures(e).unwrap();
        let op : &str = c.get(1)?.as_str();
        let a1 : String = c.get(2)?.as_str().into();
        let a2 : String = c.get(3)?.as_str().into();
        match op {
            "write" => Some(Event::Write(a1, a2)),
            "read" => Some(Event::Read(a1, a2)),
            "post" => Some(Event::Post(a1, a2)),
            _ => None,
        }
    });

    let head = vec![Event::Get(mid.into())];
    let evs :Vec<Event> = head.into_iter().chain(sevs).collect();

    let m = Message {
        id: mid.into(),
        evs,
    };
    println!("Parsed: {:?}", m);
    Some(m)
}

pub fn parse_str(s : &String) -> Vec<Handler> {
    let mut handlers : Vec<Handler> = vec![];
    let mut active_handler : String = "NONE".into();
    let mut msgs : Vec<Message> = vec![];
    let handler_strs : Vec<String> = s.split('@').map(|x| x.replace('\n', " ").into()).skip(1).collect();

    let msg_regex = Regex::new(r"\{.*\}").unwrap();

    handler_strs.iter().map(|h| {
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
    }).collect()
}
