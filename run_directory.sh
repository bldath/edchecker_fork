#!/bin/sh -x


heuristics=(no simple full)
adt=(multiset queue stack)


for h in "${heuristics[@]}"; do
    for adt in "${adt[@]}"; do
        #parallel "echo {}" ::: /home/grahnen/build/nidhugg/EDC-traces/*/*/*.trace
        parallel --timeout 120 --bar "./target/release/z3checker ${adt} ${h} {} > {.}_${adt}_${h}.out" ::: /home/grahnen/nidhugg/EDC-traces/*/*/*.trace
        

    done
done
