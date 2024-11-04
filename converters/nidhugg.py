#!/usr/bin/env python3

from example_nidhugg import example
import re

ev_regex = re.compile(r"\((<.*>),(\d+)\)\s*(.*)\s*SLP")

for ev in example.split("\n"):
    m = ev_regex.match(ev)
    if m:
        print(m[1], m[2], m[3])
