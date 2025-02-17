#!/usr/bin/env python3
import os, glob
from collections import defaultdict as dd
from dataclasses import dataclass, field
from pprint import pprint
from statistics import mean

@dataclass
class Res:
    times: list[int] = field(default_factory=lambda: [])
    num_ok: int = 0
    completed: int = 0
    timeout: int = 0

    def __repr__(self):
        ok = f"{self.num_ok}" if self.completed > 0 else "-"
        num_inc = self.completed - self.num_ok
        inc = f"{num_inc}" if num_inc > 0 else "-"
        to = f"{self.timeout}" if self.timeout > 0 else "-"
        mn = f"{(mean(self.times) / 1000000.0):.2f}" if len(self.times) > 0 else "-"

        return f"{ok} & {inc} & {to} & {mn}"

expts= dd(lambda: dd(lambda: dd(lambda: dd(lambda: Res()))))

for i in glob.glob("../nidhugg/EDC-traces/**/*-*-*.out", recursive=True):
    splits = i.split('/')
    t = splits[-2]
    [test, param] = t.split('_')
    filename = splits[-1].split('.')[0]
    data = filename.split('-')

    with open(i) as f:
        r = f.readlines()
        if len(r) < 7:
            expts[test][param][data[1]][data[2]].timeout += 1
            continue

        expts[test][param][data[1]][data[2]].completed += 1
        expts[test][param][data[1]][data[2]].times.append(int(r[3][7:-3]))
        if r[4][-5:-1] == 'true':
            expts[test][param][data[1]][data[2]].num_ok += 1

# expts['experiment_name']['structure']['heuristic']


def getres(ex, adt):
    mres = dd(lambda: dd(lambda: []))
    s = ""
    for test, q in expts.items():
        w = list(q.items())
        w.sort()
        for param, rs in w:
            res = rs[adt]
            if res['full'].completed + res['simple'].completed + res['no'].completed == 0:
                continue
            s += f"{test}({param}) & {res['full']} & {res['simple']} & {res['no']}\\\\"
    return s

q = getres(expts, "queue")
s = getres(expts, "stack")
m = getres(expts, "multiset")


def mktable(expts, adt):
    res = getres(expts, adt)
    s = '''
    \\documentclass{standalone}
    \\begin{document}
    \\begin{tabular}{| l | c | c | c | c | c | c | c | c | c | c | c | c |}
    Benchmark & \\multicolumn{12}{c|}{''' + adt + '''}\\\\
    (parameter) & \\multicolumn{4}{c|}{All Heuristics} & \\multicolumn{4}{c|}{Only Heuristic 1} & \\multicolumn{4}{c|}{No Heuristics}\\\\
    & \#OK & \#Err & \#T/O & Time(s) & \#OK & \#Err & \#T/O & Time(s) & \#OK & \#Err & \#T/O & Time(s) \\\\
    \\hline
    '''
    s += res
    s += '''
    \\end{tabular}
    \\end{document}
    '''

    return s

for adt in ["multiset", "queue", "stack"]:
    with open(f"{adt}.tex", "w") as f:
        f.write(mktable(expts, adt))

# print('''
# \\documentclass{standalone}
# \\begin{document}
# \\begin{table}
# \\begin{tabular}{| l | c | c | c | c | c | c | c | c | c | c | c | c | c | c | c | c | c | c | c |}
# Benchmark & \\multicolumn{6}{c|}{Multiset} & \\multicolumn{6}{c|}{Queue} & \\multicolumn{6}{c|}{Stack} \\\\
# (parameter) & \\multicolumn{2}{c|}{Full} & \\multicolumn{2}{c|}{Simple} & \\multicolumn{2}{c|}{No} & \\multicolumn{2}{c|}{Full} & \\multicolumn{2}{c|}{Simple} & \\multicolumn{2}{c|}{No} & \\multicolumn{2}{c|}{Full} & \\multicolumn{2}{c|}{Simple} & \\multicolumn{2}{c|}{No}\\\\
# & OK/Completed & Time(s) & OK/Completed & Time(s)& OK/Completed & Time(s)& OK/Completed & Time(s)& OK/Completed & Time(s)& OK/Completed & Time(s)& OK/Completed & Time(s)& OK/Completed & Time(s)& OK/Completed & Time(s) \\\\
# \\hline
# ''')

# for test, q in expts.items():
#     for param, rs in q.items():
#         infos = " & ".join([" & ".join([str(q['full']), str(q['simple']), str(q['no'])]) for q in [rs['multiset'], rs['queue'], rs['stack']]])
#         print(f"{test} ({param}) & {infos}\\\\")

# print('''
# \\hline
# \\end{tabular}
# \\label{tab:experiment_results}
# \\end{table}
# \\end{document}
# ''')
