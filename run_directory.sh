#!/bin/bash

target=./traces
heuristics=(no simple full)
adt=(multiset queue stack)

for h in "${heuristics[@]}"; do
    for adt in "${adt[@]}"; do
        #parallel --bar "echo {.}" ::: /home/grahnen/nidhugg/EDC-traces/*/*/*.trace
        parallel --rpl '{expt} s:(\.?/[-\w]+)*/([-\w]+/[-\w]+)\.\w+:\2:;' \
                 --timeout 120 --bar \
                 "dirname {expt} | xargs -I% mkdir -p $target/%; ./target/release/z3checker ${adt} ${h} {} > $target/{expt}_z3_${adt}_${h}.out" \
                 ::: EDC-traces/*/*.trace

        parallel --rpl '{expr} s:(\.?/[-\w]+)*/([-\w]+/[-\w]+)\.\w+:\2:;' \
                 --timeout 120 --bar \
                 "dirname {expt} | xargs -I% mkdir -p $target/%; ./target/release/edchecker ${adt} ${h} {} > $target/{expt}_graph_${adt}_${h}.out" \
                 ::: EDC-traces/*/*.trace
    done
done
