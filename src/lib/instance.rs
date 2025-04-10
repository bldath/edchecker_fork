use std::{collections::HashMap, fs::read, ops::Not, process::exit};

use ast::{forall_const, Ast, Bool};
use itertools::Itertools;

use petgraph::algo::has_path_connecting;
use serde_json::de::Read;
use z3::{ast::Dynamic, *};

use crate::{
    cli::ADT,
    model::{EdgeTp, Event, ExecutionGraph, Idx, ReadResult},
    msg_algorithms::transitive_closure,
};

pub struct InstanceBuilder {
    pub data: HashMap<String, HashMap<String, Vec<Event>>>,
    pub edges: Vec<(EdgeTp, Idx, Idx)>,
}

impl InstanceBuilder {
    fn new(data: ReadResult) -> Self {
        Self {
            data: data.events,
            edges: data.edges,
        }
    }

    fn build(self, ctx: &Context) -> Instance<'_> {
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

        //println!("Edges: {:?}", edges);

        let order = FuncDecl::partial_order(ctx, &tp, 0);
        //let order = FuncDecl::new(ctx, "hb", &[&tp, &tp], &Sort::bool(ctx));

        //println!("Returning distilled instance");
        Instance {
            z3_ctx: ctx,
            solver: Solver::new(ctx),
            events: messages,
            msg_type: tp,
            order,
            consts,
            indices,
            checks,
            edges: self.edges,
        }
    }
}

pub struct Instance<'ctx> {
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

impl<'ctx> Instance<'ctx> {
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
        //println!("CO/RF/FR/PB edges");
        for (_, m1, m2) in self.edges.iter() {
            match (consts.get(m1), consts.get(m2)) {
                (Some(constvl), Some(constvl2)) => {
                    solver.assert(&hb.apply(&[constvl, constvl2]).as_bool().unwrap());
                }
                _ => {
                    //println!("Failed to find consts for edge {:?} -> {:?}", m1, m2);
                }
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

pub fn construct_instance(ctx: &Context, read_res: ReadResult) -> Instance<'_> {
    let instance = InstanceBuilder::new(read_res);
    let solver = Solver::new(ctx);

    let mut params = Params::new(ctx);
    params.set_bool("model.compact", false);
    solver.set_params(&params);

    //println!("Distilling instance");
    let distilled = instance.build(ctx);
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
