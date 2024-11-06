#!/usr/bin/env python3
from pprint import pprint
from collections import defaultdict as dd
from example_nidhugg import *
import re
import sys
import os
ev_regex = re.compile(r"^\s*\(<(?P<tid>.*?)>,(?P<eid>\d+-?\d*)\)\s*(?P<hdl>-?\d+):\s*(?P<evt>.*)\s*SLP")

post_re = re.compile(r"Post\(<(?P<mid>.*?)>\)\s*")
store_re = re.compile(r"Store\(Global(?P<var>.*),(?P<val>.*)\)\s*")
load_re = re.compile(r"Load\((?P<var>.*)\)")


var_ctrs = dd(lambda: 0)
var_ctr = 0
var_ids = {}

hdl_of_msg = {}


def get_hist(trace):
    global var_ctrs, var_ctr, var_ids, hdl_of_msg
    var_ctrs.clear()
    var_ctr = 0
    var_ids.clear()
    hdl_of_msg.clear()

    def get_var(s):
        global var_ctr, var_ids
        if s in var_ids:
            return var_ids[s]

        vn = "x" + str(var_ctr)
        var_ctr += 1
        var_ids[s] = vn
        return var_ids[s]

    def wt_val(s):
        global var_ctrs
        var_ctrs[s] += 1
        return var_ctrs[s]

    def rd_val(s):
        global var_ctrs
        return var_ctrs[s]

    co_var = dd(lambda: [])


    evs = dd(lambda: dd(lambda: []))
    for ev in trace.split("\n"):
        m = ev_regex.match(ev)
        if m:
            evt = m.groupdict()

            pre = post_re.match(evt['evt'])
            sre = store_re.match(evt['evt'])
            lre = load_re.match(evt['evt'])
            hdl_of_msg[evt['tid']] = evt['hdl']
            if pre:
                q = pre.groupdict()

                evs[evt['hdl']][evt['tid']].append(('post', None, q['mid']))
            elif sre:
                q = sre.groupdict()
                var = get_var(q['var'])
                val = wt_val(var)
                evs[evt['hdl']][evt['tid']].append(('write', var, val))
                co_var[var].append(f"write({var}, {val})")
            elif lre:
                q = lre.groupdict()
                var = get_var(q['var'])
                val = rd_val(var)
                if val > 0:
                    evs[evt['hdl']][evt['tid']].append(('read', var, val))
        else:
            print(ev, " unmatched")

    return (co_var, evs)


trace_ctr = 0

def write_trace(fn, co_var, evs):
    global trace_ctr
    trace_ctr += 1
    print(f"Writing trace {fn + str(trace_ctr)}")
    with open(fn + str(trace_ctr) + ".trace", "w") as f:
        for (hdl, msgs) in evs.items():
            f.write(f"@h{hdl}\n")
            for (mid, evs) in msgs.items():
                s = "{ get(" + mid + ")"
                for q in evs:
                    op, a1, a2 = q
                    if a1 is None:
                        a1 = "h" + str(hdl_of_msg[a2])
                    s += " -> " + op + "(" + str(a1) + "," + str(a2) + ")"
                s += "}\n"
                f.write(s)
        if any(len(v) > 1 for (q, v) in co_var.items()):
            f.write("$(CO)\n")
            for (var, evs) in co_var.items():
                if len(evs) > 1:
                    f.write(" -> ".join(evs))
                    f.write(";\n")


if __name__=="__main__":
    print(sys.argv[1])
    traces = []
    with open(sys.argv[1]) as f:
        txt = f.read()
        print(f"{len(txt)} lines")
        traces = txt.split("EventTraceBuilder (debug print)")
        print(f"{len(traces)} traces")


    basename = sys.argv[1][0:-4] + "/"
    outdir = sys.argv[2]
    os.makedirs(basename, exist_ok=True)
    for t in traces[1:]:
        h = get_hist(t)
        os.makedirs(outdir + "/" + basename, exist_ok=True)
        write_trace(outdir + "/" + basename + "trace", *h)
