#!/usr/bin/env python3
import os, glob
from dataclasses import dataclass, field
from pathlib import Path
from collections import defaultdict as dd
from statistics import mean

@dataclass
class Res:
    results: list[int] = field(default_factory=lambda: [])
    ms: int = 0
    qu: int = 0
    st: int = 0
    rg: int = 0

data = dd(lambda: Res())

for i in glob.glob("./traces/**/*.out", recursive=True):
    with open(i) as f:
        exp = i.split('/')[-2]
        r = f.readlines()
        if r[5][-5:-1] == 'true':
            data[exp].ms += 1
        if r[6][-5:-1] == "true":
            data[exp].qu += 1
        if r[7][-5:-1] == "true":
            data[exp].st += 1
        if r[8][-5:-1] == "true":
            data[exp].rg += 1

        time = r[3][7:-3]

        data[exp].results.append(int(time))


print('''
\\documentclass{standalone}
\\begin{document}
''')
print("\\begin{tabular}{l l l | l | l l l l}")
print("Experiment & Size & Trace Count & Average runtime (ms)& Queue & Stack & Multiset & Register\\\\\\hline\\\\")
for k,rs in data.items():
    test = k[:-2]
    l = test.split('_')
    test = "-".join(l)
    n = k[-1:]
    print(f"{test} & {n} & {len(rs.results)} & {mean(rs.results) / 1000.0:.2f} & {rs.qu} & {rs.st} & {rs.ms} & {rs.rg}\\\\")
print("\\end{tabular}")
print("\\end{document}")
