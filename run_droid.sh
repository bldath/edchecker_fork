#!/usr/bin/env sh

#!/bin/bash

target=./droid_output
heuristics=(no simple full)
adt=(multiset queue stack)

for h in "${heuristics[@]}"; do
    for adt in "${adt[@]}"; do
        #parallel --bar "echo {.}" ::: /home/grahnen/nidhugg/EDC-traces/*/*/*.trace
        parallel --rpl '{expt} s:(\.*/[-_\w]+)*/([-_\w]+/[-_\w]+)\.trace:\2:;' \
                 --timeout 60 --bar \
                 "dirname {expt} | xargs -I% mkdir -p $target/%; ./target/release/z3checker ${adt} ${h} {} > $target/{expt}_z3_${adt}_${h}.out" \
                 ::: droid_traces/*/*.trace

        # parallel --rpl '{expt} s:(\.*/[-_\w]+)*/([-_\w]+/[-_\w]+)\.trace:\2:;' \
        #          --timeout 60 --bar \
        #          "dirname {expt} | xargs -I% mkdir -p $target/%; ./target/release/edchecker ${adt} ${h} {} > $target/{expt}_graph_${adt}_${h}.out" \
        #          ::: EDC-traces/*/*.trace
    done
done
