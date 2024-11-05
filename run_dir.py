#!/usr/bin/env python3
import os, glob
from pathlib import Path
import subprocess as sp

for i in glob.glob("../nidhugg/benchmarks/event-driven/**/*.trace", recursive=True):
    p = Path(i)
    print(p.parent)
    sp.run(["./target/debug/EDConsistency", p])
