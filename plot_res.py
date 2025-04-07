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

for i in glob.glob("./traces/**/*.out", recursive=True):
    with open(i) as f:
        path = i.split('/')
        exp = path[-2]
        [trace, adt, heur] = path[-1].split('.')[0].split('_')
        
        d = data[adt][heur][exp]

        d.num_checked += 1
        
        [exp, param] = exp.split('_')
        d.test = exp
        d.param = int(param)
        r = f.readlines()
        
        q = list(map(lambda x: x.split(':')[1].rstrip(), r))
        if len(q) < 8:
            continue
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

if not os.path.exists("tables"):
    os.mkdir("tables")
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
    shutil.copy(pdf, f"tables/{pdf.split('/')[-1]}")
