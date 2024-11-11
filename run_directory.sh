#!/usr/bin/env sh

for i in ../nidhugg/benchmarks/event-driven/**/*.trace; do
    echo $(dirname $i)
    #cargo run $i
done
