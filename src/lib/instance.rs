use std::{collections::HashMap, fs::read, ops::Not, process::exit};

use ast::{forall_const, Ast, Bool};
use itertools::Itertools;

use petgraph::algo::has_path_connecting;
use z3::{ast::Dynamic, *};

use crate::{
    cli::ADT,
    model::{EdgeTp, Event, ExecutionGraph, ReadResult},
    msg_algorithms::transitive_closure,
};

type Idx = (String, String, usize);
pub struct Instance {
    pub data: HashMap<String, HashMap<String, Vec<Event>>>,
    pub rf: Vec<(EdgeTp, Idx, Idx)>,
    pub fr: Vec<(EdgeTp, Idx, Idx)>,
    pub co: Vec<(EdgeTp, Idx, Idx)>,
    pub pb: Vec<(EdgeTp, Idx, Idx)>,
}

impl Instance {
    fn new(data: &ReadResult) -> Self {
        let (data, edges) = data;
        let iter = data
            .iter()
            .flat_map(|(hdl, msgs)| {
                msgs.iter().flat_map(|(mid, evs)| {
                    evs.iter()
                        .enumerate()
                        .map(|(idx, ev)| (hdl.clone(), mid.clone(), idx, ev.clone()))
                })
            })
            .collect_vec();

        let mut writers = HashMap::<(String, String), Idx>::new();
        let mut readers = HashMap::<(String, String), Vec<Idx>>::new();
        let mut posts = HashMap::<(String, String), Idx>::new();

        for (hdl, mid, idx, ev) in iter {
            let this = (hdl.clone(), mid.clone(), idx);
            match ev {
                Event::Write(var, val) => {
                    writers.insert((var.clone(), val.clone()), this);
                }
                Event::Read(var, val) => {
                    readers
                        .entry((var.clone(), val.clone()))
                        .or_default()
                        .push(this);
                }
                Event::Post(hdl, msg) => {
                    posts.insert((hdl.clone(), msg.clone()), this);
                }
                _ => {}
            }
        }

        let rf = readers
            .iter()
            .flat_map(|(vv, reads)| {
                if let Some(write) = writers.get(vv) {
                    reads
                        .iter()
                        .map(|r| (EdgeTp::RF, write.clone(), r.clone()))
                        .collect_vec()
                } else {
                    vec![]
                }
            })
            .collect_vec();

        let pb = posts
            .iter()
            .map(|((hdl, msg), post)| {
                (
                    EdgeTp::PB,
                    post.clone(),
                    (hdl.clone(), msg.clone(), 0_usize),
                )
            })
            .collect_vec();

        let co = edges
            .iter()
            .filter_map(|q| {
                if let (EdgeTp::CO, Event::Write(var, val), Event::Write(var2, val2)) = q {
                    Some((
                        EdgeTp::CO,
                        writers[&(var.clone(), val.clone())].clone(),
                        writers[&(var2.clone(), val2.clone())].clone(),
                    ))
                } else {
                    //println!("Not a CO edge: {:?}", q);
                    None
                }
            })
            .collect_vec();

        let fr = vec![];

        Self {
            data: data.clone(),
            rf,
            fr,
            pb,
            co,
        }
    }

    fn distill(self, ctx: &Context) -> DistilledInstance<'_> {
        let messages: HashMap<String, HashMap<String, usize>> = self
            .data
            .iter()
            .map(|(hdl, msgs)| {
                (
                    hdl.clone(),
                    msgs.iter()
                        .map(|(mid, evs)| (mid.clone(), evs.len()))
                        .collect(),
                )
            })
            .collect();

        let msg_ids: Vec<(String, String, String, usize)> = messages
            .iter()
            .flat_map(|(hdl, msg)| {
                msg.iter().flat_map(move |(mid, len)| {
                    (0..*len).map(move |i| {
                        (
                            format!("{}_{}_{}", hdl, mid, i),
                            hdl.clone(),
                            mid.clone(),
                            i,
                        )
                    })
                })
            })
            .collect_vec();

        let (tp, const_vec, check_vec) = Sort::enumeration(
            ctx,
            "Msg".into(),
            &msg_ids.iter().map(|x| x.0.clone().into()).collect_vec(),
        );

        //println!("Message sort: {:?}", const_vec);

        //println!("Creating consts");
        let consts: HashMap<Idx, Dynamic<'_>> = msg_ids
            .iter()
            .cloned()
            .map(move |(x, y, z, w)| (y, z, w))
            .zip(const_vec.into_iter().map(|x| x.apply(&[])))
            .collect();
        let indices = consts.iter().map(|(k, v)| (v.clone(), k.clone())).collect();

        //println!("Consts: {:?}", consts);
        //println!("Creating checks");
        let checks: HashMap<Idx, FuncDecl<'_>> = msg_ids
            .iter()
            .cloned()
            .map(move |(x, y, z, w)| (y, z, w))
            .zip(check_vec)
            .collect();

        let edges = self
            .rf
            .iter()
            .chain(self.co.iter())
            .chain(self.pb.iter())
            .chain(self.fr.iter())
            .cloned()
            .collect_vec();
        //println!("Edges: {:?}", edges);

        let order = FuncDecl::partial_order(ctx, &tp, 0);
        //let order = FuncDecl::new(ctx, "hb", &[&tp, &tp], &Sort::bool(ctx));

        //println!("Returning distilled instance");
        DistilledInstance {
            z3_ctx: ctx,
            solver: Solver::new(ctx),
            events: messages,
            msg_type: tp,
            order,
            consts,
            indices,
            checks,
            edges,
        }
    }
}

pub struct DistilledInstance<'ctx> {
    z3_ctx: &'ctx Context,
    solver: Solver<'ctx>,
    events: HashMap<String, HashMap<String, usize>>,
    msg_type: Sort<'ctx>,
    pub order: FuncDecl<'ctx>,
    consts: HashMap<Idx, Dynamic<'ctx>>,
    pub indices: HashMap<Dynamic<'ctx>, Idx>,
    checks: HashMap<Idx, FuncDecl<'ctx>>,
    edges: Vec<(EdgeTp, Idx, Idx)>,
}

impl<'ctx> DistilledInstance<'ctx> {
    pub fn assert(&self, solver: &Solver) {
        let ctx: &Context = self.z3_ctx;
        let (msg_type, consts, checks) = (&self.msg_type, &self.consts, &self.checks);

        let hb = &self.order;

        // PO
        //println!("Program order");
        for (hdl, msgs) in self.events.iter() {
            for (mid, evs) in msgs.iter() {
                for i in 1..*evs {
                    //println!("{} {}: {} -> {}", hdl, mid, i-1, i);
                    let last = (hdl.clone(), mid.clone(), i - 1);
                    let this = (hdl.clone(), mid.clone(), i);

                    let constvl = &consts[&last];
                    let constvl2 = &consts[&this];
                    solver.assert(&hb.apply(&[constvl, constvl2]).as_bool().unwrap());
                    solver.assert(&hb.apply(&[constvl2, constvl]).as_bool().unwrap().not());
                }
            }
        }

        // edges from base
        //println!("CO/RF/PB edges");
        for (_, m1, m2) in self.edges.iter() {
            match (consts.get(m1), consts.get(m2)) {
                (Some(constvl), Some(constvl2)) => {
                    solver.assert(&hb.apply(&[constvl, constvl2]).as_bool().unwrap());
                }
                _ => {
                    println!("Failed to find consts for edge {:?} -> {:?}", m1, m2);
                }
            }
        }

        // FR edges
        //println!("FR edges");
        let co = self.edges.iter().filter(|(tp, _, _)| *tp == EdgeTp::CO);
        let rf = self
            .edges
            .iter()
            .filter(|(tp, _, _)| *tp == EdgeTp::RF)
            .collect_vec();

        for (_, a, b) in co {
            for (_, c, d) in rf.iter() {
                let a = &consts[a];
                let b = &consts[b];
                let c = &consts[c];
                let d = &consts[d];

                if c != a {
                    continue;
                } // We have d --[rf^-1 . co]-> b

                solver.assert(&hb.apply(&[d, b]).as_bool().unwrap());
            }
        }

        // EO requirement
        //println!("EO requirement");
        for (hdl, msgs) in self.events.iter() {
            // For every handler
            for (m1, m2) in msgs.iter().tuple_combinations() {
                // And every pair of messages in that handler
                //println!("{:?} {:?}", m1, m2);
                let m1getidx: &Idx = &(hdl.clone(), m1.0.clone(), 0_usize);
                let m1get = &consts[m1getidx];
                let m1done = &consts[&(hdl.clone(), m1.0.clone(), *m1.1 - 1)];
                let m2get = &consts[&(hdl.clone(), m2.0.clone(), 0)];
                let m2done = &consts[&(hdl.clone(), m2.0.clone(), *m2.1 - 1)];

                //println!("{:?} → {:?} ∨ {:?} → {:?}", m1done, m2get, m2done, m1get);

                let m12 = &hb.apply(&[m1done, m2get]).as_bool().unwrap();
                let m21 = &hb.apply(&[m2done, m1get]).as_bool().unwrap();

                solver.assert(&Bool::or(ctx, &[m12, m21]));
            }
        }

        // ???
        //println!("{:?}", solver);
    }

    pub fn add_do(&self, solver: &Solver, adt: ADT) {
        match adt {
            ADT::Multiset => (),
            ADT::Queue => self.queue_do(solver),
            ADT::Stack => self.stack_do(solver),
            ADT::Register => self.reg_do(solver),
        }
    }

    pub fn queue_do(&self, solver: &Solver) {
        let ctx = self.z3_ctx;
        let consts = &self.consts;
        let hb = &self.order;

        //println!("Adding queue do edges");
        // do = pb^{-1} . mo . pb

        let pb = self
            .edges
            .iter()
            .filter(|(tp, _, _)| *tp == EdgeTp::PB)
            .map(|(_, a, b)| (a, b));
        // MO is represented by hb. If hb then mo.

        let pairs = pb.permutations(2).map(|it| (it[0], it[1]));

        for ((ai, bi), (ci, di)) in pairs {
            if let (Some(a), Some(b), Some(c), Some(d)) = (
                consts.get(ai),
                consts.get(bi),
                consts.get(ci),
                consts.get(di),
            ) {
                if a == c {
                    continue;
                } // We do not want to add do edges for the same message
                if di.0 != bi.0 {
                    continue;
                } // We need to be on the same handler
                  //println!("Adding do edge constraint: {:?} -> {:?}", b, d);

                // We know:
                //      a (post) --[pb] -> b (get)
                //      c (post) --[pb] -> d (get)
                // Now, if a --[hb] -> c
                let ac = hb.apply(&[a, c]).as_bool().unwrap();
                // b --[pb^{-1}]-> a --[hb]-> c --[pb]-> d
                // b.done --[do]-> d
                let (bh, bm, bidx) = &bi;
                let b_done = (bh.clone(), bm.clone(), self.events[bh][bm] - 1);
                let b_done = &consts[&b_done];

                let bd = hb.apply(&[b_done, d]).as_bool().unwrap();
                solver.assert(&ac.implies(&bd));
            } else {
                continue;
            }
        }
    }

    fn stack_do(&self, solver: &Solver) {
        let consts = &self.consts;
        let ctx = self.z3_ctx;

        let pb = self
            .edges
            .iter()
            .filter(|(tp, _, _)| *tp == EdgeTp::PB)
            .map(|(_, a, b)| (a, b));

        let pairs = pb.permutations(2).map(|it| (it[0], it[1]));

        for ((ai, bi), (ci, di)) in pairs {
            if let (Some(a), Some(b), Some(c), Some(d)) = (
                consts.get(ai),
                consts.get(bi),
                consts.get(ci),
                consts.get(di),
            ) {
                if a == c {
                    continue;
                } // We do not want to add do edges for the same message
                if di.0 != bi.0 {
                    continue;
                } // We need to be on the same handler

                // a --[pb] -> b

                // We know:
                //      a (post) --[pb] -> b (get)
                //      c (post) --[pb] -> d (get)
                let post_mo_post = &self.order.apply(&[a, c]).as_bool().unwrap();
                // b --[pb^-1]-> a --[mo]-> c
                let hb_pb = &self.order.apply(&[b, d]).as_bool().unwrap();
                // b --[hb/eo]-> d --[pb^-1]-> c

                let do_ord = &self.order.apply(&[b, c]).as_bool().unwrap();

                solver.assert(&Bool::and(ctx, &[post_mo_post, hb_pb]).implies(do_ord));
            }
        }
    }
    fn reg_do(&self, solver: &Solver) {}
}

pub fn construct_instance<'ctx>(
    ctx: &'ctx Context,
    read_res: &ReadResult,
) -> DistilledInstance<'ctx> {
    let instance = Instance::new(read_res);
    let solver = Solver::new(ctx);

    let mut params = Params::new(ctx);
    params.set_bool("model.compact", false);
    solver.set_params(&params);

    //println!("Distilling instance");

    let distilled = instance.distill(ctx);
    distilled
}

#[test]
pub fn test_order() {
    let cfg = Config::default();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let (fintype, fintype_consts, fintype_check) = Sort::enumeration(
        &ctx,
        "Fintype".into(),
        &["A".into(), "B".into(), "C".into()],
    );

    let x = fintype_consts[0].apply(&[]);
    let y = fintype_consts[1].apply(&[]);

    let po = FuncDecl::linear_order(&ctx, &fintype, 0);

    solver.assert(&po.apply(&[&x, &y]).as_bool().unwrap());

    println!("{:?}", solver.check());
    println!("{:?}", solver.get_model());
}
