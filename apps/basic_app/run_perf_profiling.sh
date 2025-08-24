#!/bin/bash
set -e

OUTPUT_DIR=./dist/release
LOG_LEVEL=${1:-error}

OUTPUT_DIR=$OUTPUT_DIR \
PROFILING=1 \
roc build.roc

sudo env RUST_LOG="$LOG_LEVEL,calloop=error,naga=error,wgpu_core=error,wgpu_hal=error" \
perf record --delay 5000 --freq 1000 --call-graph dwarf $OUTPUT_DIR/basic_app run -c config/config.ron
# Note: If this errors with "could not read first record", install a better addr2line:
# cargo install --locked addr2line --features="bin"
sudo chown $USER "perf.data"
perf script -i "perf.data" > "profile.perf"
perf report --call-graph -M intel -i "perf.data"
