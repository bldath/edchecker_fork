use std::collections::vec_deque::Iter;
use std::collections::VecDeque;
use itertools::{Combinations, CombinationsWithReplacement, Itertools};
use petgraph::algo::{has_path_connecting, is_cyclic_directed, toposort, Cycle};
use petgraph::graph::NodeIndex;
use crate::write_dot;
use crate::{model::{EGraph, EdgeTp::*, MGraph, MGraphE}, preprocess::get_pairs};


pub fn transitive_closure(g : &mut MGraph) {
    let q = get_pairs(&g, |x, y| {
        x != y && !(g.contains_edge(x, y) || g.contains_edge(y, x))
    });
    let mut m = false;
    for (q1, q2) in q {
        if has_path_connecting(&*g, q1, q2, None) {
            g.add_edge(q1, q2, ());
            m = true;
        } else if has_path_connecting(&*g, q2, q1, None) {
            g.add_edge(q2, q1, ());
            m = true;
        }
    }
    if m {
        //println!("Modified once");
        transitive_closure(g)
    }
}

pub fn flip_iterator<A>(v: Vec<(A,A)>) -> impl Iterator<Item = Vec<(A, A)>>  where A : Clone {
    vec![true, false].into_iter().combinations_with_replacement(v.len()).map(move |q| {
        v.iter().zip(q.iter()).map(| ((a, b), flip) | {
            if *flip { (b.clone(), a.clone()) } else { (a.clone(), b.clone()) }
        }).collect_vec()
    })
}

pub fn get_total_mo(g : &MGraph) -> impl Iterator<Item = MGraph> {
    let v = get_pairs(g, | x, y | {
        x < y && !(g.contains_edge(x, y) || g.contains_edge(y, x))
    });
    let tmp = g.clone();

    flip_iterator(v).map(move | bv | {
        let mut ng = tmp.clone();
        for (a, b) in bv {
            ng.add_edge(a, b, ());
        }
        ng
    })
}

pub fn extend_graph(g : &EGraph, mg : &MGraph) -> EGraph {
    let mut gp = g.clone();
    //println!("Extending");
    for e in mg.edge_indices() {
        if let Some ((q1i, q2i)) = mg.edge_endpoints(e) {
            let MGraphE(b1, n1, m1) = &mg[q1i];
            let MGraphE(b2, n2, m2) = &mg[q2i];
            if !(has_path_connecting(g, *n1, *n2, None)) {
                //println!("Adding edge: ({:?}, {:?})", n1, n2);
                gp.add_edge(*n1, *n2, MO);
            }
        }
    }
    gp
}


pub fn get_sequence(mg : &MGraph) -> Result<Vec<MGraphE>, Cycle<NodeIndex>> {
    toposort(mg, None).map(| x | x.iter().map(| xx | mg[*xx].clone()).collect())
}

pub fn extend_valid_multiset(g : &EGraph, mg : &MGraph) -> bool {
    let gp = extend_graph(g, mg);
    let _ = write_dot(&gp, "res.dot".into());
    //println!("Cyclic: {:?}", is_cyclic_directed(&gp));
    !is_cyclic_directed(&gp)

}

pub fn valid_queue(q : Vec<MGraphE>) -> bool {
    let mut s : VecDeque<String> = VecDeque::from([]);
    for MGraphE(bl, _on, mid) in q {
        if bl {
            // Get
            if let Some(m) = s.pop_front() {
                if m != mid {
                    //println!("{:?} != {:?}", m, mid);
                    return false
                }
            } else {
                // For now, allow all deqEmpty, assume it is an init event.
            }

        } else {
            s.push_back(mid);
        }
    }
    true
}

pub fn extend_valid_queue(g : &EGraph, mg : &MGraph) -> bool {
    if let Ok(v) = get_sequence(mg) {
        //println!("{:?}: {:?} && {:?}", v, valid_queue(v.clone()), extend_valid_multiset(g, mg));
        valid_queue(v) && extend_valid_multiset(g, mg)
    } else { false }
}
