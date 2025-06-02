#!/usr/bin/env python3
import os, glob
from dataclasses import dataclass, field
from pathlib import Path
from collections import defaultdict as dd
from statistics import mean

import subprocess as sp
import shutil


def preamble(adt, heur):
    return f'''
    \\documentclass{{standalone}}
    \\begin{{document}}
    \\begin{{tabular}}{{l l | l l l l | l l | r}}
    \\multicolumn{{9}}{{c}}{{{adt} {heur}}}\\\\
    Experiment & Size & Events & Messages & Handlers & Traces & Completed & OK & Avg. Runtime (s)\\\\\\hline'''

def line(rs):
    if len(rs.results) > 0:
        return f'''
        {rs.test} & {rs.param} & {rs.events} & {rs.messages} & {rs.handlers} & {rs.num_checked} & {len(rs.results)} & {rs.num_ok} & {mean(rs.results) / 1000000.0: 8.3f}\\\\'''
    else:
        return ""


postamble='''
\\end{tabular}
\\end{document}
'''

@dataclass
class Res:
    results: list[int] = field(default_factory=lambda: [])
    test: str = ""
    param: int = 0
    events: int = 0
    messages: int = 0
    handlers: int = 0
    num_checked: int = 0
    num_ok: int = 0


data = dd(lambda: dd(lambda: dd(lambda: Res())))

#data[ADT][heur][expt] : Res
def search_matches(lines):
    hdl_start = "Handlers:"
    msg_start = "Messages:"
    evt_start = "Events:"
    parse_start = "Parsing:"
    pp_start = "Preprocessing:"
    check_start = "Check:"
    tot_start = "Total:"
    res_start = "Result:"

    pats = [hdl_start, msg_start, evt_start, parse_start, pp_start, check_start, tot_start, res_start]
    lctr = 0
    res = [None for i in range(len(pats))]

    for pi, p in enumerate(pats):
        while lctr < len(lines) and (lines[lctr][0:len(p)] != p):
            lctr += 1

        if lctr < len(lines):
            # Now they match
            res[pi] = lines[lctr]
            lctr += 1

    if any((x is None for x in res)):
        return None

    return res

    

for i in glob.glob("./droid_output/**/*.out", recursive=True):
    with open(i) as f:
        path = i.split('/')
        print(path)
        exp = path[-2]
        [trace, tool, adt, heur] = path[-1].split('.')[0].split('_')

        d = data[adt][heur][exp]

        d.num_checked += 1
        
        [exp, param] = exp.split('_')
        d.test = exp
        #d.param = int(param)
        r = f.readlines()
        res_strs = search_matches(r)
        if res_strs is None:
            continue
        q = list(map(lambda x: x.split(':')[1].rstrip(), res_strs))

        r = q[-1][1:] # Strip initial space
        result = True
        if r == 'false':
            result = False
        else:
            assert(r == 'true')

        
        # handlers, messages, events, parsing, pprocess, check, total, res
        [d.handlers, d.messages, d.events, parsing, pprocess, check, total] = list(map(lambda k: int(''.join(filter(str.isdigit, k))), q[0:-1:]))
        
        d.results.append(total)
        if result:
            d.num_ok += 1

if not os.path.exists("tables_droid"):
    os.mkdir("tables_droid")
if not os.path.exists("table_tex"):
    os.mkdir("table_tex")
if not os.path.exists("table_build"):
    os.mkdir("table_build")


for adt, data in data.items():
    for heur, data in data.items():
        with open(f"table_tex/{adt}_{heur}.tex", "w+") as f:
            f.write(preamble(adt, heur))
            for expt, res in data.items():
                f.write(line(res))
            f.write(postamble)

for tex in glob.glob("./table_tex/*.tex"):
    sp.call(["pdflatex", "-output-directory=table_build", tex])

for pdf in glob.glob("./table_build/*.pdf"):
    shutil.copy(pdf, f"tables_droid/{pdf.split('/')[-1]}")
