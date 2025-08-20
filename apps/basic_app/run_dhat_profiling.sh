#!/bin/bash
set -e

OUTPUT_DIR=./dist/release
LOG_LEVEL=${1:-info}

OUTPUT_DIR=$OUTPUT_DIR \
VALGRIND=1 \
ALLOCATOR=dhat \
roc build.roc

RUST_LOG="$LOG_LEVEL,calloop=error,naga=error,wgpu_core=error,wgpu_hal=error" \
RUST_BACKTRACE=1 \
valgrind --tool=dhat --num-callers=500 $OUTPUT_DIR/basic_app run -c config/config.ron
