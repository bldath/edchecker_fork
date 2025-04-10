#!/bin/bash

function usage() {
    echo "Usage: $0 <trace dir> <output dir>"
}

target=${1:?$(usage)}
output=${2:?$(usage)}
timeout=${3:-60}

heuristics=(no simple full)
adt=(multiset queue stack)

echo "Running benchmarks with timeout $timeout s"

for h in "${heuristics[@]}"; do
    for adt in "${adt[@]}"; do
        #parallel --bar "echo {.}" ::: /home/grahnen/nidhugg/EDC-traces/*/*/*.trace
        parallel --rpl '{expt} s:(\.*/*[-_\w]+)*/([-_\w]+/[-_\w]+)\.\w+:\2:;' \
                 --timeout $timeout --bar \
                 "dirname {expt} | xargs -I% mkdir -p $output/%; ./target/release/z3checker ${adt} ${h} {} > $output/{expt}_z3_${adt}_${h}.out" \
                 ::: $target/*/*.json

        parallel --rpl '{expt} s:(\.*/*[-_\w]+)*/([-_\w]+/[-_\w]+)\.\w+:\2:;' \
                 --timeout $timeout --bar \
                 "dirname {expt} | xargs -I% mkdir -p $output/%; ./target/release/edchecker ${adt} ${h} {} > $output/{expt}_graph_${adt}_${h}.out" \
                 ::: $target/*/*.json
    done
done
