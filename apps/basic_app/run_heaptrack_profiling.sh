#!/bin/bash
set -e

OUTPUT_DIR=./dist/release
LOG_LEVEL=${1:-info}

OUTPUT_DIR=$OUTPUT_DIR \
HEAP_PROFILING=1 \
roc build.roc

RUST_LOG="$LOG_LEVEL,calloop=error,naga=error,wgpu_core=error,wgpu_hal=error" \
RUST_BACKTRACE=1 \
heaptrack $OUTPUT_DIR/basic_app run -c config/config.ron
