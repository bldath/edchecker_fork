#!/usr/bin/env python3
from pprint import pprint
from collections import defaultdict as dd
from example_nidhugg import *
import re

ev_regex = re.compile(r"^\s*\(<(?P<tid>.*?)>,(?P<eid>\d+-?\d*)\)\s*(?P<hdl>-?\d+):\s*(?P<evt>.*)\s*SLP")

post_re = re.compile(r"Post\(<(?P<mid>.*?)>\)\s*")
store_re = re.compile(r"Store\((?P<var>.*),(?P<val>.*)\)\s*")
load_re = re.compile(r"Load\((?P<var>.*)\)")

var_ctrs = dd(lambda: 0)
var_ctr = 0
var_ids = {}

evs = dd(lambda: dd(lambda: []))

hdl_of_msg = {}

def get_var(s):
    global var_ctr, var_idx
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

for ev in example2.split("\n"):
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

pprint(evs)

for (hdl, msgs) in evs.items():
    print(f"@h{hdl}")
    for (mid, evs) in msgs.items():
        s = "{ get(" + mid + ")"
        for q in evs:
            op, a1, a2 = q
            if a1 is None:
                a1 = "h" + str(hdl_of_msg[a2])

            s += " -> " + op + "(" + str(a1) + "," + str(a2) + ")"
        s += "}"
        print(s)

print("$(CO)")
for (var, evs) in co_var.items():
    if len(evs) > 1:
        print(" -> ".join(evs))
