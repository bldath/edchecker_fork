use std::marker::PhantomData;

use clap::ValueEnum;
use itertools::{iproduct, Itertools};
use petgraph::{
    algo::has_path_connecting,
    graph::{Frozen, NodeIndex},
};
use serde::Serialize;

use crate::{
    algorithms::add_edges,
    cli::ADT,
    model::{
        Argument, EGraph, EGraphData, EPair,
        EdgeTp::{self, *},
        Event,
    },
    preprocess::pair_fmap,
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Hash, Serialize)]
pub enum Heuristic {
    No,
    Simple,
    Full,
}

pub fn heuristic_1(g: &mut EGraph, data: &EGraphData) {
    // If there is a path from some event in a message to some event in another message,
    // if they are on the same handler, the first message must happen before the other.
    let mut mg = g.clone();
    let fg = Frozen::new(&mut mg);
    let q = data
        .iter()
        .flat_map(|(hdl, msgs)| {
            msgs.iter()
                .tuple_combinations()
                .filter_map(|((m1, evs1), (m2, evs2))| {
                    let x1 = *evs1.first().unwrap();
                    let xn = *evs1.last().unwrap();

                    let y1 = *evs2.first().unwrap();
                    let yn = *evs2.last().unwrap();

                    if has_path_connecting(&*fg, x1, yn, None) && !g.contains_edge(xn, y1) {
                        Some((EO, xn, y1))
                    } else if has_path_connecting(&*fg, y1, xn, None) && !g.contains_edge(yn, x1) {
                        Some((EO, yn, x1))
                    } else {
                        None
                    }
                })
        })
        .collect_vec();
    if !q.is_empty() {
        add_edges(g, q);
        heuristic_1(g, data);
    }
}

pub fn simple_heuristic_mo(g: &mut EGraph) {
    // If there is a path from one post to another, then we add a MO in that direction.
    let q = pair_fmap(g, |x, y| match (&g[x], &g[y]) {
        (EPair(hdl1, mid1, Event::Post(rcv1, rm1)), EPair(hdl2, mid2, Event::Post(rcv2, rm2))) => {
            if rcv1 == rcv2 && x != y && has_path_connecting(&g.clone(), x, y, None) {
                Some((MO, x, y))
            } else {
                None
            }
        }
        _ => None,
    })
    .into_iter()
    .unique()
    .collect_vec();

    add_edges(g, q);
}

pub fn heuristic_2(g: &mut EGraph, data: &EGraphData) {
    let fg = g.clone();
    let q: Vec<(EdgeTp, NodeIndex, NodeIndex)> = data
        .iter()
        .tuple_combinations()
        .flat_map(|(a, b)| vec![(a, b), (b, a)])
        .flat_map(|((h1, msgs1), (h2, msgs2))| {
            iproduct!(
                msgs1
                    .iter()
                    .tuple_combinations()
                    .flat_map(|(a, b)| vec![(a, b), (b, a)]),
                msgs2
                    .iter()
                    .tuple_combinations()
                    .flat_map(|(a, b)| vec![(a, b), (b, a)]),
            )
            .filter_map(|(((m1, ev1), (m2, ev2)), ((m3, ev3), (m4, ev4)))| {
                // Guess two events in each message
                iproduct!(
                    ev1.iter().tuple_combinations(),
                    ev2.iter().tuple_combinations(),
                    ev3.iter().tuple_combinations(),
                    ev4.iter().tuple_combinations()
                )
                .find(|((&m11, &m12), (&m21, &m22), (&m31, &m32), (&m41, &m42))| {
                    has_path_connecting(&fg, m11, m31, None)
                        && has_path_connecting(&fg, m21, m32, None)
                        && has_path_connecting(&fg, m41, m12, None)
                        && has_path_connecting(&fg, m42, m22, None)
                })
                .map(|qq| (EdgeTp::EO, *ev4.last().unwrap(), *ev3.first().unwrap()))
            })
        })
        .collect_vec();
    add_edges(g, q);
}

pub fn heuristic_3(g: &mut EGraph, data: &EGraphData) {
    let fg = g.clone();
    let q: Vec<(EdgeTp, NodeIndex, NodeIndex)> = data
        .iter()
        .tuple_combinations()
        .flat_map(|(a, b)| vec![(a, b), (b, a)])
        .flat_map(|((h1, msgs1), (h2, msgs2))| {
            iproduct!(
                msgs1
                    .iter()
                    .tuple_combinations()
                    .flat_map(|(a, b)| vec![(a, b), (b, a)]),
                msgs2
                    .iter()
                    .tuple_combinations()
                    .flat_map(|(a, b)| vec![(a, b), (b, a)]),
            )
            .filter_map(|(((m1, ev1), (m2, ev2)), ((m3, ev3), (m4, ev4)))| {
                // Guess two events in each message
                iproduct!(ev1.iter(), ev2.iter(),)
                    .find(|((&m11), (&m21))| {
                        let post1 = &fg[m11].2;
                        let post2 = &fg[m21].2;

                        if let (Event::Post(p1h, p1m), Event::Post(p2h, p2m)) = (post1, post2) {
                            //the events are posts
                            if *p1h == *h2 && *p2h == *h2 && p1m == m3 && p2m == m4 {
                                // The posts are the right posts
                                if has_path_connecting(&fg, m11, m21, None) {
                                    return true;
                                }
                            }
                        }
                        false
                    })
                    .map(|qq| (EdgeTp::EO, *ev3.last().unwrap(), *ev4.first().unwrap()))
            })
        })
        .collect_vec();

    add_edges(g, q);
}

pub fn heuristic_4(g: &mut EGraph, data: &EGraphData) {
    let fg = g.clone();
    let q = data
        .iter()
        .tuple_combinations()
        .flat_map(|(a, b)| vec![(a, b), (b, a)])
        .flat_map(|((h1, msgs1), (h2, msgs2))| {
            iproduct!(
                msgs1
                    .iter()
                    .tuple_combinations()
                    .flat_map(|(a, b)| vec![(a, b), (b, a)]),
                msgs2
                    .iter()
                    .tuple_combinations()
                    .flat_map(|(a, b, c)| [a, b, c]
                        .iter()
                        .permutations(3)
                        .map(|v| (*v[0], *v[1], *v[2]))
                        .collect_vec())
            )
            .filter_map(
                |(((m1, evs1), (m2, evs2)), ((m3, evs3), (m4, evs4), (m5, evs5)))| {
                    if let Some(qq) =
                        iproduct!(evs1.iter(), evs2.iter(), evs3.iter().tuple_combinations(),).find(
                            |(e1, e2, (e31, e32))| {
                                let p1 = &fg[**e1].2;
                                let p2 = &fg[**e2].2;

                                if let (Event::Post(p1h, p1m), Event::Post(p2h, p2m)) = (p1, p2) {
                                    if *p1h == *h2
                                        && *p2h == *h2
                                        && p1m == m4
                                        && p2m == m5
                                        && has_path_connecting(&fg, **e31, **e1, None)
                                        && has_path_connecting(&fg, **e2, **e32, None)
                                        && has_path_connecting(&fg, **e1, **e2, None)
                                    {
                                        return true;
                                    }
                                }
                                false
                            },
                        )
                    {
                        return Some((EdgeTp::EO, *evs4.last().unwrap(), *evs5.last().unwrap()));
                    }
                    None
                },
            )
        })
        .collect_vec();
    add_edges(g, q);
}

pub fn add_heuristics(g: &mut EGraph, data: &EGraphData, heur: Heuristic, adt: ADT) {
    match heur {
        Heuristic::No => (),
        Heuristic::Simple => {
            //simple_heuristic_mo(g);
            heuristic_1(g, data);
        }
        Heuristic::Full => {
            //simple_heuristic_mo(g);
            heuristic_1(g, data);
            heuristic_2(g, data);
            match adt {
                ADT::Multiset => (),
                ADT::Queue => {
                    heuristic_3(g, data);
                }
                ADT::Stack => {
                    heuristic_4(g, data);
                }
                ADT::Register => (),
            }
        }
    }
}
