#!/usr/bin/env bash
shopt -s globstar

for f in $(ls ../droidracer-related-files/**/abc_log.txt1); do
    expt=$(echo $f | sed "s,\([^/]*/\)*\([^/]*\)/abc_log\.txt1,\2,g")
    echo $f $expt
    cargo run --release --bin convert_droidracer $f droid_traces
done
