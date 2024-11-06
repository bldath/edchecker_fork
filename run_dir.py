#!/usr/bin/env python3
import os, glob
from pathlib import Path
import subprocess as sp
from collections import defaultdict as dd

results = dd(lambda: [])

sp.run(["cargo", "build", "--release"])

for i in glob.glob("../nidhugg/traces/**/*.trace", recursive=True):
    p = Path(i)
    print(p.parent)
    with open(str(p) + ".out", "w") as f:
        sp.run(["./target/release/EDConsistency", p], stdout=f)
