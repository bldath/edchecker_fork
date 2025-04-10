#!/bin/bash

function usage() {
    echo "Usage: $0 <nidhugg dir> <droid dir> [out dir] [timeout]"
}

nidhugg=${1:?$(usage)}
droidracer=${2:?$(usage)}
outdir=${3:benchmark_output}
timeout=${4:-60}

echo "Nidhugg traces: $nidhugg"
echo "DroidRacer traces: $droidracer"
echo "Output directory: $outdir"
echo "Timeout: $timeout s"

mkdir -p $outdir
mkdir -p $outdir/nidhugg
mkdir -p $outdir/droidracer

echo "Compiling release build"
cargo build --release || exit


echo "Running nidhugg benchmarks with timeout $timeout s"
./run_directory.sh $nidhugg $outdir/nidhugg $timeout

echo "Running droidracer benchmarks with timeout $timeout s"
./run_directory.sh $droidracer $outdir/droidracer $timeout

echo "Collecting results"
./target/release/collect_statistics $outdir/nidhugg $outdir/nidhugg $outdir/nidhugg.csv
./target/release/collect_statistics $outdir/droidracer $outdir/droidracer $outdir/droidracer.csv

echo "Done!"
echo "Results saved to $outdir/nidhugg.csv and $outdir/droidracer.csv"
echo "Remember to import the CSV files into a spreadsheet."