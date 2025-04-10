#!/bin/bash

function usage() {
    echo "Usage: $0 <trace dir> <output dir>"
}

echo "Compiling release build"
cargo build --release || exit

target=${1:?$(usage)}
output=${2:?$(usage)}

heuristics=(no simple full)
adt=(multiset queue stack)

echo "Running benchmarks"

for h in "${heuristics[@]}"; do
    for adt in "${adt[@]}"; do
        #parallel --bar "echo {.}" ::: /home/grahnen/nidhugg/EDC-traces/*/*/*.trace
        parallel --rpl '{expt} s:(\.*/*[-_\w]+)*/([-_\w]+/[-_\w]+)\.trace:\2:;' \
                 --timeout 60 --bar \
                 "dirname {expt} | xargs -I% mkdir -p $output/%; ./target/release/z3checker ${adt} ${h} {} > $output/{expt}_z3_${adt}_${h}.out" \
                 ::: $target/*/*.trace

        parallel --rpl '{expt} s:(\.*/*[-_\w]+)*/([-_\w]+/[-_\w]+)\.trace:\2:;' \
                 --timeout 60 --bar \
                 "dirname {expt} | xargs -I% mkdir -p $output/%; ./target/release/edchecker ${adt} ${h} {} > $output/{expt}_graph_${adt}_${h}.out" \
                 ::: $target/*/*.trace
    done
done
