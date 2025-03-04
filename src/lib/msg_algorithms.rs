
use crate::output::write_dot;
use crate::{
    model::{EGraph, EdgeTp::*, MGraph, MGraphE},
    preprocess::get_pairs,
};
use itertools::{repeat_n, Combinations, CombinationsWithReplacement, Itertools};
use petgraph::algo::{has_path_connecting, is_cyclic_directed, toposort, Cycle};
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use std::collections::vec_deque::Iter;
use std::collections::VecDeque;

pub fn transitive_closure<V, E>(g: &mut Graph<V, E>, ins_val: E)
where
    E: Clone,
{
    let q = get_pairs(&g, |x, y| {
        x != y && !(g.contains_edge(x, y) || g.contains_edge(y, x))
    });
    let mut m = false;
    for (q1, q2) in q {
        if has_path_connecting(&*g, q1, q2, None) {
            g.add_edge(q1, q2, ins_val.clone());
            m = true;
        } else if has_path_connecting(&*g, q2, q1, None) {
            g.add_edge(q2, q1, ins_val.clone());
            m = true;
        }
    }
    if m {
        //println!("Modified once");
        transitive_closure(g, ins_val)
    }
}

pub fn flip_iter<A>(v: &Vec<(A, A)>) -> impl Iterator<Item = Vec<(A, A)>> + '_
where
    A: Clone,
{
    v.iter().map(|(a1, a2)| {
        vec![(a1.clone(), a2.clone()), (a2.clone(), a1.clone())]
    }).multi_cartesian_product()
}


// Given a vector of tuples, return an iterator consistingof all possible flips of the tuples.
// For example, given [(1, 2), (3, 4)], the iterator will return [(1, 2), (3, 4)], [(2, 1), (3, 4)], [(1, 2), (4, 3)], [(2, 1), (4, 3)].
// With th edge type added before.
pub fn flip_iterator<A, B>(v: &Vec<(B, A, A)>) -> impl Iterator<Item = Vec<(B, A, A)>> + '_
where
    A: Clone,
    B: Clone,
{
    v.iter().map(|(b, a1, a2)| {
        vec![
            (b.clone(), a1.clone(), a2.clone()),
            (b.clone(), a2.clone(), a1.clone()),
        ]
    }).multi_cartesian_product()
}

#[cfg(test)]
mod alg_test {
    use itertools::Itertools;

    use crate::msg_algorithms::flip_iterator;

    #[test]
    fn flip_iterator_test() {
        let q = vec![("a", 1, 2), ("b", 3, 4), ("c", 5, 6)];
        let flipped = flip_iterator(&q).collect_vec();
        let res = vec![
            vec![("a", 1, 2), ("b", 3, 4), ("c", 5, 6)],
            vec![("a", 1, 2), ("b", 3, 4), ("c", 6, 5)],
            vec![("a", 1, 2), ("b", 4, 3), ("c", 5, 6)],
            vec![("a", 1, 2), ("b", 4, 3), ("c", 6, 5)],
            vec![("a", 2, 1), ("b", 3, 4), ("c", 5, 6)],
            vec![("a", 2, 1), ("b", 3, 4), ("c", 6, 5)],
            vec![("a", 2, 1), ("b", 4, 3), ("c", 5, 6)],
            vec![("a", 2, 1), ("b", 4, 3), ("c", 6, 5)],
        ];
        println!("{:?}", flipped);
        println!("{:?}", res);
        assert!(flipped == res);
    }

    #[test]
    fn flip_iterator_test2() {
        let q = vec![("a", 1, 2), ("b", 3, 4)];
        let flipped = flip_iterator(&q).collect_vec();
        let res = vec![
            vec![("a", 1, 2), ("b", 3, 4)],
            vec![("a", 1, 2), ("b", 4, 3)],
            vec![("a", 2, 1), ("b", 3, 4)],
            vec![("a", 2, 1), ("b", 4, 3)],
        ];
        println!("{:?}", flipped);
        println!("{:?}", res);
        assert!(flipped == res);
    }
}

// pub fn get_total_mo(g : &MGraph) -> impl Iterator<Item = MGraph> {
//     let v = get_pairs(g, | x, y | {
//         x < y && !(g.contains_edge(x, y) || g.contains_edge(y, x))
//     });
//     let tmp = g.clone();

//     flip_iterator(&v).map(move | bv | {
//         let mut ng = tmp.clone();
//         for (a, b) in bv {
//             ng.add_edge(a, b, ());
//         }
//         ng
//     })
// }

pub fn extend_graph(g: &EGraph, mg: &MGraph) -> EGraph {
    let mut gp = g.clone();
    //println!("Extending");
    for e in mg.edge_indices() {
        if let Some((q1i, q2i)) = mg.edge_endpoints(e) {
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

pub fn get_sequence(mg: &MGraph) -> Result<Vec<MGraphE>, Cycle<NodeIndex>> {
    toposort(mg, None).map(|x| x.iter().map(|xx| mg[*xx].clone()).collect())
}

pub fn extend_valid_multiset(g: &EGraph, mg: &MGraph) -> bool {
    let gp = extend_graph(g, mg);
    //let _ = write_dot(&gp, "res.dot".into());
    //println!("Cyclic: {:?}", is_cyclic_directed(&gp));
    !is_cyclic_directed(&gp)
}

pub fn valid_queue(q: Vec<MGraphE>) -> bool {
    let mut s: VecDeque<String> = VecDeque::from([]);
    for MGraphE(bl, _on, mid) in q {
        if bl {
            // Get
            if let Some(m) = s.pop_front() {
                if m != mid {
                    //println!("{:?} != {:?}", m, mid);
                    return false;
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

pub fn extend_valid_queue(g: &EGraph, mg: &MGraph) -> bool {
    if let Ok(v) = get_sequence(mg) {
        //println!("{:?}: {:?} && {:?}", v, valid_queue(v.clone()), extend_valid_multiset(g, mg));
        valid_queue(v) && extend_valid_multiset(g, mg)
    } else {
        false
    }
}
