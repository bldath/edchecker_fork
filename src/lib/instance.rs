use std::{collections::HashMap, fs::read};

use ast::{forall_const, Ast, Bool};
use itertools::Itertools;

use petgraph::algo::has_path_connecting;
use z3::{*, ast::Dynamic};

use crate::{model::{EdgeTp, Event, ExecutionGraph, ReadResult}, msg_algorithms::transitive_closure};

pub fn run(eg: &ExecutionGraph) -> Result<(), Box<dyn std::error::Error>> {
    let (graph, data) = eg;

    let ctx = Context::new(&Config::default());

    /// Get all handlers and messages as strings
    let handler_strs : Vec<String> = data.keys().cloned().collect();
    let handler_map : HashMap<String, usize> = handler_strs.iter().enumerate().map(|(idx, x)| (x.clone(), idx)).collect();
    /// Define the type of messages on a given handler
    let message_types = data.iter().map(|( hdl , v)| {
        Sort::enumeration(&ctx, hdl.clone().into(), &v.iter().map(|(mid, evs)| mid.clone().into()).collect_vec())
    }).collect_vec();


    let solver = Solver::new(&ctx);
    let mut params = Params::new(&ctx);
    params.set_bool("model.compact", false);
    solver.set_params(&params);

    let mut rel_idx = 0;

    let eos = message_types.iter().map(| (sort, consts, check) | {
        rel_idx += 1;
        FuncDecl::new(&ctx, format!("eo_{}", rel_idx), &[&sort, &sort], &Sort::bool(&ctx))
    }).collect_vec();


    for (eo_idx, ((hdl, messages), types)) in data.iter().zip(message_types.iter()).enumerate() {
        println!("{}", hdl);
        let mid_tp = &types.0;
        let iter = messages.iter().zip(types.1.iter()).zip(types.2.iter());

        let eo = &eos[eo_idx];

        for (m1, m2) in iter.tuple_combinations() {
            let (((mid, evs), constvl), check) = m1;
            let (((mid2, evs2), constvl2), check2) = m2;
            println!("{:?} {:?}", mid, mid2);


            let m1get = evs[0];
            let m2get = evs2[0];
            let m1done = *evs.last().unwrap();
            let m2done = *evs2.last().unwrap();

            if has_path_connecting(graph, m1get, m2done, None) {
                println!("Path?");
                solver.assert(&eo.apply(&[&constvl.apply(&[]), &constvl2.apply(&[])]).as_bool().unwrap());
            }
            if has_path_connecting(graph, m2get, m1done, None) {
                println!("Path?");
                solver.assert(&eo.apply(&[&constvl2.apply(&[]), &constvl.apply(&[])]).as_bool().unwrap());
            }
        }
    }


    println!("{:?}", solver.to_smt2());

    
    println!("{:?}", solver.check());
    println!("{:?}", solver.get_model());
    Ok(())




    // let s : FuncDecl = "(define-fun R ((x A) (y A)) Bool ((_ partial-order 0) x y))";
    // let f = FuncDecl::new(&ctx, "hdl_of", &[&msg_sort], &msg_sort);

    // let mut ctr = 0;
    // for (h, hmsgs) in data {
    //     for (mid, msg) in hmsgs {
    //         let vl = z3::ast::Int::new_const(&ctx, mid.clone());
    //         solver.assert(
    //             &f.apply(&[&vl]).as_int().unwrap()._eq(&Int::from_u64(&ctx, ctr))
    //         )
    //     }
    //     ctr += 1

    // }

    // let x = ast::Int::new_const(&ctx, "x");
    // let y = ast::Int::new_const(&ctx, "y");
    // let fxy = f.apply(&[&x]).as_int().unwrap()._eq(&f.apply(&[&y]).as_int().unwrap());
    // let fxy_pat = Pattern::new(&ctx, &[&fxy]);

    // let forall: Bool = forall_const(&ctx, &[&x, &y], &[&fxy_pat],
    //                                 &R.apply(&[&x, &y]).as_bool().unwrap()
    // );

    // solver.assert(&forall);
    // for (m1, m2) in msgs.iter().tuple_combinations() {
    //     solver.assert(
    //         &m1.lt(&m2)
    //     );
    //     solver.assert(
    //         &m2.lt(&m1)
    //     );
    // }

    // let res = solver.check();

    // println!("{:?}", res);
    // if res == SatResult::Sat {
    //     println!("{:?}", solver.get_model());
    // }


    // let cells: [[z3::ast::BV; 9]; 9] = [[0; 9]; 9];
    // for rr in 0..9 {
    // for cc in 0..9 {
    //     // using z3
    //     cells[rr][cc] = z3::ast::BV::new_const(&ctx, format!("cell_{}_{}", rr, cc), 16);

    //     if let Some(val) = known_values[rr][cc] {
    //         // using z3d
    //         //                   ^^^^^^^^^^^^^      ^^^^^^^^^^^^^^^^^
    //         //     arbitrary Rust expression ^      ^ cast to bitvector

    //         // using z3
    //         //solver.assert(&cells[rr][cc]._eq(&ctx.bitvector_sort(16).from_i64(val)));
    //     }
    // }    
}


type Idx = (String, String, usize);
pub struct Instance {
    pub data: HashMap<String, HashMap<String, Vec<Event>>>,
    pub rf: Vec<(Idx, Idx)>,
    pub fr: Vec<(Idx, Idx)>,
    pub co: Vec<(Idx, Idx)>,
    pub pb: Vec<(Idx, Idx)>,
}

impl Instance {

    fn new(data: &ReadResult) -> Self {
        let (data, edges) = data;
        let iter = data.iter().flat_map(|(hdl, msgs)| {
            msgs.iter().flat_map(|(mid, evs)| {
                evs.iter().enumerate().map(|(idx, ev)| (hdl.clone(), mid.clone(), idx.clone(), ev.clone()))
            })
        }).collect_vec();

        let mut writers = HashMap::<(String, String), Idx>::new();
        let mut readers = HashMap::<(String, String), Vec<Idx>>::new();
        let mut posts = HashMap::<(String, String), Idx>::new();

        for (hdl, mid, idx, ev) in iter {
            let this = (hdl.clone(), mid.clone(), idx);
            match ev {
                Event::Write(var, val) => { writers.insert((var.clone(), val.clone()), this); },
                Event::Read(var, val) => {
                    readers.entry((var.clone(), val.clone())).or_insert(vec![]).push(this);
                }
                Event::Post(hdl, msg) => { posts.insert((hdl.clone(), msg.clone()), this); }
                _ => {}
            }
        }

        let rf = readers.iter().flat_map(|(vv, reads)| {
            let write = writers.get(vv).unwrap();
            reads.iter().map(|r| (write.clone(), r.clone())).collect_vec()
        }).collect_vec();

        let pb = posts.iter().map(|((hdl, msg), post)| {
            (post.clone(), (hdl.clone(), msg.clone(), 0 as usize))
        }).collect_vec();

        let co = edges.iter().filter_map(|q| {
            if let (EdgeTp::CO, Event::Write(var, val), Event::Write(var2, val2)) = q {
                Some((writers[&(var.clone(), val.clone())].clone(), writers[&(var2.clone(), val2.clone())].clone()))
            } else {
                None
            }
        }).collect_vec();

        let fr = vec![];

        Self {
            data: data.clone(),
            rf,
            fr,
            pb,
            co,
        }
    }


    fn distill<'ctx> (self, ctx: &'ctx Context) -> DistilledInstance<'ctx> {
        let messages : HashMap<String, HashMap<String, usize>> = self.data.iter().map(| (hdl, msgs) | {
            (hdl.clone(), msgs.iter().map(|(mid, evs)| (mid.clone(), evs.len())).collect())
        }).collect();

        let msg_ids: Vec<(String, String, String, usize)> = messages.iter().flat_map(| (hdl, msg) | {
            msg.iter().map(move |(mid, len) | (format!("{}_{}_{}", hdl, mid, len), hdl.clone(), mid.clone(), *len))
        }).collect_vec();


        println!("Creating message sort");
        let (tp, const_vec, check_vec) = Sort::enumeration(
            &ctx, 
            "Msg".into(), 
            &msg_ids.iter().map(|x| x.0.clone().into()).collect_vec()
        );

        println!("Creating consts");
        let consts: HashMap<Idx, Dynamic<'_>> = msg_ids.iter().cloned().map(move | (x, y, z, w) | (y, z, w)).zip(const_vec.into_iter().map(|x| x.apply(&[]))).collect();

        println!("Creating checks");
        let checks: HashMap<Idx, FuncDecl<'_>> = msg_ids.iter().cloned().map(move |(x, y, z, w)| (y, z, w)).zip(check_vec.into_iter()).collect();

        let edges = self.rf.iter().chain(self.co.iter()).chain(self.pb.iter()).chain(self.fr.iter()).cloned().collect_vec();

        println!("Returning distilled instance");
        DistilledInstance {
            z3_ctx: ctx,
            events: messages,
            msg_type: tp,
            consts,
            checks,
            edges,
        }

    }

}


struct DistilledInstance<'ctx> {
    z3_ctx: &'ctx Context,
    events: HashMap<String, HashMap<String, usize>>,
    msg_type: Sort<'ctx>,
    consts: HashMap<Idx, Dynamic<'ctx>>,
    checks: HashMap<Idx, FuncDecl<'ctx>>,
    edges: Vec<(Idx, Idx)>,
}


impl<'ctx> DistilledInstance<'ctx> {
    pub fn assert(&self, solver: &Solver) {
        let ctx = self.z3_ctx;
        let (msg_type, consts, checks) = (&self.msg_type, &self.consts, &self.checks);


        // Construct PO
        let po = FuncDecl::piecewise_linear_order(ctx, msg_type, 0);
        
        self.events.iter().map(| (hdl, msgs) | {
            for (mid, evs) in msgs.iter() {
                for i in 1..*evs {
                    let constvl = &consts[&(*hdl, *mid, i - 1)];
                    let constvl2 = &consts[&(*hdl, *mid, i)];
                    solver.assert(&po.apply(&[constvl, constvl2]).as_bool().unwrap());
                }
                
            }
        });

        // Construct RF U CO U FR U PB
        let hb = FuncDecl::partial_order(&ctx, msg_type, 1);

        for (m1, m2) in self.edges.iter() {
            let constvl = &consts[m1];
            let constvl2 = &consts[m2];
            solver.assert(&hb.apply(&[constvl, constvl2]).as_bool().unwrap());
        }

        // Construct EO
        let eo = FuncDecl::piecewise_linear_order(&ctx, msg_type, 2);
        for (hdl, msgs) in self.events.iter() {
            for (m1, m2) in msgs.iter().tuple_combinations() {
                let m1getidx : &Idx = &(*hdl, *m1.0, 0 as usize); 
                let m1get = &consts[&(*hdl, *m1.0, 0)];
                let m1done = &consts[&(*hdl, *m1.0, *m1.1 - 1)];
                let m2get = &consts[&(*hdl, *m2.0, 0)];
                let m2done = &consts[&(*hdl, *m2.0, *m2.1 - 1)];

                let m12 = &eo.apply(&[m1done, m2get]).as_bool().unwrap();
                let m21 = &eo.apply(&[m2done, m1get]).as_bool().unwrap();

                solver.assert(&Bool::or(&ctx, &[m12, m21]));
            }
        }
    }
}



pub fn construct_instance(read_res: &ReadResult) {
    let instance = Instance::new(read_res);

    let ctx = Context::new(&Config::default());
    let solver = Solver::new(&ctx);

    let mut params = Params::new(&ctx);
    params.set_bool("model.compact", false);
    solver.set_params(&params);

    println!("Distilling instance");

    let distilled = instance.distill(&ctx);

    // This is a bit shady, since we do not require solver to be mutable
    // despite the fact that we are mutating its internal state in z3
    distilled.assert(&solver);

    println!("{:?}", solver.check());
    println!("{:?}", solver.get_model());    
}


#[test]
pub fn test_order() {
    let ctx = Context::new(&Config::default());
    let solver = Solver::new(&ctx);

    let (fintype, fintype_consts, fintype_check) = Sort::enumeration(&ctx, "Fintype".into(), &["A".into(), "B".into(), "C".into()]);

    let x = fintype_consts[0].apply(&[]);
    let y = fintype_consts[1].apply(&[]);

    
    let po = FuncDecl::linear_order(&ctx, &fintype, 0);

    solver.assert(&po.apply(&[&x, &y]).as_bool().unwrap());

    println!("{:?}", solver.check());
    println!("{:?}", solver.get_model());

}
